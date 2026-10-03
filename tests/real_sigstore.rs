//! Genuine offline cryptographic integration. The explicitly ignored test is
//! required in hosted CI after qualifying the native verifier distribution.
use armorer::{
    trust::{
        ByteIdentity, Review, RunIdentity, Source, WorkflowIdentity,
        policy::{
            EvidenceScope, Predicate, SignerRule, TrustBackend, TrustRoots, VerificationMode,
            VerificationPolicy,
        },
    },
    verification::sigstore::{ExpectedAttestation, OfflineVerifier, Trigger},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
/// Locate the inert, immutable upstream signature fixtures retained in this repository.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/sigstore")
        .join(name)
}
/// Observe the clock for short-lived fixture-only policy review records.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
/// Define exact source/signer/run identities independently of the downloaded signature payload.
fn expectation() -> ExpectedAttestation {
    let source = Source {
        repository: "malancas/attest-demo".into(),
        commit: "95baf27389e83e6a5c48f42e190d48d7abcea19e".into(),
        git_ref: "refs/heads/main".into(),
    };
    ExpectedAttestation {
        caller_workflow: WorkflowIdentity {
            repository: source.repository.clone(),
            path: ".github/workflows/shared.yml".into(),
            commit: source.commit.clone(),
        },
        source,
        run: RunIdentity {
            id: 9228858953,
            attempt: 1,
            workflow: WorkflowIdentity {
                repository: "github/artifact-attestations-workflows".into(),
                path: ".github/workflows/attest.yml".into(),
                commit: "09b495c3f12c7881b3cc17209a327792065c1a1d".into(),
            },
        },
        trigger: Trigger::WorkflowDispatch,
        predicate: Predicate::SlsaProvenanceV1,
        scope: EvidenceScope::FinalArtifact,
        subject_name: "github_provenance_demo-0.0.0-py3-none-any.whl".into(),
    }
}
/// Require the native test executable to match both retained and compiled official pins.
fn pinned_verifier() -> (PathBuf, ByteIdentity) {
    let path = PathBuf::from(std::env::var_os("ARMORER_TEST_GH").expect(
        "Explicit real test requires ARMORER_TEST_GH; use scripts/qualify-test-verifier.py",
    ));
    let pins: Value =
        serde_json::from_str(include_str!("fixtures/sigstore/verifier-pins.json")).unwrap();
    let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "darwin-arm64",
        ("linux", "x86_64") => "linux-amd64",
        ("linux", "aarch64") => "linux-arm64",
        _ => panic!("Unsupported real verifier test platform"),
    };
    let pin = pins["pins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["platform"] == platform)
        .unwrap();
    let identity: ByteIdentity = serde_json::from_value(pin["executable"].clone()).unwrap();
    identity.matches(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        identity,
        armorer::verification::sigstore::qualified_native_verifier().unwrap()
    );
    (path, identity)
}
/// Build an explicitly fixture-only policy; its review record is not production root approval.
fn policy(expected: &ExpectedAttestation, verifier: ByteIdentity) -> VerificationPolicy {
    let review = Review {owner: "fixture-reviewer".into(), rationale: "Authentic external integration fixture only; not production roots or Armorer release acceptance".into(),
        reviewed_at: now() - 60, expires_at: now() + 3600, record: ByteIdentity::from_bytes(include_bytes!("fixtures/sigstore/source-receipt.json"))};
    VerificationPolicy {
        schema_version: 1,
        mode: VerificationMode::Rehearsal,
        repository: expected.source.repository.clone(),
        sources: vec![expected.source.clone()],
        signers: [
            (EvidenceScope::FinalArtifact, Predicate::SlsaProvenanceV1),
            (EvidenceScope::CargoSbomFile, Predicate::SlsaProvenanceV1),
            (EvidenceScope::SbomPredicate, Predicate::CycloneDxV15),
            (EvidenceScope::EvidenceFile, Predicate::SlsaProvenanceV1),
            (EvidenceScope::Inventory, Predicate::SlsaProvenanceV1),
        ]
        .into_iter()
        .map(|(scope, predicate)| SignerRule {
            scope,
            predicate,
            workflow: expected.run.workflow.clone(),
        })
        .collect(),
        roots: TrustRoots {
            backend: TrustBackend::SigstorePublicGood,
            trusted_root: ByteIdentity {
                sha256: "455b3fe53e2678889f093d66ebbfda5f12ebb9d682b146c3535fa04e97689787".into(),
                size: 3806,
            },
            verifier,
            verifier_version: "2.102.0".into(),
            review: review.clone(),
        },
        deny_self_hosted_runners: true,
        supplemental_assets: BTreeMap::new(),
        apple_team: None,
        historical: vec![],
        review,
    }
}
/// Instantiate a fixture verifier with an explicit test policy approval digest.
fn open(policy: &VerificationPolicy, executable: &Path) -> OfflineVerifier {
    let file = tempfile::NamedTempFile::new().unwrap();
    let bytes = serde_json::to_vec(policy).unwrap();
    fs::write(file.path(), &bytes).unwrap();
    OfflineVerifier::open(
        file.path(),
        &ByteIdentity::from_bytes(&bytes).sha256,
        executable,
        &fixture("trusted_root.json"),
    )
    .unwrap()
}
#[test]
#[ignore = "Required explicit CI gate with independently hash-qualified native gh"]
/// Run real offline signatures and reject all tamper, identity, subset and approval errors.
fn genuine_pinned_gh_verifies_and_every_offered_tamper_or_identity_error_fails() {
    let (executable, verifier_pin) = pinned_verifier();
    let expected = expectation();
    let policy = policy(&expected, verifier_pin);
    let verifier = open(&policy, &executable);
    let artifact = fixture("reusable-workflow-artifact");
    let bundle = fixture("reusable-workflow-attestation.sigstore.json");
    let artifact_bytes = fs::read(&artifact).unwrap();
    let expected_bytes = ByteIdentity {
        sha256: "49a3aa6075e0f49f82843e74b5baa614ad2a588e6675612bf108a0a008c5ac25".into(),
        size: 2962,
    };
    expected_bytes.matches(&artifact_bytes).unwrap();
    let proof = verifier
        .verify(&artifact, &expected_bytes, &bundle, &expected)
        .unwrap();
    assert_eq!(proof.subject(), &expected_bytes);
    assert_eq!(proof.expected().run.attempt, 1);
    assert_eq!(proof.verifier(), &policy.roots.verifier);
    let original: Value = serde_json::from_slice(&fs::read(&bundle).unwrap()).unwrap();
    let mut signature = original.clone();
    let sig = signature["dsseEnvelope"]["signatures"][0]["sig"]
        .as_str()
        .unwrap();
    signature["dsseEnvelope"]["signatures"][0]["sig"] = format!("A{}", &sig[1..]).into();
    let mut payload = original.clone();
    let encoded = payload["dsseEnvelope"]["payload"].as_str().unwrap();
    let mut statement: Value = serde_json::from_slice(&STANDARD.decode(encoded).unwrap()).unwrap();
    statement["predicate"]["buildDefinition"]["buildType"] =
        "https://attacker.invalid/build".into();
    payload["dsseEnvelope"]["payload"] = STANDARD
        .encode(serde_json::to_vec(&statement).unwrap())
        .into();
    let mut certificate = original.clone();
    let cert = certificate["verificationMaterial"]["certificate"]["rawBytes"]
        .as_str()
        .unwrap();
    certificate["verificationMaterial"]["certificate"]["rawBytes"] =
        format!("A{}", &cert[1..]).into();
    for (case, value) in [
        ("signature", signature.clone()),
        ("signed-predicate", payload),
        ("certificate", certificate),
    ] {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(
            verifier
                .verify(&artifact, &expected_bytes, file.path(), &expected)
                .is_err(),
            "{case}"
        );
    }
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut changed = artifact_bytes.clone();
    changed[0] ^= 1;
    fs::write(file.path(), &changed).unwrap();
    assert!(
        verifier
            .verify(file.path(), &expected_bytes, &bundle, &expected)
            .is_err()
    );
    assert!(
        verifier
            .verify(
                file.path(),
                &ByteIdentity::from_bytes(&changed),
                &bundle,
                &expected,
            )
            .is_err()
    );
    // The same real signature is offered with independently reviewed alternate
    // expectations. Exact cryptographic CLI flags or post-crypto identities reject.
    for case in [
        "repository",
        "commit",
        "ref",
        "signer-repository",
        "signer-workflow",
        "signer-digest",
        "predicate",
        "run",
        "attempt",
        "caller-workflow",
    ] {
        let mut wrong = expected.clone();
        match case {
            "repository" => {
                wrong.source.repository = "malancas/wrong-repo".into();
                wrong.caller_workflow.repository = wrong.source.repository.clone();
            }
            "commit" => {
                wrong.source.commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();
                wrong.caller_workflow.commit = wrong.source.commit.clone();
            }
            "ref" => wrong.source.git_ref = "refs/heads/wrong".into(),
            "signer-repository" => wrong.run.workflow.repository = "github/wrong-workflows".into(),
            "signer-workflow" => wrong.run.workflow.path = ".github/workflows/wrong.yml".into(),
            "signer-digest" => {
                wrong.run.workflow.commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()
            }
            "predicate" => {
                wrong.predicate = Predicate::CycloneDxV15;
                wrong.scope = EvidenceScope::SbomPredicate;
            }
            "run" => wrong.run.id += 1,
            "attempt" => wrong.run.attempt += 1,
            "caller-workflow" => wrong.caller_workflow.path = ".github/workflows/wrong.yml".into(),
            _ => unreachable!(),
        }
        let wrong_policy = self::policy(&wrong, policy.roots.verifier.clone());
        let wrong_verifier = open(&wrong_policy, &executable);
        assert!(
            wrong_verifier
                .verify(&artifact, &expected_bytes, &bundle, &wrong)
                .is_err(),
            "{case}"
        );
    }
    let download = tempfile::NamedTempFile::new().unwrap();
    let wrapper = json!({"bundle":original, "bundle_url":"https://untrusted.invalid/never-follow", "initiator":"untrusted-metadata"});
    fs::write(download.path(), serde_json::to_vec(&wrapper).unwrap()).unwrap();
    assert_eq!(
        verifier
            .verify_download(&artifact, &expected_bytes, download.path(), &expected,)
            .unwrap()
            .len(),
        1
    );
    fs::write(
        download.path(),
        format!(
            "{}\n{}\n",
            serde_json::to_string(&wrapper).unwrap(),
            serde_json::to_string(&signature).unwrap()
        ),
    )
    .unwrap();
    assert!(
        verifier
            .verify_download(&artifact, &expected_bytes, download.path(), &expected,)
            .is_err(),
        "Mixed valid/invalid download must not return a verified subset"
    );
    let mut private_policy = policy.clone();
    private_policy.roots.backend = TrustBackend::GithubPrivate;
    assert!(
        open(&private_policy, &executable)
            .verify(&artifact, &expected_bytes, &bundle, &expected)
            .is_err()
    );
    let policy_file = tempfile::NamedTempFile::new().unwrap();
    let bytes = serde_json::to_vec(&policy).unwrap();
    fs::write(policy_file.path(), &bytes).unwrap();
    assert!(
        OfflineVerifier::open(
            policy_file.path(),
            &"0".repeat(64),
            &executable,
            &fixture("trusted_root.json"),
        )
        .is_err()
    );
    let wrong_root = tempfile::NamedTempFile::new().unwrap();
    fs::write(wrong_root.path(), b"unapproved root").unwrap();
    assert!(
        OfflineVerifier::open(
            policy_file.path(),
            &ByteIdentity::from_bytes(&bytes).sha256,
            &executable,
            wrong_root.path(),
        )
        .is_err()
    );
    let mut expired = policy.clone();
    expired.review.expires_at = now() - 1;
    let expired_bytes = serde_json::to_vec(&expired).unwrap();
    fs::write(policy_file.path(), &expired_bytes).unwrap();
    assert!(
        OfflineVerifier::open(
            policy_file.path(),
            &ByteIdentity::from_bytes(&expired_bytes).sha256,
            &executable,
            &fixture("trusted_root.json")
        )
        .is_err()
    );
    // Even a separately approved policy cannot substitute arbitrary native code
    // for the compiled independently qualified verifier distribution.
    let mut changed_verifier = policy.clone();
    changed_verifier.roots.verifier.sha256 = "a".repeat(64);
    let changed_bytes = serde_json::to_vec(&changed_verifier).unwrap();
    fs::write(policy_file.path(), &changed_bytes).unwrap();
    assert!(
        OfflineVerifier::open(
            policy_file.path(),
            &ByteIdentity::from_bytes(&changed_bytes).sha256,
            &executable,
            &fixture("trusted_root.json")
        )
        .is_err()
    );
    assert_eq!(fs::read(&artifact).unwrap(), artifact_bytes);
}
