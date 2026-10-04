//! Owner-only staging primitives. Unsupported platforms fail before creating anything.
use std::{
    fs::{File, OpenOptions},
    io,
    path::Path,
};

/// Request private directory permissions in the creation syscall, including under a permissive umask.
pub(crate) fn private_tempdir(
    prefix: &str,
    parent: Option<&Path>,
) -> io::Result<tempfile::TempDir> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut builder = tempfile::Builder::new();
        builder
            .prefix(prefix)
            .permissions(std::fs::Permissions::from_mode(0o700));
        match parent {
            Some(parent) => builder.tempdir_in(parent),
            None => builder.tempdir(),
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (prefix, parent);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "private staging requires a supported Unix host",
        ))
    }
}

/// Create one new owner-only leaf without replacing an existing file or symlink.
pub(crate) fn private_file(path: &Path) -> io::Result<File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(path)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "private staging requires a supported Unix host",
        ))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    /// Directory and leaf modes are private without later chmod and creation never clobbers another inode.
    fn private_creation_and_no_clobber() {
        let directory = private_tempdir("armorer-private-test-", None).unwrap();
        assert_eq!(
            directory.path().metadata().unwrap().permissions().mode() & 0o777,
            0o700
        );
        let path = directory.path().join("snapshot");
        let file = private_file(&path).unwrap();
        assert_eq!(file.metadata().unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(
            private_file(&path).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        let linked = directory.path().join("link");
        std::os::unix::fs::symlink(&path, &linked).unwrap();
        assert_eq!(
            private_file(&linked).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        let nested = private_tempdir("request-", Some(directory.path())).unwrap();
        assert_eq!(
            nested.path().metadata().unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
}
