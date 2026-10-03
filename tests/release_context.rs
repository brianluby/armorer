//! Synthetic authority tests never establish signatures, upstream tool approval or release acceptance.
use armorer::{
    trust::ByteIdentity,
    verification::{
        context::{CONTEXT_NAME, TrustedReleaseContext},
        cyclonedx::qualified_native_validator,
    },
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// Locate frozen examples without writing to their source directory.
fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/trust-v1/linux-cli")
        .join(name)
}
/// Read one inert synthetic source example.
fn read(name: &str) -> Value {
    serde_json::from_slice(&fs::read(example(name)).unwrap()).unwrap()
}
/// Read the actual clock for explicitly test-only, short-lived approval records.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
/// Serialize exact test-owned JSON bytes and return their identity.
fn write(path: &Path, value: &Value) -> Value {
    let bytes = serde_json::to_vec(value).unwrap();
    fs::write(path, &bytes).unwrap();
    serde_json::to_value(ByteIdentity::from_bytes(&bytes)).unwrap()
}
/// Construct independent synthetic authorities, never a context learned from downloaded release JSON.
fn fixture() -> (tempfile::TempDir, Value) {
    let directory = tempfile::tempdir().unwrap();
    for name in ["armorer.toml", "armorer.lock"] {
        fs::copy(example(name), directory.path().join(name)).unwrap();
    }
    let cargo = b"# Inert context hashing fixture; no Cargo invocation or dependency resolution\nversion = 4\n";
    fs::write(directory.path().join("Cargo.lock"), cargo).unwrap();
    let review = json!({"owner":"fixture-reviewer", "rationale":"Synthetic context checks only; not upstream approval or release acceptance", "reviewed_at":now()-60, "expires_at":now()+3600, "record":ByteIdentity::from_bytes(b"synthetic approval receipt")});
    let mut catalog = read("catalog.json");
    catalog["adapters"][0]["review"] = review.clone();
    for (id, capability, name) in [
        ("rust-native", "dependency-policy", "rustc"),
        (
            "github-sigstore",
            "platform-attestations",
            "attest-build-provenance",
        ),
    ] {
        let mut adapter = catalog["adapters"][0].clone();
        adapter["id"] = id.into();
        adapter["capability"] = capability.into();
        adapter["pins"][0]["name"] = name.into();
        adapter["pins"][0]["distribution"] =
            serde_json::to_value(ByteIdentity::from_bytes(name.as_bytes())).unwrap();
        catalog["adapters"].as_array_mut().unwrap().push(adapter);
    }
    let catalog_identity = write(&directory.path().join("catalog.json"), &catalog);
    let mut policy = read("verification-policy.json");
    policy["review"] = review.clone();
    policy["roots"]["review"] = review.clone();
    let policy_identity = write(&directory.path().join("verification-policy.json"), &policy);
    let mut requirements = read("evidence-requirements.json");
    requirements["review"] = review.clone();
    requirements["inputs"]["cargo_lock_sha256"] = ByteIdentity::from_bytes(cargo).sha256.into();
    requirements["catalog"] = catalog_identity.clone();
    let tool = requirements["tools"][0].clone();
    requirements["tools"] = Value::Array(
        catalog["adapters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|adapter| {
                let pin = &adapter["pins"][0];
                let mut item = tool.clone();
                item["name"] = pin["name"].clone();
                item["version"] = pin["version"].clone();
                item["bytes"] = pin["distribution"].clone();
                item["authentication_record"] = pin["authentication_record"].clone();
                item["observed_at"] = (now() - 30).into();
                item
            })
            .collect(),
    );
    let coverage = requirements["required_coverage"][0].clone();
    for capability in ["dependency-policy", "platform-attestations"] {
        let mut item = coverage.clone();
        item["capability"] = capability.into();
        requirements["required_coverage"]
            .as_array_mut()
            .unwrap()
            .push(item);
    }
    let input = requirements["inputs"].clone();
    let signers = policy["signers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|signer| {
            (
                signer["scope"].as_str().unwrap().to_owned(),
                signer["workflow"].clone(),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let native = qualified_native_validator()
        .unwrap_or_else(|_| ByteIdentity::from_bytes(b"unsupported host rejection fixture"));
    let context = json!({"schema_version":1, "inputs":input, "caller_workflow":{"repository":"fixture/project", "path":".github/workflows/release.yml", "commit":"a".repeat(40)}, "trigger":"push", "catalog":catalog_identity, "verification_policy":policy_identity, "native_sbom_validator":native, "signers":signers, "selections":[{"selection":requirements["selection"], "root_component_name":"app", "evidence_requirements":requirements}], "review":review});
    (directory, context)
}
/// Approve the exact synthetic context bytes for a semantic check, separate from release trust.
fn open(directory: &Path, context: &Value) -> armorer::Result<TrustedReleaseContext> {
    let identity: ByteIdentity =
        serde_json::from_value(write(&directory.join(CONTEXT_NAME), context)).unwrap();
    TrustedReleaseContext::open(directory, &identity.sha256)
}
/// Check host support explicitly; the production constructor still rejects all unsupported hosts.
fn supported() -> bool {
    qualified_native_validator().is_ok()
}

#[test]
/// Approved context loading is read-only and never executes consuming build scripts.
fn approved_context_reads_exact_authorities_without_executing_repository_code() {
    let (directory, context) = fixture();
    fs::write(
        directory.path().join("build.rs"),
        b"fn main() { std::fs::write(\"executed\", b\"bad\").unwrap(); }",
    )
    .unwrap();
    fs::write(
        directory.path().join("Cargo.toml"),
        b"[package]\nname='app'\nversion='1.0.0'\nbuild='build.rs'\n",
    )
    .unwrap();
    let before: Vec<_> = [
        "armorer.toml",
        "armorer.lock",
        "Cargo.lock",
        "catalog.json",
        "verification-policy.json",
        "Cargo.toml",
        "build.rs",
    ]
    .into_iter()
    .map(|name| (name, fs::read(directory.path().join(name)).unwrap()))
    .collect();
    let result = open(directory.path(), &context);
    if supported() {
        let trusted = result.unwrap();
        assert_eq!(trusted.inputs().source.commit, "a".repeat(40));
        assert_eq!(trusted.selections().len(), 1);
        assert_eq!(
            trusted.identity(),
            &ByteIdentity::from_bytes(&fs::read(directory.path().join(CONTEXT_NAME)).unwrap())
        );
    } else {
        assert!(result.is_err());
    }
    assert!(!directory.path().join("executed").exists());
    assert!(!directory.path().join("target").exists());
    for (name, bytes) in before {
        assert_eq!(fs::read(directory.path().join(name)).unwrap(), bytes);
    }
}

#[test]
/// A newly supplied producer-looking context cannot replace the approved context digest.
fn context_digest_is_checked_before_parsing_or_other_authorities() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join(CONTEXT_NAME),
        b"not JSON and no other files",
    )
    .unwrap();
    let error = TrustedReleaseContext::open(directory.path(), &"f".repeat(64))
        .err()
        .unwrap();
    assert!(error.to_string().contains("unapproved-release-context"));
    assert!(TrustedReleaseContext::open(directory.path(), "moving-ref").is_err());
}

#[test]
/// Changing any separately retained authority is rejected despite unchanged approved context bytes.
fn exact_config_lock_catalog_policy_and_cargo_lock_substitution_fail() {
    for name in [
        "armorer.toml",
        "armorer.lock",
        "Cargo.lock",
        "catalog.json",
        "verification-policy.json",
    ] {
        let (directory, context) = fixture();
        let mut bytes = fs::read(directory.path().join(name)).unwrap();
        bytes.push(b'\n');
        fs::write(directory.path().join(name), bytes).unwrap();
        assert!(
            open(directory.path(), &context).is_err(),
            "accepted substituted {name}"
        );
    }
}

#[test]
/// An approved digest does not excuse expired reviews or mismatched source, caller, workflow and run bindings.
fn context_rejects_wrong_source_caller_workflow_run_trigger_and_review() {
    if !supported() {
        return;
    }
    for pointer in [
        "/inputs/source/commit",
        "/caller_workflow/commit",
        "/signers/inventory/commit",
        "/inputs/run/workflow/commit",
        "/inputs/run/attempt",
        "/trigger",
        "/review/expires_at",
    ] {
        let (directory, mut context) = fixture();
        *context.pointer_mut(pointer).unwrap() = match pointer {
            "/inputs/run/attempt" => 0.into(),
            "/review/expires_at" => (now() - 1).into(),
            "/trigger" => "pull_request".into(),
            _ => "c".repeat(40).into(),
        };
        assert!(
            open(directory.path(), &context).is_err(),
            "accepted wrong {pointer}"
        );
    }
    let (directory, mut context) = fixture();
    context["signers"]
        .as_object_mut()
        .unwrap()
        .remove("inventory");
    assert!(open(directory.path(), &context).is_err());
}

#[test]
/// Selection roots and required pins/coverage/steps cannot silently disappear or be independently substituted.
fn approved_context_still_requires_complete_baseline_selection_and_evidence() {
    if !supported() {
        return;
    }
    for case in 0..14 {
        let (directory, mut context) = fixture();
        let selected = &mut context["selections"][0];
        match case {
            0 => selected["root_component_name"] = "".into(),
            1 => selected["selection"]["package_version"] = "latest".into(),
            2 => selected["selection"]["target"] = "aarch64-unknown-linux-gnu".into(),
            3 => selected["evidence_requirements"]["steps"] = json!({}),
            4 => selected["evidence_requirements"]["tools"] = json!([]),
            5 => selected["evidence_requirements"]["required_coverage"] = json!([]),
            6 => selected["evidence_requirements"]["tools"][0]["bytes"]["size"] = 123.into(),
            7 => {
                selected["evidence_requirements"]["tools"][0]["authentication_record"]["sha256"] =
                    "d".repeat(64).into()
            }
            8 => {
                selected["evidence_requirements"]["tools"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            9 => {
                selected["evidence_requirements"]["required_coverage"][0]["outcome"] =
                    "unknown".into()
            }
            10 => {
                selected["evidence_requirements"]["required_coverage"][0]["enforcement"] =
                    "reporting".into()
            }
            11 => {
                selected["evidence_requirements"]["steps"]["build"]["commit"] =
                    "d".repeat(40).into()
            }
            12 => {
                selected["evidence_requirements"]["tools"][0]["observed_at"] = (now() + 3600).into()
            }
            _ => selected["evidence_requirements"]["apple_team"] = "ABCDE12345".into(),
        }
        assert!(
            open(directory.path(), &context).is_err(),
            "accepted missing/substituted baseline case {case}"
        );
    }
    let (directory, mut context) = fixture();
    let selected = context["selections"][0].clone();
    context["selections"].as_array_mut().unwrap().push(selected);
    assert!(open(directory.path(), &context).is_err());
}

#[test]
/// Catalog admission and native validator approval remain independently enforced.
fn context_rejects_missing_baseline_adapter_and_unqualified_validator() {
    if !supported() {
        return;
    }
    let (directory, mut context) = fixture();
    let mut catalog: Value =
        serde_json::from_slice(&fs::read(directory.path().join("catalog.json")).unwrap()).unwrap();
    catalog["adapters"].as_array_mut().unwrap().pop();
    let identity = write(&directory.path().join("catalog.json"), &catalog);
    context["catalog"] = identity.clone();
    context["selections"][0]["evidence_requirements"]["catalog"] = identity;
    assert!(open(directory.path(), &context).is_err());
    let (directory, mut context) = fixture();
    context["native_sbom_validator"]["sha256"] = "e".repeat(64).into();
    assert!(open(directory.path(), &context).is_err());
}

#[test]
/// Duplicate fields and trailing documents cannot acquire meaning through a reviewed digest.
fn approved_duplicate_or_trailing_context_json_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    for bytes in [
        br#"{"schema_version":1,"schema_version":1}"#.as_slice(),
        b"{} {}",
    ] {
        fs::write(directory.path().join(CONTEXT_NAME), bytes).unwrap();
        assert!(
            TrustedReleaseContext::open(directory.path(), &ByteIdentity::from_bytes(bytes).sha256)
                .is_err()
        );
    }
}

#[cfg(unix)]
#[test]
/// Approved-looking sidecars and the context directory must be regular, non-symlink inputs.
fn context_rejects_symlinked_authorities_and_directory() {
    use std::os::unix::fs::symlink;
    for name in [
        CONTEXT_NAME,
        "armorer.toml",
        "armorer.lock",
        "Cargo.lock",
        "catalog.json",
        "verification-policy.json",
    ] {
        let (directory, context) = fixture();
        let context_identity: ByteIdentity =
            serde_json::from_value(write(&directory.path().join(CONTEXT_NAME), &context)).unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        fs::copy(directory.path().join(name), outside.path()).unwrap();
        fs::remove_file(directory.path().join(name)).unwrap();
        symlink(outside.path(), directory.path().join(name)).unwrap();
        assert!(TrustedReleaseContext::open(directory.path(), &context_identity.sha256).is_err());
    }
    let (directory, context) = fixture();
    let id: ByteIdentity =
        serde_json::from_value(write(&directory.path().join(CONTEXT_NAME), &context)).unwrap();
    let parent = tempfile::tempdir().unwrap();
    symlink(directory.path(), parent.path().join("linked")).unwrap();
    assert!(TrustedReleaseContext::open(&parent.path().join("linked"), &id.sha256).is_err());
}

#[test]
#[ignore = "Required explicit native CI gate with independently qualified gh and CycloneDX"]
/// A genuine unrelated bundle cannot authenticate malformed inventory or activate a compatibility fallback.
fn real_inventory_authentication_failure_precedes_json_and_asset_processing() {
    use armorer::{
        trust::inventory::{INVENTORY_BUNDLE_NAME, INVENTORY_NAME},
        verification::{
            cyclonedx::OfflineSbomValidator,
            release::AuthenticatedReleaseFiles,
            sigstore::{OfflineVerifier, qualified_native_verifier},
        },
    };
    let gh = PathBuf::from(
        std::env::var_os("ARMORER_TEST_GH")
            .expect("qualify the exact native gh for this explicit integration"),
    );
    let cdx = PathBuf::from(
        std::env::var_os("ARMORER_TEST_CDX")
            .expect("qualify the exact native CycloneDX for this explicit integration"),
    );
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sigstore");
    let (directory, mut context) = fixture();
    let policy_path = directory.path().join("verification-policy.json");
    let mut policy: Value = serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy["roots"]["trusted_root"] = serde_json::to_value(ByteIdentity::from_bytes(
        &fs::read(source.join("trusted_root.json")).unwrap(),
    ))
    .unwrap();
    policy["roots"]["verifier"] =
        serde_json::to_value(qualified_native_verifier().unwrap()).unwrap();
    context["verification_policy"] = write(&policy_path, &policy);
    let trusted = open(directory.path(), &context).unwrap();
    let verifier = OfflineVerifier::open(
        &policy_path,
        context["verification_policy"]["sha256"].as_str().unwrap(),
        &gh,
        &source.join("trusted_root.json"),
    )
    .unwrap();
    let sbom_validator = OfflineSbomValidator::open(&cdx, trusted.sbom_validator()).unwrap();
    for inventory_bytes in [
        b"not JSON; authenticate before parsing".as_slice(),
        br#"{"historical":true,"assets":[{"name":"../../escape"}]}"#,
    ] {
        let download = tempfile::tempdir().unwrap();
        fs::write(download.path().join(INVENTORY_NAME), inventory_bytes).unwrap();
        fs::copy(
            source.join("reusable-workflow-attestation.sigstore.json"),
            download.path().join(INVENTORY_BUNDLE_NAME),
        )
        .unwrap();
        let trap = b"#!/bin/sh\ntouch executed\n";
        fs::write(download.path().join("offered-artifact.tar.gz"), trap).unwrap();
        let error = AuthenticatedReleaseFiles::verify(
            download.path(),
            &trusted,
            &verifier,
            &sbom_validator,
        )
        .err()
        .expect("unrelated genuine bundle must never authenticate the inventory");
        assert!(
            error
                .to_string()
                .contains("attestation-authentication-failed"),
            "inventory was parsed or asset set processed before authentication: {error}"
        );
        assert_eq!(
            fs::read(download.path().join(INVENTORY_NAME)).unwrap(),
            inventory_bytes
        );
        assert_eq!(
            fs::read(download.path().join("offered-artifact.tar.gz")).unwrap(),
            trap
        );
        assert!(!download.path().join("executed").exists());
        assert!(!directory.path().join("executed").exists());
    }
}

#[test]
/// A matching bundle filename cannot override the frozen inventory's exact target-subject relationship.
fn inventory_rejects_cross_target_sbom_subject_swap_before_bundle_processing() {
    use armorer::{
        config::Config,
        trust::{
            inventory::{AssetRole, ReleaseInventory},
            policy::{Predicate, VerificationPolicy},
        },
    };
    const FIXTURE_NOW: u64 = 1_790_870_400;
    let mut config: Config =
        toml::from_str(&fs::read_to_string(example("armorer.toml")).unwrap()).unwrap();
    config.deliverables[0]
        .targets
        .push("aarch64-unknown-linux-gnu".into());
    let policy: VerificationPolicy =
        serde_json::from_value(read("verification-policy.json")).unwrap();
    let mut inventory: ReleaseInventory =
        serde_json::from_value(read("release-inventory.json")).unwrap();
    let original = inventory.assets.clone();
    for mut asset in original {
        asset.name = asset
            .name
            .replace("x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu");
        for name in &mut asset.subjects {
            *name = name.replace("x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu");
        }
        if let Some(name) = &mut asset.predicate_asset {
            *name = name.replace("x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu");
        }
        inventory.assets.push(asset);
    }
    // Equal synthetic SBOM/artifact identities cannot bypass the independently derived layout.
    inventory
        .validate_against(&config, &policy, &inventory.inputs, FIXTURE_NOW)
        .unwrap();
    let bundle = inventory
        .assets
        .iter_mut()
        .find(|asset| {
            asset.role == AssetRole::AttestationBundle
                && asset.predicate == Some(Predicate::CycloneDxV15)
                && asset.name.contains("x86_64-unknown-linux-gnu")
        })
        .unwrap();
    bundle.subjects[0] = "app--aarch64-unknown-linux-gnu--minimal.tar.gz".into();
    let error = inventory
        .validate_against(&config, &policy, &inventory.inputs, FIXTURE_NOW)
        .err()
        .unwrap();
    assert!(error.to_string().contains("asset-relationship-mismatch"));
}
