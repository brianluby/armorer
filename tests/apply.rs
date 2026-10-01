use armorer::{
    apply::{apply, load_plan},
    plan::inspect,
};
use std::{fs, path::Path, process::Command};

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    fs::write(
        root.path().join("src/lib.rs"),
        "compile_error!(\"must not build\");",
    )
    .unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"pilot\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(root.path().join("LICENSE"), "MIT\n").unwrap();
    fs::write(
        root.path().join("armorer.toml"),
        r#"schema_version = 1
repository = "example/pilot"
toolchain = "1.95.0"
[[deliverables]]
id = "library"
profile = "library"
package = "pilot"
targets = ["x86_64-unknown-linux-gnu"]
feature_set = "standard"
[feature_sets.standard]
default_features = true
features = []
[policy]
license_file = "LICENSE"
attestations = "required"
"#,
    )
    .unwrap();
    root
}

fn bytes(root: &Path, path: &str) -> Vec<u8> {
    fs::read(root.join(path)).unwrap()
}

#[test]
fn applies_approved_bytes_and_same_plan_replay_changes_nothing() {
    let root = fixture();
    fs::create_dir_all(root.path().join(".github/workflows")).unwrap();
    fs::write(
        root.path().join(".github/workflows/custom.yml"),
        "custom CI",
    )
    .unwrap();
    fs::write(root.path().join("deny.toml"), "custom license policy").unwrap();
    let plan = inspect(root.path(), "plan").unwrap();
    let receipt = apply(root.path(), &plan, &plan.plan_sha256).unwrap();
    assert_eq!(receipt.outcome, "applied");
    assert!(!receipt.configured);
    assert!(receipt.toolchain_configured);
    assert!(
        !receipt.ci_verified
            && !receipt.release_rehearsed
            && !receipt.published
            && !receipt.provenance_verified
    );
    assert_eq!(
        bytes(root.path(), "rust-toolchain.toml"),
        plan.changes[0].proposed_content.as_bytes()
    );
    let state = bytes(root.path(), ".armorer/state.json");
    let modified = fs::metadata(root.path().join("rust-toolchain.toml"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(
        apply(root.path(), &plan, &plan.plan_sha256)
            .unwrap()
            .outcome,
        "unchanged"
    );
    assert_eq!(bytes(root.path(), ".armorer/state.json"), state);
    assert_eq!(
        fs::metadata(root.path().join("rust-toolchain.toml"))
            .unwrap()
            .modified()
            .unwrap(),
        modified
    );
    assert_eq!(
        bytes(root.path(), ".github/workflows/custom.yml"),
        b"custom CI"
    );
    assert_eq!(bytes(root.path(), "deny.toml"), b"custom license policy");
    assert!(!root.path().join("Cargo.lock").exists());
    assert!(!root.path().join("target").exists());
    assert!(!root.path().join(".armorer/journal.json").exists());
}

#[test]
fn plan_file_rejects_tampering_wrong_digest_and_unknown_fields() {
    let root = fixture();
    let plan = inspect(root.path(), "plan").unwrap();
    let output = root.path().join("approved.json");
    let mut value = serde_json::to_value(&plan).unwrap();
    fs::write(&output, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(load_plan(&output, &plan.plan_sha256).is_ok());
    assert!(load_plan(&output, &"a".repeat(64)).is_err());
    value["changes"][0]["proposed_content"] = "arbitrary data".into();
    fs::write(&output, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(load_plan(&output, &plan.plan_sha256).is_err());
    value = serde_json::to_value(&plan).unwrap();
    value["shell"] = "DO-NOT-ECHO".into();
    fs::write(&output, serde_json::to_vec(&value).unwrap()).unwrap();
    let error = load_plan(&output, &plan.plan_sha256).unwrap_err();
    assert!(!error.to_string().contains("DO-NOT-ECHO"));
    assert!(!root.path().join(".armorer").exists());
}

#[test]
fn rejects_stale_config_manifest_license_inventory_and_lock_absence() {
    for input in [
        "armorer.toml",
        "Cargo.toml",
        "LICENSE",
        "Cargo.lock",
        "src/bin/new.rs",
    ] {
        let root = fixture();
        let plan = inspect(root.path(), "plan").unwrap();
        let path = root.path().join(input);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = fs::read_to_string(&path).unwrap_or_default();
        let content = if input == "Cargo.lock" {
            "version = 4\n".to_owned()
        } else if input.ends_with(".rs") {
            "fn main() {}".to_owned()
        } else {
            format!("{original}\n# changed input\n")
        };
        fs::write(path, content).unwrap();
        assert!(
            apply(root.path(), &plan, &plan.plan_sha256).is_err(),
            "{input}"
        );
        assert!(!root.path().join("rust-toolchain.toml").exists());
        assert!(!root.path().join(".armorer").exists());
    }
}

#[test]
fn preserves_conflicts_and_does_not_adopt_identical_existing_files() {
    let root = fixture();
    let plan = inspect(root.path(), "plan").unwrap();
    fs::write(
        root.path().join("rust-toolchain.toml"),
        &plan.changes[0].proposed_content,
    )
    .unwrap();
    let plan = inspect(root.path(), "plan").unwrap();
    apply(root.path(), &plan, &plan.plan_sha256).unwrap();
    let state: serde_json::Value =
        serde_json::from_slice(&bytes(root.path(), ".armorer/state.json")).unwrap();
    assert!(state["managed"].as_object().unwrap().is_empty());
    fs::write(
        root.path().join("rust-toolchain.toml"),
        "[toolchain]\nchannel = \"stable\"\n",
    )
    .unwrap();
    let conflict = inspect(root.path(), "plan").unwrap();
    assert_eq!(conflict.changes[0].disposition, "conflict");
    assert!(apply(root.path(), &conflict, &conflict.plan_sha256).is_err());
    assert_eq!(
        bytes(root.path(), "rust-toolchain.toml"),
        b"[toolchain]\nchannel = \"stable\"\n"
    );
}

#[test]
fn rejects_preimage_changes_and_edited_owned_files() {
    let root = fixture();
    let plan = inspect(root.path(), "plan").unwrap();
    fs::write(root.path().join("rust-toolchain.toml"), "customization").unwrap();
    assert!(apply(root.path(), &plan, &plan.plan_sha256).is_err());
    fs::remove_file(root.path().join("rust-toolchain.toml")).unwrap();
    apply(root.path(), &plan, &plan.plan_sha256).unwrap();
    fs::write(root.path().join("rust-toolchain.toml"), "intervening edit").unwrap();
    assert!(apply(root.path(), &plan, &plan.plan_sha256).is_err());
    assert_eq!(
        inspect(root.path(), "plan").unwrap().changes[0].disposition,
        "conflict"
    );
}

#[test]
fn concurrent_kernel_lock_blocks_apply_and_releases_on_close() {
    let root = fixture();
    let plan = inspect(root.path(), "plan").unwrap();
    fs::create_dir(root.path().join(".armorer")).unwrap();
    let guard = fs::File::create(root.path().join(".armorer/apply.lock")).unwrap();
    guard.try_lock().unwrap();
    assert!(
        apply(root.path(), &plan, &plan.plan_sha256)
            .unwrap_err()
            .to_string()
            .contains("another Armorer")
    );
    assert!(!root.path().join("rust-toolchain.toml").exists());
    drop(guard);
    assert_eq!(
        apply(root.path(), &plan, &plan.plan_sha256)
            .unwrap()
            .outcome,
        "applied"
    );
}

#[cfg(unix)]
#[test]
fn rejects_symlink_control_directory_and_targets_without_following() {
    let root = fixture();
    let outside = tempfile::tempdir().unwrap();
    let plan = inspect(root.path(), "plan").unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join(".armorer")).unwrap();
    assert!(apply(root.path(), &plan, &plan.plan_sha256).is_err());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    fs::remove_file(root.path().join(".armorer")).unwrap();
    fs::write(outside.path().join("toolchain"), "outside customization").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("toolchain"),
        root.path().join("rust-toolchain.toml"),
    )
    .unwrap();
    assert!(apply(root.path(), &plan, &plan.plan_sha256).is_err());
    assert_eq!(bytes(outside.path(), "toolchain"), b"outside customization");
}

#[test]
fn cli_requires_approval_and_emits_honest_receipt() {
    let root = fixture();
    let plan = inspect(root.path(), "plan").unwrap();
    let output = root.path().join("approved.json");
    fs::write(&output, serde_json::to_vec(&plan).unwrap()).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_armorer"))
        .arg("--repository")
        .arg(root.path())
        .args(["apply", "--plan"])
        .arg(&output)
        .args(["--expect-plan-sha256", &plan.plan_sha256])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(receipt["outcome"], "applied");
    assert_eq!(receipt["provenance_verified"], false);
}

#[test]
fn replay_rechecks_derived_intent_and_receipt_ownership() {
    let root = fixture();
    let plan = inspect(root.path(), "plan").unwrap();
    apply(root.path(), &plan, &plan.plan_sha256).unwrap();
    let state_path = root.path().join(".armorer/state.json");
    let original: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    let mut state = original.clone();
    let mut forged = plan.clone();
    forged.repository = "example/forged".into();
    forged.intent.repository = "example/forged".into();
    forged.plan_sha256 = forged.digest().unwrap();
    state["last_plan_sha256"] = forged.plan_sha256.clone().into();
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    assert!(apply(root.path(), &forged, &forged.plan_sha256).is_err());
    state = original;
    state["managed"] = serde_json::json!({});
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    assert!(apply(root.path(), &plan, &plan.plan_sha256).is_err());
}

#[test]
fn strict_plan_rejects_nested_extras_and_duplicate_map_keys() {
    let root = fixture();
    let plan = inspect(root.path(), "plan").unwrap();
    let output = root.path().join("approved.json");
    let mut value = serde_json::to_value(&plan).unwrap();
    value["workspace"]["packages"][0]["secret_extra"] = "DO-NOT-ECHO".into();
    fs::write(&output, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(load_plan(&output, &plan.plan_sha256).is_err());
    let raw = serde_json::to_string(&plan).unwrap().replace(
        "\"input_preimages\":{",
        "\"input_preimages\":{\"LICENSE\":null,",
    );
    fs::write(&output, raw).unwrap();
    assert!(load_plan(&output, &plan.plan_sha256).is_err());
}
