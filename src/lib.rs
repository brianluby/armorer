//! Rust configuration validation, read-only planning and approved local transactions.
//!
//! Discovery inspects a temporary manifest snapshot using installed Cargo;
//! it does not compile repository code, resolve the dependency graph, or install tools.
//!
//! ```
//! use armorer::config::repository_name;
//! assert!(repository_name("example/my-project"));
//! assert!(!repository_name("https://example.invalid/project"));
//! ```

pub mod bootstrap;
pub mod catalog;
pub mod ci_policy;
pub mod config;
pub mod discovery;
mod filesystem;
pub mod init;
pub mod plan;
mod runtime;
mod schema_bounds;
mod transaction;
pub mod trust;
pub mod upgrade;
pub mod verification;
// Kept last and out of alphabetical order so it never collides with a new module line.
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub mod fuzzing;

use std::path::{Component, Path, PathBuf};

/// Stable machine-readable failures. Values from configuration are not echoed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("local filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid TOML input (values omitted)")]
    Toml,
    #[error("invalid TOML input at byte offset {offset} (values omitted)")]
    TomlAt { offset: usize },
    #[error("invalid configuration: {0}")]
    Invalid(String),
    #[error("unsafe or unsupported repository path: {0}")]
    Path(String),
    #[error("Cargo discovery failed at {0}; no repository code was built")]
    Cargo(&'static str),
    #[error("unsupported Cargo metadata response")]
    Metadata,
    #[error("JSON serialization failed")]
    Json,
    #[error("transaction blocked: {0}")]
    Transaction(&'static str),
    #[error("transaction blocked: {message}; original failure: {cause}")]
    TransactionCause {
        message: &'static str,
        #[source]
        cause: Box<Error>,
    },
    #[error(
        "transaction blocked: {message}; original failure: {cause}; rollback failure: {rollback}"
    )]
    TransactionRollback {
        message: &'static str,
        #[source]
        cause: Box<Error>,
        rollback: Box<Error>,
    },
}

impl Error {
    /// Return the stable category code used in CLI JSON errors.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Toml | Self::TomlAt { .. } => "invalid-toml",
            Self::Invalid(_) => "invalid-config",
            Self::Path(_) => "unsafe-path",
            Self::Cargo(_) => "cargo-discovery",
            Self::Metadata => "cargo-metadata",
            Self::Json => "json",
            Self::Transaction(_)
            | Self::TransactionCause { .. }
            | Self::TransactionRollback { .. } => "transaction",
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Refusal for a symlink at any component below a repository root.
pub(crate) const SYMLINK_COMPONENT: &str = "symlink input or parent";
/// Refusal for a managed input that is not a regular file.
pub(crate) const REGULAR_INPUT: &str = "expected a regular input file";
/// Refusal when the opened inode differs from the one inspected before opening.
#[cfg(unix)]
pub(crate) const INPUT_CHANGED: &str = "input changed during read";

/// Split a relative repository path into its plain components.
///
/// This is the only lexical gate for managed paths: [`safe_path`] and the
/// directory-handle walk used for managed reads and writes both call it, so
/// they cannot disagree about which inputs are refused or how `a//b`, `a/./b`
/// and trailing separators normalize.
///
/// # Errors
/// Returns `Error::Path` for an empty path, backslashes, control characters or
/// any non-normal component (root, prefix, `.` or `..`).
pub(crate) fn plain_components(relative: &str) -> Result<Vec<&std::ffi::OsStr>> {
    let refuse = || Error::Path("expected a plain relative path".into());
    if relative.is_empty() || relative.contains('\\') || relative.chars().any(char::is_control) {
        return Err(refuse());
    }
    let components = Path::new(relative)
        .components()
        .map(|component| match component {
            Component::Normal(name) => Ok(name),
            _ => Err(refuse()),
        })
        .collect::<Result<Vec<_>>>()?;
    if components.is_empty() {
        return Err(refuse());
    }
    Ok(components)
}

/// Relative configuration paths never allow parent/absolute components or symlinks.
///
/// Returns the path joined to `root`, allowing missing components. The supplied
/// `root` itself is not checked for symlinks.
///
/// # Errors
/// Returns `Error::Path` for an empty path, backslashes, control characters,
/// non-normal path components, or symlinks below `root`. Filesystem inspection
/// errors other than not-found propagate as `Error::Io`.
pub fn safe_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let mut joined = root.to_path_buf();
    for component in plain_components(relative)? {
        joined.push(component);
        match std::fs::symlink_metadata(&joined) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(Error::Path(SYMLINK_COMPONENT.into()));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(joined)
}

/// Read a regular file containing at most 1 MiB (1,048,576 bytes).
///
/// Returns `Error::Invalid` if the input exceeds that limit and propagates file
/// open and read failures as `Error::Io`. Rejects special files and symlinks
/// with `Error::Path` before opening; inspection assumes a stable filesystem.
pub(crate) fn read_small(path: &Path) -> Result<Vec<u8>> {
    // Pre-stat rejects FIFOs; the open itself is nonblocking and refuses symlinks so a
    // FIFO/symlink swapped in after the stat cannot block or redirect the read. The
    // opened inode is then bound to the stat.
    let before = std::fs::symlink_metadata(path)?;
    if !before.file_type().is_file() {
        return Err(Error::Path(REGULAR_INPUT.into()));
    }
    #[cfg(unix)]
    let file = {
        use rustix::fs::{Mode, OFlags};
        let fd = rustix::fs::open(
            path,
            OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(std::io::Error::from)?;
        std::fs::File::from(fd)
    };
    #[cfg(not(unix))]
    let file = std::fs::File::open(path)?;
    let opened = file.metadata()?;
    if !opened.is_file() {
        return Err(Error::Path(REGULAR_INPUT.into()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.dev() != opened.dev() || before.ino() != opened.ino() {
            return Err(Error::Path(INPUT_CHANGED.into()));
        }
    }
    read_limited(file)
}

/// Read an already opened and validated input, refusing more than 1 MiB.
pub(crate) fn read_limited(file: std::fs::File) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut bytes = Vec::new();
    file.take(1_048_577).read_to_end(&mut bytes)?;
    if bytes.len() > 1_048_576 {
        return Err(Error::Invalid("input exceeds 1 MiB".into()));
    }
    Ok(bytes)
}

/// Return the SHA-256 digest of the exact bytes as lowercase hexadecimal.
pub(crate) fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex_lower(&Sha256::digest(bytes)[..])
}

/// Lowercase hexadecimal encoding of `bytes`, independent of digest-type formatting traits.
pub(crate) fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

#[cfg(test)]
mod tests {
    /// Known SHA-256 vectors pin `digest` so a hashing-API change cannot alter output.
    #[test]
    fn digest_matches_known_sha256_vectors() {
        assert_eq!(
            super::digest(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            super::digest(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    /// Hex encoding is lowercase and zero-padded per byte.
    #[test]
    fn hex_lower_encodes_bytes() {
        assert_eq!(super::hex_lower(&[]), "");
        assert_eq!(super::hex_lower(&[0x00, 0x0f, 0xa0, 0xff]), "000fa0ff");
    }

    #[cfg(unix)]
    #[test]
    /// The read guard rejects symlinks, directories and oversized inputs before returning bytes.
    fn read_small_rejects_symlink_directory_and_oversize() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file");
        std::fs::write(&file, b"ok").unwrap();
        assert_eq!(super::read_small(&file).unwrap(), b"ok");
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        assert!(matches!(
            super::read_small(&link),
            Err(super::Error::Path(_))
        ));
        assert!(matches!(
            super::read_small(dir.path()),
            Err(super::Error::Path(_))
        ));
        std::fs::write(&file, vec![b'x'; 1_048_577]).unwrap();
        assert!(matches!(
            super::read_small(&file),
            Err(super::Error::Invalid(_))
        ));
    }
}
