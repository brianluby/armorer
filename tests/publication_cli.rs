//! CLI intent authentication precedes all credentials, native tools and consuming mutations.
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
/// An unapproved context rejects every controller action before running offered tools or creating ownership state.
fn unapproved_context_precedes_tools_credentials_and_state_for_all_actions() {
    for action in ["plan", "inspect", "stage", "publish", "recover-published"] {
        let root = tempfile::tempdir().unwrap();
        let inputs = root.path().join("inputs");
        fs::create_dir(&inputs).unwrap();
        fs::write(inputs.join("armorer-verification-context.json"), b"{}").unwrap();
        let marker = root.path().join("unexpected-execution");
        let tool = root.path().join("offered-tool");
        fs::write(
            &tool,
            format!("#!/bin/sh\n/usr/bin/touch '{}'\n", marker.display()),
        )
        .unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        let state = root.path().join("state");
        fs::create_dir(&state).unwrap();
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
        let zero = "0".repeat(64);
        let mut command = Command::new(env!("CARGO_BIN_EXE_armorer"));
        command
            .arg("publication")
            .arg("--directory")
            .arg(root.path())
            .arg("--trusted-inputs")
            .arg(&inputs)
            .args([
                "--expect-context-sha256",
                &zero,
                "--context-kind",
                "legacy-v1",
            ])
            .arg("--gh")
            .arg(&tool)
            .arg("--trusted-root")
            .arg(&tool)
            .arg("--cyclonedx")
            .arg(&tool)
            .arg("--publication-policy")
            .arg(&tool)
            .args(["--expect-publication-policy-sha256", &zero])
            .arg("--release-attestation-root")
            .arg(&tool)
            .arg(action);
        if action != "plan" {
            command.arg("--state-directory").arg(&state);
        }
        if matches!(action, "stage" | "publish") {
            command
                .arg("--approval")
                .arg(&tool)
                .args(["--expect-approval-sha256", &zero]);
        }
        let output = command.output().unwrap();
        assert!(!output.status.success());
        let diagnostic: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(
            diagnostic["error"]["message"]
                .as_str()
                .unwrap()
                .contains("unapproved-release-context")
        );
        assert!(!marker.exists());
        assert_eq!(fs::read_dir(&state).unwrap().count(), 0);
        assert_eq!(
            fs::read(inputs.join("armorer-verification-context.json")).unwrap(),
            b"{}"
        );
    }
}
