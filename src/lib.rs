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

pub mod apply;
pub mod bootstrap;
pub mod catalog;
pub mod ci_policy;
pub mod config;
pub mod discovery;
pub mod plan;
pub mod trust;
pub mod upgrade;

use std::path::{Component, Path, PathBuf};

/// Stable machine-readable failures. Values from configuration are not echoed.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot read local input: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid TOML input (values omitted)")]
    Toml,
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
}

impl Error {
    /// Return the stable category code used in CLI JSON errors.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Toml => "invalid-toml",
            Self::Invalid(_) => "invalid-config",
            Self::Path(_) => "unsafe-path",
            Self::Cargo(_) => "cargo-discovery",
            Self::Metadata => "cargo-metadata",
            Self::Json => "json",
            Self::Transaction(_) => "transaction",
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

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
    let path = Path::new(relative);
    if relative.is_empty()
        || relative.contains('\\')
        || relative.chars().any(char::is_control)
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(Error::Path("expected a plain relative path".into()));
    }
    let mut joined = root.to_path_buf();
    for component in path.components() {
        joined.push(component);
        match std::fs::symlink_metadata(&joined) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(Error::Path("symlink input or parent".into()));
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
    use std::io::Read;
    if !std::fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(Error::Path("expected a regular input file".into()));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(1_048_577)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 1_048_576 {
        return Err(Error::Invalid("input exceeds 1 MiB".into()));
    }
    Ok(bytes)
}

/// Return the SHA-256 digest of the exact bytes as lowercase hexadecimal.
pub(crate) fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
