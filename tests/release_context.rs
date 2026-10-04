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
/// Select positive or unsupported-host assertions; never skip a test silently.
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

#[cfg_attr(
    not(any(
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
        all(target_os = "macos", target_arch = "aarch64")
    )),
    ignore = "native SBOM identity is unsupported on this host"
)]
#[test]
/// An approved digest does not excuse expired reviews or mismatched source, caller, workflow and run bindings.
fn context_rejects_wrong_source_caller_workflow_run_trigger_and_review() {
    qualified_native_validator().expect("required native identity on a supported host");
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

#[cfg_attr(
    not(any(
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
        all(target_os = "macos", target_arch = "aarch64")
    )),
    ignore = "native SBOM identity is unsupported on this host"
)]
#[test]
/// Selection roots and required pins/coverage/steps cannot silently disappear or be independently substituted.
fn approved_context_still_requires_complete_baseline_selection_and_evidence() {
    qualified_native_validator().expect("required native identity on a supported host");
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

#[cfg_attr(
    not(any(
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
        all(target_os = "macos", target_arch = "aarch64")
    )),
    ignore = "native SBOM identity is unsupported on this host"
)]
#[test]
/// Catalog admission and native validator approval remain independently enforced.
fn context_rejects_missing_baseline_adapter_and_unqualified_validator() {
    qualified_native_validator().expect("required native identity on a supported host");
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
    assert_eq!(
        trusted.policy_identity(),
        &ByteIdentity::from_bytes(&fs::read(&policy_path).unwrap())
    );
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
        let cli = release_cli(
            download.path(),
            directory.path(),
            &trusted.identity().sha256,
            "legacy-v1",
            &gh,
            &source.join("trusted_root.json"),
            &cdx,
        );
        assert_eq!(cli.status.code(), Some(1));
        let cli_error: Value = serde_json::from_slice(&cli.stdout).unwrap();
        assert!(
            cli_error["error"]["message"]
                .as_str()
                .unwrap()
                .contains("attestation-authentication-failed")
        );
        assert!(cli_error.get("provenance_verified").is_none());
        assert!(cli.stderr.is_empty());
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

/// Build an explicit three-target synthetic context with different archive and executable identities.
fn native_fixture() -> (tempfile::TempDir, Value, Value) {
    use armorer::config::{Config, Lock, Profile, ToolPin};
    let (directory, mut release) = fixture();
    let archive = include_bytes!("fixtures/runtime/synthetic-runtime.tar");
    let targets = [
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ];
    let mut config: Config =
        toml::from_str(&fs::read_to_string(directory.path().join("armorer.toml")).unwrap())
            .unwrap();
    config.deliverables[0].profile = Profile::Library;
    config.deliverables[0].binary = None;
    config.deliverables[0].targets = targets.iter().map(|target| target.to_string()).collect();
    let config_bytes = toml::to_string(&config).unwrap().into_bytes();
    fs::write(directory.path().join("armorer.toml"), &config_bytes).unwrap();
    let old: Value =
        serde_json::from_slice(&fs::read(directory.path().join("catalog.json")).unwrap()).unwrap();
    let mut adapters = old["adapters"].clone();
    let mut lock: Lock =
        toml::from_str(&fs::read_to_string(directory.path().join("armorer.lock")).unwrap())
            .unwrap();
    lock.config_sha256 = ByteIdentity::from_bytes(&config_bytes).sha256;
    lock.tools.clear();
    for adapter in adapters.as_array_mut().unwrap() {
        let old_pin = adapter["pins"][0].clone();
        let mut pins = Vec::new();
        for target in targets {
            let name = format!("{}--{target}", old_pin["name"].as_str().unwrap());
            let distribution =
                ByteIdentity::from_bytes(format!("synthetic compressed archive {name}").as_bytes());
            let executable =
                ByteIdentity::from_bytes(format!("synthetic executed member {name}").as_bytes());
            lock.tools.insert(
                name.clone(),
                ToolPin {
                    version: old_pin["version"].as_str().unwrap().into(),
                    sha256: distribution.sha256.clone(),
                },
            );
            pins.push(json!({"name":name,"version":old_pin["version"],"kind":"tool","format":"tar-xz","distribution":distribution,"authentication_record":old_pin["authentication_record"],"material":{"material":"native-member","target":target,"name":old_pin["name"],"bytes":executable}}));
        }
        adapter["pins"] = pins.into();
    }
    let lock_bytes = toml::to_string(&lock).unwrap().into_bytes();
    fs::write(directory.path().join("armorer.lock"), &lock_bytes).unwrap();
    let runtime_source =
        json!({"repository":"fixture/armorer","commit":"d".repeat(40),"git_ref":"refs/heads/main"});
    let members: serde_json::Map<_,_> = targets.iter().enumerate().map(|(index,target)|
        (target.to_string(),json!({"name":format!("armorer--{target}"),"bytes":ByteIdentity::from_bytes(&archive[index*1024+512..index*1024+640])}))).collect();
    let runtime = json!({"schema_version":1,"version":"0.1.0","source":runtime_source,"compiler":"1.95.0","cargo_lock":ByteIdentity::from_bytes(b"synthetic runtime Cargo.lock"),"build_workflow":{"repository":"fixture/armorer","path":".github/workflows/development.yml","commit":"d".repeat(40)},"authentication_record":ByteIdentity::from_bytes(b"synthetic runtime qualification; no provenance claim"),"distribution":ByteIdentity::from_bytes(archive),"members":members});
    let catalog = json!({"schema_version":2,"previous_catalog":release["catalog"],"runtime":runtime,"adapters":adapters});
    let catalog_identity = write(&directory.path().join("catalog.json"), &catalog);
    release["catalog"] = catalog_identity.clone();
    release["inputs"]["config_sha256"] = lock.config_sha256.into();
    release["inputs"]["lock_sha256"] = ByteIdentity::from_bytes(&lock_bytes).sha256.into();
    release["inputs"]["runtime"] = runtime["distribution"].clone();
    let selected = release["selections"][0].clone();
    release["selections"] = targets
        .iter()
        .map(|target| {
            let mut item = selected.clone();
            item["selection"]["profile"] = "library".into();
            item["selection"]["binary"] = Value::Null;
            item["selection"]["target"] = (*target).into();
            let selection = item["selection"].clone();
            let requirements = &mut item["evidence_requirements"];
            requirements["selection"] = selection;
            requirements["inputs"] = release["inputs"].clone();
            requirements["catalog"] = catalog_identity.clone();
            let tool = selected["evidence_requirements"]["tools"][0].clone();
            requirements["tools"] = catalog["adapters"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|a| a["pins"].as_array().unwrap())
                .filter(|pin| pin["material"]["target"] == *target)
                .map(|pin| {
                    let mut result = tool.clone();
                    result["name"] = pin["name"].clone();
                    result["kind"] = pin["kind"].clone();
                    result["version"] = pin["version"].clone();
                    result["bytes"] = pin["material"]["bytes"].clone();
                    result["authentication_record"] = pin["authentication_record"].clone();
                    result
                })
                .collect::<Vec<_>>()
                .into();
            item
        })
        .collect::<Vec<_>>()
        .into();
    fs::write(directory.path().join("runtime.tar"), archive).unwrap();
    (
        directory,
        json!({"schema_version":2,"runtime_source":runtime_source,"release":release}),
        catalog,
    )
}

/// Update approved catalog bindings in synthetic tests without approving real upstream material.
fn bind_native_catalog(directory: &Path, outer: &mut Value, catalog: &Value) {
    let identity = write(&directory.join("catalog.json"), catalog);
    outer["release"]["catalog"] = identity.clone();
    for selected in outer["release"]["selections"].as_array_mut().unwrap() {
        selected["evidence_requirements"]["catalog"] = identity.clone();
    }
}

/// Independently approve a synthetic outer v2 context for semantic tests only.
fn open_native(directory: &Path, outer: &Value) -> armorer::Result<TrustedReleaseContext> {
    use armorer::verification::context::NATIVE_CONTEXT_NAME;
    let identity: ByteIdentity =
        serde_json::from_value(write(&directory.join(NATIVE_CONTEXT_NAME), outer)).unwrap();
    TrustedReleaseContext::open_native_v2(directory, &identity.sha256)
}

#[cfg_attr(
    not(any(
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
        all(target_os = "macos", target_arch = "aarch64")
    )),
    ignore = "native SBOM identity is unsupported on this host"
)]
#[test]
/// Every target uses its executed members while retaining compressed archive digests in the lock.
fn native_context_selects_target_members_and_never_reinterprets_v1_catalogs() {
    qualified_native_validator().expect("required native identity on a supported host");
    let (directory, outer, catalog) = native_fixture();
    let context = open_native(directory.path(), &outer).unwrap();
    assert_eq!(context.selections().len(), 3);
    assert_eq!(
        context.inputs().runtime,
        serde_json::from_value(catalog["runtime"]["distribution"].clone()).unwrap()
    );
    for selected in context.selections() {
        assert_eq!(selected.evidence_requirements.tools.len(), 3);
        for tool in &selected.evidence_requirements.tools {
            assert!(tool.name.ends_with(&selected.selection.target));
            assert_ne!(tool.bytes.sha256, context.lock().tools[&tool.name].sha256);
        }
    }
    assert!(open(directory.path(), &outer["release"]).is_err());
    let (legacy, context) = fixture();
    assert!(
        open_native(
            legacy.path(),
            &json!({"schema_version":2,"runtime_source":outer["runtime_source"],"release":context})
        )
        .is_err()
    );
}

#[cfg_attr(
    not(any(
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
        all(target_os = "macos", target_arch = "aarch64")
    )),
    ignore = "native SBOM identity is unsupported on this host"
)]
#[test]
/// Wrong archive/member/target/kind/source/runtime/compiler and missing baseline pins fail closed.
fn native_context_rejects_archive_member_and_cross_target_substitution() {
    qualified_native_validator().expect("required native identity on a supported host");
    for case in 0..15 {
        let (directory, mut outer, mut catalog) = native_fixture();
        match case {
            0 => outer["schema_version"] = 1.into(),
            1 => outer["runtime_source"]["commit"] = "e".repeat(40).into(),
            2 => catalog["runtime"]["distribution"]["size"] = 1.into(),
            3 => catalog["runtime"]["compiler"] = "stable".into(),
            4 => catalog["runtime"]["version"] = "0.2.0".into(),
            5 => catalog["runtime"]["members"]
                .as_object_mut()
                .unwrap()
                .remove("aarch64-apple-darwin")
                .map(|_| ())
                .unwrap(),
            6 => catalog["adapters"][0]["pins"][0]["material"]["name"] = "../escape".into(),
            7 => {
                catalog["adapters"][0]["pins"][0]["distribution"]["sha256"] = "f".repeat(64).into()
            }
            8 => {
                catalog["adapters"][0]["pins"][0]["material"]["bytes"]["size"] =
                    134217729_u64.into()
            }
            9 => {
                outer["release"]["selections"][0]["evidence_requirements"]["tools"][0]["bytes"] =
                    catalog["adapters"][0]["pins"][0]["distribution"].clone()
            }
            10 => {
                outer["release"]["selections"][0]["evidence_requirements"]["tools"][0]["kind"] =
                    "action".into()
            }
            11 => {
                outer["release"]["selections"][0]["evidence_requirements"]["tools"][0] =
                    outer["release"]["selections"][1]["evidence_requirements"]["tools"][0].clone()
            }
            12 => {
                outer["release"]["selections"][0]["evidence_requirements"]["tools"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            13 => catalog["adapters"][0]["pins"][0]["format"] = "raw".into(),
            _ => catalog["runtime"]["build_workflow"]["commit"] = "e".repeat(40).into(),
        }
        bind_native_catalog(directory.path(), &mut outer, &catalog);
        assert!(
            open_native(directory.path(), &outer).is_err(),
            "accepted native substitution {case}"
        );
        assert!(!directory.path().join("executed").exists());
    }
}

#[cfg_attr(
    not(any(
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
        all(target_os = "macos", target_arch = "aarch64")
    )),
    ignore = "native SBOM identity is unsupported on this host"
)]
#[test]
/// Approved synthetic headers are retained read-only; loader performs no executable invocation.
fn native_runtime_snapshots_only_approved_members_and_never_runs_them() {
    use armorer::verification::runtime::ApprovedRuntimeFiles;
    qualified_native_validator().expect("required native identity on a supported host");
    let (directory, outer, _) = native_fixture();
    let context = open_native(directory.path(), &outer).unwrap();
    let before = fs::read(directory.path().join("runtime.tar")).unwrap();
    let runtime =
        ApprovedRuntimeFiles::open(&directory.path().join("runtime.tar"), &context).unwrap();
    assert_eq!(
        runtime.distribution_identity(),
        &ByteIdentity::from_bytes(&before)
    );
    let executable = runtime.native_executable().unwrap();
    assert_eq!(fs::read(executable).unwrap().len(), 128);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(executable).unwrap().permissions().mode() & 0o777,
            0o500
        );
    }
    assert_eq!(
        fs::read(directory.path().join("runtime.tar")).unwrap(),
        before
    );
    assert!(!directory.path().join("target").exists());
    assert!(!directory.path().join("executed").exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(executable, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(executable, b"changed private executable").unwrap();
        assert!(runtime.native_executable().is_err());
    }
    let (legacy, context) = fixture();
    let legacy_context = open(legacy.path(), &context).unwrap();
    assert!(
        ApprovedRuntimeFiles::open(&directory.path().join("runtime.tar"), &legacy_context).is_err()
    );
}

#[cfg_attr(
    not(any(
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
        all(target_os = "macos", target_arch = "aarch64")
    )),
    ignore = "native SBOM identity is unsupported on this host"
)]
#[test]
/// Even separately approved malformed archives cannot introduce paths, links, extensions or foreign native headers.
fn native_runtime_rejects_unsafe_containers_and_retained_executable_mutation() {
    use armorer::verification::runtime::ApprovedRuntimeFiles;
    qualified_native_validator().expect("required native identity on a supported host");
    for case in 0..12 {
        let (directory, mut outer, mut catalog) = native_fixture();
        let mut archive = fs::read(directory.path().join("runtime.tar")).unwrap();
        match case {
            0 => archive[0] = b'/',
            1 => archive[156] = b'2',
            2 => archive[156] = b'x',
            3 => archive[257] = b'g',
            4 => archive[148] ^= 1,
            5 => archive[640] = 1,
            6 => archive[3500] = 1,
            7 => {
                archive.truncate(3500);
            }
            8 => {
                archive.extend_from_slice(&[0; 512]);
            }
            9 => archive[512] = b'#',
            10 => archive[512 + 18] = 183,
            _ => archive[1536] ^= 1,
        }
        if case == 9 || case == 10 {
            catalog["runtime"]["members"]["x86_64-unknown-linux-gnu"]["bytes"] =
                serde_json::to_value(ByteIdentity::from_bytes(&archive[512..640])).unwrap();
        }
        let identity = serde_json::to_value(ByteIdentity::from_bytes(&archive)).unwrap();
        catalog["runtime"]["distribution"] = identity.clone();
        outer["release"]["inputs"]["runtime"] = identity.clone();
        for selected in outer["release"]["selections"].as_array_mut().unwrap() {
            selected["evidence_requirements"]["inputs"]["runtime"] = identity.clone();
        }
        bind_native_catalog(directory.path(), &mut outer, &catalog);
        fs::write(directory.path().join("runtime.tar"), archive).unwrap();
        let context = open_native(directory.path(), &outer).unwrap();
        assert!(
            ApprovedRuntimeFiles::open(&directory.path().join("runtime.tar"), &context).is_err(),
            "accepted malformed archive {case}"
        );
    }
    let (directory, outer, _) = native_fixture();
    let context = open_native(directory.path(), &outer).unwrap();
    fs::write(directory.path().join("runtime.tar"), b"not a tar archive").unwrap();
    let error = ApprovedRuntimeFiles::open(&directory.path().join("runtime.tar"), &context)
        .err()
        .unwrap();
    assert!(
        error
            .to_string()
            .contains("runtime-distribution-byte-mismatch")
    );
}

#[test]
#[ignore = "Explicit operator gate after independently qualifying all three exact-head native CI artifacts"]
/// Load actual three-host candidate bytes against a separately approved spec; invoke only fixed native --version.
fn real_native_runtime_distribution_matches_independent_provider_qualification() {
    use armorer::{trust::Source, verification::runtime::ApprovedRuntimeFiles};
    use std::process::Command;
    let archive = PathBuf::from(
        std::env::var_os("ARMORER_TEST_RUNTIME_ARCHIVE")
            .expect("qualify actual provider archive first"),
    );
    let spec_path = PathBuf::from(
        std::env::var_os("ARMORER_TEST_RUNTIME_SPEC")
            .expect("retain independently approved candidate spec first"),
    );
    let spec_sha = std::env::var("ARMORER_TEST_RUNTIME_SPEC_SHA256")
        .expect("independently approve exact spec bytes");
    let source_commit = std::env::var("ARMORER_TEST_RUNTIME_SOURCE_COMMIT")
        .expect("independently bind immutable source checkout");
    let source_ref =
        std::env::var("ARMORER_TEST_RUNTIME_SOURCE_REF").expect("independently bind checkout ref");
    let bytes = fs::read(&spec_path).unwrap();
    assert!(bytes.len() <= 1048576);
    assert_eq!(ByteIdentity::from_bytes(&bytes).sha256, spec_sha);
    let spec: Value = serde_json::from_slice(&bytes).unwrap();
    let expected_source = Source {
        repository: "brianluby/armorer".into(),
        commit: source_commit,
        git_ref: source_ref,
    };
    expected_source.validate().unwrap();
    assert_eq!(
        spec["source"],
        serde_json::to_value(&expected_source).unwrap()
    );
    let (directory, mut outer, mut catalog) = native_fixture();
    catalog["runtime"] = spec.clone();
    outer["runtime_source"] = serde_json::to_value(expected_source).unwrap();
    outer["release"]["inputs"]["runtime"] = spec["distribution"].clone();
    for selected in outer["release"]["selections"].as_array_mut().unwrap() {
        selected["evidence_requirements"]["inputs"]["runtime"] = spec["distribution"].clone();
    }
    bind_native_catalog(directory.path(), &mut outer, &catalog);
    let context = open_native(directory.path(), &outer).unwrap();
    let runtime = ApprovedRuntimeFiles::open(&archive, &context).unwrap();
    let home = tempfile::tempdir().unwrap();
    let output = Command::new(runtime.native_executable().unwrap())
        .arg("--version")
        .env_clear()
        .env("HOME", home.path())
        .env("PATH", "/usr/bin:/bin")
        .current_dir(home.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"armorer 0.1.0\n");
    assert!(output.stderr.is_empty());
    assert_eq!(
        runtime.distribution_identity(),
        &serde_json::from_value(spec["distribution"].clone()).unwrap()
    );
    // Synthetic selection authorities above test member loading, not an actual signed release or production catalog.
}

/// Invoke the complete consumer with cleared ambient credentials and paths confined to test-owned inputs.
fn release_cli(
    download: &Path,
    trusted: &Path,
    digest: &str,
    kind: &str,
    gh: &Path,
    root: &Path,
    cdx: &Path,
) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_armorer"))
        .env_clear()
        .current_dir(trusted)
        .arg("verify-release")
        .arg("--directory")
        .arg(download)
        .arg("--trusted-inputs")
        .arg(trusted)
        .arg("--expect-context-sha256")
        .arg(digest)
        .arg("--context-kind")
        .arg(kind)
        .arg("--gh")
        .arg(gh)
        .arg("--trusted-root")
        .arg(root)
        .arg("--cyclonedx")
        .arg(cdx)
        .output()
        .unwrap()
}

#[test]
/// CLI verification rejects offered approval digests before parsing context or touching tools/release files.
fn cli_rejects_unapproved_context_before_release_and_tools() {
    let directory = tempfile::tempdir().unwrap();
    let context = b"malformed offered context; reject its independent digest first";
    fs::write(directory.path().join(CONTEXT_NAME), context).unwrap();
    let absent = directory.path().join("never-open-this");
    let cli = release_cli(
        &absent,
        directory.path(),
        &"a".repeat(64),
        "legacy-v1",
        &absent,
        &absent,
        &absent,
    );
    assert_eq!(cli.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("unapproved-release-context")
    );
    assert!(error.get("provenance_verified").is_none());
    assert!(cli.stderr.is_empty());
    assert_eq!(
        fs::read(directory.path().join(CONTEXT_NAME)).unwrap(),
        context
    );
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
/// Selecting native v2 never falls back to an available legacy context when v2 bytes fail.
fn cli_context_version_is_explicit_without_fallback() {
    use armorer::verification::context::NATIVE_CONTEXT_NAME;
    let (directory, context) = fixture();
    let _ = write(&directory.path().join(CONTEXT_NAME), &context);
    let malformed = b"invalid native v2 context";
    fs::write(directory.path().join(NATIVE_CONTEXT_NAME), malformed).unwrap();
    let absent = directory.path().join("never-open-this");
    let cli = release_cli(
        &absent,
        directory.path(),
        &ByteIdentity::from_bytes(malformed).sha256,
        "native-v2",
        &absent,
        &absent,
        &absent,
    );
    assert_eq!(cli.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert_eq!(error["error"]["code"], "json");
    assert!(error.get("provenance_verified").is_none());
    assert!(cli.stderr.is_empty());
    assert_eq!(
        fs::read(directory.path().join(NATIVE_CONTEXT_NAME)).unwrap(),
        malformed
    );
}

#[test]
/// Approved synthetic intent cannot execute a caller-selected fake native verifier or consume a release.
fn cli_requires_compiled_native_tool_approval_before_release_processing() {
    let (directory, context) = fixture();
    let digest: ByteIdentity =
        serde_json::from_value(write(&directory.path().join(CONTEXT_NAME), &context)).unwrap();
    let marker = directory.path().join("executed");
    let tool = directory.path().join("fake-gh");
    fs::write(&tool, b"#!/bin/sh\ntouch executed\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let absent = directory.path().join("never-open-this");
    let cli = release_cli(
        &absent,
        directory.path(),
        &digest.sha256,
        "legacy-v1",
        &tool,
        &absent,
        &absent,
    );
    assert_eq!(cli.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&cli.stdout).unwrap();
    if supported() {
        assert!(
            error["error"]["message"]
                .as_str()
                .unwrap()
                .contains("unqualified-native-verifier-policy")
        );
    }
    assert!(error.get("provenance_verified").is_none());
    assert!(!marker.exists());
    assert!(!directory.path().join("target").exists());
}

#[test]
/// Parsing requires an explicit catalog version and offers no source/signer/provenance override.
fn cli_requires_context_kind_and_rejects_claim_overrides() {
    let cli = std::process::Command::new(env!("CARGO_BIN_EXE_armorer"))
        .env_clear()
        .args([
            "verify-release",
            "--directory",
            ".",
            "--trusted-inputs",
            ".",
            "--expect-context-sha256",
            &"a".repeat(64),
            "--gh",
            "gh",
            "--trusted-root",
            "root.json",
            "--cyclonedx",
            "cyclonedx",
        ])
        .output()
        .unwrap();
    assert_eq!(cli.status.code(), Some(2));
    assert!(
        String::from_utf8(cli.stderr)
            .unwrap()
            .contains("--context-kind")
    );
    assert!(cli.stdout.is_empty());
    let override_attempt = std::process::Command::new(env!("CARGO_BIN_EXE_armorer"))
        .env_clear()
        .args(["verify-release", "--source-commit", &"a".repeat(40)])
        .output()
        .unwrap();
    assert_eq!(override_attempt.status.code(), Some(2));
    assert!(
        String::from_utf8(override_attempt.stderr)
            .unwrap()
            .contains("--source-commit")
    );
    assert!(override_attempt.stdout.is_empty());
}

/// Native-v3 authenticates an exact complete feature map and never retries the older context file.
#[cfg_attr(
    not(any(
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
        all(target_os = "macos", target_arch = "aarch64")
    )),
    ignore = "native SBOM identity is unsupported on this host"
)]
#[test]
fn native_v3_requires_complete_independently_approved_root_features() {
    use armorer::verification::context::FEATURE_CONTEXT_NAME;
    qualified_native_validator().expect("required native identity on a supported host");
    let (directory, mut outer, _) = native_fixture();
    outer["schema_version"] = 3.into();
    let keys = outer["release"]["selections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|selected| {
            let selection = &selected["selection"];
            format!(
                "{}--{}--{}",
                selection["deliverable_id"].as_str().unwrap(),
                selection["target"].as_str().unwrap(),
                selection["feature_set"].as_str().unwrap()
            )
        })
        .collect::<Vec<_>>();
    let features = keys
        .iter()
        .map(|key| (key.clone(), json!([])))
        .collect::<serde_json::Map<_, _>>();
    outer["root_features"] = features.into();
    let identity: ByteIdentity =
        serde_json::from_value(write(&directory.path().join(FEATURE_CONTEXT_NAME), &outer))
            .unwrap();
    TrustedReleaseContext::open_native_v3(directory.path(), &identity.sha256).unwrap();
    // The same approved digest cannot select another format/name, even if a v2 file exists.
    let mut old = outer.clone();
    old["schema_version"] = 2.into();
    old.as_object_mut().unwrap().remove("root_features");
    write(
        &directory
            .path()
            .join(armorer::verification::context::NATIVE_CONTEXT_NAME),
        &old,
    );
    assert!(TrustedReleaseContext::open_native_v2(directory.path(), &identity.sha256).is_err());
    for case in 0..6 {
        let mut changed = outer.clone();
        match case {
            0 => {
                changed["root_features"]
                    .as_object_mut()
                    .unwrap()
                    .remove(&keys[0]);
            }
            1 => changed["root_features"]["foreign--target--variant"] = json!([]),
            2 => changed["root_features"][&keys[0]] = json!(["duplicate", "duplicate"]),
            3 => changed["root_features"][&keys[0]] = json!(["z", "a"]),
            4 => changed["root_features"][&keys[0]] = json!(["bad\nfeature"]),
            5 => {
                changed.as_object_mut().unwrap().remove("root_features");
            }
            _ => unreachable!(),
        }
        let identity: ByteIdentity = serde_json::from_value(write(
            &directory.path().join(FEATURE_CONTEXT_NAME),
            &changed,
        ))
        .unwrap();
        assert!(
            TrustedReleaseContext::open_native_v3(directory.path(), &identity.sha256).is_err(),
            "{case}"
        );
    }
}
