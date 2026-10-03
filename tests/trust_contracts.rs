use armorer::{
    config::{Config, load_lock},
    trust::{
        ByteIdentity, InputIdentity,
        capability::{
            Availability, CapabilityConfig, CapabilityId, CapabilityObservation, CapabilityPolicy,
            Catalog, Enforcement,
        },
        evidence::{ArtifactEvidence, EvidenceRequirements, Outcome, StepKind},
        inventory::{AssetRole, INVENTORY_BUNDLE_NAME, ReleaseInventory, expected_assets},
        policy::{EvidenceScope, Predicate, VerificationPolicy},
        publication::{
            GithubReceipt, GithubState, LifecycleRecord, PublishSet, RegistryReceipt,
            RegistryState, Trigger, TriggerContext, validate_trigger,
        },
    },
};
use serde::de::DeserializeOwned;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

const NOW: u64 = 1_790_870_400;
const CASES: &[&str] = &["linux-cli", "library", "workspace-service", "macos-final"];
/// Locate the frozen profile example without using consuming repository paths.
fn root(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/trust-v1")
        .join(name)
}
/// Deserialize an explicitly named frozen trust example for contract tests.
fn read<T: DeserializeOwned>(name: &str, file: &str) -> T {
    serde_json::from_slice(&std::fs::read(root(name).join(format!("{file}.json"))).unwrap())
        .unwrap()
}
/// Load the independently supplied configuration for the selected profile.
fn config(name: &str) -> Config {
    toml::from_str(&std::fs::read_to_string(root(name).join("armorer.toml")).unwrap()).unwrap()
}
/// Measure the exact committed catalog bytes used by the example.
fn catalog_bytes(name: &str) -> ByteIdentity {
    ByteIdentity::from_bytes(&std::fs::read(root(name).join("catalog.json")).unwrap())
}
/// Load the Linux CLI inventory used by isolated adversarial mutations.
fn inventory() -> ReleaseInventory {
    read("linux-cli", "release-inventory")
}
/// Load the independent Linux CLI verification policy.
fn policy() -> VerificationPolicy {
    read("linux-cli", "verification-policy")
}
/// Load the Linux CLI producer evidence before applying a single adversary.
fn evidence() -> ArtifactEvidence {
    read("linux-cli", "artifact-evidence")
}
/// Load independent evidence expectations separately from producer records.
fn requirements() -> EvidenceRequirements {
    read("linux-cli", "evidence-requirements")
}
/// Reconstruct every synthetic inventory asset as its expected exact fixture bytes.
fn bytes(i: &ReleaseInventory) -> BTreeMap<String, Vec<u8>> {
    i.assets
        .iter()
        .map(|a| (a.name.clone(), format!("fixture:{}", a.name).into_bytes()))
        .collect()
}
/// Check an offered inventory against fixed configuration, policy and input identity.
fn validate(i: &ReleaseInventory, expected: &InputIdentity) -> armorer::Result<()> {
    i.validate_against(&config("linux-cli"), &policy(), expected, NOW)
}

#[test]
/// Validate complete positive profile fixtures against exact config, lock, catalog and release contracts.
fn positive_profile_examples_bind_exact_config_lock_and_complete_chain() {
    for name in CASES {
        let c = config(name);
        c.validate(&root(name)).unwrap();
        let i: ReleaseInventory = read(name, "release-inventory");
        let p: VerificationPolicy = read(name, "verification-policy");
        let e: ArtifactEvidence = read(name, "artifact-evidence");
        let r: EvidenceRequirements = read(name, "evidence-requirements");
        let config_bytes = std::fs::read(root(name).join("armorer.toml")).unwrap();
        assert_eq!(
            ByteIdentity::from_bytes(&config_bytes).sha256,
            i.inputs.config_sha256
        );
        let (lock, lock_bytes) = load_lock(&root(name), &i.inputs.config_sha256)
            .unwrap()
            .unwrap();
        assert_eq!(
            ByteIdentity::from_bytes(&lock_bytes).sha256,
            i.inputs.lock_sha256
        );
        i.validate_against(&c, &p, &i.inputs, NOW).unwrap();
        i.compare_asset_bytes(&bytes(&i)).unwrap();
        e.validate_against_requirements(&c, &i, &r, NOW).unwrap();
        let capabilities: CapabilityConfig = read(name, "capability-config");
        capabilities
            .validate(&c, &i.inputs.config_sha256, &catalog_bytes(name), NOW)
            .unwrap();
        let catalog: Catalog = read(name, "catalog");
        catalog.validate(&lock, NOW).unwrap();
        let observation: CapabilityObservation = read(name, "capability-observation");
        observation
            .satisfies(
                CapabilityId::ImmutableReleases,
                CapabilityPolicy::Required,
                NOW,
                300,
            )
            .unwrap();
        let receipt: GithubReceipt = read(name, "github-receipt");
        receipt.validate(NOW, 300).unwrap();
        let lifecycle: LifecycleRecord = read(name, "lifecycle-record");
        lifecycle.validate(NOW).unwrap();
        let set: PublishSet = read(name, "publish-set");
        set.validate().unwrap();
        let registry: RegistryReceipt = read(name, "registry-receipt");
        registry.validate_against(&set, NOW).unwrap();
    }
}

#[test]
/// Reject altered distributable or SBOM bytes and mismatched inventory sizes.
fn rejects_tampered_artifact_and_sbom_bytes_and_wrong_sizes() {
    let i = inventory();
    for role in [
        AssetRole::Distributable,
        AssetRole::CargoSbom,
        AssetRole::AttestationBundle,
    ] {
        let mut supplied = bytes(&i);
        let name = &i.assets.iter().find(|a| a.role == role).unwrap().name;
        supplied.get_mut(name).unwrap()[0] ^= 1;
        assert!(i.compare_asset_bytes(&supplied).is_err());
    }
    let mut wrong = i.clone();
    wrong.assets[0].bytes.size += 1;
    assert!(wrong.compare_asset_bytes(&bytes(&i)).is_err());
    let mut supplied = bytes(&i);
    supplied.insert("unexpected.bin".into(), vec![1]);
    assert!(i.compare_asset_bytes(&supplied).is_err());
}

#[test]
/// Reject incomplete, expanded, duplicated or cyclic release inventories.
fn rejects_missing_extra_duplicate_assets_and_cycles() {
    let original = inventory();
    for index in 0..original.assets.len() {
        let mut i = original.clone();
        i.assets.remove(index);
        assert!(
            validate(&i, &original.inputs).is_err(),
            "missing asset {index}"
        );
    }
    let mut i = original.clone();
    i.assets.push(i.assets[0].clone());
    assert!(validate(&i, &original.inputs).is_err());
    let mut i = original.clone();
    i.assets[0].name = "extra.tar.gz".into();
    assert!(validate(&i, &original.inputs).is_err());
    let mut i = original.clone();
    let own_name = i.assets[0].name.clone();
    i.assets[0].subjects.push(own_name);
    assert!(validate(&i, &original.inputs).is_err());
    let mut i = original.clone();
    i.assets[0].name = INVENTORY_BUNDLE_NAME.into();
    assert!(validate(&i, &original.inputs).is_err());
    let mut i = original.clone();
    i.assets[0].name = "../escape".into();
    assert!(validate(&i, &original.inputs).is_err());
}

#[test]
/// Keep optional diagnostics separate from required assets and bind their provenance.
fn optional_diagnostics_cannot_replace_baseline_or_omit_their_own_provenance() {
    let original = inventory();
    let specs = expected_assets(&config("linux-cli"), &policy()).unwrap();
    let spec = specs
        .iter()
        .find(|s| s.role == AssetRole::Diagnostic)
        .unwrap();
    let mut i = original.clone();
    i.assets.push(armorer::trust::inventory::Asset {
        name: spec.name.clone(),
        role: spec.role,
        bytes: ByteIdentity::from_bytes(b"report"),
        subjects: spec.subjects.clone(),
        predicate: None,
        predicate_asset: None,
    });
    assert!(validate(&i, &original.inputs).is_err());
    let bundle = specs
        .iter()
        .find(|s| s.role == AssetRole::AttestationBundle && s.subjects == [spec.name.clone()])
        .unwrap();
    i.assets.push(armorer::trust::inventory::Asset {
        name: bundle.name.clone(),
        role: bundle.role,
        bytes: ByteIdentity::from_bytes(b"synthetic bundle"),
        subjects: bundle.subjects.clone(),
        predicate: bundle.predicate,
        predicate_asset: None,
    });
    validate(&i, &original.inputs).unwrap();
    i.assets.retain(|a| a.role != AssetRole::CargoSbom);
    assert!(validate(&i, &original.inputs).is_err());
}

#[test]
/// Reject producer substitutions for independently approved source and signer identities.
fn independently_reviewed_source_and_signer_cannot_be_replaced_by_release_claims() {
    let original = inventory();
    for mutation in [
        |i: &mut ReleaseInventory| i.inputs.source.repository = "attacker/project".into(),
        |i: &mut ReleaseInventory| i.inputs.source.commit = "c".repeat(40),
        |i: &mut ReleaseInventory| i.inputs.run.attempt += 1,
        |i: &mut ReleaseInventory| i.inputs.config_sha256 = "c".repeat(64),
    ] {
        let mut i = original.clone();
        mutation(&mut i);
        assert!(validate(&i, &original.inputs).is_err());
    }
    let p = policy();
    let source = &original.inputs.source;
    let workflow = &p.signers[0].workflow;
    p.accept_signer(
        source,
        workflow,
        Predicate::SlsaProvenanceV1,
        EvidenceScope::FinalArtifact,
        true,
    )
    .unwrap();
    let mut wrong_workflow = workflow.clone();
    wrong_workflow.commit = "f".repeat(40);
    assert!(
        p.accept_signer(
            source,
            &wrong_workflow,
            Predicate::SlsaProvenanceV1,
            EvidenceScope::FinalArtifact,
            true
        )
        .is_err()
    );
    wrong_workflow = workflow.clone();
    wrong_workflow.repository = "attacker/workflows".into();
    assert!(
        p.accept_signer(
            source,
            &wrong_workflow,
            Predicate::SlsaProvenanceV1,
            EvidenceScope::FinalArtifact,
            true
        )
        .is_err()
    );
    assert!(
        p.accept_signer(
            source,
            workflow,
            Predicate::CycloneDxV15,
            EvidenceScope::FinalArtifact,
            true
        )
        .is_err()
    );
    assert!(
        p.accept_signer(
            source,
            workflow,
            Predicate::SlsaProvenanceV1,
            EvidenceScope::FinalArtifact,
            false
        )
        .is_err()
    );
    let mut missing = p.clone();
    missing
        .signers
        .retain(|s| s.scope != EvidenceScope::Inventory);
    assert!(missing.validate(NOW).is_err());
}

#[test]
/// Reject unsafe trigger, fork, source and dispatch identities.
fn rejects_unauthorized_events_forks_source_and_dispatch_refs() {
    let source = inventory().inputs.source;
    let context = TriggerContext {
        event: "push".into(),
        repository: source.repository.clone(),
        source_commit: source.commit.clone(),
        git_ref: source.git_ref.clone(),
        fork: false,
    };
    validate_trigger(&context, &source, Trigger::StableTagPush, "main").unwrap();
    for event in [
        "pull_request",
        "pull_request_target",
        "workflow_run",
        "release",
        "workflow_dispatch",
    ] {
        let mut c = context.clone();
        c.event = event.into();
        assert!(validate_trigger(&c, &source, Trigger::StableTagPush, "main").is_err());
    }
    let mut c = context.clone();
    c.fork = true;
    assert!(validate_trigger(&c, &source, Trigger::StableTagPush, "main").is_err());
    c = context.clone();
    c.git_ref = "refs/heads/v1.0.0".into();
    assert!(validate_trigger(&c, &source, Trigger::StableTagPush, "main").is_err());
    c.event = "workflow_dispatch".into();
    c.git_ref = "refs/heads/main".into();
    let mut rehearsal_source = source.clone();
    rehearsal_source.git_ref = "refs/heads/main".into();
    validate_trigger(
        &c,
        &rehearsal_source,
        Trigger::ProtectedDefaultBranchRehearsal,
        "main",
    )
    .unwrap();
    c.git_ref = "refs/heads/feature".into();
    assert!(
        validate_trigger(
            &c,
            &source,
            Trigger::ProtectedDefaultBranchRehearsal,
            "main"
        )
        .is_err()
    );
}

#[test]
/// Reject unsigned substitution, Apple identity drift, failed notarization and broken step chains.
fn macos_chain_rejects_unsigned_final_substitution_wrong_team_and_failed_notary() {
    let name = "macos-final";
    let i: ReleaseInventory = read(name, "release-inventory");
    let e: ArtifactEvidence = read(name, "artifact-evidence");
    let r: EvidenceRequirements = read(name, "evidence-requirements");
    let check = |e: &ArtifactEvidence| e.validate_against_requirements(&config(name), &i, &r, NOW);
    let mut wrong = e.clone();
    wrong.steps[3].output = wrong.steps[0].output.clone();
    assert!(check(&wrong).is_err());
    let mut wrong = e.clone();
    wrong.steps[1].inputs = vec![ByteIdentity::from_bytes(b"other unsigned build")];
    assert!(check(&wrong).is_err());
    let mut wrong = e.clone();
    wrong.apple_assertions.as_mut().unwrap().team_id = "ZYXWVUTSRQ".into();
    assert!(check(&wrong).is_err());
    let mut wrong = e.clone();
    wrong
        .apple_assertions
        .as_mut()
        .unwrap()
        .notarization_outcome = Outcome::Failed;
    assert!(check(&wrong).is_err());
    let mut wrong = e.clone();
    wrong.steps[1].run.attempt += 1;
    assert!(check(&wrong).is_err());
    let mut wrong = e.clone();
    wrong.steps[1].run.workflow.commit = "e".repeat(40);
    assert!(check(&wrong).is_err());
    let mut wrong = e.clone();
    wrong.steps.remove(2);
    assert!(check(&wrong).is_err());
    assert_eq!(e.steps[0].kind, StepKind::Build);
    assert_ne!(e.steps[0].output, e.steps[1].output);
    assert_eq!(e.steps[1].output, e.steps[2].output); // Standalone ticket doesn't mutate executable.
}

#[test]
/// Reject tool pin, coverage scope, freshness and enforcement drift.
fn tool_policy_scope_freshness_and_outcomes_cannot_be_weakened_by_producer() {
    let e = evidence();
    let i = inventory();
    let r = requirements();
    let check =
        |e: &ArtifactEvidence| e.validate_against_requirements(&config("linux-cli"), &i, &r, NOW);
    for mutation in [
        |e: &mut ArtifactEvidence| e.tools[0].bytes.sha256 = "f".repeat(64),
        |e: &mut ArtifactEvidence| e.tools[0].version = "0.5.8".into(),
        |e: &mut ArtifactEvidence| e.coverage[0].scope = "different-scope".into(),
        |e: &mut ArtifactEvidence| e.coverage[0].omissions.clear(),
        |e: &mut ArtifactEvidence| e.coverage[0].outcome = Outcome::Unknown,
        |e: &mut ArtifactEvidence| e.coverage[0].enforcement = Enforcement::Reporting,
        |e: &mut ArtifactEvidence| e.tools.clear(),
        |e: &mut ArtifactEvidence| e.coverage.clear(),
    ] {
        let mut wrong = e.clone();
        mutation(&mut wrong);
        assert!(check(&wrong).is_err());
    }
    assert!(
        e.validate_against_requirements(&config("linux-cli"), &i, &r, NOW + 3601)
            .is_err()
    );
}

#[test]
/// Require an explicit historical source and exact approved asset bytes.
fn historical_policy_requires_explicit_exact_source_and_byte_allowlist() {
    let p: VerificationPolicy = serde_json::from_slice(
        &std::fs::read(root("").join("historical-verification-policy.json")).unwrap(),
    )
    .unwrap();
    let source = p.historical[0].source.clone();
    let mut assets = BTreeMap::from([(
        "legacy.tar.gz".into(),
        b"synthetic historical artifact".to_vec(),
    )]);
    p.compare_historical_bytes("v0.1.0", &source, &assets, NOW)
        .unwrap();
    assert!(p.accept_source(&source).is_err());
    assert!(
        p.compare_historical_bytes("v1.0.0", &inventory().inputs.source, &assets, NOW)
            .is_err()
    );
    assets.get_mut("legacy.tar.gz").unwrap()[0] ^= 1;
    assert!(
        p.compare_historical_bytes("v0.1.0", &source, &assets, NOW)
            .is_err()
    );
    assert!(
        p.compare_historical_bytes("v0.1.0", &source, &assets, NOW + 86400)
            .is_err()
    );
    let mut collision = p.clone();
    collision.historical[0].source = collision.sources[0].clone();
    assert!(collision.validate(NOW).is_err());
}

#[test]
/// Keep unavailable, unknown, errored and reporting capabilities from satisfying required gates.
fn capability_unknown_unsupported_error_and_reporting_fail_required_gate() {
    let original: CapabilityObservation = read("linux-cli", "capability-observation");
    for availability in [
        Availability::Unknown,
        Availability::Unsupported,
        Availability::Error,
    ] {
        let mut observation = original.clone();
        observation.availability = availability;
        assert!(
            observation
                .satisfies(
                    CapabilityId::ImmutableReleases,
                    CapabilityPolicy::Required,
                    NOW,
                    300
                )
                .is_err()
        );
    }
    let mut observation = original.clone();
    observation.enforcement = Enforcement::Reporting;
    assert!(
        observation
            .satisfies(
                CapabilityId::ImmutableReleases,
                CapabilityPolicy::Required,
                NOW,
                300
            )
            .is_err()
    );
    observation
        .satisfies(
            CapabilityId::ImmutableReleases,
            CapabilityPolicy::Reporting,
            NOW,
            300,
        )
        .unwrap();
    assert!(
        original
            .satisfies(
                CapabilityId::ImmutableReleases,
                CapabilityPolicy::Required,
                NOW + 301,
                300
            )
            .is_err()
    );
    let mut capabilities: CapabilityConfig = read("linux-cli", "capability-config");
    capabilities.decisions.insert(
        CapabilityId::PlatformAttestations,
        CapabilityPolicy::Disabled,
    );
    assert!(
        capabilities
            .validate(
                &config("linux-cli"),
                &inventory().inputs.config_sha256,
                &catalog_bytes("linux-cli"),
                NOW
            )
            .is_err()
    );
}

#[test]
/// Reject conflicting draft retry identities and absent publication receipts.
fn conflicting_github_retry_identity_and_missing_publication_receipts_are_rejected() {
    let receipt: GithubReceipt = read("linux-cli", "github-receipt");
    receipt.same_owned_retry(&receipt).unwrap();
    for mutation in [
        |r: &mut GithubReceipt| r.inputs.run.attempt += 1,
        |r: &mut GithubReceipt| r.inventory.sha256 = "f".repeat(64),
        |r: &mut GithubReceipt| r.release_id += 1,
        |r: &mut GithubReceipt| r.inputs.source.commit = "f".repeat(40),
    ] {
        let mut wrong = receipt.clone();
        mutation(&mut wrong);
        assert!(receipt.same_owned_retry(&wrong).is_err());
    }
    for state in [
        GithubState::DraftBytesVerified,
        GithubState::Approved,
        GithubState::Published,
        GithubState::ReleaseAttestationVerified,
        GithubState::ProvenanceVerified,
    ] {
        let mut wrong = receipt.clone();
        wrong.state = state;
        assert!(wrong.validate(NOW, 300).is_err());
    }
}

#[test]
/// Validate explicit registry progress, dependency ordering, byte identity and retry conflicts.
fn registry_partial_dag_index_lag_wrong_bytes_and_retry_conflicts_are_explicit() {
    let set: PublishSet = read("linux-cli", "publish-set");
    let mut receipt: RegistryReceipt = read("linux-cli", "registry-receipt");
    receipt.crates[0].state = RegistryState::UploadResultUnknown;
    receipt.validate_against(&set, NOW).unwrap();
    receipt.crates[1].state = RegistryState::Uploaded;
    assert!(receipt.validate_against(&set, NOW).is_err());
    receipt.crates[1].state = RegistryState::Prepared;
    receipt.crates[0].state = RegistryState::RegistryObserved;
    receipt.crates[0].observation = Some(ByteIdentity::from_bytes(b"index observation"));
    receipt.validate_against(&set, NOW).unwrap();
    receipt.crates[0].state = RegistryState::RegistryBytesVerified;
    assert!(receipt.validate_against(&set, NOW).is_err());
    receipt.crates[0].registry_bytes = Some(set.crates[0].archive.clone());
    receipt.crates[0].index_sha256 = Some(set.crates[0].archive.sha256.clone());
    receipt.crates[1].state = RegistryState::Uploaded;
    receipt.validate_against(&set, NOW).unwrap();
    let mut wrong = receipt.clone();
    wrong.crates[0].registry_bytes.as_mut().unwrap().sha256 = "f".repeat(64);
    assert!(wrong.validate_against(&set, NOW).is_err());
    let mut set2 = set.clone();
    set2.crates[0].dependencies = vec![set2.crates[1].name.clone()];
    assert!(set2.validate().is_err());
    set2 = set.clone();
    set2.crates[0].archive.sha256 = "f".repeat(64);
    let mut retry = receipt.clone();
    retry.publish_set = set2.identity().unwrap();
    assert!(receipt.same_retry_set(&retry).is_err());
}

#[test]
/// Reject unsupported commands, offered URLs and custom predicate values during parsing.
fn unknown_commands_urls_and_custom_predicates_never_deserialize() {
    let mut value = serde_json::to_value(policy()).unwrap();
    value["signers"][0]["predicate"] = "https://attacker.invalid/predicate".into();
    assert!(serde_json::from_value::<VerificationPolicy>(value).is_err());
    let mut value = serde_json::to_value(evidence()).unwrap();
    value["command"] = "echo caller".into();
    assert!(serde_json::from_value::<ArtifactEvidence>(value).is_err());
    let mut value: serde_json::Value = read("linux-cli", "catalog");
    value["adapters"][0]["url"] = "https://attacker.invalid".into();
    assert!(serde_json::from_value::<Catalog>(value).is_err());
}

#[test]
/// Compare generated version-one schemas with their exact committed bytes.
fn generated_schemas_match_all_committed_version_one_schemas() {
    let kinds = [
        "config",
        "lock",
        "plan",
        "release-inventory",
        "verification-policy",
        "artifact-evidence",
        "evidence-requirements",
        "capability-config",
        "catalog",
        "capability-observation",
        "lifecycle-record",
        "github-receipt",
        "publish-set",
        "registry-receipt",
    ];
    for kind in kinds {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_armorer"))
            .args(["schema", kind])
            .output()
            .unwrap();
        assert!(output.status.success(), "schema {kind}");
        let committed = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("schemas/{kind}-v1.json")),
        )
        .unwrap();
        assert_eq!(output.stdout, committed, "schema drift: {kind}");
    }
}

#[test]
/// Reject ambiguous, expanded, indirect and oversized trust records before use.
fn strict_loader_rejects_duplicate_map_keys_unknown_fields_symlinks_and_size_limit() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("contract.json");
    std::fs::write(&path, br#"{"outer":{"required":true,"required":false}}"#).unwrap();
    assert!(armorer::trust::load_json::<serde_json::Value>(temp.path(), "contract.json").is_err());
    std::fs::write(&path, vec![b' '; 1_048_577]).unwrap();
    assert!(armorer::trust::load_json::<serde_json::Value>(temp.path(), "contract.json").is_err());
    assert!(armorer::trust::load_json::<serde_json::Value>(temp.path(), "../escape.json").is_err());
    let (loaded, exact): (VerificationPolicy, Vec<u8>) =
        armorer::trust::load_json(&root("linux-cli"), "verification-policy.json").unwrap();
    loaded.validate(NOW).unwrap();
    assert_eq!(
        exact,
        std::fs::read(root("linux-cli").join("verification-policy.json")).unwrap()
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            root("linux-cli").join("verification-policy.json"),
            temp.path().join("link.json"),
        )
        .unwrap();
        assert!(armorer::trust::load_json::<VerificationPolicy>(temp.path(), "link.json").is_err());
    }
}

#[test]
/// Reject expired reviews, nonwaivable exceptions and stale database evidence.
fn expired_review_nonwaivable_exception_and_stale_database_fail_closed() {
    let mut p = policy();
    p.review.expires_at = NOW;
    assert!(p.validate(NOW).is_err());
    let mut e = evidence();
    let mut r = requirements();
    let i = inventory();
    for t in [&mut e.tools[0], &mut r.tools[0]] {
        t.kind = armorer::trust::evidence::InputKind::Database;
        t.max_age_seconds = Some(10);
    }
    assert!(
        e.validate_against_requirements(&config("linux-cli"), &i, &r, NOW)
            .is_err()
    );
    e.tools[0].max_age_seconds = Some(1000);
    assert!(
        e.validate_against_requirements(&config("linux-cli"), &i, &r, NOW)
            .is_err()
    );
    let exception = armorer::trust::evidence::Exception {
        id: "fixture-exception".into(),
        capability: CapabilityId::PlatformAttestations,
        tool: "cargo-cyclonedx".into(),
        rule: "rule".into(),
        tool_version: "0.5.9".into(),
        subject: i.assets[0].bytes.clone(),
        policy: ByteIdentity::from_bytes(b"reviewed policy"),
        review: policy().review,
        positive_control: ByteIdentity::from_bytes(b"adjacent positive control"),
    };
    assert!(exception.validate_at(NOW).is_err());
}

#[test]
/// Require separate publication receipts and a current immutable-release setting.
fn complete_publication_snapshot_requires_separate_receipts_and_current_setting() {
    let mut receipt: GithubReceipt = read("linux-cli", "github-receipt");
    receipt.state = GithubState::ProvenanceVerified;
    receipt.draft_download_receipt =
        Some(ByteIdentity::from_bytes(b"draft exact-byte verification"));
    receipt.approval = Some(policy().review);
    receipt.immutable_setting = Some(read("linux-cli", "capability-observation"));
    receipt.published_at = Some(NOW);
    receipt.release_attestation = Some(ByteIdentity::from_bytes(
        b"release attestation verification",
    ));
    receipt.provenance_verification = Some(ByteIdentity::from_bytes(b"provenance verification"));
    receipt.validate(NOW, 300).unwrap();
    receipt.immutable_setting.as_mut().unwrap().availability = Availability::Unknown;
    assert!(receipt.validate(NOW, 300).is_err());
}

#[test]
/// Require detached inventory authentication without embedding its own digest cycle.
fn detached_inventory_authentication_is_required_without_creating_a_hash_cycle() {
    let i = inventory();
    let serialized = serde_json::to_vec(&i).unwrap();
    let detached = b"synthetic detached authentication bundle".to_vec();
    let expected = ByteIdentity::from_bytes(&serialized);
    let expected_bundle = ByteIdentity::from_bytes(&detached);
    let mut all = bytes(&i);
    all.insert(armorer::trust::inventory::INVENTORY_NAME.into(), serialized);
    all.insert(INVENTORY_BUNDLE_NAME.into(), detached);
    i.compare_published_bytes(&all, &expected, &expected_bundle)
        .unwrap();
    all.remove(INVENTORY_BUNDLE_NAME);
    assert!(
        i.compare_published_bytes(&all, &expected, &expected_bundle)
            .is_err()
    );
    let mut receipt: GithubReceipt = read("linux-cli", "github-receipt");
    receipt.state = GithubState::Published;
    assert!(receipt.same_owned_retry(&receipt).is_err());
}

#[test]
/// Keep branch and rehearsal lifecycle records from authorizing stable publication.
fn branch_lifecycle_and_rehearsal_identity_cannot_authorize_release_publication() {
    let mut i = inventory();
    i.inputs.source.git_ref = "refs/heads/main".into();
    i.inputs.validate().unwrap();
    let mut p = policy();
    p.mode = armorer::trust::policy::VerificationMode::Rehearsal;
    p.sources = vec![i.inputs.source.clone()];
    p.validate(NOW).unwrap();
    i.validate_against(&config("linux-cli"), &p, &i.inputs, NOW)
        .unwrap();
    p.mode = armorer::trust::policy::VerificationMode::Release;
    assert!(p.validate(NOW).is_err());
    let mut receipt: GithubReceipt = read("linux-cli", "github-receipt");
    receipt.inputs = i.inputs;
    assert!(receipt.validate(NOW, 300).is_err());
    let mut lifecycle: LifecycleRecord = read("linux-cli", "lifecycle-record");
    lifecycle.inputs.source.git_ref = "refs/heads/main".into();
    lifecycle.validate(NOW).unwrap();
    let mut set: PublishSet = read("linux-cli", "publish-set");
    set.inputs.source.git_ref = "refs/heads/main".into();
    assert!(set.validate().is_err());
}

#[test]
/// Derive distinct assets for each workspace package, target and feature selection.
fn workspace_derived_assets_keep_each_package_target_and_feature_case_separate() {
    let c = config("workspace");
    c.validate(&root("workspace")).unwrap();
    let specs = expected_assets(&c, &policy()).unwrap();
    let final_names: Vec<_> = specs
        .iter()
        .filter(|s| s.role == AssetRole::Distributable)
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(
        final_names,
        [
            "library--x86_64-unknown-linux-gnu--minimal.crate",
            "cli--x86_64-unknown-linux-gnu--minimal.tar.gz",
            "cli--aarch64-apple-darwin--minimal.tar.gz",
            "service--aarch64-unknown-linux-gnu--server.tar.gz"
        ]
    );
}

#[test]
/// Reject substituted catalog identities and unsupported evidence versions.
fn catalog_binding_and_unknown_schema_versions_fail_closed() {
    let c: Catalog = read("linux-cli", "catalog");
    let i = inventory();
    let (mut lock, _) = load_lock(&root("linux-cli"), &i.inputs.config_sha256)
        .unwrap()
        .unwrap();
    lock.tools.get_mut("cargo-cyclonedx").unwrap().sha256 = "f".repeat(64);
    assert!(c.validate(&lock, NOW).is_err());
    let mut i = inventory();
    i.schema_version = 2;
    assert!(validate(&i, &inventory().inputs).is_err());
    let mut p = policy();
    p.schema_version = 2;
    assert!(p.validate(NOW).is_err());
    let mut e = evidence();
    e.schema_version = 2;
    assert!(
        e.validate_against_requirements(&config("linux-cli"), &inventory(), &requirements(), NOW)
            .is_err()
    );
}

#[test]
/// Bind each capability observation to its explicitly named capability.
fn capability_observation_cannot_satisfy_a_different_capability() {
    let observation: CapabilityObservation = read("linux-cli", "capability-observation");
    observation
        .satisfies(
            CapabilityId::ImmutableReleases,
            CapabilityPolicy::Required,
            NOW,
            300,
        )
        .unwrap();
    for policy in [
        CapabilityPolicy::Required,
        CapabilityPolicy::Reporting,
        CapabilityPolicy::Disabled,
    ] {
        assert!(
            observation
                .satisfies(CapabilityId::ProtectedTags, policy, NOW, 300)
                .is_err()
        );
    }
}

#[test]
/// Reject catalog byte substitutions against independently approved capability configuration.
fn capability_config_rejects_independent_catalog_digest_and_size_substitution() {
    let c = config("linux-cli");
    let i = inventory();
    let original: CapabilityConfig = read("linux-cli", "capability-config");
    let expected = catalog_bytes("linux-cli");
    original
        .validate(&c, &i.inputs.config_sha256, &expected, NOW)
        .unwrap();
    let mut forged = original.clone();
    forged.catalog.sha256 = "f".repeat(64);
    assert!(
        forged
            .validate(&c, &i.inputs.config_sha256, &expected, NOW)
            .is_err()
    );
    let mut forged = original.clone();
    forged.catalog.size += 1;
    assert!(
        forged
            .validate(&c, &i.inputs.config_sha256, &expected, NOW)
            .is_err()
    );
}

#[test]
/// Admit registry byte evidence only in verified or explicit conflict states.
fn registry_byte_evidence_requires_verified_or_conflict_state() {
    let set: PublishSet = read("linux-cli", "publish-set");
    let base: RegistryReceipt = read("linux-cli", "registry-receipt");
    for state in [
        RegistryState::Prepared,
        RegistryState::Uploaded,
        RegistryState::UploadResultUnknown,
        RegistryState::RegistryObserved,
    ] {
        for digest_only in [false, true] {
            let mut wrong = base.clone();
            wrong.crates[0].state = state;
            wrong.crates[0].observation = Some(ByteIdentity::from_bytes(b"registry observation"));
            if digest_only {
                wrong.crates[0].index_sha256 = Some(set.crates[0].archive.sha256.clone());
            } else {
                wrong.crates[0].registry_bytes = Some(set.crates[0].archive.clone());
            }
            assert!(wrong.validate_against(&set, NOW).is_err());
        }
    }
    let mut conflict = base;
    conflict.crates[0].state = RegistryState::Conflict;
    conflict.crates[0].registry_bytes = Some(ByteIdentity::from_bytes(b"conflicting served bytes"));
    conflict.crates[0].index_sha256 = Some("f".repeat(64));
    conflict.crates[0].conflict =
        Some(armorer::trust::publication::ConflictRecovery::ReviewNewVersion);
    conflict.validate_against(&set, NOW).unwrap();
}

#[test]
/// Reject nonstable tag references at every release authorization gate.
fn all_release_gates_reject_nonstable_tag_refs() {
    for invalid in [
        "refs/tags/not-semver",
        "refs/tags/v1.0.0-rc.1",
        "refs/tags/v1.0.0+metadata",
        "refs/heads/v1.0.0",
    ] {
        let mut lifecycle: LifecycleRecord = read("linux-cli", "lifecycle-record");
        lifecycle.stage = armorer::trust::publication::LifecycleStage::Published;
        lifecycle.inputs.source.git_ref = invalid.into();
        assert!(lifecycle.validate(NOW).is_err());
        let mut set: PublishSet = read("linux-cli", "publish-set");
        set.inputs.source.git_ref = invalid.into();
        assert!(set.validate().is_err());
        let source = set.inputs.source;
        let trigger = TriggerContext {
            event: "push".into(),
            repository: source.repository.clone(),
            source_commit: source.commit.clone(),
            git_ref: source.git_ref.clone(),
            fork: false,
        };
        assert!(validate_trigger(&trigger, &source, Trigger::StableTagPush, "main").is_err());
        let mut receipt: GithubReceipt = read("linux-cli", "github-receipt");
        receipt.inputs.source.git_ref = invalid.into();
        assert!(receipt.validate(NOW, 300).is_err());
    }
}

/// Preserve independently approved observation times even when producer freshness checks pass.
#[test]
fn producer_cannot_refresh_independently_approved_tool_observation_time() {
    use armorer::trust::evidence::InputKind;
    let c = config("linux-cli");
    let i = inventory();
    for kind in [InputKind::Tool, InputKind::Database] {
        let mut e = evidence();
        let mut r = requirements();
        e.tools[0].kind = kind;
        r.tools[0].kind = kind;
        e.tools[0].max_age_seconds = Some(60);
        r.tools[0].max_age_seconds = Some(60);
        e.tools[0].observed_at = NOW - 30;
        r.tools[0].observed_at = NOW - 30;
        e.validate_against_requirements(&c, &i, &r, NOW).unwrap();
        // Same pins and authenticated record, but an independent observation is stale.
        r.tools[0].observed_at = NOW - 61;
        e.validate_release_chain(&c, &i, &r.inputs, &r.catalog, None, NOW)
            .unwrap();
        assert_eq!(
            e.validate_against_requirements(&c, &i, &r, NOW)
                .unwrap_err()
                .to_string(),
            "invalid configuration: evidence-tool-pin-mismatch"
        );
        // The timestamp cannot change even when both observations are still fresh.
        r.tools[0].observed_at = NOW - 31;
        assert_eq!(
            e.validate_against_requirements(&c, &i, &r, NOW)
                .unwrap_err()
                .to_string(),
            "invalid configuration: evidence-tool-pin-mismatch"
        );
    }
}

/// Bind use of each reviewed exception to its independently approved coverage record.
#[test]
fn producer_cannot_attach_allowed_exceptions_to_exception_free_coverage() {
    use armorer::trust::evidence::Exception;
    let c = config("linux-cli");
    let i = inventory();
    let mut e = evidence();
    let mut r = requirements();
    // Dependency policy permits explicitly reviewed exceptions, unlike trust checks.
    e.coverage[0].capability = CapabilityId::DependencyPolicy;
    r.required_coverage[0].capability = CapabilityId::DependencyPolicy;
    let exception = Exception {
        id: "reviewed-dependency-rule".into(),
        capability: CapabilityId::DependencyPolicy,
        tool: e.tools[0].name.clone(),
        rule: "reviewed-rule".into(),
        tool_version: e.tools[0].version.clone(),
        subject: e.coverage[0].tested_subject.clone(),
        policy: ByteIdentity::from_bytes(b"independent policy"),
        review: r.review.clone(),
        positive_control: ByteIdentity::from_bytes(b"retained positive control"),
    };
    e.exceptions.push(exception.clone());
    r.allowed_exceptions.push(exception.clone());
    e.validate_against_requirements(&c, &i, &r, NOW).unwrap();
    e.coverage[0].exception_ids.push(exception.id.clone());
    // The exception itself is valid and allowed: only its unapproved use is rejected.
    e.validate_release_chain(&c, &i, &r.inputs, &r.catalog, None, NOW)
        .unwrap();
    assert_eq!(
        e.validate_against_requirements(&c, &i, &r, NOW)
            .unwrap_err()
            .to_string(),
        "invalid configuration: required-coverage-not-enforced"
    );
    r.required_coverage[0].exception_ids.push(exception.id);
    e.validate_against_requirements(&c, &i, &r, NOW).unwrap();
    e.coverage[0].exception_ids.clear();
    assert_eq!(
        e.validate_against_requirements(&c, &i, &r, NOW)
            .unwrap_err()
            .to_string(),
        "invalid configuration: required-coverage-not-enforced"
    );
}
