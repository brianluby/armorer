//! The journaled write/restore mechanism shared by every provisioning contract.
//! Callers own approval validation, locking, journal formats and commit/cleanup
//! error classification; this module owns the mechanical file transitions.
use crate::{
    Error, Result,
    filesystem::{optional_bytes, replace, sync_directory},
    safe_path,
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

/// One approved file transition. `None` before means absent; `None` after means
/// deletion. Serde encodes `Some` exactly like a plain field, so journals written
/// by either contract keep their existing bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Write {
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// Compare current optional file bytes with an approved image without adopting a mismatch.
pub(crate) fn matches(current: Option<&[u8]>, image: Option<&str>) -> bool {
    current == image.map(str::as_bytes)
}

/// Create only required parents, sync each creation, and recheck for symlinks.
/// Empty created directories are retained on rollback; unrelated directories are never removed.
pub(crate) fn ensure_parent(root: &Path, relative: &str) -> Result<()> {
    let parent = Path::new(relative)
        .parent()
        .ok_or(Error::Transaction("invalid bootstrap parent"))?;
    let mut current = std::path::PathBuf::new();
    for component in parent.components() {
        current.push(component);
        let relative = current
            .to_str()
            .ok_or(Error::Transaction("invalid bootstrap path"))?;
        let directory = safe_path(root, relative)?;
        match fs::create_dir(&directory) {
            Ok(()) => sync_directory(
                directory
                    .parent()
                    .ok_or(Error::Transaction("invalid directory parent"))?,
            )?,
            Err(error)
                if error.kind() == std::io::ErrorKind::AlreadyExists
                    && fs::symlink_metadata(&directory)?.is_dir() => {}
            Err(error) => return Err(error.into()),
        }
    }
    safe_path(root, relative)?;
    Ok(())
}

/// Replace or delete one approved image, preserving mode and syncing the parent.
pub(crate) fn write_image(
    root: &Path,
    path: &str,
    image: Option<&str>,
    parent_msg: &'static str,
) -> Result<()> {
    let current = optional_bytes(root, path)?;
    match image {
        Some(image) => {
            ensure_parent(root, path)?;
            replace(root, path, image.as_bytes(), current.is_none())
        }
        None => {
            if current.is_some() {
                let path = safe_path(root, path)?;
                fs::remove_file(&path)?;
                sync_directory(path.parent().ok_or(Error::Transaction(parent_msg))?)?;
            }
            Ok(())
        }
    }
}

/// Apply the persisted journal's forward writes, refusing any target that moved
/// off its approved before image. `after_write` is the fault-injection seam.
pub(crate) fn apply_writes(
    root: &Path,
    operations: &[Write],
    target_changed: &'static str,
    parent_msg: &'static str,
    mut after_write: impl FnMut(usize) -> Result<()>,
) -> Result<()> {
    for (index, w) in operations.iter().enumerate() {
        if !matches(
            optional_bytes(root, &w.path)?.as_deref(),
            w.before.as_deref(),
        ) {
            return Err(Error::Transaction(target_changed));
        }
        write_image(root, &w.path, w.after.as_deref(), parent_msg)?;
        after_write(index)?;
    }
    Ok(())
}

/// Reverse-restore prior images after the caller has prechecked intervening
/// edits. Files already at their approved before image are left untouched.
pub(crate) fn restore(root: &Path, operations: &[Write], parent_msg: &'static str) -> Result<()> {
    for w in operations.iter().rev() {
        if matches(
            optional_bytes(root, &w.path)?.as_deref(),
            w.before.as_deref(),
        ) {
            continue;
        }
        write_image(root, &w.path, w.before.as_deref(), parent_msg)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The engine's Write must serialize exactly like each contract's pre-refactor
    /// struct: bootstrap's plain `after: String` and upgrade's optional deletion.
    /// Journals on disk and recovery digests bind these bytes.
    #[test]
    fn write_wire_bytes_match_the_pre_refactor_journals() {
        let w = Write {
            path: "rust-toolchain.toml".into(),
            before: None,
            after: Some("[toolchain]\n".into()),
        };
        assert_eq!(
            crate::filesystem::json(&w).unwrap(),
            b"{\n  \"path\": \"rust-toolchain.toml\",\n  \"before\": null,\n  \"after\": \"[toolchain]\\n\"\n}"
        );
        let deletion = Write {
            path: ".armorer/bootstrap-state-v2.json".into(),
            before: Some("{}".into()),
            after: None,
        };
        assert_eq!(
            crate::filesystem::json(&deletion).unwrap(),
            b"{\n  \"path\": \".armorer/bootstrap-state-v2.json\",\n  \"before\": \"{}\",\n  \"after\": null\n}"
        );
    }
}
