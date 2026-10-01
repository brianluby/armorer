//! Read-only configuration validation and Cargo workspace planning.
//!
//! Discovery inspects a temporary manifest snapshot using installed Cargo;
//! it does not compile repository code, resolve the dependency graph, or install tools.
//!
//! ```
//! use armorer::config::repository_name;
//! assert!(repository_name("example/my-project"));
//! assert!(!repository_name("https://example.invalid/project"));
//! ```

pub mod config;
pub mod discovery;
pub mod plan;

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
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io(_) => "io",
            Self::Toml => "invalid-toml",
            Self::Invalid(_) => "invalid-config",
            Self::Path(_) => "unsafe-path",
            Self::Cargo(_) => "cargo-discovery",
            Self::Metadata => "cargo-metadata",
            Self::Json => "json",
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Relative configuration paths never allow parent/absolute components or symlinks.
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

pub(crate) fn read_small(path: &Path) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(1_048_577)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 1_048_576 {
        return Err(Error::Invalid("input exceeds 1 MiB".into()));
    }
    Ok(bytes)
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
