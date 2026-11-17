//! Bounded, symlink-refusing transaction I/O shared by every provisioning path.
//! Owner-only staging primitives. Unsupported platforms fail before creating anything.

use crate::{Error, Result, read_small, safe_path};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::{self, Write},
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
        use std::fs::OpenOptions;
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

    /// Retain a duplicate descriptor while requiring immediate post-transaction reacquisition.
    #[test]
    fn transaction_lock_releases_before_duplicate_descriptors_close() {
        let root = tempfile::tempdir().unwrap();
        let guard = lock(root.path()).unwrap();
        let inherited = guard.0.try_clone().unwrap();
        assert!(matches!(
            lock(root.path()),
            Err(Error::Transaction("another Armorer transaction is active"))
        ));
        drop(guard);
        let next = lock(root.path()).unwrap();
        drop(inherited);
        // Closing the previous duplicate must not release the new independent lock.
        assert!(matches!(
            lock(root.path()),
            Err(Error::Transaction("another Armorer transaction is active"))
        ));
        drop(next);
        lock(root.path()).unwrap();
    }

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
        // Dangling target: O_NOFOLLOW must reject the link itself, not report target-exists.
        std::os::unix::fs::symlink(directory.path().join("missing-target"), &linked).unwrap();
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

/// Read a regular optional input while preserving absence and propagating unsafe-file failures.
pub(crate) fn optional_bytes(root: &Path, relative: &str) -> Result<Option<Vec<u8>>> {
    let path = safe_path(root, relative)?;
    match fs::symlink_metadata(&path) {
        Ok(_) => Ok(Some(read_small(&path)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Encode the record as stable JSON bytes for persistence and identity checks.
pub(crate) fn json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| Error::Json)?;
    if bytes.len() > 1_048_576 {
        return Err(Error::Transaction("transaction exceeds 1 MiB"));
    }
    Ok(bytes)
}

/// Convert exact record bytes to UTF-8, rejecting unsupported text.
pub(crate) fn text(bytes: &[u8]) -> Result<String> {
    String::from_utf8(bytes.to_vec()).map_err(|_| Error::Transaction("managed input must be UTF-8"))
}

// serde's Value/BTreeMap parsers otherwise accept repeated object keys. Strict
// parsing also checks nested metadata, whose Cargo-facing structs tolerate extras.
struct StrictValue(serde_json::Value);

impl<'de> Deserialize<'de> for StrictValue {
    /// Use the strict JSON visitor so duplicate fields and unsupported values cannot conceal record content.
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = StrictValue;
            /// Describe the strict JSON value shape to the deserializer without echoing input values.
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("JSON without duplicate keys")
            }
            /// Retain a JSON boolean without numeric coercion.
            fn visit_bool<E: serde::de::Error>(
                self,
                value: bool,
            ) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            /// Retain a signed integer as a JSON number.
            fn visit_i64<E: serde::de::Error>(
                self,
                value: i64,
            ) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            /// Retain an unsigned integer as a JSON number.
            fn visit_u64<E: serde::de::Error>(
                self,
                value: u64,
            ) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            /// Reject nonfinite floating-point values before constructing a JSON number.
            fn visit_f64<E: serde::de::Error>(
                self,
                value: f64,
            ) -> std::result::Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(|number| StrictValue(number.into()))
                    .ok_or_else(|| E::custom("invalid JSON number"))
            }
            /// Copy a borrowed JSON string into the strict value tree.
            fn visit_str<E: serde::de::Error>(
                self,
                value: &str,
            ) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            /// Retain an owned JSON string without interpreting it as another type.
            fn visit_string<E: serde::de::Error>(
                self,
                value: String,
            ) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            /// Represent an absent optional JSON value as null.
            fn visit_none<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(StrictValue(serde_json::Value::Null))
            }
            /// Represent a JSON unit value as null.
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
                self.visit_none()
            }
            /// Recursively deserialize each array member through the same strict value reader.
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(StrictValue(value)) = sequence.next_element()? {
                    values.push(value);
                }
                Ok(StrictValue(values.into()))
            }
            /// Reject duplicate object keys while recursively reading every value.
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, StrictValue(value))) =
                    map.next_entry::<String, StrictValue>()?
                {
                    if values.insert(key, value).is_some() {
                        return Err(serde::de::Error::custom("duplicate JSON key"));
                    }
                }
                Ok(StrictValue(values.into()))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

/// Parse the bounded JSON record through the strict duplicate-key-rejecting reader.
pub(crate) fn decode<T: serde::de::DeserializeOwned + Serialize>(bytes: &[u8]) -> Result<T> {
    let StrictValue(value) = serde_json::from_slice(bytes)
        .map_err(|_| Error::Transaction("invalid transaction JSON"))?;
    let typed: T = serde_json::from_value(value.clone())
        .map_err(|_| Error::Transaction("invalid transaction JSON"))?;
    if serde_json::to_value(&typed).map_err(|_| Error::Json)? != value {
        return Err(Error::Transaction(
            "unknown or missing transaction JSON fields",
        ));
    }
    Ok(typed)
}

/// Own a transaction lock and release it before closing its file descriptor.
/// A concurrent spawn can briefly inherit a duplicate descriptor; closing only
/// the parent descriptor can leave a completed transaction's flock active.
pub(crate) struct TransactionLock(File);

impl Drop for TransactionLock {
    /// End the transaction's lock even while a duplicate descriptor remains open.
    fn drop(&mut self) {
        // Close is NOT a fallback while a child holds a duplicate descriptor; retry EINTR.
        while let Err(error) = self.0.unlock() {
            if error.kind() != std::io::ErrorKind::Interrupted {
                break;
            }
        }
    }
}

/// Acquire the persistent kernel lock shared by Armorer transaction versions.
pub(crate) fn lock(root: &Path) -> Result<TransactionLock> {
    let directory = safe_path(root, ".armorer")?;
    match fs::create_dir(&directory) {
        Ok(()) => sync_directory(root)?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && directory.is_dir() => {}
        Err(error) => return Err(error.into()),
    }
    let path = safe_path(root, ".armorer/apply.lock")?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    // Validate the opened handle; a path re-stat would race a symlink swap.
    if !file.metadata()?.is_file() {
        return Err(Error::Transaction("apply lock must be a regular file"));
    }
    match file.try_lock() {
        Ok(()) => Ok(TransactionLock(file)),
        Err(TryLockError::WouldBlock) => {
            Err(Error::Transaction("another Armorer transaction is active"))
        }
        Err(TryLockError::Error(error)) => Err(error.into()),
    }
}

/// Sync parent metadata so completed file replacements have a durable directory entry.
pub(crate) fn sync_directory(path: &Path) -> Result<()> {
    // Initial mutation support is Unix, matching the supported host platforms.
    #[cfg(unix)]
    File::open(path)?.sync_all()?;
    #[cfg(not(unix))]
    {
        let _ = path;
        return Err(Error::Transaction(
            "durable transactions require a supported Unix host",
        ));
    }
    Ok(())
}

/// Write and sync a temporary regular file, then atomically replace the approved destination.
pub(crate) fn replace(root: &Path, relative: &str, bytes: &[u8], absent: bool) -> Result<()> {
    let destination = safe_path(root, relative)?;
    let parent = destination
        .parent()
        .ok_or(Error::Transaction("invalid destination"))?;
    let mut staging = tempfile::NamedTempFile::new_in(parent)?;
    staging.write_all(bytes)?;
    if !absent {
        staging
            .as_file()
            .set_permissions(fs::metadata(&destination)?.permissions())?;
    }
    staging.as_file().sync_all()?;
    if absent {
        staging
            .persist_noclobber(&destination)
            .map_err(|error| error.error)?;
    } else {
        staging.persist(&destination).map_err(|error| error.error)?;
    }
    sync_directory(parent)
}
