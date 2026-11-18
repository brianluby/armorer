//! The release-inputs manifest is an alternative to the individual flags. Its digest is the
//! approval: bytes are hashed before parsing, and verification behaves exactly as the flag form.
use armorer::{trust::ByteIdentity, verification::manifest::ReleaseInputs};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const CONTEXT_NAME: &str = "armorer-verification-context-v3.json";
const MISMATCH: &str = "release-inputs-manifest-digest-mismatch";

/// A hermetic invocation of the built binary with no ambient environment.
fn bin() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_armorer"));
    command.env_clear();
    command
}

fn sha256(bytes: &[u8]) -> String {
    ByteIdentity::from_bytes(bytes).sha256
}

/// Write exact manifest bytes and return the path with their independently computed digest.
fn put(dir: &Path, name: &str, bytes: &[u8]) -> (PathBuf, String) {
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    (path, sha256(bytes))
}

/// A complete manifest document with absolute inputs.
fn document(trusted: &Path, context_digest: &str) -> Value {
    json!({
        "schema_version": 1,
        "contract": "armorer-release-inputs",
        "directory": trusted.join("never-open-release"),
        "trusted_inputs": trusted,
        "expect_context_sha256": context_digest,
        "context_kind": "native-v3",
        "gh": trusted.join("never-open-gh"),
        "trusted_root": trusted.join("never-open-root"),
        "cyclonedx": trusted.join("never-open-cdx"),
        "publication": {
            "policy": trusted.join("never-open-policy"),
            "expect_policy_sha256": "0".repeat(64),
            "release_attestation_root": trusted.join("never-open-attestation-root")
        }
    })
}

fn put_json(dir: &Path, name: &str, value: &Value) -> (PathBuf, String) {
    put(dir, name, &serde_json::to_vec(value).unwrap())
}

/// A trusted-input directory whose context file is offered, malformed bytes.
fn trusted_with(context: &[u8]) -> tempfile::TempDir {
    let trusted = tempfile::tempdir().unwrap();
    fs::write(trusted.path().join(CONTEXT_NAME), context).unwrap();
    trusted
}

fn parse(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

fn message(output: &Output) -> String {
    parse(output)["error"]["message"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// Run the given arguments with the working directory set explicitly.
fn run(cwd: &Path, args: &[&str]) -> Output {
    bin().current_dir(cwd).args(args).output().unwrap()
}

fn manifest_args<'a>(path: &'a Path, digest: &'a str) -> Vec<&'a str> {
    vec![
        "--manifest",
        path.to_str().unwrap(),
        "--expect-manifest-sha256",
        digest,
    ]
}

// ---- digest gate -------------------------------------------------------------------

#[test]
/// A wrong digest is reported for a valid file and for garbage alike, with exit 1.
fn wrong_digest_fails_before_parsing_for_valid_and_garbage_files() {
    let dir = tempfile::tempdir().unwrap();
    let trusted = trusted_with(b"never read");
    let valid = document(trusted.path(), &"a".repeat(64));
    let (valid_path, _) = put_json(dir.path(), "valid.json", &valid);
    let (garbage_path, _) = put(dir.path(), "garbage.json", b"\xff\xfe not json {{{");
    let wrong = "b".repeat(64);
    for path in [&valid_path, &garbage_path] {
        let mut args = vec!["verify-release"];
        args.extend(manifest_args(path, &wrong));
        let output = run(dir.path(), &args);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(parse(&output)["error"]["code"], "invalid-config");
        assert!(message(&output).contains(MISMATCH), "{}", message(&output));
        assert!(output.stderr.is_empty());
        // The library reports the same fixed error and opens nothing named by the file.
        assert!(matches!(
            ReleaseInputs::open(path, &wrong),
            Err(armorer::Error::Invalid(code)) if code == MISMATCH
        ));
    }
}

#[test]
/// Uppercase, short and long digests are rejected before the file is read.
fn malformed_manifest_digests_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("not-even-read.json");
    for bad in [
        "A".repeat(64),
        "a".repeat(63),
        "a".repeat(65),
        String::new(),
        format!("{}G", "a".repeat(63)),
    ] {
        let output = run(
            dir.path(),
            &[
                "verify-release",
                "--manifest",
                missing.to_str().unwrap(),
                "--expect-manifest-sha256",
                &bad,
            ],
        );
        assert_eq!(output.status.code(), Some(1), "{bad}");
        assert!(message(&output).contains("invalid-release-inputs-manifest-digest"));
    }
}

#[test]
/// The correct digest passes the gate; the next, unchanged step is context authentication.
fn correct_digest_proceeds_to_context_authentication() {
    let dir = tempfile::tempdir().unwrap();
    let trusted = trusted_with(b"offered context");
    let (path, digest) = put_json(
        dir.path(),
        "m.json",
        &document(trusted.path(), &"a".repeat(64)),
    );
    let mut args = vec!["verify-release"];
    args.extend(manifest_args(&path, &digest));
    let output = run(dir.path(), &args);
    assert_eq!(output.status.code(), Some(1));
    assert!(message(&output).contains("unapproved-release-context"));
}

// ---- strictness --------------------------------------------------------------------

/// Open a manifest built from `mutate` with its true digest, so only strictness can reject it.
fn open_mutated(mutate: impl FnOnce(&mut Value)) -> armorer::Result<ReleaseInputs> {
    let dir = tempfile::tempdir().unwrap();
    let mut value = document(Path::new("/trusted"), &"a".repeat(64));
    mutate(&mut value);
    let (path, digest) = put_json(dir.path(), "m.json", &value);
    ReleaseInputs::open(&path, &digest)
}

#[test]
/// A complete manifest opens; the strictness cases below are meaningful because this succeeds.
fn complete_manifest_opens() {
    let inputs = open_mutated(|_| {}).unwrap();
    assert_eq!(inputs.trusted_inputs, Path::new("/trusted"));
    assert!(inputs.publication.is_some());
    assert!(inputs.manifest.is_some());
}

#[test]
fn unknown_fields_are_rejected_at_both_levels() {
    assert!(open_mutated(|v| v["extra"] = "x".into()).is_err());
    assert!(open_mutated(|v| v["publication"]["extra"] = "x".into()).is_err());
}

#[test]
fn duplicate_keys_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let text = serde_json::to_string(&document(Path::new("/t"), &"a".repeat(64))).unwrap();
    let duplicated = text.replacen('{', "{\"gh\":\"/other\",", 1);
    let (path, digest) = put(dir.path(), "m.json", duplicated.as_bytes());
    assert!(ReleaseInputs::open(&path, &digest).is_err());
}

#[test]
fn wrong_version_contract_and_missing_fields_are_rejected() {
    assert!(open_mutated(|v| v["schema_version"] = 2.into()).is_err());
    assert!(open_mutated(|v| v["contract"] = "armorer-other".into()).is_err());
    for field in [
        "schema_version",
        "contract",
        "directory",
        "trusted_inputs",
        "expect_context_sha256",
        "context_kind",
        "gh",
        "trusted_root",
        "cyclonedx",
    ] {
        assert!(
            open_mutated(|v| {
                v.as_object_mut().unwrap().remove(field);
            })
            .is_err(),
            "{field}"
        );
    }
    assert!(open_mutated(|v| v["publication"].as_object_mut().unwrap().clear()).is_err());
    assert!(open_mutated(|v| v["context_kind"] = "native-v2".into()).is_err());
}

#[test]
fn empty_and_control_character_paths_are_rejected() {
    for field in [
        "directory",
        "trusted_inputs",
        "gh",
        "trusted_root",
        "cyclonedx",
    ] {
        assert!(open_mutated(|v| v[field] = "".into()).is_err(), "{field}");
        assert!(
            open_mutated(|v| v[field] = "a\u{1}b".into()).is_err(),
            "{field}"
        );
        assert!(
            open_mutated(|v| v[field] = "a\nb".into()).is_err(),
            "{field}"
        );
    }
    for field in ["policy", "release_attestation_root"] {
        assert!(open_mutated(|v| v["publication"][field] = "".into()).is_err());
        assert!(open_mutated(|v| v["publication"][field] = "x\u{7f}".into()).is_err());
    }
}

#[test]
fn oversized_manifest_is_rejected_even_with_its_true_digest() {
    let dir = tempfile::tempdir().unwrap();
    let mut bytes = serde_json::to_vec(&document(Path::new("/t"), &"a".repeat(64))).unwrap();
    bytes.resize(64 * 1024 + 1, b' ');
    let (path, digest) = put(dir.path(), "big.json", &bytes);
    assert!(ReleaseInputs::open(&path, &digest).is_err());
    // The exact boundary is accepted.
    bytes.truncate(64 * 1024);
    let (path, digest) = put(dir.path(), "edge.json", &bytes);
    assert!(ReleaseInputs::open(&path, &digest).is_ok());
}

#[test]
#[cfg(unix)]
fn symlinked_and_directory_manifests_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let (real, digest) = put_json(
        dir.path(),
        "real.json",
        &document(Path::new("/t"), &"a".repeat(64)),
    );
    let link = dir.path().join("link.json");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert!(ReleaseInputs::open(&real, &digest).is_ok());
    assert!(ReleaseInputs::open(&link, &digest).is_err());
    assert!(ReleaseInputs::open(dir.path(), &digest).is_err());
    let mut args = vec!["verify-release"];
    args.extend(manifest_args(&link, &digest));
    assert_eq!(run(dir.path(), &args).status.code(), Some(1));
}

// ---- path resolution ---------------------------------------------------------------

#[test]
/// Relative paths resolve against the manifest's directory, never the working directory.
fn relative_paths_resolve_against_the_manifest_directory() {
    let root = tempfile::tempdir().unwrap();
    let bundle = root.path().join("bundle");
    let trusted = bundle.join("trusted");
    fs::create_dir_all(&trusted).unwrap();
    fs::write(trusted.join(CONTEXT_NAME), b"offered context").unwrap();
    let mut value = document(Path::new("/unused"), &"a".repeat(64));
    for (field, relative) in [
        ("directory", "release"),
        ("trusted_inputs", "trusted"),
        ("gh", "tools/gh"),
        ("trusted_root", "trusted/root.json"),
        ("cyclonedx", "tools/cyclonedx"),
    ] {
        value[field] = relative.into();
    }
    value["publication"]["policy"] = "policy.json".into();
    value["publication"]["release_attestation_root"] = "/abs/attestation-root.json".into();
    let (path, digest) = put_json(&bundle, "manifest.json", &value);
    let inputs = ReleaseInputs::open(&path, &digest).unwrap();
    assert_eq!(inputs.directory, bundle.join("release"));
    assert_eq!(inputs.trusted_inputs, trusted);
    assert_eq!(inputs.gh, bundle.join("tools/gh"));
    let publication = inputs.publication().unwrap();
    assert_eq!(publication.policy, bundle.join("policy.json"));
    assert_eq!(
        publication.release_attestation_root,
        Path::new("/abs/attestation-root.json")
    );
    // From an unrelated working directory the binary still finds the context beside the
    // manifest: a cwd-relative resolution would fail with an I/O error instead.
    let elsewhere = tempfile::tempdir().unwrap();
    let mut args = vec!["verify-release"];
    args.extend(manifest_args(&path, &digest));
    let output = run(elsewhere.path(), &args);
    assert!(message(&output).contains("unapproved-release-context"));
    // A relative manifest path from its own directory resolves the same way.
    let mut args = vec!["verify-release"];
    args.extend(manifest_args(Path::new("manifest.json"), &digest));
    let output = run(&bundle, &args);
    assert!(message(&output).contains("unapproved-release-context"));
}

#[test]
/// An absolute path is used as written, wherever the manifest lives.
fn absolute_paths_are_used_as_written() {
    let manifest_dir = tempfile::tempdir().unwrap();
    let trusted = trusted_with(b"offered context");
    let (path, digest) = put_json(
        manifest_dir.path(),
        "m.json",
        &document(trusted.path(), &"a".repeat(64)),
    );
    let inputs = ReleaseInputs::open(&path, &digest).unwrap();
    assert_eq!(inputs.trusted_inputs, trusted.path());
    let mut args = vec!["verify-release"];
    args.extend(manifest_args(&path, &digest));
    let output = run(manifest_dir.path(), &args);
    assert!(message(&output).contains("unapproved-release-context"));
}

// ---- mutual exclusion --------------------------------------------------------------

const RELEASE_FLAGS: [(&str, &str); 7] = [
    ("--directory", "d"),
    ("--trusted-inputs", "t"),
    ("--expect-context-sha256", "c"),
    ("--context-kind", "native-v3"),
    ("--gh", "g"),
    ("--trusted-root", "r"),
    ("--cyclonedx", "x"),
];
const PUBLICATION_FLAGS: [(&str, &str); 3] = [
    ("--publication-policy", "p"),
    ("--expect-publication-policy-sha256", "s"),
    ("--release-attestation-root", "a"),
];

#[test]
/// `--manifest` with any individual flag is a usage error, for both commands.
fn manifest_conflicts_with_every_individual_flag() {
    let dir = tempfile::tempdir().unwrap();
    let digest = "0".repeat(64);
    for (flag, value) in RELEASE_FLAGS {
        let mut args = vec!["verify-release"];
        args.extend(manifest_args(Path::new("m.json"), &digest));
        args.extend([flag, value]);
        let output = run(dir.path(), &args);
        assert_eq!(output.status.code(), Some(2), "{flag}");
        assert!(output.stdout.is_empty());
        let mut args = vec!["publication"];
        args.extend(manifest_args(Path::new("m.json"), &digest));
        args.extend([flag, value, "plan"]);
        assert_eq!(run(dir.path(), &args).status.code(), Some(2), "{flag}");
    }
    for (flag, value) in PUBLICATION_FLAGS {
        let mut args = vec!["publication"];
        args.extend(manifest_args(Path::new("m.json"), &digest));
        args.extend([flag, value, "plan"]);
        assert_eq!(run(dir.path(), &args).status.code(), Some(2), "{flag}");
    }
}

#[test]
/// The two manifest flags travel together.
fn manifest_and_its_digest_require_each_other() {
    let dir = tempfile::tempdir().unwrap();
    let digest = "0".repeat(64);
    for args in [
        vec!["verify-release", "--manifest", "m.json"],
        vec!["verify-release", "--expect-manifest-sha256", &digest],
        vec!["publication", "--manifest", "m.json", "plan"],
        vec!["publication", "--expect-manifest-sha256", &digest, "plan"],
    ] {
        let output = run(dir.path(), &args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
    }
}

#[test]
/// Without a manifest every flag that is required today is still required.
fn flag_form_still_requires_each_flag() {
    let dir = tempfile::tempdir().unwrap();
    for (omitted, required) in RELEASE_FLAGS.iter().enumerate() {
        let mut args = vec!["verify-release"];
        for (index, (flag, value)) in RELEASE_FLAGS.iter().enumerate() {
            if index != omitted {
                args.extend([*flag, *value]);
            }
        }
        let output = run(dir.path(), &args);
        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(required.0), "{stderr}");
    }
    for (omitted, required) in PUBLICATION_FLAGS.iter().enumerate() {
        let mut args = vec!["publication"];
        for (flag, value) in RELEASE_FLAGS {
            args.extend([flag, value]);
        }
        for (index, (flag, value)) in PUBLICATION_FLAGS.iter().enumerate() {
            if index != omitted {
                args.extend([*flag, *value]);
            }
        }
        args.push("plan");
        let output = run(dir.path(), &args);
        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(required.0), "{stderr}");
    }
}

// ---- equivalence with the flag form -----------------------------------------------

/// One scenario expressed as individual flag values.
struct Scenario {
    trusted: PathBuf,
    context_digest: String,
    gh: PathBuf,
}

impl Scenario {
    fn release_flags(&self) -> Vec<String> {
        let t = self.trusted.display().to_string();
        let never = |n: &str| self.trusted.join(n).display().to_string();
        [
            "--directory",
            &never("never-open-release"),
            "--trusted-inputs",
            &t,
            "--expect-context-sha256",
            &self.context_digest,
            "--context-kind",
            "native-v3",
            "--gh",
            &self.gh.display().to_string(),
            "--trusted-root",
            &never("never-open-root"),
            "--cyclonedx",
            &never("never-open-cdx"),
        ]
        .map(String::from)
        .to_vec()
    }
    fn publication_flags(&self) -> Vec<String> {
        let never = |n: &str| self.trusted.join(n).display().to_string();
        [
            "--publication-policy",
            &never("never-open-policy"),
            "--expect-publication-policy-sha256",
            &"0".repeat(64),
            "--release-attestation-root",
            &never("never-open-attestation-root"),
        ]
        .map(String::from)
        .to_vec()
    }
    /// The equivalent manifest, with the same values as the flags above.
    fn manifest(&self, dir: &Path) -> (PathBuf, String) {
        let mut value = document(&self.trusted, &self.context_digest);
        value["gh"] = self.gh.display().to_string().into();
        put_json(dir, "m.json", &value)
    }
    fn flag_form(&self, command: &[&str]) -> Output {
        let mut args: Vec<String> = vec![command[0].into()];
        args.extend(self.release_flags());
        if command[0] == "publication" {
            args.extend(self.publication_flags());
        }
        args.extend(command[1..].iter().map(|s| s.to_string()));
        bin()
            .current_dir(&self.trusted)
            .args(&args)
            .output()
            .unwrap()
    }
    fn manifest_form(&self, command: &[&str]) -> Output {
        let dir = tempfile::tempdir().unwrap();
        let (path, digest) = self.manifest(dir.path());
        let mut args: Vec<String> = vec![command[0].into()];
        args.extend(manifest_args(&path, &digest).into_iter().map(String::from));
        args.extend(command[1..].iter().map(|s| s.to_string()));
        bin()
            .current_dir(&self.trusted)
            .args(&args)
            .output()
            .unwrap()
    }
    /// Run both ways and require equal status, stdout and stderr; return the shared output.
    fn equivalent(&self, command: &[&str]) -> Output {
        let (flags, manifest) = (self.flag_form(command), self.manifest_form(command));
        assert_eq!(flags.status.code(), manifest.status.code(), "{command:?}");
        assert_eq!(flags.stderr, manifest.stderr, "{command:?}");
        let (flag_out, mut manifest_out) = (parse(&flags), parse(&manifest));
        // On success the manifest form adds exactly one field; failures must match whole.
        if flags.status.success() {
            let added = manifest_out
                .as_object_mut()
                .unwrap()
                .remove("inputs_manifest");
            assert!(added.is_some());
            assert!(flag_out.get("inputs_manifest").is_none());
        }
        assert_eq!(flag_out, manifest_out, "{command:?}");
        flags
    }
}

#[test]
/// Scenario from `cli_rejects_unapproved_context_before_release_and_tools`.
fn equivalence_unapproved_context_digest() {
    let trusted = trusted_with(b"malformed offered context; reject its independent digest first");
    let scenario = Scenario {
        trusted: trusted.path().to_owned(),
        context_digest: "a".repeat(64),
        gh: trusted.path().join("never-open-gh"),
    };
    let output = scenario.equivalent(&["verify-release"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(message(&output).contains("unapproved-release-context"));
    assert_eq!(fs::read_dir(trusted.path()).unwrap().count(), 1);
}

#[test]
/// Scenario from `cli_context_version_is_explicit_without_fallback`: approved but malformed bytes.
fn equivalence_approved_malformed_context() {
    let malformed = b"invalid native v3 context";
    let trusted = trusted_with(malformed);
    let scenario = Scenario {
        trusted: trusted.path().to_owned(),
        context_digest: sha256(malformed),
        gh: trusted.path().join("never-open-gh"),
    };
    let output = scenario.equivalent(&["verify-release"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(parse(&output)["error"]["code"], "json");
}

#[test]
/// A malformed context digest gets the flag form's error from the manifest form too.
fn equivalence_malformed_context_digest() {
    let trusted = trusted_with(b"offered context");
    let scenario = Scenario {
        trusted: trusted.path().to_owned(),
        context_digest: "NOT-A-DIGEST".into(),
        gh: trusted.path().join("never-open-gh"),
    };
    let output = scenario.equivalent(&["verify-release"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(message(&output).contains("invalid-approved-release-context-digest"));
}

#[test]
#[cfg(unix)]
/// Order: with the right manifest digest but an unapproved context digest inside it, the
/// context error is the flag form's, and neither the release nor any adapter is touched.
fn manifest_form_keeps_context_before_release_and_adapters() {
    use std::os::unix::fs::PermissionsExt;
    let trusted = trusted_with(b"offered context");
    let marker = trusted.path().join("executed");
    let tool = trusted.path().join("offered-tool");
    fs::write(&tool, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
    let scenario = Scenario {
        trusted: trusted.path().to_owned(),
        context_digest: "c".repeat(64),
        gh: tool,
    };
    // The release directory does not exist: reading it would give an I/O error instead.
    let output = scenario.equivalent(&["verify-release"]);
    assert!(message(&output).contains("unapproved-release-context"));
    assert!(!marker.exists());
    assert!(!trusted.path().join("never-open-release").exists());
    assert_eq!(parse(&output)["error"]["code"], "invalid-config");
}

// ---- publication -------------------------------------------------------------------

#[test]
/// Publication via manifest matches the flag form for the scenario in `publication_cli`.
fn publication_with_block_matches_flag_form() {
    let trusted = trusted_with(b"{}");
    let state = tempfile::tempdir().unwrap();
    let scenario = Scenario {
        trusted: trusted.path().to_owned(),
        context_digest: "0".repeat(64),
        gh: trusted.path().join("never-open-gh"),
    };
    let output = scenario.equivalent(&["publication", "plan"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(message(&output).contains("unapproved-release-context"));
    let state_arg = state.path().to_str().unwrap();
    for action in [
        vec!["publication", "inspect", "--state-directory", state_arg],
        vec![
            "publication",
            "recover-published",
            "--state-directory",
            state_arg,
        ],
    ] {
        let output = scenario.equivalent(&action);
        assert!(message(&output).contains("unapproved-release-context"));
    }
    assert_eq!(fs::read_dir(state.path()).unwrap().count(), 0);
}

#[test]
/// A manifest with no `publication` block serves `verify-release` but refuses every action.
fn publication_without_block_is_a_clear_error() {
    let trusted = trusted_with(b"offered context");
    let dir = tempfile::tempdir().unwrap();
    let mut value = document(trusted.path(), &"a".repeat(64));
    value.as_object_mut().unwrap().remove("publication");
    let (path, digest) = put_json(dir.path(), "m.json", &value);
    let state = dir.path().to_str().unwrap();
    for action in [
        vec!["plan"],
        vec!["inspect", "--state-directory", state],
        vec!["recover-published", "--state-directory", state],
        vec![
            "stage",
            "--approval",
            "a",
            "--expect-approval-sha256",
            &"0".repeat(64),
            "--state-directory",
            state,
        ],
    ] {
        let mut args = vec!["publication"];
        args.extend(manifest_args(&path, &digest));
        args.extend(action.iter().copied());
        let output = run(dir.path(), &args);
        assert_eq!(output.status.code(), Some(1), "{action:?}");
        assert_eq!(parse(&output)["error"]["code"], "invalid-config");
        assert_eq!(
            message(&output),
            "invalid configuration: release-inputs-manifest-has-no-publication-block"
        );
    }
    // The same manifest still reaches context authentication for verify-release.
    let mut args = vec!["verify-release"];
    args.extend(manifest_args(&path, &digest));
    assert!(message(&run(dir.path(), &args)).contains("unapproved-release-context"));
}

// ---- schema and example ------------------------------------------------------------

#[test]
fn committed_schema_matches_current_runtime() {
    let committed: Value =
        serde_json::from_str(include_str!("../schemas/release-inputs-manifest-v1.json")).unwrap();
    assert_eq!(
        serde_json::to_value(schemars::schema_for!(
            armorer::verification::manifest::ReleaseInputsManifest
        ))
        .unwrap(),
        committed
    );
}

#[test]
/// The example parses strictly, resolves, and its placeholder digests are structurally valid.
fn example_manifest_parses_and_validates() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/release-inputs/manifest.example.json");
    let bytes = fs::read(&path).unwrap();
    let inputs = ReleaseInputs::open(&path, &sha256(&bytes)).unwrap();
    assert_eq!(inputs.expect_context_sha256.len(), 64);
    assert_eq!(inputs.publication().unwrap().expect_policy_sha256.len(), 64);
    assert!(inputs.trusted_inputs.starts_with(path.parent().unwrap()));
    let parsed: armorer::verification::manifest::ReleaseInputsManifest =
        serde_json::from_slice(&bytes).unwrap();
    assert_eq!(parsed.contract, "armorer-release-inputs");
}
