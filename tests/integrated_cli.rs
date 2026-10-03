use serde_json::Value;
use std::{collections::BTreeMap, fs, path::Path, process::Command};

/// Copy an inert profile example into isolated storage, without invoking its package.
fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let destination = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

/// Capture every consuming file's exact bytes to detect unintended discovery or verification writes.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        let relative = path
            .strip_prefix(root)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        if path.is_dir() {
            for (name, bytes) in snapshot(&path) {
                files.insert(format!("{relative}/{name}"), bytes);
            }
        } else {
            files.insert(relative, fs::read(path).unwrap());
        }
    }
    files
}

/// Execute the compiled checkout CLI and require the documented status and JSON contract.
fn cli(root: &Path, arguments: &[&str], status: i32) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_armorer"))
        .arg("--repository")
        .arg(root)
        .args(arguments)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(status),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Exercise preview, bootstrap, upgrade and reversal through one binary while build traps remain inert.
#[test]
fn all_profiles_share_the_integrated_cli_without_building_or_losing_customizations() {
    for profile in ["library", "cli", "service"] {
        let root = tempfile::tempdir().unwrap();
        copy(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("examples/bootstrap-v2")
                .join(profile),
            root.path(),
        );
        fs::write(root.path().join("build.rs"), "fn main(){std::fs::write(\"BUILD_EXECUTED\",\"bad\").unwrap();panic!(\"must not build\");}").unwrap();
        fs::create_dir_all(root.path().join(".github/workflows")).unwrap();
        fs::write(
            root.path().join(".github/workflows/bespoke.yml"),
            "bespoke bytes\r\n",
        )
        .unwrap();
        let before = snapshot(root.path());
        let policy = root.path().join("ci-policy.reviewed.toml");
        let policy = policy.to_str().unwrap();
        let catalog = cli(root.path(), &["catalog"], 0);
        assert!(catalog.is_object());
        assert_eq!(
            cli(root.path(), &["plan", "--preview"], 0),
            cli(root.path(), &["preview"], 0)
        );
        cli(root.path(), &["bootstrap", "check", "--policy", policy], 2);
        let plan = cli(root.path(), &["bootstrap", "plan", "--policy", policy], 0);
        assert_eq!(snapshot(root.path()), before);
        let saved = tempfile::NamedTempFile::new().unwrap();
        fs::write(saved.path(), serde_json::to_vec(&plan).unwrap()).unwrap();
        let digest = plan["plan_sha256"].as_str().unwrap();
        let saved_path = saved.path().to_str().unwrap();
        cli(
            root.path(),
            &[
                "bootstrap",
                "apply",
                "--plan",
                saved_path,
                "--expect-plan-sha256",
                &"a".repeat(64),
            ],
            1,
        );
        assert_eq!(snapshot(root.path()), before);
        let receipt = cli(
            root.path(),
            &[
                "bootstrap",
                "apply",
                "--plan",
                saved_path,
                "--expect-plan-sha256",
                digest,
            ],
            0,
        );
        assert_eq!(receipt["configured"], true);
        for field in [
            "ci_verified",
            "release_rehearsed",
            "published",
            "provenance_verified",
        ] {
            assert_eq!(receipt[field], false);
        }
        let configured = snapshot(root.path());
        cli(
            root.path(),
            &[
                "bootstrap",
                "apply",
                "--plan",
                saved_path,
                "--expect-plan-sha256",
                digest,
            ],
            0,
        );
        assert_eq!(snapshot(root.path()), configured);
        let upgrade = cli(
            root.path(),
            &[
                "upgrade",
                "plan",
                "--policy",
                policy,
                "--catalog",
                "bootstrap-v1",
            ],
            0,
        );
        assert_eq!(snapshot(root.path()), configured);
        let upgrade_file = tempfile::NamedTempFile::new().unwrap();
        fs::write(upgrade_file.path(), serde_json::to_vec(&upgrade).unwrap()).unwrap();
        let upgrade_path = upgrade_file.path().to_str().unwrap();
        let upgrade_digest = upgrade["plan_sha256"].as_str().unwrap();
        cli(
            root.path(),
            &[
                "upgrade",
                "apply",
                "--plan",
                upgrade_path,
                "--expect-plan-sha256",
                upgrade_digest,
            ],
            0,
        );
        let reverse = cli(
            root.path(),
            &[
                "upgrade",
                "rollback-plan",
                "--upgrade-plan",
                upgrade_path,
                "--expect-upgrade-plan-sha256",
                upgrade_digest,
                "--allow-downgrade",
            ],
            0,
        );
        let reverse_file = tempfile::NamedTempFile::new().unwrap();
        fs::write(reverse_file.path(), serde_json::to_vec(&reverse).unwrap()).unwrap();
        cli(
            root.path(),
            &[
                "upgrade",
                "rollback-apply",
                "--plan",
                reverse_file.path().to_str().unwrap(),
                "--expect-plan-sha256",
                reverse["plan_sha256"].as_str().unwrap(),
            ],
            0,
        );
        assert_eq!(snapshot(root.path()), configured);
        assert_eq!(
            fs::read(root.path().join(".github/workflows/bespoke.yml")).unwrap(),
            b"bespoke bytes\r\n"
        );
        assert!(!root.path().join("BUILD_EXECUTED").exists());
        assert!(!root.path().join("target").exists());
        assert_eq!(
            fs::read(root.path().join("Cargo.lock")).ok(),
            before.get("Cargo.lock").cloned()
        );
    }
}

/// Reject unapproved release and historical contexts through the same binary before inspecting offered assets.
#[test]
fn integrated_verification_commands_preserve_context_first_rejection() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("unchanged"), b"consumer bytes").unwrap();
    let before = snapshot(root.path());
    let zero = "0".repeat(64);
    let trusted = tempfile::tempdir().unwrap();
    let unapproved = b"unapproved bytes that are deliberately not JSON";
    for name in [
        armorer::verification::context::CONTEXT_NAME,
        armorer::verification::context::NATIVE_CONTEXT_NAME,
    ] {
        fs::write(trusted.path().join(name), unapproved).unwrap();
    }
    let trusted_before = snapshot(trusted.path());
    let policy = tempfile::NamedTempFile::new().unwrap();
    fs::write(policy.path(), unapproved).unwrap();
    for kind in ["legacy-v1", "native-v2"] {
        let result = cli(
            root.path(),
            &[
                "verify-release",
                "--directory",
                "/nonexistent/offered-release",
                "--trusted-inputs",
                trusted.path().to_str().unwrap(),
                "--expect-context-sha256",
                &zero,
                "--context-kind",
                kind,
                "--gh",
                "/nonexistent/native-gh",
                "--trusted-root",
                "/nonexistent/root",
                "--cyclonedx",
                "/nonexistent/cyclonedx",
            ],
            1,
        );
        assert_eq!(
            result["error"],
            serde_json::json!({
                "code": "invalid-config",
                "message": "invalid configuration: unapproved-release-context"
            })
        );
    }
    let result = cli(
        root.path(),
        &[
            "verify-historical-bytes",
            "--directory",
            "/nonexistent/offered-release",
            "--policy",
            policy.path().to_str().unwrap(),
            "--expect-policy-sha256",
            &zero,
            "--source-repository",
            "example/pilot",
            "--source-commit",
            &"a".repeat(40),
            "--source-tag",
            "v0.1.0",
        ],
        1,
    );
    assert_eq!(
        result["error"],
        serde_json::json!({
            "code": "invalid-config",
            "message": "invalid configuration: unapproved-historical-policy"
        })
    );
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(snapshot(trusted.path()), trusted_before);
    assert_eq!(fs::read(policy.path()).unwrap(), unapproved);
}
