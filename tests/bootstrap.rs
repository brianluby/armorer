use armorer::{
    bootstrap::{apply, inspect, load_plan},
    config::load_lock,
    plan,
};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
const STATE: &str = ".armorer/bootstrap-state-v2.json";
const POLICY: &str = "schema_version = 1\n[licenses]\nallow = [\"MIT\"]\n[advisories]\nexceptions = []\n[sources]\nallow_git = []\n[bans]\nmultiple_versions = \"warn\"\ndeny = []\n";
/// Create an isolated repository fixture with explicit inputs for the surrounding transaction tests.
fn fixture(profile: &str) -> (tempfile::TempDir, tempfile::NamedTempFile) {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    fs::write(
        root.path().join("src/lib.rs"),
        "compile_error!(\"must not build\");",
    )
    .unwrap();
    fs::write(
        root.path().join("src/main.rs"),
        "compile_error!(\"must not build\");",
    )
    .unwrap();
    fs::write(root.path().join("build.rs"), "fn main() { std::fs::write(\"BUILD_EXECUTED\", \"bad\").unwrap(); panic!(\"no builds\"); }").unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"pilot\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(root.path().join("LICENSE"), "MIT\n").unwrap();
    let binary = if profile == "library" {
        ""
    } else {
        "binary = \"pilot\"\n"
    };
    fs::write(root.path().join("armorer.toml"), format!("schema_version = 1\nrepository = \"example/pilot\"\ntoolchain = \"1.95.0\"\n[[deliverables]]\nid = \"primary\"\nprofile = \"{profile}\"\npackage = \"pilot\"\n{binary}targets = [\"x86_64-unknown-linux-gnu\", \"aarch64-apple-darwin\"]\nfeature_set = \"standard\"\n[feature_sets.standard]\ndefault_features = true\nfeatures = []\n[policy]\nlicense_file = \"LICENSE\"\nattestations = \"required\"\n")).unwrap();
    let mut policy = tempfile::NamedTempFile::new().unwrap();
    policy.write_all(POLICY.as_bytes()).unwrap();
    (root, policy)
}
/// The fixed managed inventory, mirrored from src/bootstrap/mod.rs TARGETS.
const MANAGED: [&str; 5] = [
    ".armorer/ci-policy.toml",
    ".github/workflows/armorer-build.yml",
    ".github/workflows/armorer-ci.yml",
    "armorer.lock",
    "rust-toolchain.toml",
];

/// Select a change by its stable path, never by inventory position.
fn change<'a>(plan: &'a armorer::bootstrap::Plan, path: &str) -> &'a armorer::bootstrap::Change {
    plan.changes.iter().find(|c| c.path == path).unwrap()
}

/// Capture every regular fixture file as exact bytes for mutation and recovery comparisons.
fn snapshot(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    /// Recursively record relative fixture paths and bytes without running repository code.
    fn walk(root: &Path, dir: &Path, files: &mut std::collections::BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            // Non-following dispatch: a stray symlink is recorded, never traversed.
            let kind = fs::symlink_metadata(&path).unwrap().file_type();
            if kind.is_symlink() {
                files.insert(
                    relative,
                    format!("symlink:{}", fs::read_link(&path).unwrap().display()).into_bytes(),
                );
            } else if kind.is_dir() {
                files.insert(format!("{relative}/"), b"directory".to_vec());
                walk(root, &path, files);
            } else {
                files.insert(relative, fs::read(path).unwrap());
            }
        }
    }
    let mut files = std::collections::BTreeMap::new();
    walk(root, root, &mut files);
    files
}
/// Verify that bootstrap all profiles is read only then provisions all five files with no builds.
#[test]
fn bootstrap_all_profiles_is_read_only_then_provisions_all_five_files_with_no_builds() {
    for profile in ["library", "cli", "service"] {
        let (root, policy) = fixture(profile);
        fs::create_dir_all(root.path().join(".github/workflows")).unwrap();
        fs::write(
            root.path().join(".github/workflows/bespoke.yml"),
            "custom CI",
        )
        .unwrap();
        fs::write(root.path().join("deny.toml"), "custom policy").unwrap();
        let before = snapshot(root.path());
        let plan = inspect(root.path(), policy.path()).unwrap();
        assert_eq!(snapshot(root.path()), before);
        assert_eq!(plan.changes.len(), 5);
        assert!(
            plan.changes
                .iter()
                .all(|change| change.disposition == plan::Disposition::Create)
        );
        assert_eq!(plan.capability_state, "unknown");
        let receipt = apply(root.path(), &plan, &plan.plan_sha256).unwrap();
        assert!(receipt.configured);
        assert!(
            !receipt.ci_verified
                && !receipt.release_rehearsed
                && !receipt.published
                && !receipt.provenance_verified
        );
        for change in &plan.changes {
            assert_eq!(
                fs::read(root.path().join(&change.path)).unwrap(),
                change.proposed_content.as_bytes()
            );
        }
        // Every pre-existing entry must survive unchanged, including type changes
        // (file replaced by a directory) and deletions; new generated files are fine.
        let after_first = snapshot(root.path());
        for (path, bytes) in &before {
            assert_eq!(after_first.get(path), Some(bytes), "{path}");
        }
        let lock = load_lock(root.path(), &plan.config_sha256)
            .unwrap()
            .unwrap()
            .0;
        assert_eq!(lock.workflows.commit, armorer::catalog::WORKFLOW_COMMIT_V2);
        assert_eq!(lock.tools.len(), 18);
        assert!(!root.path().join("BUILD_EXECUTED").exists());
        assert!(!root.path().join("target").exists());
        assert!(!root.path().join("Cargo.lock").exists());
        let after = snapshot(root.path());
        // Capture inode and mtime: an atomic replace can preserve mtime, so bytes and
        // timestamps alone would not prove the no-op replay left the files untouched.
        let modified: Vec<_> = plan
            .changes
            .iter()
            .map(|change| {
                let metadata = fs::metadata(root.path().join(&change.path)).unwrap();
                #[cfg(unix)]
                let inode = {
                    use std::os::unix::fs::MetadataExt;
                    (metadata.dev(), metadata.ino())
                };
                #[cfg(not(unix))]
                let inode = ();
                (inode, metadata.modified().unwrap())
            })
            .collect();
        assert_eq!(
            apply(root.path(), &plan, &plan.plan_sha256)
                .unwrap()
                .outcome,
            "unchanged"
        );
        assert_eq!(snapshot(root.path()), after);
        for (change, (inode, timestamp)) in plan.changes.iter().zip(modified) {
            let metadata = fs::metadata(root.path().join(&change.path)).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                assert_eq!((metadata.dev(), metadata.ino()), inode, "{}", change.path);
            }
            #[cfg(not(unix))]
            let _ = inode;
            assert_eq!(metadata.modified().unwrap(), timestamp, "{}", change.path);
        }
        assert!(
            inspect(root.path(), policy.path())
                .unwrap()
                .changes
                .iter()
                .all(|c| c.disposition == plan::Disposition::Unchanged)
        );
    }
}
/// Verify that every unowned conflict is preserved and every identical file remains unowned.
#[test]
fn every_unowned_conflict_is_preserved_and_every_identical_file_remains_unowned() {
    for target in MANAGED {
        let (root, policy) = fixture("library");
        let initial = inspect(root.path(), policy.path()).unwrap();
        let initial_change = change(&initial, target);
        let path = root.path().join(&initial_change.path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "bespoke λ\r\nno final newline").unwrap();
        let before = snapshot(root.path());
        let conflict = inspect(root.path(), policy.path()).unwrap();
        assert_eq!(
            change(&conflict, target).disposition,
            plan::Disposition::Conflict
        );
        assert_eq!(
            change(&conflict, target).before_content.as_deref(),
            Some("bespoke λ\r\nno final newline")
        );
        assert!(apply(root.path(), &conflict, &conflict.plan_sha256).is_err());
        assert_eq!(snapshot(root.path()), before);
        fs::write(&path, &initial_change.proposed_content).unwrap();
        let plan = inspect(root.path(), policy.path()).unwrap();
        assert_eq!(
            change(&plan, target).disposition,
            plan::Disposition::Unchanged
        );
        assert!(!change(&plan, target).owned);
        apply(root.path(), &plan, &plan.plan_sha256).unwrap();
        let state: serde_json::Value =
            serde_json::from_slice(&fs::read(root.path().join(STATE)).unwrap()).unwrap();
        assert!(!state["managed"].as_object().unwrap().contains_key(target));
    }
}
/// Verify that edits and deletions of each owned file are conflicts without readoption.
#[test]
fn edits_and_deletions_of_each_owned_file_are_conflicts_without_readoption() {
    for target in MANAGED {
        for delete in [false, true] {
            let (root, policy) = fixture("library");
            let plan = inspect(root.path(), policy.path()).unwrap();
            apply(root.path(), &plan, &plan.plan_sha256).unwrap();
            let path = root.path().join(target);
            if delete {
                fs::remove_file(path).unwrap();
            } else {
                fs::write(path, "customization").unwrap();
            }
            let before = snapshot(root.path());
            let fresh = inspect(root.path(), policy.path()).unwrap();
            assert_eq!(
                change(&fresh, target).disposition,
                plan::Disposition::Conflict,
                "{target} {delete}"
            );
            assert!(apply(root.path(), &fresh, &fresh.plan_sha256).is_err());
            assert!(apply(root.path(), &plan, &plan.plan_sha256).is_err());
            assert_eq!(snapshot(root.path()), before);
        }
    }
}
/// Verify that independently supplied digest and strict fields reject forged plan authority.
#[test]
fn independently_supplied_digest_and_strict_fields_reject_forged_plan_authority() {
    let (root, policy) = fixture("library");
    let plan = inspect(root.path(), policy.path()).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    fs::write(file.path(), serde_json::to_vec(&plan).unwrap()).unwrap();
    assert!(load_plan(file.path(), &plan.plan_sha256).is_ok());
    assert!(load_plan(file.path(), &"a".repeat(64)).is_err());
    for mutation in 0..7 {
        let mut value = serde_json::to_value(&plan).unwrap();
        match mutation {
            0 => value["changes"][0]["path"] = "LICENSE".into(),
            1 => value["changes"][0]["proposed_content"] = "unreviewed policy".into(),
            2 => value["catalog_sha256"] = "a".repeat(64).into(),
            3 => value["schema_version"] = 1.into(),
            4 => value["workspace"]["packages"][0]["unknown"] = "DO-NOT-ECHO".into(),
            5 => {
                value.as_object_mut().unwrap().remove("state_sha256");
            }
            6 => value["changes"][0]["unified_diff"] = "concealed changes".into(),
            _ => unreachable!(),
        }
        fs::write(file.path(), serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(load_plan(file.path(), &plan.plan_sha256).is_err());
    }
    let raw = serde_json::to_string(&plan).unwrap().replace(
        "\"preserved_inputs\":{",
        "\"preserved_inputs\":{\"LICENSE\":null,",
    );
    fs::write(file.path(), raw).unwrap();
    assert!(load_plan(file.path(), &plan.plan_sha256).is_err());
    let mut forged = plan.clone();
    forged.changes[1].path = "LICENSE".into();
    forged.plan_sha256 = forged.digest().unwrap();
    assert!(apply(root.path(), &forged, &forged.plan_sha256).is_err());
    assert!(!root.path().join(".armorer").exists());
}
/// Verify that config manifest license lock and inventory preimages are rechecked.
#[test]
fn config_manifest_license_lock_and_inventory_preimages_are_rechecked() {
    for input in [
        "armorer.toml",
        "Cargo.toml",
        "LICENSE",
        "Cargo.lock",
        "src/bin/new.rs",
        "armorer.lock",
    ] {
        let (root, policy) = fixture("library");
        let plan = inspect(root.path(), policy.path()).unwrap();
        let path = root.path().join(input);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = fs::read_to_string(&path).unwrap_or_default();
        let content = match input {
            "Cargo.lock" => "version = 4\n".into(),
            "src/bin/new.rs" => "fn main() {}".into(),
            _ => format!("{original}\n# changed input\n"),
        };
        fs::write(path, content).unwrap();
        let before = snapshot(root.path());
        assert!(
            apply(root.path(), &plan, &plan.plan_sha256).is_err(),
            "{input}"
        );
        assert_eq!(snapshot(root.path()), before);
    }
}
/// Verify that exact policy bytes are frozen after review and invalid policies never write.
#[test]
fn exact_policy_bytes_are_frozen_after_review_and_invalid_policies_never_write() {
    let (root, policy) = fixture("library");
    let plan = inspect(root.path(), policy.path()).unwrap();
    fs::write(policy.path(), "[invalid] arbitrary = true").unwrap();
    assert!(inspect(root.path(), policy.path()).is_err());
    // Approval binds the displayed exact policy, not a subsequently replaced external file.
    apply(root.path(), &plan, &plan.plan_sha256).unwrap();
    assert_eq!(
        fs::read(root.path().join(".armorer/ci-policy.toml")).unwrap(),
        POLICY.as_bytes()
    );
    let (root, policy) = fixture("library");
    fs::write(policy.path(), "schema_version = 1").unwrap();
    let before = snapshot(root.path());
    assert!(inspect(root.path(), policy.path()).is_err());
    assert_eq!(snapshot(root.path()), before);
}
/// Verify that kernel lock serializes versions and v1 state requires reviewed migration.
#[test]
fn kernel_lock_serializes_versions_and_v1_state_requires_reviewed_migration() {
    let (root, policy) = fixture("library");
    let bootstrap = inspect(root.path(), policy.path()).unwrap();
    let v1 = plan::inspect(root.path(), "plan").unwrap();
    fs::create_dir(root.path().join(".armorer")).unwrap();
    let guard = fs::File::create(root.path().join(".armorer/apply.lock")).unwrap();
    guard.try_lock().unwrap();
    assert!(
        apply(root.path(), &bootstrap, &bootstrap.plan_sha256)
            .unwrap_err()
            .to_string()
            .contains("another Armorer")
    );
    assert!(armorer::apply::apply(root.path(), &v1, &v1.plan_sha256).is_err());
    drop(guard);
    armorer::apply::apply(root.path(), &v1, &v1.plan_sha256).unwrap();
    let before = snapshot(root.path());
    assert!(
        apply(root.path(), &bootstrap, &bootstrap.plan_sha256)
            .unwrap_err()
            .to_string()
            .contains("migration")
    );
    assert_eq!(snapshot(root.path()), before);
    let (root, policy) = fixture("library");
    let bootstrap = inspect(root.path(), policy.path()).unwrap();
    let v1 = plan::inspect(root.path(), "plan").unwrap();
    apply(root.path(), &bootstrap, &bootstrap.plan_sha256).unwrap();
    let before = snapshot(root.path());
    assert!(armorer::apply::apply(root.path(), &v1, &v1.plan_sha256).is_err());
    assert_eq!(snapshot(root.path()), before);
}
/// Verify that legacy toolchain and v1 journal are preserved and block provisioning.
#[test]
fn legacy_toolchain_and_v1_journal_are_preserved_and_block_provisioning() {
    for input in ["rust-toolchain", ".armorer/journal.json"] {
        let (root, policy) = fixture("library");
        let path = root.path().join(input);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "legacy bytes").unwrap();
        let plan = inspect(root.path(), policy.path()).unwrap();
        let before = snapshot(root.path());
        if input == "rust-toolchain" {
            assert_eq!(
                plan.preserved_inputs[input].as_deref(),
                Some("legacy bytes")
            );
        }
        assert!(apply(root.path(), &plan, &plan.plan_sha256).is_err());
        assert_eq!(snapshot(root.path()), before);
    }
}
/// Verify that symlink parents targets and control files do not redirect writes.
#[cfg(unix)]
#[test]
fn symlink_parents_targets_and_control_files_do_not_redirect_writes() {
    for input in [".github", ".armorer", "armorer.lock", "rust-toolchain.toml"] {
        let (root, policy) = fixture("library");
        let plan = inspect(root.path(), policy.path()).unwrap();
        let outside = tempfile::tempdir().unwrap();
        let destination = outside.path().join("preserve");
        fs::write(&destination, "outside bytes").unwrap();
        let link = if input.starts_with('.') {
            outside.path()
        } else {
            &destination
        };
        std::os::unix::fs::symlink(link, root.path().join(input)).unwrap();
        assert!(apply(root.path(), &plan, &plan.plan_sha256).is_err());
        assert_eq!(fs::read(destination).unwrap(), b"outside bytes");
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 1);
    }
}
/// Verify that independently applied complete diffs reconstruct every target.
#[cfg(unix)]
#[test]
fn independently_applied_complete_diffs_reconstruct_every_target() {
    let (root, policy) = fixture("library");
    let initial = inspect(root.path(), policy.path()).unwrap();
    for change in &initial.changes {
        let path = root.path().join(&change.path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "bespoke λ\r\nno final newline").unwrap();
    }
    let conflict = inspect(root.path(), policy.path()).unwrap();
    for plan in [initial, conflict] {
        let output = tempfile::tempdir().unwrap();
        for change in &plan.changes {
            let path = output.path().join(&change.path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            if let Some(before) = &change.before_content {
                fs::write(path, before).unwrap();
            }
            let mut patch = Command::new("patch")
                .current_dir(output.path())
                .args(["-p1", "--batch"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            patch
                .stdin
                .take()
                .unwrap()
                .write_all(change.unified_diff.as_bytes())
                .unwrap();
            let result = patch.wait_with_output().unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stdout)
            );
            assert_eq!(
                fs::read(output.path().join(&change.path)).unwrap(),
                change.proposed_content.as_bytes()
            );
        }
    }
}
/// Verify that cli requires explicit policy approval and keeps v1 plan separate.
#[test]
fn cli_requires_explicit_policy_approval_and_keeps_v1_plan_separate() {
    let (root, policy) = fixture("library");
    let cli = || {
        let mut c = Command::new(env!("CARGO_BIN_EXE_armorer"));
        c.args(["--repository"]).arg(root.path());
        c
    };
    assert!(
        !cli()
            .args(["bootstrap", "plan"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let before = snapshot(root.path());
    let result = cli()
        .args(["bootstrap", "check", "--policy"])
        .arg(policy.path())
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(snapshot(root.path()), before);
    let result = cli()
        .args(["bootstrap", "plan", "--policy"])
        .arg(policy.path())
        .output()
        .unwrap();
    assert!(result.status.success());
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["schema_version"], 2);
    let file = tempfile::NamedTempFile::new().unwrap();
    fs::write(file.path(), result.stdout).unwrap();
    assert!(
        !cli()
            .args(["bootstrap", "apply", "--plan"])
            .arg(file.path())
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        !cli()
            .args(["apply", "--plan"])
            .arg(file.path())
            .args([
                "--expect-plan-sha256",
                value["plan_sha256"].as_str().unwrap()
            ])
            .output()
            .unwrap()
            .status
            .success()
    );
    let result = cli()
        .args(["bootstrap", "apply", "--plan"])
        .arg(file.path())
        .args([
            "--expect-plan-sha256",
            value["plan_sha256"].as_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(receipt["configured"], true);
    assert_eq!(receipt["provenance_verified"], false);
    let v1 = cli().arg("plan").output().unwrap();
    let v1: serde_json::Value = serde_json::from_slice(&v1.stdout).unwrap();
    assert_eq!(v1["schema_version"], 1);
    assert_eq!(v1["changes"].as_array().unwrap().len(), 1);
}
/// Verify that oversized escaped or non utf8 conflicts fail without truncation or writes.
#[test]
fn oversized_escaped_or_non_utf8_conflicts_fail_without_truncation_or_writes() {
    for bytes in [vec![0xff], vec![b'\t'; 300_000]] {
        let (root, policy) = fixture("library");
        fs::write(root.path().join("rust-toolchain.toml"), &bytes).unwrap();
        let before = snapshot(root.path());
        assert!(inspect(root.path(), policy.path()).is_err());
        assert_eq!(snapshot(root.path()), before);
    }
}

/// Verify that replay rejects removed adopted or substituted ownership receipts.
#[test]
fn replay_rejects_removed_adopted_or_substituted_ownership_receipts() {
    for mutation in 0..6 {
        let (root, policy) = fixture("library");
        let initial = inspect(root.path(), policy.path()).unwrap();
        if mutation == 1 {
            let path = root.path().join(&initial.changes[4].path);
            fs::write(path, &initial.changes[4].proposed_content).unwrap();
        }
        let plan = inspect(root.path(), policy.path()).unwrap();
        apply(root.path(), &plan, &plan.plan_sha256).unwrap();
        let state_path = root.path().join(STATE);
        let mut state: serde_json::Value =
            serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
        match mutation {
            0 => {
                state["managed"]
                    .as_object_mut()
                    .unwrap()
                    .remove(&plan.changes[0].path);
            }
            1 => {
                let c = &plan.changes[4];
                state["managed"][&c.path] =
                    serde_json::json!({"content": c.proposed_content, "sha256": c.after_sha256});
            }
            2 => state["catalog_sha256"] = "a".repeat(64).into(),
            3 => state["runtime_version"] = "0.0.1".into(),
            4 => state["workflow"]["repository"] = "example/substitution".into(),
            5 => state["workflow"]["commit"] = "a".repeat(40).into(),
            _ => unreachable!(),
        }
        fs::write(state_path, serde_json::to_vec(&state).unwrap()).unwrap();
        let before = snapshot(root.path());
        assert!(apply(root.path(), &plan, &plan.plan_sha256).is_err());
        assert_eq!(snapshot(root.path()), before);
    }
}
/// Verify that special managed inputs are rejected before opening or mutation.
#[cfg(unix)]
#[test]
fn special_managed_inputs_are_rejected_before_opening_or_mutation() {
    let (root, policy) = fixture("library");
    let plan = inspect(root.path(), policy.path()).unwrap();
    let result = Command::new("mkfifo")
        .arg(root.path().join("armorer.lock"))
        .status()
        .unwrap();
    assert!(result.success());
    assert!(apply(root.path(), &plan, &plan.plan_sha256).is_err());
    assert!(inspect(root.path(), policy.path()).is_err());
    assert!(!root.path().join(".armorer").exists());
}
/// Verify that committed schemas and all profile previews match current runtime.
#[test]
fn committed_schemas_and_all_profile_previews_match_current_runtime() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/bootstrap-plan-v2.json")).unwrap();
    assert_eq!(
        serde_json::to_value(schemars::schema_for!(armorer::bootstrap::Plan)).unwrap(),
        schema
    );
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/ci-policy-v1.json")).unwrap();
    assert_eq!(
        serde_json::to_value(schemars::schema_for!(armorer::ci_policy::CiPolicy)).unwrap(),
        schema
    );
    for profile in ["library", "cli", "service"] {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/bootstrap-v2")
            .join(profile);
        let expected: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("bootstrap-plan.json")).unwrap()).unwrap();
        let before = snapshot(&root);
        let plan = inspect(&root, &root.join("ci-policy.reviewed.toml")).unwrap();
        assert_eq!(serde_json::to_value(plan).unwrap(), expected, "{profile}");
        assert_eq!(snapshot(&root), before);
    }
}

/// Verify that a brand-new bootstrap defaults to the reviewed v2 successor authority.
#[test]
fn new_bootstrap_defaults_to_the_v2_successor_authority() {
    let (root, policy) = fixture("library");
    assert!(!root.path().join(STATE).exists());
    let plan = inspect(root.path(), policy.path()).unwrap();
    assert_eq!(plan.catalog_id, "bootstrap-v2");
    let lock = plan
        .changes
        .iter()
        .find(|c| c.path == "armorer.lock")
        .unwrap();
    assert!(
        lock.proposed_content
            .contains(armorer::catalog::WORKFLOW_COMMIT_V2)
    );
    assert!(
        !lock
            .proposed_content
            .contains(armorer::catalog::WORKFLOW_COMMIT)
    );
    for path in [
        ".github/workflows/armorer-ci.yml",
        ".github/workflows/armorer-build.yml",
    ] {
        let change = plan.changes.iter().find(|c| c.path == path).unwrap();
        assert!(
            change
                .proposed_content
                .contains(&format!("@{}", armorer::catalog::WORKFLOW_COMMIT_V2))
        );
    }
}

/// `bootstrap check` is a drift detector: 0 only when every managed file holds the
/// generated bytes and no setup finding is open, 2 with reasons on stderr otherwise.
/// Release readiness is a separate, explicit and currently always-failing gate.
#[test]
fn cli_check_exit_status_reports_drift_separately_from_release_readiness() {
    let (root, policy) = fixture("library");
    let check = |extra: &[&str]| {
        let before = snapshot(root.path());
        let output = Command::new(env!("CARGO_BIN_EXE_armorer"))
            .arg("--repository")
            .arg(root.path())
            .args(["bootstrap", "check", "--policy"])
            .arg(policy.path())
            .args(extra)
            .output()
            .unwrap();
        // Check never writes, whatever it reports.
        assert_eq!(snapshot(root.path()), before);
        let plan: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(plan["schema_version"], 2);
        (
            output.status.code().unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        )
    };

    // Nothing provisioned: every missing file and the missing Cargo.lock are drift.
    let (status, reasons) = check(&[]);
    assert_eq!(status, 2);
    assert!(reasons.contains("create:armorer.lock"), "{reasons}");
    assert!(reasons.contains("cargo-lock-missing"), "{reasons}");
    assert!(inspect(root.path(), policy.path()).unwrap().drift().len() >= 5);

    let plan = inspect(root.path(), policy.path()).unwrap();
    apply(root.path(), &plan, &plan.plan_sha256).unwrap();

    // Provisioned files alone are not enough while a setup finding stays open.
    let (status, reasons) = check(&[]);
    assert_eq!(status, 2);
    assert_eq!(
        reasons.trim(),
        "armorer: provisioning drift: cargo-lock-missing"
    );
    fs::write(
        root.path().join("Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"pilot\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    // In sync: open release gates are still listed in the plan but do not fail check.
    let (status, reasons) = check(&[]);
    assert_eq!((status, reasons.as_str()), (0, ""));
    let synced = inspect(root.path(), policy.path()).unwrap();
    assert!(synced.drift().is_empty());
    assert!(!synced.findings.is_empty());

    // The explicit release gate fails closed and names what is missing.
    let (status, reasons) = check(&["--release-ready"]);
    assert_eq!(status, 2);
    assert!(
        reasons.contains("release readiness not established")
            && reasons.contains("release-runtime-unavailable")
            && reasons.contains("capability-state-unknown"),
        "{reasons}"
    );
    assert!(!synced.release_blockers().is_empty());

    // An edited or deleted owned file is drift; restoring the bytes clears it.
    let workflow = root.path().join(".github/workflows/armorer-ci.yml");
    let original = fs::read(&workflow).unwrap();
    fs::write(&workflow, b"name: edited\n").unwrap();
    let (status, reasons) = check(&[]);
    assert_eq!(status, 2);
    assert!(
        reasons.contains("conflict:.github/workflows/armorer-ci.yml"),
        "{reasons}"
    );
    fs::remove_file(&workflow).unwrap();
    assert_eq!(check(&[]).0, 2);
    fs::write(&workflow, original).unwrap();
    assert_eq!(check(&[]).0, 0);
}
