use armorer::{
    apply::apply,
    plan::inspect,
    preview::{preview, preview_plan},
};
use std::{fs, path::Path, process::Command};

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    fs::write(
        root.path().join("src/lib.rs"),
        "compile_error!(\"preview must not compile source\");",
    )
    .unwrap();
    fs::write(root.path().join("build.rs"), format!("fn main() {{ std::fs::write({:?}, \"executed\").unwrap(); panic!(\"must not run\"); }}", root.path().join("build-executed"))).unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"pilot\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(root.path().join("LICENSE"), "MIT").unwrap();
    fs::write(root.path().join("armorer.toml"), "schema_version = 1\nrepository = \"example/pilot\"\ntoolchain = \"1.95.0\"\n[[deliverables]]\nid = \"library\"\nprofile = \"library\"\npackage = \"pilot\"\ntargets = [\"x86_64-unknown-linux-gnu\"]\nfeature_set = \"standard\"\n[feature_sets.standard]\ndefault_features = true\nfeatures = []\n[policy]\nlicense_file = \"LICENSE\"\nattestations = \"required\"\n").unwrap();
    root
}

fn snapshot(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, path: &Path, output: &mut std::collections::BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, output);
            } else {
                output.insert(
                    path.strip_prefix(root).unwrap().to_string_lossy().into(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut output = std::collections::BTreeMap::new();
    walk(root, root, &mut output);
    output
}

#[test]
fn creation_is_exact_deterministic_read_only_and_keeps_v1_plan_identity() {
    let root = fixture();
    fs::write(root.path().join("deny.toml"), "bespoke policy").unwrap();
    let before = snapshot(root.path());
    let result = preview(root.path()).unwrap();
    let plan = inspect(root.path(), "plan").unwrap();
    assert_eq!(
        serde_json::to_value(&result.plan).unwrap(),
        serde_json::to_value(plan).unwrap()
    );
    assert_eq!(result.files[0].disposition, "create");
    assert_eq!(result.files[0].before_content, None);
    assert_eq!(
        result.files[0].proposed_content,
        result.plan.changes[0].proposed_content
    );
    assert!(result.files[0].unified_diff.starts_with("--- /dev/null\n"));
    assert_eq!(
        serde_json::to_vec(&result).unwrap(),
        serde_json::to_vec(&preview(root.path()).unwrap()).unwrap()
    );
    assert_eq!(snapshot(root.path()), before);
    assert!(!root.path().join("target").exists());
    assert!(!root.path().join("build-executed").exists());
    assert!(!root.path().join(".armorer").exists());
    assert_eq!(result.plan.capability_state, "unknown");
}

#[test]
fn conflicts_show_all_custom_bytes_and_block_apply() {
    let root = fixture();
    let custom = "# λ custom\r\n[toolchain]\r\nchannel = \"stable\"\r\ncomponents = [\"rust-src\"]";
    fs::write(root.path().join("rust-toolchain.toml"), custom).unwrap();
    let before = snapshot(root.path());
    let result = preview(root.path()).unwrap();
    assert_eq!(result.files[0].disposition, "conflict");
    assert_eq!(result.files[0].before_content.as_deref(), Some(custom));
    assert!(result.files[0].unified_diff.contains("-# λ custom\r\n"));
    assert!(
        result.files[0]
            .unified_diff
            .contains("\\ No newline at end of file")
    );
    assert!(apply(root.path(), &result.plan, &result.plan.plan_sha256).is_err());
    assert_eq!(snapshot(root.path()), before);
}

#[test]
fn owned_update_and_unchanged_views_preserve_ownership_rules() {
    let root = fixture();
    let plan = inspect(root.path(), "plan").unwrap();
    apply(root.path(), &plan, &plan.plan_sha256).unwrap();
    let unchanged = preview(root.path()).unwrap();
    assert_eq!(unchanged.files[0].disposition, "unchanged");
    assert_eq!(unchanged.files[0].unified_diff, "");
    assert_eq!(
        unchanged.files[0].before_content.as_deref(),
        Some(plan.changes[0].proposed_content.as_str())
    );
    let old = plan.changes[0]
        .proposed_content
        .replace("minimal", "default");
    let state_path = root.path().join(".armorer/state.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    use sha2::{Digest, Sha256};
    state["managed"]["rust-toolchain.toml"]["content"] = old.clone().into();
    state["managed"]["rust-toolchain.toml"]["sha256"] =
        format!("{:x}", Sha256::digest(old.as_bytes())).into();
    fs::write(state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    fs::write(root.path().join("rust-toolchain.toml"), &old).unwrap();
    let before = snapshot(root.path());
    let result = preview(root.path()).unwrap();
    assert_eq!(result.files[0].disposition, "update");
    assert_eq!(
        result.files[0].before_content.as_deref(),
        Some(old.as_str())
    );
    assert!(
        result.files[0]
            .unified_diff
            .contains("-profile = \"default\"\n")
    );
    assert!(
        result.files[0]
            .unified_diff
            .contains("+profile = \"minimal\"\n")
    );
    assert_eq!(snapshot(root.path()), before);
}

#[test]
fn legacy_toolchain_is_visible_even_when_toml_has_no_diff() {
    let root = fixture();
    let plan = inspect(root.path(), "plan").unwrap();
    fs::write(
        root.path().join("rust-toolchain.toml"),
        &plan.changes[0].proposed_content,
    )
    .unwrap();
    fs::write(
        root.path().join("rust-toolchain"),
        "stable\n# keep this customization",
    )
    .unwrap();
    let before = snapshot(root.path());
    let result = preview(root.path()).unwrap();
    assert_eq!(result.files[0].disposition, "conflict");
    assert_eq!(result.files[0].unified_diff, "");
    assert_eq!(result.preserved_inputs[0].path, "rust-toolchain");
    assert_eq!(
        result.preserved_inputs[0].content,
        "stable\n# keep this customization"
    );
    assert!(apply(root.path(), &result.plan, &result.plan.plan_sha256).is_err());
    assert_eq!(snapshot(root.path()), before);
}

#[test]
fn saved_plan_preview_rejects_stale_input_without_mutation() {
    for input in [
        "armorer.toml",
        "LICENSE",
        "Cargo.toml",
        "rust-toolchain.toml",
        "rust-toolchain",
    ] {
        let root = fixture();
        let plan = inspect(root.path(), "plan").unwrap();
        assert!(preview_plan(root.path(), &plan).is_ok());
        let path = root.path().join(input);
        let old = fs::read_to_string(&path).unwrap_or_default();
        fs::write(path, format!("{old}\n# changed\n")).unwrap();
        let before = snapshot(root.path());
        assert!(preview_plan(root.path(), &plan).is_err(), "{input}");
        assert_eq!(snapshot(root.path()), before);
    }
}

#[test]
fn invalid_utf8_and_oversized_review_views_fail_without_partial_output() {
    for content in [vec![0xff, 0xfe], vec![b'x'; 600_000], vec![b'x'; 1_048_577]] {
        let root = fixture();
        fs::write(root.path().join("rust-toolchain.toml"), content).unwrap();
        let before = snapshot(root.path());
        let output = Command::new(env!("CARGO_BIN_EXE_armorer"))
            .arg("--repository")
            .arg(root.path())
            .arg("preview")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(value.get("error").is_some());
        assert!(value.get("files").is_none());
        assert_eq!(snapshot(root.path()), before);
    }
}

#[cfg(unix)]
#[test]
fn symlink_inputs_fail_without_reading_the_destination() {
    let root = fixture();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("input"), "DO-NOT-ECHO").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("input"),
        root.path().join("rust-toolchain.toml"),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_armorer"))
        .arg("--repository")
        .arg(root.path())
        .arg("preview")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("DO-NOT-ECHO"));
    assert_eq!(
        fs::read(outside.path().join("input")).unwrap(),
        b"DO-NOT-ECHO"
    );
}

#[test]
fn cli_saved_preview_requires_both_digest_and_plan_and_rejects_forgery() {
    let root = fixture();
    let plan = inspect(root.path(), "plan").unwrap();
    let saved = tempfile::NamedTempFile::new().unwrap();
    fs::write(saved.path(), serde_json::to_vec(&plan).unwrap()).unwrap();
    let run = |digest: Option<&str>| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_armorer"));
        command
            .arg("--repository")
            .arg(root.path())
            .args(["preview", "--plan"])
            .arg(saved.path());
        if let Some(digest) = digest {
            command.args(["--expect-plan-sha256", digest]);
        }
        command.output().unwrap()
    };
    assert!(!run(None).status.success());
    assert!(!run(Some(&"a".repeat(64))).status.success());
    let valid = run(Some(&plan.plan_sha256));
    assert!(valid.status.success());
    let result: serde_json::Value = serde_json::from_slice(&valid.stdout).unwrap();
    assert_eq!(result["plan"]["plan_sha256"], plan.plan_sha256);
    let mut tampered = serde_json::to_value(&plan).unwrap();
    tampered["changes"][0]["proposed_content"] = "forged".into();
    fs::write(saved.path(), serde_json::to_vec(&tampered).unwrap()).unwrap();
    assert!(!run(Some(&plan.plan_sha256)).status.success());
    assert!(!root.path().join(".armorer").exists());
}

#[cfg(unix)]
#[test]
fn independent_patch_reconstructs_exact_proposed_bytes() {
    for before in [
        None,
        Some(""),
        Some("[toolchain]\nchannel = \"stable\"\n"),
        Some("# λ custom\r\nno final newline"),
    ] {
        let root = fixture();
        if let Some(before) = before {
            fs::write(root.path().join("rust-toolchain.toml"), before).unwrap();
        }
        let result = preview(root.path()).unwrap();
        let reconstruction = tempfile::tempdir().unwrap();
        if let Some(before) = before {
            fs::write(reconstruction.path().join("rust-toolchain.toml"), before).unwrap();
        }
        let patch = tempfile::NamedTempFile::new().unwrap();
        fs::write(patch.path(), &result.files[0].unified_diff).unwrap();
        let output = Command::new("/usr/bin/patch")
            .current_dir(reconstruction.path())
            .args(["--batch", "-p1", "-i"])
            .arg(patch.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read(reconstruction.path().join("rust-toolchain.toml")).unwrap(),
            result.files[0].proposed_content.as_bytes()
        );
    }
}

#[test]
fn plan_preview_alias_matches_the_exact_view_and_default_plan_is_unchanged() {
    let root = fixture();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_armorer"))
            .arg("--repository")
            .arg(root.path())
            .args(args)
            .output()
            .unwrap()
    };
    let alias = run(&["plan", "--preview"]);
    let direct = run(&["preview"]);
    assert!(alias.status.success() && direct.status.success());
    assert_eq!(alias.stdout, direct.stdout);
    let default = run(&["plan"]);
    assert!(default.status.success());
    let plan: serde_json::Value = serde_json::from_slice(&default.stdout).unwrap();
    let view: serde_json::Value = serde_json::from_slice(&alias.stdout).unwrap();
    assert_eq!(view["plan"], plan);
    assert!(plan.get("files").is_none());
}
