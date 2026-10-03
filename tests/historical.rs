//! Exact historical byte tests establish no signature, source authenticity or release approval.
use armorer::{
    trust::{ByteIdentity, Source, policy::VerificationPolicy},
    verification::historical::HistoricalByteMatch,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

/// Obtain the actual clock for short-lived, explicitly synthetic reviews.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Create a test-owned policy and inert legacy artifact independently of downloaded claims.
fn fixture() -> (tempfile::TempDir, Value, Source) {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("download")).unwrap();
    let example = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/trust-v1/historical-verification-policy.json");
    let mut policy: Value = serde_json::from_slice(&fs::read(example).unwrap()).unwrap();
    let review = json!({"owner":"fixture-reviewer", "rationale":"Synthetic byte-comparison test only; no release or trust approval", "reviewed_at":now()-60, "expires_at":now()+3600, "record":ByteIdentity::from_bytes(b"synthetic historical review")});
    policy["review"] = review.clone();
    policy["roots"]["review"] = review.clone();
    policy["historical"][0]["review"] = review;
    fs::write(
        directory.path().join("download/legacy.tar.gz"),
        b"synthetic historical artifact",
    )
    .unwrap();
    let source: Source = serde_json::from_value(policy["historical"][0]["source"].clone()).unwrap();
    (directory, policy, source)
}

/// Write independently selected test policy bytes and derive their test-only approved digest.
fn approve(directory: &Path, policy: &Value) -> (PathBuf, String) {
    let path = directory.join("policy.json");
    let bytes = serde_json::to_vec(policy).unwrap();
    fs::write(&path, &bytes).unwrap();
    (path, ByteIdentity::from_bytes(&bytes).sha256)
}

/// Run only the historical comparison against the current test-owned authorities.
fn verify(
    directory: &Path,
    policy: &Value,
    source: &Source,
) -> armorer::Result<HistoricalByteMatch> {
    let (path, digest) = approve(directory, policy);
    HistoricalByteMatch::verify(&directory.join("download"), &path, &digest, source)
}

#[test]
/// Exact historical bytes return a distinct weaker result and leave repository inputs untouched.
fn explicit_exact_allowlist_is_read_only_and_never_executes_or_extracts_payloads() {
    let (directory, mut policy, source) = fixture();
    let payload = b"#!/bin/sh\ntouch executed\n";
    fs::write(directory.path().join("download/legacy.tar.gz"), payload).unwrap();
    policy["historical"][0]["assets"]["legacy.tar.gz"] =
        serde_json::to_value(ByteIdentity::from_bytes(payload)).unwrap();
    fs::write(
        directory.path().join("build.rs"),
        b"fn main() { panic!(\"executed\"); }",
    )
    .unwrap();
    let result = verify(directory.path(), &policy, &source).unwrap();
    assert_eq!(result.source(), &source);
    assert_eq!(result.assets().len(), 1);
    assert_eq!(
        result.assets()["legacy.tar.gz"],
        ByteIdentity::from_bytes(payload)
    );
    assert_eq!(
        result.policy_identity(),
        &ByteIdentity::from_bytes(&serde_json::to_vec(&policy).unwrap())
    );
    assert!(!result.limitations().is_empty());
    assert_eq!(
        fs::read(directory.path().join("download/legacy.tar.gz")).unwrap(),
        payload
    );
    assert!(!directory.path().join("executed").exists());
    assert!(!directory.path().join("download/executed").exists());
    assert!(!directory.path().join("target").exists());
}

#[test]
/// Wrong approval fails before policy parsing or artifact directory access; malformed approved JSON also fails.
fn independent_digest_precedes_parsing_and_download_access() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("policy.json");
    fs::write(&path, b"not JSON").unwrap();
    let error = HistoricalByteMatch::verify(
        &directory.path().join("missing"),
        &path,
        &"f".repeat(64),
        &Source {
            repository: "fixture/project".into(),
            commit: "e".repeat(40),
            git_ref: "refs/tags/v0.1.0".into(),
        },
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains("unapproved-historical-policy"));
    for bytes in [
        b"{} {}".as_slice(),
        br#"{"schema_version":1,"schema_version":1}"#,
    ] {
        fs::write(&path, bytes).unwrap();
        assert!(
            HistoricalByteMatch::verify(
                &directory.path().join("missing"),
                &path,
                &ByteIdentity::from_bytes(bytes).sha256,
                &Source {
                    repository: "fixture/project".into(),
                    commit: "e".repeat(40),
                    git_ref: "refs/tags/v0.1.0".into()
                },
            )
            .is_err()
        );
    }
}

#[test]
/// Modern, wrong-repository, wrong-commit, wrong-tag and unstable-tag sources cannot use historical mode.
fn exact_source_selection_and_modern_ref_separation_block_downgrade() {
    let (directory, policy, source) = fixture();
    let modern: Source = serde_json::from_value(policy["sources"][0].clone()).unwrap();
    assert!(verify(directory.path(), &policy, &modern).is_err());
    for case in 0..5 {
        let mut wrong = source.clone();
        match case {
            0 => wrong.repository = "attacker/project".into(),
            1 => wrong.commit = "d".repeat(40),
            2 => wrong.git_ref = "refs/tags/v0.1.1".into(),
            3 => wrong.git_ref = "refs/tags/v0.1.0-rc.1".into(),
            _ => wrong.git_ref = "refs/heads/main".into(),
        }
        assert!(verify(directory.path(), &policy, &wrong).is_err());
    }
    let mut collision = policy.clone();
    collision["sources"][0] = serde_json::to_value(&source).unwrap();
    assert!(verify(directory.path(), &collision, &source).is_err());
    let mut rehearsal = policy;
    rehearsal["mode"] = "rehearsal".into();
    rehearsal["sources"][0]["git_ref"] = "refs/heads/main".into();
    assert!(verify(directory.path(), &rehearsal, &source).is_err());
}

#[test]
/// Every review remains current, including the exact historical exception and unused root policy review.
fn expired_or_future_reviews_and_unbounded_limitations_fail() {
    for pointer in ["/review", "/roots/review", "/historical/0/review"] {
        for future in [false, true] {
            let (directory, mut policy, source) = fixture();
            policy.pointer_mut(pointer).unwrap()
                [if future { "reviewed_at" } else { "expires_at" }] =
                (if future { now() + 3600 } else { now() - 1 }).into();
            assert!(verify(directory.path(), &policy, &source).is_err());
        }
    }
    for limitations in [
        json!([]),
        json!([""]),
        json!(["bad\nclaim"]),
        json!(["x".repeat(4097)]),
        json!(vec!["x"; 129]),
    ] {
        let (directory, mut policy, source) = fixture();
        policy["historical"][0]["limitations"] = limitations;
        assert!(verify(directory.path(), &policy, &source).is_err());
    }
}

#[test]
/// Byte substitutions, omissions and extra files fail the complete explicit comparison.
fn complete_download_set_and_each_byte_are_mandatory() {
    for case in 0..4 {
        let (directory, policy, source) = fixture();
        let download = directory.path().join("download");
        match case {
            0 => fs::write(download.join("legacy.tar.gz"), b"tampered").unwrap(),
            1 => fs::remove_file(download.join("legacy.tar.gz")).unwrap(),
            2 => fs::write(download.join("extra.tar.gz"), b"extra").unwrap(),
            _ => fs::create_dir(download.join("extra-directory")).unwrap(),
        }
        assert!(verify(directory.path(), &policy, &source).is_err());
    }
}

#[test]
/// Attestation/inventory evidence cannot be discarded or even added to an approved weaker allowlist.
fn modern_evidence_is_forbidden_even_when_historical_hashes_were_approved() {
    for name in [
        "armorer-release-inventory.json",
        "armorer-release-inventory.provenance.sigstore.json",
        "legacy.SIGSTORE.JSON",
        "legacy.sigstore.jsonl",
        "legacy.intoto.jsonl",
        "legacy.attestation.json",
        "legacy.attestations.json",
    ] {
        for approved in [false, true] {
            let (directory, mut policy, source) = fixture();
            let bytes = b"invalid authentication evidence must never enable fallback";
            fs::write(directory.path().join("download").join(name), bytes).unwrap();
            if approved {
                policy["historical"][0]["assets"][name] =
                    serde_json::to_value(ByteIdentity::from_bytes(bytes)).unwrap();
            }
            let error = verify(directory.path(), &policy, &source).err().unwrap();
            assert!(
                error
                    .to_string()
                    .contains("historical-attestation-evidence-forbidden"),
                "{name}: {error}"
            );
        }
    }
}

#[test]
/// Approved metadata cannot authorize oversized aggregate copies or traversal paths.
fn policy_and_aggregate_byte_limits_fail_before_copying() {
    let (directory, mut policy, source) = fixture();
    policy["historical"][0]["assets"] = json!({});
    for index in 0..5 {
        policy["historical"][0]["assets"][format!("large-{index}.tar.gz")] =
            json!({"sha256":"a".repeat(64), "size":1024_u64*1024*1024});
    }
    let error = verify(directory.path(), &policy, &source).err().unwrap();
    assert!(error.to_string().contains("historical-total-byte-limit"));
    let (directory, mut policy, source) = fixture();
    policy["historical"][0]["assets"]["../escape"] =
        serde_json::to_value(ByteIdentity::from_bytes(b"escape")).unwrap();
    assert!(verify(directory.path(), &policy, &source).is_err());
    let path = directory.path().join("huge-policy.json");
    fs::File::create(&path)
        .unwrap()
        .set_len(1024 * 1024 + 1)
        .unwrap();
    assert!(
        HistoricalByteMatch::verify(
            &directory.path().join("download"),
            &path,
            &"a".repeat(64),
            &source
        )
        .is_err()
    );
}

#[cfg(unix)]
#[test]
/// Symlinked policies/directories/assets and special leaves cannot be opened as historical inputs.
fn symlinks_and_special_files_are_rejected_without_following_or_execution() {
    use std::os::unix::{fs::symlink, net::UnixListener};
    for case in 0..4 {
        let (directory, policy, source) = fixture();
        let (mut path, digest) = approve(directory.path(), &policy);
        let mut download = directory.path().join("download");
        let outside = tempfile::tempdir().unwrap();
        match case {
            0 => {
                let link = directory.path().join("linked-policy");
                symlink(&path, &link).unwrap();
                path = link;
            }
            1 => {
                let link = directory.path().join("linked-download");
                symlink(&download, &link).unwrap();
                download = link;
            }
            2 => {
                fs::remove_file(download.join("legacy.tar.gz")).unwrap();
                fs::write(
                    outside.path().join("artifact"),
                    b"synthetic historical artifact",
                )
                .unwrap();
                symlink(
                    outside.path().join("artifact"),
                    download.join("legacy.tar.gz"),
                )
                .unwrap();
            }
            _ => {
                let _listener = UnixListener::bind(download.join("socket")).unwrap();
            }
        }
        assert!(HistoricalByteMatch::verify(&download, &path, &digest, &source).is_err());
    }
}

#[test]
/// CLI success explicitly reports weaker byte matching; wrong approval exits nonzero and never auto-selects policy.
fn cli_requires_independent_selection_and_reports_no_authenticity_or_slsa_level() {
    let (directory, policy, source) = fixture();
    let (path, digest) = approve(directory.path(), &policy);
    for (approval, success) in [(&digest, true), (&"f".repeat(64), false)] {
        let output = Command::new(env!("CARGO_BIN_EXE_armorer"))
            .args([
                "--repository",
                "/nonexistent-not-inspected",
                "verify-historical-bytes",
                "--directory",
            ])
            .arg(directory.path().join("download"))
            .arg("--policy")
            .arg(&path)
            .args([
                "--expect-policy-sha256",
                approval,
                "--source-repository",
                &source.repository,
                "--source-commit",
                &source.commit,
                "--source-tag",
                "v0.1.0",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.success(), success);
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        if success {
            assert_eq!(result["status"], "historical-byte-match");
            assert_eq!(result["authenticity"], "not-established");
            assert_eq!(result["provenance_verified"], false);
            assert!(result["slsa_build_level"].is_null());
            assert_eq!(
                result["requested_source"],
                serde_json::to_value(&source).unwrap()
            );
        } else {
            assert!(result.get("status").is_none());
            assert!(
                result["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("unapproved-historical-policy")
            );
        }
    }
    let parsed: VerificationPolicy = serde_json::from_value(policy).unwrap();
    assert_eq!(parsed.historical[0].source, source);
}

#[cfg(target_os = "linux")]
#[test]
/// Linux permits non-UTF-8 leaves, which must still fail the historical input contract.
fn non_utf8_download_name_is_rejected_on_linux() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let (directory, policy, source) = fixture();
    fs::write(
        directory
            .path()
            .join("download")
            .join(OsString::from_vec(vec![0xff])),
        b"invalid name",
    )
    .unwrap();
    assert!(verify(directory.path(), &policy, &source).is_err());
}
