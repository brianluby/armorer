//! The journaled write/restore mechanism shared by every provisioning contract.
//! Callers own approval validation, locking, journal formats and commit/cleanup
//! error classification; this module owns the mechanical file transitions.
use crate::{
    Error, Result,
    filesystem::{ensure_parent, optional_bytes, remove, replace},
};
use serde::{Deserialize, Serialize};
use std::path::Path;

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

/// Replace or delete one approved image, preserving mode and syncing the parent.
/// Parents are created and files removed only through held directory handles.
pub(crate) fn write_image(
    root: &Path,
    path: &str,
    image: Option<&str>,
    absent: bool,
    parent_msg: &'static str,
) -> Result<()> {
    match image {
        Some(image) => {
            ensure_parent(root, path)?;
            // No-clobber binds the caller's approved absence, never a fresh read:
            // a file created after the preimage check must fail, not be overwritten.
            replace(root, path, image.as_bytes(), absent)
        }
        None => {
            if optional_bytes(root, path)?.is_some() {
                remove(root, path, parent_msg)?.sync()?;
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
        write_image(
            root,
            &w.path,
            w.after.as_deref(),
            w.before.is_none(),
            parent_msg,
        )?;
        after_write(index)?;
    }
    Ok(())
}

/// Reverse-restore prior images after the caller has prechecked intervening
/// edits. Files already at their approved before image are left untouched.
pub(crate) fn restore(root: &Path, operations: &[Write], parent_msg: &'static str) -> Result<()> {
    for w in operations.iter().rev() {
        let current = optional_bytes(root, &w.path)?;
        if matches(current.as_deref(), w.before.as_deref()) {
            continue;
        }
        write_image(
            root,
            &w.path,
            w.before.as_deref(),
            current.is_none(),
            parent_msg,
        )?;
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

#[cfg(test)]
mod absence_tests {
    use super::*;

    /// Approved absence pins no-clobber: a file created after the preimage
    /// check must fail the write, never be overwritten.
    #[test]
    fn approved_absent_write_refuses_an_intervening_file() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("target"), "intervening").unwrap();
        assert!(
            write_image(
                root.path(),
                "target",
                Some("approved"),
                true,
                "invalid parent"
            )
            .is_err()
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("target")).unwrap(),
            "intervening"
        );
    }
}
