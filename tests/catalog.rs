use armorer::{
    catalog::{WORKFLOW_COMMIT, WORKFLOW_REPOSITORY, authenticate, reviewed},
    config::{TARGETS, load_config, load_lock},
};
use std::{fs, process::Command};

const TOOLS: &[u8] = include_bytes!("../catalogs/bootstrap-tools-v1.json");
fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("armorer.toml"), "schema_version = 1\nrepository = \"example/pilot\"\ntoolchain = \"1.95.0\"\n[[deliverables]]\nid = \"library\"\nprofile = \"library\"\npackage = \"pilot\"\ntargets = [\"x86_64-unknown-linux-gnu\", \"aarch64-unknown-linux-gnu\", \"aarch64-apple-darwin\"]\nfeature_set = \"standard\"\n[feature_sets.standard]\ndefault_features = true\nfeatures = []\n[policy]\nlicense_file = \"LICENSE\"\nattestations = \"required\"\n").unwrap();
    root
}

#[test]
fn substituted_catalogs_fail_even_with_valid_schema_and_matching_claimed_digest() {
    let mut candidate: serde_json::Value = serde_json::from_slice(TOOLS).unwrap();
    candidate["tools"]["cargo-deny"]["platforms"]["aarch64-apple-darwin"]["sha256"] =
        "a".repeat(64).into();
    assert!(authenticate(&serde_json::to_vec(&candidate).unwrap()).is_err());
    candidate = serde_json::from_slice(TOOLS).unwrap();
    candidate["tools"]["cargo-deny"]["platforms"]["aarch64-apple-darwin"]["url"] =
        "https://example.invalid/DO-NOT-ECHO".into();
    let error = authenticate(&serde_json::to_vec(&candidate).unwrap()).unwrap_err();
    assert!(!error.to_string().contains("DO-NOT-ECHO"));
    let mut reformatted = TOOLS.to_vec();
    reformatted.push(b'\n');
    assert!(
        authenticate(&reformatted).is_err(),
        "authenticity binds exact bytes, not schema equivalence"
    );
    assert!(authenticate(&vec![b' '; 1_048_577]).is_err());
    assert!(authenticate(TOOLS).is_ok());
}

#[test]
fn lock_is_exact_config_bound_and_each_native_distribution_is_unambiguous() {
    let root = fixture();
    let catalog = reviewed().unwrap();
    let before = fs::read(root.path().join("armorer.toml")).unwrap();
    let rendered = catalog.render_lock(root.path()).unwrap();
    assert_eq!(rendered, catalog.render_lock(root.path()).unwrap());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    assert_eq!(fs::read(root.path().join("armorer.toml")).unwrap(), before);
    fs::write(root.path().join("armorer.lock"), rendered).unwrap();
    use sha2::{Digest, Sha256};
    let config_digest = format!("{:x}", Sha256::digest(&before));
    let lock = load_lock(root.path(), &config_digest).unwrap().unwrap().0;
    assert_eq!(lock.workflows.repository, WORKFLOW_REPOSITORY);
    assert_eq!(lock.workflows.commit, WORKFLOW_COMMIT);
    assert_eq!(lock.tools.len(), 18);
    for (name, tool) in catalog.tools() {
        for target in TARGETS {
            let id = format!("{name}--{target}");
            assert_eq!(lock.tools[&id].version, tool.version);
            assert_eq!(lock.tools[&id].sha256, tool.platforms[*target].sha256);
        }
    }
    fs::write(
        root.path().join("armorer.toml"),
        [before.as_slice(), b"\n# reviewed change\n"].concat(),
    )
    .unwrap();
    let (_, current) = load_config(root.path()).unwrap();
    let current = format!("{:x}", Sha256::digest(current));
    assert!(load_lock(root.path(), &current).is_err());
}

#[test]
fn render_preserves_existing_lock_and_bespoke_workflows() {
    let root = fixture();
    fs::write(root.path().join("armorer.lock"), "unowned legacy lock").unwrap();
    fs::create_dir_all(root.path().join(".github/workflows")).unwrap();
    fs::write(
        root.path().join(".github/workflows/custom.yml"),
        "bespoke job",
    )
    .unwrap();
    let catalog = reviewed().unwrap();
    assert!(catalog.render_lock(root.path()).is_ok());
    let callers = catalog.caller_workflows();
    assert_eq!(callers.len(), 2);
    assert!(!callers.contains_key(".github/workflows/custom.yml"));
    assert_eq!(
        fs::read(root.path().join("armorer.lock")).unwrap(),
        b"unowned legacy lock"
    );
    assert_eq!(
        fs::read(root.path().join(".github/workflows/custom.yml")).unwrap(),
        b"bespoke job"
    );
    assert!(
        !root
            .path()
            .join(".github/workflows/armorer-ci.yml")
            .exists()
    );
    assert!(!root.path().join(".armorer").exists());
}

#[test]
fn caller_workflows_have_only_reviewed_full_pins_and_read_permissions() {
    let callers = reviewed().unwrap().caller_workflows();
    for value in callers.values() {
        assert!(value.contains(&format!("@{WORKFLOW_COMMIT}\n")));
        assert!(value.contains("permissions:\n  contents: read\n"));
        for forbidden in [
            "secrets:",
            "with:",
            "run:",
            "id-token:",
            "contents: write",
            "pull_request_target:",
            "workflow_run:",
            "environment:",
        ] {
            assert!(!value.contains(forbidden), "{forbidden}");
        }
    }
    let build = &callers[".github/workflows/armorer-build.yml"];
    assert!(build.contains("on:\n  workflow_dispatch:\n"));
    assert!(!build.contains("push:") && !build.contains("release:"));
}

#[test]
fn catalog_cli_needs_no_consumer_or_installed_build_tools() {
    let root = tempfile::tempdir().unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_armorer"))
        .arg("--repository")
        .arg(root.path())
        .arg("catalog")
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(result.status.success());
    let output: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(output["workflow"]["commit"], WORKFLOW_COMMIT);
    assert_eq!(output["tools"].as_object().unwrap().len(), 6);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn committed_caller_fixtures_match_the_reviewed_renderer() {
    let catalog = reviewed().unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/bootstrap-catalog");
    for (path, content) in catalog.caller_workflows() {
        assert_eq!(fs::read(root.join(path)).unwrap(), content.as_bytes());
    }
}
