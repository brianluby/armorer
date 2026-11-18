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
