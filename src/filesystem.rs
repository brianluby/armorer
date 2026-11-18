//! Bounded, symlink-refusing transaction I/O shared by every provisioning path.
//! Owner-only staging primitives. Unsupported platforms fail before creating anything.
//!
//! On Unix every managed read and write below a repository root goes through
//! directory handles: the root is opened once, each intermediate component is
//! opened relative to its parent with `O_DIRECTORY | O_NOFOLLOW`, and the leaf
//! is opened, created or renamed relative to the held parent handle. A symlink
//! at any component is refused and never followed, so a component swapped after
//! validation cannot redirect the operation away from the directory inode that
//! was walked.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, TryLockError},
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

    use super::handles::{self, STAGING_PREFIX};
    use std::{
        ffi::OsStr,
        fs,
        os::unix::fs::{MetadataExt, symlink},
    };

    /// Sorted entry names of one directory, without following anything.
    fn listing(path: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    /// Assert the shared symlink refusal, not an incidental I/O failure.
    #[track_caller]
    fn assert_symlink_refusal<T: std::fmt::Debug>(result: Result<T>) {
        assert!(
            matches!(&result, Err(Error::Path(message)) if message == crate::SYMLINK_COMPONENT),
            "{result:?}"
        );
    }

    /// Assert that no staging entry survived in a directory.
    #[track_caller]
    fn assert_no_staging(path: &Path) {
        let names = listing(path);
        assert!(
            names.iter().all(|name| !name.starts_with(STAGING_PREFIX)),
            "{names:?}"
        );
    }

    /// Record bytes, mode and modification time so any outside change is visible.
    fn fingerprint(path: &Path) -> (Vec<u8>, u32, i64, i64) {
        let metadata = fs::symlink_metadata(path).unwrap();
        (
            fs::read(path).unwrap(),
            metadata.mode(),
            metadata.mtime(),
            metadata.mtime_nsec(),
        )
    }

    /// A lock symlink to an outside regular file is refused before that file is created, opened or locked.
    #[test]
    fn lock_refuses_symlinked_lock_file_outside_root() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("target");
        fs::write(&target, b"outside bytes").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
        let before = fingerprint(&target);
        fs::create_dir(root.path().join(".armorer")).unwrap();
        let link = root.path().join(".armorer/apply.lock");
        symlink(&target, &link).unwrap();
        assert_symlink_refusal(lock(root.path()).map(|_| ()));
        assert_eq!(fingerprint(&target), before);
        assert_eq!(listing(outside.path()), ["target"]);
        // Nothing holds a lock on the outside file.
        File::open(&target).unwrap().try_lock().unwrap();
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );

        // A dangling link must not create its outside target either.
        fs::remove_file(&link).unwrap();
        symlink(outside.path().join("created"), &link).unwrap();
        assert_symlink_refusal(lock(root.path()).map(|_| ()));
        assert_eq!(listing(outside.path()), ["target"]);
        assert_eq!(listing(&root.path().join(".armorer")), ["apply.lock"]);
    }

    /// A symlinked control directory is refused and nothing is created through it.
    #[test]
    fn lock_refuses_symlinked_control_directory() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), root.path().join(".armorer")).unwrap();
        assert_symlink_refusal(lock(root.path()).map(|_| ()));
        assert!(listing(outside.path()).is_empty());
        // Dangling control directory link: the outside directory is not created.
        fs::remove_file(root.path().join(".armorer")).unwrap();
        symlink(outside.path().join("made"), root.path().join(".armorer")).unwrap();
        assert_symlink_refusal(lock(root.path()).map(|_| ()));
        assert!(listing(outside.path()).is_empty());
    }

    /// The lock creates a real control directory and regular lock file, and refuses special files unopened.
    #[test]
    fn lock_creates_regular_control_files_and_refuses_special_files() {
        let root = tempfile::tempdir().unwrap();
        let guard = lock(root.path()).unwrap();
        let control = fs::symlink_metadata(root.path().join(".armorer")).unwrap();
        assert!(control.is_dir());
        let lock_file = fs::symlink_metadata(root.path().join(".armorer/apply.lock")).unwrap();
        assert!(lock_file.is_file());
        drop(guard);

        let fifo_root = tempfile::tempdir().unwrap();
        fs::create_dir(fifo_root.path().join(".armorer")).unwrap();
        let status = std::process::Command::new("mkfifo")
            .arg(fifo_root.path().join(".armorer/apply.lock"))
            .status()
            .unwrap();
        assert!(status.success());
        assert!(matches!(
            lock(fifo_root.path()),
            Err(Error::Transaction("apply lock must be a regular file"))
        ));

        let file_root = tempfile::tempdir().unwrap();
        fs::write(file_root.path().join(".armorer"), b"not a directory").unwrap();
        assert!(matches!(
            lock(file_root.path()),
            Err(Error::Io(error)) if error.kind() == io::ErrorKind::AlreadyExists
        ));
    }

    /// Leaf symlinks, existing or dangling, are refused for both creation and update.
    #[test]
    fn replace_refuses_symlinked_leaf() {
        for absent in [true, false] {
            for dangling in [false, true] {
                let root = tempfile::tempdir().unwrap();
                let outside = tempfile::tempdir().unwrap();
                let target = outside.path().join("target");
                if !dangling {
                    fs::write(&target, b"outside bytes").unwrap();
                }
                fs::create_dir(root.path().join("dir")).unwrap();
                let link = root.path().join("dir/leaf");
                symlink(&target, &link).unwrap();
                assert_symlink_refusal(replace(root.path(), "dir/leaf", b"new", absent));
                assert_eq!(listing(&root.path().join("dir")), ["leaf"]);
                assert!(
                    fs::symlink_metadata(&link)
                        .unwrap()
                        .file_type()
                        .is_symlink()
                );
                if dangling {
                    assert!(listing(outside.path()).is_empty());
                } else {
                    assert_eq!(listing(outside.path()), ["target"]);
                    assert_eq!(fs::read(&target).unwrap(), b"outside bytes");
                }
            }
        }
    }

    /// Any symlinked intermediate directory, existing or dangling, refuses the write.
    #[test]
    fn replace_refuses_symlinked_intermediate_directory() {
        for relative in ["linked/file", "real/linked/file", "real/linked/deeper/file"] {
            for dangling in [false, true] {
                let root = tempfile::tempdir().unwrap();
                let outside = tempfile::tempdir().unwrap();
                fs::create_dir(root.path().join("real")).unwrap();
                let target = if dangling {
                    outside.path().join("missing")
                } else {
                    fs::create_dir(outside.path().join("deeper")).unwrap();
                    outside.path().to_path_buf()
                };
                let link = if relative.starts_with("real/") {
                    root.path().join("real/linked")
                } else {
                    root.path().join("linked")
                };
                symlink(&target, &link).unwrap();
                let outside_before = listing(outside.path());
                for absent in [true, false] {
                    assert_symlink_refusal(replace(root.path(), relative, b"new", absent));
                }
                assert_eq!(listing(outside.path()), outside_before, "{relative}");
                if !dangling {
                    assert!(listing(&outside.path().join("deeper")).is_empty());
                }
                assert_no_staging(root.path());
                assert_no_staging(&root.path().join("real"));
            }
        }
    }

    /// A newly created managed file requests 0644 under the process umask; updates keep their mode.
    #[test]
    fn replace_creates_new_files_with_shared_mode() {
        use std::os::unix::fs::OpenOptionsExt;
        let root = tempfile::tempdir().unwrap();
        // Same request through std reveals the umask without changing it.
        let reference = root.path().join("reference");
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o644)
            .open(&reference)
            .unwrap();
        let expected = fs::metadata(&reference).unwrap().mode() & 0o7777;
        replace(root.path(), "created", b"new", true).unwrap();
        let created = root.path().join("created");
        assert_eq!(fs::metadata(&created).unwrap().mode() & 0o7777, expected);
        assert_eq!(expected & !0o644, 0, "never wider than requested");
        fs::set_permissions(&created, fs::Permissions::from_mode(0o600)).unwrap();
        replace(root.path(), "created", b"updated", false).unwrap();
        assert_eq!(fs::metadata(&created).unwrap().mode() & 0o7777, 0o600);
    }

    /// Journals are the only new files created private; every journal constant must match.
    #[test]
    fn journal_paths_are_private_and_constants_are_covered() {
        for journal in [
            crate::bootstrap::JOURNAL,
            crate::upgrade::JOURNAL,
            ".armorer/journal.json",
        ] {
            assert!(is_journal(journal), "{journal}");
        }
        for committed in [
            ".armorer/state.json",
            ".armorer/ci-policy.toml",
            ".armorer/sub/upgrade-journal-v1.json",
            "upgrade-journal-v1.json",
            "other/.armorer/journal.json",
            ".github/workflows/ci.yml",
        ] {
            assert!(!is_journal(committed), "{committed}");
        }
    }

    /// A new journal requests 0600 and a new workflow 0644, both under the process umask.
    #[test]
    fn replace_creates_journals_private_and_workflows_shared() {
        use std::os::unix::fs::OpenOptionsExt;
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join(".armorer")).unwrap();
        fs::create_dir_all(root.path().join(".github/workflows")).unwrap();
        let umask_applied = |relative: &str, requested: u32| {
            let reference = root
                .path()
                .join(format!("reference-{}", relative.replace('/', "_")));
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(requested)
                .open(&reference)
                .unwrap();
            fs::metadata(&reference).unwrap().mode() & 0o7777
        };
        for (relative, requested) in [
            (crate::upgrade::JOURNAL, 0o600),
            (crate::bootstrap::JOURNAL, 0o600),
            (".github/workflows/ci.yml", 0o644),
            (".armorer/ci-policy.toml", 0o644),
        ] {
            replace(root.path(), relative, b"new", true).unwrap();
            let mode = fs::metadata(root.path().join(relative)).unwrap().mode() & 0o7777;
            assert_eq!(mode, umask_applied(relative, requested), "{relative}");
            assert_eq!(
                mode & !requested,
                0,
                "{relative} never wider than requested"
            );
        }
    }

    /// Creation never clobbers a destination that already exists.
    #[test]
    fn replace_absent_refuses_existing_destination() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("file"), b"keep").unwrap();
        assert!(matches!(
            replace(root.path(), "file", b"new", true),
            Err(Error::Io(error)) if error.kind() == io::ErrorKind::AlreadyExists
        ));
        assert_eq!(fs::read(root.path().join("file")).unwrap(), b"keep");
        assert_eq!(listing(root.path()), ["file"]);
    }

    /// Successful writes land exact bytes, keep an updated file's mode and leave no staging entry.
    #[test]
    fn replace_writes_exact_bytes_and_preserves_existing_mode() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join(".github/workflows")).unwrap();
        let relative = ".github/workflows/ci.yml";
        let path = root.path().join(relative);
        let bytes: Vec<u8> = (0..=255).cycle().take(70_000).collect();
        replace(root.path(), relative, &bytes, true).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(listing(path.parent().unwrap()), ["ci.yml"]);
        for mode in [0o640, 0o755, 0o600] {
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            let inode = fs::metadata(&path).unwrap().ino();
            replace(
                root.path(),
                relative,
                format!("mode {mode:o}").as_bytes(),
                false,
            )
            .unwrap();
            let metadata = fs::symlink_metadata(&path).unwrap();
            assert!(metadata.is_file());
            assert_eq!(metadata.mode() & 0o7777, mode);
            assert_ne!(metadata.ino(), inode, "replacement is a new inode");
            assert_eq!(
                fs::read(&path).unwrap(),
                format!("mode {mode:o}").as_bytes()
            );
            assert_eq!(listing(path.parent().unwrap()), ["ci.yml"]);
        }
        // Updating a missing destination fails without leaving staging entries.
        assert!(matches!(
            replace(root.path(), ".github/workflows/missing.yml", b"x", false),
            Err(Error::Io(error)) if error.kind() == io::ErrorKind::NotFound
        ));
        // A missing parent fails the same way it did before handle-based writes.
        assert!(matches!(
            replace(root.path(), "absent/dir/file", b"x", true),
            Err(Error::Io(error)) if error.kind() == io::ErrorKind::NotFound
        ));
        assert_eq!(listing(path.parent().unwrap()), ["ci.yml"]);
        assert_eq!(listing(root.path()), [".github"]);
    }

    /// Optional reads refuse symlinks anywhere, preserve absence and keep the regular-file and size rules.
    #[test]
    fn optional_bytes_reads_through_handles() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret"), b"outside").unwrap();
        fs::create_dir(root.path().join("dir")).unwrap();
        fs::write(root.path().join("dir/file"), b"inside").unwrap();
        assert_eq!(
            optional_bytes(root.path(), "dir/file").unwrap().as_deref(),
            Some(&b"inside"[..])
        );
        assert!(optional_bytes(root.path(), "dir/absent").unwrap().is_none());
        assert!(
            optional_bytes(root.path(), "absent/file")
                .unwrap()
                .is_none()
        );
        assert!(
            optional_bytes(&root.path().join("no-root"), "file")
                .unwrap()
                .is_none()
        );

        symlink(outside.path(), root.path().join("linked")).unwrap();
        assert_symlink_refusal(optional_bytes(root.path(), "linked/secret"));
        symlink(outside.path().join("gone"), root.path().join("dangling")).unwrap();
        assert_symlink_refusal(optional_bytes(root.path(), "dangling/secret"));
        symlink(outside.path().join("secret"), root.path().join("dir/leaf")).unwrap();
        assert_symlink_refusal(optional_bytes(root.path(), "dir/leaf"));
        symlink(outside.path().join("gone"), root.path().join("dir/dead")).unwrap();
        assert_symlink_refusal(optional_bytes(root.path(), "dir/dead"));

        fs::write(root.path().join("limit"), vec![b'x'; 1_048_576]).unwrap();
        assert_eq!(
            optional_bytes(root.path(), "limit").unwrap().unwrap().len(),
            1_048_576
        );
        fs::write(root.path().join("large"), vec![b'x'; 1_048_577]).unwrap();
        assert!(matches!(
            optional_bytes(root.path(), "large"),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            optional_bytes(root.path(), "dir"),
            Err(Error::Path(message)) if message == crate::REGULAR_INPUT
        ));

        let status = std::process::Command::new("mkfifo")
            .arg(root.path().join("fifo"))
            .status()
            .unwrap();
        assert!(status.success());
        let fifo_root = root.path().to_path_buf();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            sender
                .send(optional_bytes(&fifo_root, "fifo").map(|_| ()))
                .unwrap();
        });
        let result = receiver
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("FIFO read must not block");
        assert!(matches!(
            result,
            Err(Error::Path(message)) if message == crate::REGULAR_INPUT
        ));
    }

    /// Race regression: a parent swapped for a symlink after the walk cannot redirect the held handle.
    #[test]
    fn held_parent_handle_survives_directory_swap() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("managed")).unwrap();
        fs::write(root.path().join("managed/existing"), b"before").unwrap();
        let walked = fs::metadata(root.path().join("managed")).unwrap().ino();
        let parent = handles::open_parent(root.path(), "managed/existing")
            .unwrap()
            .unwrap();
        // Swap the validated directory for a symlink to an outside directory.
        fs::rename(root.path().join("managed"), root.path().join("original")).unwrap();
        symlink(outside.path(), root.path().join("managed")).unwrap();

        const NEW: rustix::fs::RawMode = handles::NEW_FILE_MODE;
        handles::replace_at(&parent.directory, &parent.leaf, b"after", false, NEW).unwrap();
        handles::replace_at(&parent.directory, OsStr::new("created"), b"new", true, NEW).unwrap();
        assert_eq!(
            handles::read_at(&parent.directory, &parent.leaf)
                .unwrap()
                .as_deref(),
            Some(&b"after"[..])
        );
        assert_eq!(parent.directory.metadata().unwrap().ino(), walked);
        let original = root.path().join("original");
        assert_eq!(fs::metadata(&original).unwrap().ino(), walked);
        assert_eq!(fs::read(original.join("existing")).unwrap(), b"after");
        assert_eq!(fs::read(original.join("created")).unwrap(), b"new");
        assert_eq!(listing(&original), ["created", "existing"]);
        assert!(listing(outside.path()).is_empty());

        // A fresh walk now sees the symlink and refuses it.
        assert_symlink_refusal(optional_bytes(root.path(), "managed/existing"));
        assert_symlink_refusal(replace(root.path(), "managed/existing", b"x", false));
        assert!(listing(outside.path()).is_empty());
    }

    /// The handle walk and `safe_path` share one lexical gate and agree on every hostile input.
    #[test]
    fn lexical_rejections_match_safe_path() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("a")).unwrap();
        fs::create_dir(root.path().join(".armorer")).unwrap();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), root.path().join("linked")).unwrap();
        // Recorded `safe_path` behavior before directory handles: `None` is the lexical refusal.
        let table: &[(&str, Option<&str>)] = &[
            ("", None),
            ("..", None),
            ("a/..", None),
            ("a/../b", None),
            ("../a", None),
            (".", None),
            ("./a", None),
            ("/", None),
            ("/a", None),
            ("a\\b", None),
            ("a\0b", None),
            ("a\nb", None),
            ("a\tb", None),
            ("a\u{7f}b", None),
            ("a\u{85}b", None),
            ("a//b", Some("a/b")),
            ("a/", Some("a")),
            ("a/./b", Some("a/b")),
            ("a/.", Some("a")),
            ("a", Some("a")),
            (".armorer/apply.lock", Some(".armorer/apply.lock")),
        ];
        for (input, expected) in table {
            let safe = crate::safe_path(root.path(), input);
            let walked = handles::open_parent(root.path(), input);
            match expected {
                None => {
                    for message in [
                        safe.map(|_| ()).unwrap_err().to_string(),
                        walked.map(|_| ()).unwrap_err().to_string(),
                    ] {
                        assert_eq!(
                            message,
                            "unsafe or unsupported repository path: expected a plain relative path",
                            "{input:?}"
                        );
                    }
                }
                Some(expected) => {
                    let joined = safe.unwrap();
                    assert_eq!(joined, root.path().join(expected), "{input:?}");
                    let parent = walked.unwrap().unwrap();
                    assert_eq!(Some(parent.leaf.as_os_str()), joined.file_name());
                    assert_eq!(
                        parent.directory.metadata().unwrap().ino(),
                        fs::metadata(joined.parent().unwrap()).unwrap().ino(),
                        "{input:?}"
                    );
                }
            }
        }
        for input in ["linked", "linked/x", "linked/x/y"] {
            assert_symlink_refusal(crate::safe_path(root.path(), input));
            if input != "linked" {
                assert_symlink_refusal(handles::open_parent(root.path(), input).map(|_| ()));
            }
            assert_symlink_refusal(optional_bytes(root.path(), input));
        }
    }
}

/// Read a regular optional input while preserving absence and propagating unsafe-file failures.
pub(crate) fn optional_bytes(root: &Path, relative: &str) -> Result<Option<Vec<u8>>> {
    #[cfg(unix)]
    {
        match handles::open_parent(root, relative)? {
            Some(parent) => handles::read_at(&parent.directory, &parent.leaf),
            None => Ok(None),
        }
    }
    #[cfg(not(unix))]
    {
        let path = crate::safe_path(root, relative)?;
        match std::fs::symlink_metadata(&path) {
            Ok(_) => Ok(Some(crate::read_small(&path)?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
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
    #[cfg(unix)]
    let file = handles::lock_file(root)?;
    #[cfg(not(unix))]
    let file = {
        let directory = crate::safe_path(root, ".armorer")?;
        match std::fs::create_dir(&directory) {
            Ok(()) => sync_directory(root)?,
            Err(error)
                if error.kind() == std::io::ErrorKind::AlreadyExists && directory.is_dir() => {}
            Err(error) => return Err(error.into()),
        }
        let path = crate::safe_path(root, ".armorer/apply.lock")?;
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?
    };
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
    #[cfg(unix)]
    {
        let parent = handles::open_parent(root, relative)?
            .ok_or_else(|| Error::Io(rustix::io::Errno::NOENT.into()))?;
        handles::replace_at(
            &parent.directory,
            &parent.leaf,
            bytes,
            absent,
            new_file_mode(relative),
        )
    }
    #[cfg(not(unix))]
    {
        use std::io::Write;
        let destination = crate::safe_path(root, relative)?;
        let parent = destination
            .parent()
            .ok_or(Error::Transaction("invalid destination"))?;
        let mut staging = tempfile::NamedTempFile::new_in(parent)?;
        staging.write_all(bytes)?;
        if !absent {
            staging
                .as_file()
                .set_permissions(std::fs::metadata(&destination)?.permissions())?;
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
}

/// Whether `relative` names a transaction journal under `.armorer/`. A journal embeds
/// the full approved plan and is transient, so unlike committed managed files it stays private.
#[cfg(unix)]
fn is_journal(relative: &str) -> bool {
    relative.strip_prefix(".armorer/").is_some_and(|name| {
        !name.contains('/')
            && (name == "journal.json"
                || name.ends_with("-journal-v1.json")
                || name.ends_with("-journal-v2.json"))
    })
}

/// Mode requested for a newly created managed file: private for journals, shareable otherwise.
#[cfg(unix)]
fn new_file_mode(relative: &str) -> rustix::fs::RawMode {
    if is_journal(relative) {
        handles::PRIVATE_NEW_FILE_MODE
    } else {
        handles::NEW_FILE_MODE
    }
}

/// Directory-handle primitives behind every managed read and write on Unix.
#[cfg(unix)]
mod handles {
    use crate::{
        Error, INPUT_CHANGED, REGULAR_INPUT, Result, SYMLINK_COMPONENT, plain_components,
        read_limited,
    };
    use rustix::{
        fs::{AtFlags, FileType, Mode, OFlags, RawMode, Stat},
        io::Errno,
    };
    use std::{
        ffi::{OsStr, OsString},
        fs::File,
        io::Write,
        os::fd::{AsFd, BorrowedFd, OwnedFd},
        path::Path,
    };

    /// Mode requested for the staging file of an update; the destination's mode is applied before rename.
    const STAGING_MODE: RawMode = 0o600;
    /// Mode requested for a newly created managed file, subject to the process umask.
    /// Managed files (workflow YAML, lock, policy) are meant to be committed and shared.
    pub(super) const NEW_FILE_MODE: RawMode = 0o644;
    /// Mode requested for a newly created transaction journal, subject to the process umask.
    pub(super) const PRIVATE_NEW_FILE_MODE: RawMode = 0o600;
    /// Prefix of staging entries created next to a destination.
    pub(super) const STAGING_PREFIX: &str = ".armorer-tmp-";
    /// Bounded retries when a random staging name is already taken.
    const STAGING_ATTEMPTS: usize = 64;

    /// The held directory that directly contains a managed leaf.
    pub(crate) struct Parent {
        pub(crate) directory: File,
        pub(crate) leaf: OsString,
    }

    /// Convert a system error into the crate's I/O category.
    fn os(error: Errno) -> Error {
        Error::Io(error.into())
    }

    /// Classify an entry's file type from its non-following stat.
    fn kind(stat: &Stat) -> FileType {
        FileType::from_raw_mode(stat.st_mode)
    }

    /// Inspect one entry of a held directory without following it; a symlink is refused.
    fn entry(directory: BorrowedFd<'_>, name: &OsStr) -> Result<Option<Stat>> {
        match rustix::fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(stat) if kind(&stat) == FileType::Symlink => {
                Err(Error::Path(SYMLINK_COMPONENT.into()))
            }
            Ok(stat) => Ok(Some(stat)),
            Err(Errno::NOENT) => Ok(None),
            Err(error) => Err(os(error)),
        }
    }

    /// Report a failed relative open, preferring the symlink refusal when the entry is now a symlink.
    fn refused(directory: BorrowedFd<'_>, name: &OsStr, error: Errno) -> Error {
        match entry(directory, name) {
            Err(symlink @ Error::Path(_)) => symlink,
            _ => os(error),
        }
    }

    /// Open a repository root as a directory. The root itself may be reached
    /// through a symlink, matching `safe_path`, which never inspects the root.
    fn root_directory(root: &Path) -> Result<Option<OwnedFd>> {
        match rustix::fs::open(
            root,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => Ok(Some(fd)),
            Err(Errno::NOENT) => Ok(None),
            Err(error) => Err(os(error)),
        }
    }

    /// Open a child directory relative to a held handle without following a symlink.
    fn child_directory(parent: BorrowedFd<'_>, name: &OsStr) -> Result<Option<OwnedFd>> {
        match entry(parent, name)? {
            None => return Ok(None),
            Some(stat) if kind(&stat) != FileType::Directory => return Err(os(Errno::NOTDIR)),
            Some(_) => {}
        }
        match rustix::fs::openat(
            parent,
            name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => Ok(Some(fd)),
            Err(Errno::NOENT) => Ok(None),
            Err(error) => Err(refused(parent, name, error)),
        }
    }

    /// Walk every intermediate component of `relative` through held directory handles.
    ///
    /// Lexical validation is shared with `safe_path`. Returns `None` when the
    /// root or an intermediate directory is missing; the leaf is not inspected.
    pub(crate) fn open_parent(root: &Path, relative: &str) -> Result<Option<Parent>> {
        let components = plain_components(relative)?;
        let Some((leaf, intermediate)) = components.split_last() else {
            return Err(Error::Path("expected a plain relative path".into()));
        };
        let Some(mut directory) = root_directory(root)? else {
            return Ok(None);
        };
        for name in intermediate {
            match child_directory(directory.as_fd(), name)? {
                Some(child) => directory = child,
                None => return Ok(None),
            }
        }
        Ok(Some(Parent {
            directory: File::from(directory),
            leaf: leaf.to_os_string(),
        }))
    }

    /// Read a bounded regular leaf relative to a held directory, preserving absence.
    ///
    /// Keeps `read_small`'s guarantees: special files are refused before any
    /// open, the open is non-blocking and non-following, and the opened inode
    /// must be the one inspected in the same held directory.
    pub(crate) fn read_at(directory: &File, leaf: &OsStr) -> Result<Option<Vec<u8>>> {
        let directory = directory.as_fd();
        let Some(before) = entry(directory, leaf)? else {
            return Ok(None);
        };
        if kind(&before) != FileType::RegularFile {
            return Err(Error::Path(REGULAR_INPUT.into()));
        }
        let fd = rustix::fs::openat(
            directory,
            leaf,
            OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|error| refused(directory, leaf, error))?;
        let opened = rustix::fs::fstat(&fd).map_err(os)?;
        if kind(&opened) != FileType::RegularFile {
            return Err(Error::Path(REGULAR_INPUT.into()));
        }
        if before.st_dev != opened.st_dev || before.st_ino != opened.st_ino {
            return Err(Error::Path(INPUT_CHANGED.into()));
        }
        read_limited(File::from(fd)).map(Some)
    }

    /// Choose an unpredictable staging name; `O_EXCL` turns a collision into a retry, never a reuse.
    fn staging_name() -> OsString {
        use std::{
            hash::{BuildHasher, Hasher},
            sync::atomic::{AtomicU64, Ordering},
        };
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let mut hasher = std::hash::RandomState::new().build_hasher();
        hasher.write_u64(SEQUENCE.fetch_add(1, Ordering::Relaxed));
        hasher.write_u32(std::process::id());
        OsString::from(format!("{STAGING_PREFIX}{:016x}", hasher.finish()))
    }

    /// A staging file inside a held directory, removed by name on every failure path.
    struct Staged<'a> {
        directory: BorrowedFd<'a>,
        name: OsString,
        file: File,
        armed: bool,
    }

    impl<'a> Staged<'a> {
        /// Create a fresh staging leaf with `O_CREAT | O_EXCL | O_NOFOLLOW`.
        fn create(directory: BorrowedFd<'a>, mode: RawMode) -> Result<Self> {
            for _ in 0..STAGING_ATTEMPTS {
                let name = staging_name();
                match rustix::fs::openat(
                    directory,
                    &name,
                    OFlags::WRONLY
                        | OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    Mode::from_raw_mode(mode),
                ) {
                    Ok(fd) => {
                        return Ok(Self {
                            directory,
                            name,
                            file: File::from(fd),
                            armed: true,
                        });
                    }
                    Err(Errno::EXIST) => {}
                    Err(error) => return Err(os(error)),
                }
            }
            Err(os(Errno::EXIST))
        }

        /// Move the staging leaf onto `leaf` within the same held directory.
        fn persist(mut self, leaf: &OsStr, absent: bool) -> Result<()> {
            if !absent {
                rustix::fs::renameat(self.directory, &self.name, self.directory, leaf)
                    .map_err(os)?;
                self.armed = false;
                return Ok(());
            }
            #[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
            match rustix::fs::renameat_with(
                self.directory,
                &self.name,
                self.directory,
                leaf,
                rustix::fs::RenameFlags::NOREPLACE,
            ) {
                Ok(()) => {
                    self.armed = false;
                    return Ok(());
                }
                // Kernel or filesystem without the no-replace rename: fall back to a hard link.
                Err(error)
                    if error == Errno::NOSYS
                        || error == Errno::INVAL
                        || error == Errno::NOTSUP
                        || error == Errno::OPNOTSUPP => {}
                Err(error) => return Err(os(error)),
            }
            // `linkat` never replaces an existing name; dropping `self` then unlinks the staging name.
            rustix::fs::linkat(
                self.directory,
                &self.name,
                self.directory,
                leaf,
                AtFlags::empty(),
            )
            .map_err(os)
        }
    }

    impl Drop for Staged<'_> {
        /// Remove the staging name from the held directory unless a rename consumed it.
        fn drop(&mut self) {
            if self.armed {
                let _ = rustix::fs::unlinkat(self.directory, &self.name, AtFlags::empty());
            }
        }
    }

    /// Replace `leaf` inside an already held directory handle.
    ///
    /// The staging file is created, written and fsynced in `directory`, renamed
    /// within `directory`, and `directory` is then fsynced. With `absent` the
    /// destination is never clobbered and a new file is requested with `new_mode`;
    /// otherwise its mode is preserved.
    pub(crate) fn replace_at(
        directory: &File,
        leaf: &OsStr,
        bytes: &[u8],
        absent: bool,
        new_mode: RawMode,
    ) -> Result<()> {
        let held = directory.as_fd();
        let current = entry(held, leaf)?;
        let preserved = if absent {
            None
        } else {
            Some(current.ok_or_else(|| os(Errno::NOENT))?.st_mode)
        };
        let staged = Staged::create(held, if absent { new_mode } else { STAGING_MODE })?;
        (&staged.file).write_all(bytes)?;
        if let Some(mode) = preserved {
            rustix::fs::fchmod(&staged.file, Mode::from_raw_mode(mode)).map_err(os)?;
        }
        staged.file.sync_all()?;
        staged.persist(leaf, absent)?;
        directory.sync_all()?;
        Ok(())
    }

    /// Open `.armorer/apply.lock` through held handles, creating only what is missing.
    pub(crate) fn lock_file(root: &Path) -> Result<File> {
        let root = File::from(root_directory(root)?.ok_or_else(|| os(Errno::NOENT))?);
        let control = OsStr::new(".armorer");
        entry(root.as_fd(), control)?;
        let created = match rustix::fs::mkdirat(&root, control, Mode::from_raw_mode(0o777)) {
            Ok(()) => {
                root.sync_all()?;
                true
            }
            Err(Errno::EXIST) => false,
            Err(error) => return Err(os(error)),
        };
        let directory = match rustix::fs::openat(
            &root,
            control,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => fd,
            Err(error) => {
                return Err(match refused(root.as_fd(), control, error) {
                    // An existing non-directory reports the creation conflict, as before.
                    Error::Io(_) if !created => os(Errno::EXIST),
                    other => other,
                });
            }
        };
        let leaf = OsStr::new("apply.lock");
        if let Some(stat) = entry(directory.as_fd(), leaf)? {
            // Refuse special files without opening them, keeping the error a by-path open reported.
            match kind(&stat) {
                FileType::RegularFile => {}
                FileType::Directory => return Err(os(Errno::ISDIR)),
                FileType::Socket => return Err(os(Errno::NXIO)),
                _ => return Err(Error::Transaction("apply lock must be a regular file")),
            }
        }
        let fd = rustix::fs::openat(
            &directory,
            leaf,
            OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o666),
        )
        .map_err(|error| refused(directory.as_fd(), leaf, error))?;
        Ok(File::from(fd))
    }
}
