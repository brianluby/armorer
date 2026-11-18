//! Public-contract checks for managed repository paths.
//!
//! The handle-based walk in `src/filesystem.rs` shares its lexical rules with
//! `armorer::safe_path`; this table pins the behavior recorded before that walk
//! existed so the shared rules cannot drift.

use std::path::Path;

/// Hostile and normalizing inputs with the joined result `safe_path` returned
/// before directory-handle I/O was introduced (`None` means `Error::Path`).
const LEXICAL: &[(&str, Option<&str>)] = &[
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

/// `safe_path` keeps its recorded lexical contract for every table entry.
#[test]
fn safe_path_lexical_contract_is_unchanged() {
    let root = tempfile::tempdir().unwrap();
    for (input, expected) in LEXICAL {
        match (armorer::safe_path(root.path(), input), expected) {
            (Ok(joined), Some(expected)) => {
                assert_eq!(joined, root.path().join(Path::new(expected)), "{input:?}");
            }
            (Err(armorer::Error::Path(message)), None) => {
                assert_eq!(message, "expected a plain relative path", "{input:?}");
            }
            (result, _) => panic!("{input:?}: unexpected {result:?}"),
        }
    }
}

/// `safe_path` still refuses symlinked parents and leaves, including dangling ones.
#[cfg(unix)]
#[test]
fn safe_path_refuses_symlinked_components() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("real")).unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("linked")).unwrap();
    std::os::unix::fs::symlink(
        root.path().join("missing"),
        root.path().join("real/dangling"),
    )
    .unwrap();
    for input in ["linked", "linked/file", "real/dangling", "real/dangling/x"] {
        assert!(
            matches!(
                armorer::safe_path(root.path(), input),
                Err(armorer::Error::Path(ref message)) if message == "symlink input or parent"
            ),
            "{input}"
        );
    }
    assert_eq!(
        armorer::safe_path(root.path(), "real/missing/leaf").unwrap(),
        root.path().join("real/missing/leaf")
    );
}

/// End-to-end refusals through the public transaction API when a managed
/// directory is swapped for a symlink after review and before apply.
///
/// `tests/bootstrap.rs` already pins that bootstrap apply errors and leaves the
/// outside target alone for `.github` and `.armorer` links; `tests/upgrade.rs`
/// covers symlinked upgrade leaves only. These add the directory swaps for
/// upgrade apply and reviewed rollback, and for all three operations assert the
/// whole repository tree is unchanged too. The real directory is moved outside,
/// so a path-following implementation would find the reviewed bytes there and
/// write through the link; only the symlink refusal stops it.
#[cfg(unix)]
mod public_api {
    use armorer::{bootstrap, upgrade};
    use std::{collections::BTreeMap, fs, os::unix::fs::symlink, path::Path};

    /// Reviewed policy shipped with the example fixture.
    const POLICY: &str = "ci-policy.reviewed.toml";
    /// Managed directories whose replacement by a symlink must refuse every transaction.
    const SWAPPED: [&str; 2] = [".github", ".armorer"];

    /// Copy the library example into a fresh repository with an existing `.github` directory.
    fn fixture() -> tempfile::TempDir {
        /// Copy regular fixture files and directories without following or executing anything.
        fn copy(from: &Path, to: &Path) {
            fs::create_dir_all(to).unwrap();
            for entry in fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                let destination = to.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copy(&entry.path(), &destination);
                } else {
                    fs::copy(entry.path(), destination).unwrap();
                }
            }
        }
        let root = tempfile::tempdir().unwrap();
        copy(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/bootstrap-v2/library"),
            root.path(),
        );
        fs::create_dir(root.path().join(".github")).unwrap();
        fs::write(root.path().join(".github/CODEOWNERS"), "* @example\n").unwrap();
        root
    }

    /// Every entry below `dir` as exact bytes; symlinks are recorded, never followed.
    fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
        /// Recursively record one directory level without traversing links.
        fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                let relative = path.strip_prefix(root).unwrap().display().to_string();
                let kind = fs::symlink_metadata(&path).unwrap().file_type();
                if kind.is_symlink() {
                    let target = fs::read_link(&path).unwrap();
                    files.insert(relative, format!("symlink:{}", target.display()).into());
                } else if kind.is_dir() {
                    files.insert(format!("{relative}/"), b"directory".to_vec());
                    walk(root, &path, files);
                } else {
                    files.insert(relative, fs::read(&path).unwrap());
                }
            }
        }
        let mut files = BTreeMap::new();
        walk(dir, dir, &mut files);
        files
    }

    /// Move `name` (or a new empty directory) outside and leave a symlink to it in its place.
    fn swap_for_symlink(root: &Path, name: &str, outside: &Path) {
        let moved = outside.join(name);
        if fs::symlink_metadata(root.join(name)).is_ok() {
            fs::rename(root.join(name), &moved).unwrap();
        } else {
            fs::create_dir(&moved).unwrap();
        }
        symlink(&moved, root.join(name)).unwrap();
    }

    /// Run `operation` after the swap and require a symlink refusal with both trees unchanged.
    #[track_caller]
    fn assert_refused<T: std::fmt::Debug>(
        root: &Path,
        name: &str,
        operation: impl FnOnce() -> armorer::Result<T>,
    ) {
        let outside = tempfile::tempdir().unwrap();
        swap_for_symlink(root, name, outside.path());
        let (repository, external) = (tree(root), tree(outside.path()));
        let result = operation();
        assert!(
            matches!(&result, Err(armorer::Error::Path(message)) if message.contains("symlink")),
            "{name}: {result:?}"
        );
        assert_eq!(tree(root), repository, "{name}: repository changed");
        assert_eq!(
            tree(outside.path()),
            external,
            "{name}: outside target changed"
        );
    }

    /// Derive the upgrade packet the fixture's reviewer would approve.
    fn preview(root: &Path) -> upgrade::Plan {
        upgrade::inspect(root, &root.join(POLICY), "bootstrap-v2", &[], false).unwrap()
    }

    /// Bootstrap apply refuses a swapped `.github` or `.armorer` and writes nothing anywhere.
    #[test]
    fn bootstrap_apply_refuses_swapped_managed_directories() {
        for name in SWAPPED {
            let root = fixture();
            let plan = bootstrap::inspect(root.path(), &root.path().join(POLICY)).unwrap();
            assert_refused(root.path(), name, || {
                bootstrap::apply(root.path(), &plan, &plan.plan_sha256)
            });
        }
    }

    /// Upgrade apply refuses a swapped `.github` or `.armorer` and writes nothing anywhere.
    #[test]
    fn upgrade_apply_refuses_swapped_managed_directories() {
        for name in SWAPPED {
            let root = fixture();
            let plan = preview(root.path());
            assert_refused(root.path(), name, || {
                upgrade::apply(root.path(), &plan, &plan.plan_sha256)
            });
        }
    }

    /// A reviewed rollback refuses a swapped `.github` or `.armorer` and restores nothing anywhere.
    #[test]
    fn reviewed_rollback_refuses_swapped_managed_directories() {
        for name in SWAPPED {
            let root = fixture();
            let plan = preview(root.path());
            upgrade::apply(root.path(), &plan, &plan.plan_sha256).unwrap();
            let reverse =
                upgrade::inspect_rollback(root.path(), &plan, &plan.plan_sha256, true).unwrap();
            assert_refused(root.path(), name, || {
                upgrade::apply_rollback(root.path(), &reverse, &reverse.plan_sha256)
            });
        }
    }
}
