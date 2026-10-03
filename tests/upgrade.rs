use armorer::{
    bootstrap, config,
    upgrade::{self, merge::Decision},
};
use std::{collections::BTreeMap, fs, path::Path, process::Command};
/// Load the fixture lock against the current configuration byte identity.
fn load_lock(root: &Path) -> armorer::Result<Option<(config::Lock, Vec<u8>)>> {
    use sha2::{Digest, Sha256};
    let (_, bytes) = config::load_config(root)?;
    config::load_lock(root, &format!("{:x}", Sha256::digest(bytes)))
}
const POLICY: &str = "ci-policy.reviewed.toml";
/// Create an isolated repository fixture with explicit inputs for the surrounding transaction tests.
fn fixture(profile: &str) -> tempfile::TempDir {
    /// Copy profile fixture files into isolated test storage without executing them.
    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for e in fs::read_dir(from).unwrap() {
            let e = e.unwrap();
            let dest = to.join(e.file_name());
            if e.path().is_dir() {
                copy(&e.path(), &dest);
            } else {
                fs::copy(e.path(), dest).unwrap();
            }
        }
    }
    let root = tempfile::tempdir().unwrap();
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/bootstrap-v2")
            .join(profile),
        root.path(),
    );
    fs::write(
        root.path().join("build.rs"),
        "fn main(){std::fs::write(\"BUILD_EXECUTED\",\"bad\").unwrap();panic!(\"no builds\");}",
    )
    .unwrap();
    root
}
/// Capture every regular fixture file as exact bytes for mutation and recovery comparisons.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    /// Recursively record relative fixture paths and bytes without running repository code.
    fn walk(root: &Path, dir: &Path, map: &mut BTreeMap<String, Vec<u8>>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(root, &p, map);
            } else {
                map.insert(
                    p.strip_prefix(root).unwrap().to_str().unwrap().into(),
                    fs::read(p).unwrap(),
                );
            }
        }
    }
    let mut map = BTreeMap::new();
    walk(root, root, &mut map);
    map
}
/// Derive a fresh fixture plan from its explicit policy and independently selected catalog.
fn preview(root: &Path, imports: &[&str]) -> upgrade::Plan {
    upgrade::inspect(
        root,
        &root.join(POLICY),
        "bootstrap-v1",
        &imports.iter().map(|s| (*s).into()).collect::<Vec<_>>(),
        false,
    )
    .unwrap()
}
/// Provision the isolated fixture through the separately approved bootstrap contract.
fn bootstrap(root: &Path) {
    let p = bootstrap::inspect(root, &root.join(POLICY)).unwrap();
    bootstrap::apply(root, &p, &p.plan_sha256).unwrap();
}
/// Select the exact fixed target from a fixture upgrade plan.
fn change<'a>(plan: &'a upgrade::Plan, path: &str) -> &'a armorer::upgrade::Change {
    plan.changes.iter().find(|c| c.path == path).unwrap()
}
/// Verify that every profile shows full three images with no builds or consuming mutations.
#[test]
fn every_profile_shows_full_three_images_with_no_builds_or_consuming_mutations() {
    for profile in ["library", "cli", "service"] {
        let root = fixture(profile);
        let before = snapshot(root.path());
        let p = preview(root.path(), &[]);
        assert_eq!(snapshot(root.path()), before);
        assert_eq!(p.changes.len(), 5);
        assert!(p.changes.iter().all(|c| c.decision == Decision::Create
            && c.base_content.is_none()
            && c.before_content.is_none()));
        assert_eq!(
            p.target_catalog.workflow_commit,
            armorer::catalog::WORKFLOW_COMMIT
        );
        assert_eq!(p.pin_changes.len(), 18);
        assert_eq!(p.direction, "upgrade");
        assert!(p.source_catalog.is_none());
        assert!(!root.path().join("BUILD_EXECUTED").exists());
        assert!(!root.path().join("target").exists());
        upgrade::validate_fresh(root.path(), &p).unwrap();
        assert_eq!(snapshot(root.path()), before);
    }
}
/// Verify that v2 bases are recorded separately and bespoke jobs and policy are preserved.
#[test]
fn v2_bases_are_recorded_separately_and_bespoke_jobs_and_policy_are_preserved() {
    let root = fixture("library");
    bootstrap(root.path());
    let initial = preview(root.path(), &[]);
    assert!(
        initial
            .changes
            .iter()
            .all(|c| c.decision == Decision::Unchanged && c.base_content == c.before_content)
    );
    let path = ".github/workflows/armorer-ci.yml";
    let current = fs::read_to_string(root.path().join(path)).unwrap()
        + "  bespoke:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo custom\n";
    fs::write(root.path().join(path), &current).unwrap();
    let policy = fs::read_to_string(root.path().join(".armorer/ci-policy.toml")).unwrap()
        + "# owner customization\n";
    fs::write(root.path().join(".armorer/ci-policy.toml"), &policy).unwrap();
    fs::write(
        root.path().join(".github/workflows/custom.yml"),
        "bespoke pipeline",
    )
    .unwrap();
    let before = snapshot(root.path());
    let p = preview(root.path(), &[]);
    assert_eq!(change(&p, path).decision, Decision::PreserveCustomization);
    assert_eq!(
        change(&p, path).proposed_content.as_deref(),
        Some(current.as_str())
    );
    assert_eq!(change(&p, path).proposed_diff.as_deref(), Some(""));
    assert_ne!(
        change(&p, path).base_content,
        change(&p, path).before_content
    );
    assert_eq!(
        change(&p, ".armorer/ci-policy.toml")
            .proposed_content
            .as_deref(),
        Some(policy.as_str())
    );
    assert_eq!(snapshot(root.path()), before);
}
/// Verify that owner edits conflict when new policy also changes and deletions never readopt.
#[test]
fn owner_edits_conflict_when_new_policy_also_changes_and_deletions_never_readopt() {
    let root = fixture("library");
    bootstrap(root.path());
    fs::write(
        root.path().join(".armorer/ci-policy.toml"),
        fs::read_to_string(root.path().join(POLICY)).unwrap() + "# current owner edit\n",
    )
    .unwrap();
    fs::write(
        root.path().join(POLICY),
        fs::read_to_string(root.path().join(POLICY))
            .unwrap()
            .replace("[\"MIT\"]", "[\"MIT\", \"Apache-2.0\"]"),
    )
    .unwrap();
    let before = snapshot(root.path());
    let p = preview(root.path(), &[]);
    assert_eq!(
        change(&p, ".armorer/ci-policy.toml").decision,
        Decision::Conflict
    );
    assert!(
        change(&p, ".armorer/ci-policy.toml")
            .proposed_content
            .is_none()
    );
    assert!(
        p.compatibility
            .iter()
            .any(|c| c.state == "blocked" && c.code == "three-way-conflict")
    );
    assert_eq!(snapshot(root.path()), before);
    fs::remove_file(root.path().join("rust-toolchain.toml")).unwrap();
    let p = preview(root.path(), &[]);
    assert_eq!(
        change(&p, "rust-toolchain.toml").decision,
        Decision::Conflict
    );
}
/// Verify that owned configuration binding updates preserve exact previous lock base.
#[test]
fn owned_configuration_binding_updates_preserve_exact_previous_lock_base() {
    let root = fixture("library");
    bootstrap(root.path());
    let original = fs::read_to_string(root.path().join("armorer.lock")).unwrap();
    fs::write(
        root.path().join("armorer.toml"),
        fs::read_to_string(root.path().join("armorer.toml"))
            .unwrap()
            .replace("example/bootstrap-library", "example/reviewed-renaming"),
    )
    .unwrap();
    let before = snapshot(root.path());
    let p = preview(root.path(), &[]);
    let c = change(&p, "armorer.lock");
    assert_eq!(c.decision, Decision::Update);
    assert_eq!(c.base_content.as_deref(), Some(original.as_str()));
    assert_eq!(c.before_content.as_deref(), Some(original.as_str()));
    assert_ne!(c.proposed_content.as_deref(), Some(original.as_str()));
    assert!(p.pin_changes.is_empty());
    assert_eq!(snapshot(root.path()), before);
}
/// Verify that legacy v1 and declared unqualified lock require explicit scoped import.
#[test]
fn legacy_v1_and_declared_unqualified_lock_require_explicit_scoped_import() {
    let root = fixture("library");
    let p = armorer::plan::inspect(root.path(), "plan").unwrap();
    armorer::apply::apply(root.path(), &p, &p.plan_sha256).unwrap();
    let rendered = armorer::catalog::reviewed()
        .unwrap()
        .render_lock(root.path())
        .unwrap();
    let mut lock: config::Lock = toml::from_str(&rendered).unwrap();
    let old = lock
        .tools
        .iter()
        .find(|(name, _)| name.starts_with("actionlint--x86_64"))
        .unwrap()
        .1
        .clone();
    lock.tools.clear();
    lock.tools.insert("actionlint".into(), old);
    lock.workflows.repository = "example/legacy-declaration".into();
    lock.workflows.commit = "a".repeat(40);
    fs::write(
        root.path().join("armorer.lock"),
        toml::to_string_pretty(&lock).unwrap(),
    )
    .unwrap();
    let before = snapshot(root.path());
    let blocked = preview(root.path(), &[]);
    assert_eq!(
        change(&blocked, "armorer.lock").decision,
        Decision::Conflict
    );
    let imported = preview(root.path(), &["armorer.lock"]);
    assert_eq!(change(&imported, "armorer.lock").decision, Decision::Import);
    assert_eq!(imported.source_format, upgrade::SourceFormat::LegacyV1);
    assert!(imported.source_catalog.is_none());
    assert!(
        imported
            .pin_changes
            .iter()
            .any(|p| p.name == "actionlint" && p.before.is_some() && p.after.is_none())
    );
    assert!(
        imported
            .pin_changes
            .iter()
            .filter(|p| p.after.is_some())
            .count()
            == 18
    );
    assert_eq!(imported.workflow_before.unwrap().commit, "a".repeat(40));
    assert_eq!(
        imported.workflow_after.commit,
        armorer::catalog::WORKFLOW_COMMIT
    );
    assert_eq!(snapshot(root.path()), before);
}
/// Verify that imports never erase bespoke jobs or policies and reject moving declarations.
#[test]
fn imports_never_erase_bespoke_jobs_or_policies_and_reject_moving_declarations() {
    let root = fixture("library");
    let fresh = preview(root.path(), &[]);
    let c = change(&fresh, ".github/workflows/armorer-ci.yml");
    fs::create_dir_all(root.path().join(".github/workflows")).unwrap();
    let custom = c.candidate_content.clone()
        + "  bespoke:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo local\n";
    fs::write(root.path().join(&c.path), &custom).unwrap();
    let p = preview(root.path(), &[".github/workflows/armorer-ci.yml"]);
    assert_eq!(change(&p, &c.path).decision, Decision::Import);
    assert_eq!(
        change(&p, &c.path).proposed_content.as_deref(),
        Some(custom.as_str())
    );
    fs::write(
        root.path().join(&c.path),
        custom.replace(armorer::catalog::WORKFLOW_COMMIT, "main"),
    )
    .unwrap();
    let p = preview(root.path(), &[".github/workflows/armorer-ci.yml"]);
    assert_eq!(change(&p, &c.path).decision, Decision::Conflict);
    let lock = armorer::catalog::reviewed()
        .unwrap()
        .render_lock(root.path())
        .unwrap()
        .replace(armorer::catalog::WORKFLOW_COMMIT, "main");
    fs::write(root.path().join("armorer.lock"), lock).unwrap();
    let p = preview(root.path(), &["armorer.lock"]);
    assert_eq!(change(&p, "armorer.lock").decision, Decision::Conflict);
    fs::create_dir_all(root.path().join(".armorer")).unwrap();
    fs::write(
        root.path().join(".armorer/ci-policy.toml"),
        "bespoke policy",
    )
    .unwrap();
    let before = snapshot(root.path());
    let p = preview(root.path(), &[".armorer/ci-policy.toml"]);
    assert_eq!(
        change(&p, ".armorer/ci-policy.toml").decision,
        Decision::Conflict
    );
    assert_eq!(snapshot(root.path()), before);
}
/// Verify that catalog base substitution never falls back to import even with matching claimed hashes.
#[test]
fn catalog_base_substitution_never_falls_back_to_import_even_with_matching_claimed_hashes() {
    use sha2::{Digest, Sha256};
    for mutation in 0..4 {
        let root = fixture("library");
        bootstrap(root.path());
        let state = root.path().join(".armorer/bootstrap-state-v2.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&state).unwrap()).unwrap();
        match mutation {
            0 => value["workflow"]["commit"] = "a".repeat(40).into(),
            1 => value["catalog_sha256"] = "a".repeat(64).into(),
            2 | 3 => {
                let path = if mutation == 2 {
                    ".github/workflows/armorer-ci.yml"
                } else {
                    "armorer.lock"
                };
                let bad = value["managed"][path]["content"]
                    .as_str()
                    .unwrap()
                    .replace(armorer::catalog::WORKFLOW_COMMIT, &"a".repeat(40));
                value["managed"][path]["sha256"] =
                    format!("{:x}", Sha256::digest(bad.as_bytes())).into();
                value["managed"][path]["content"] = bad.into();
            }
            _ => unreachable!(),
        }
        fs::write(state, serde_json::to_vec(&value).unwrap()).unwrap();
        let before = snapshot(root.path());
        assert!(
            upgrade::inspect(
                root.path(),
                &root.path().join(POLICY),
                "bootstrap-v1",
                &["armorer.lock".into()],
                true
            )
            .is_err()
        );
        assert_eq!(snapshot(root.path()), before);
    }
}
/// Verify that strict saved packet reconstructs merges and rejects forged import or source context.
#[test]
fn strict_saved_packet_reconstructs_merges_and_rejects_forged_import_or_source_context() {
    let root = fixture("library");
    let p = preview(root.path(), &[]);
    let file = tempfile::NamedTempFile::new().unwrap();
    fs::write(file.path(), serde_json::to_vec(&p).unwrap()).unwrap();
    upgrade::load_plan(file.path(), &p.plan_sha256).unwrap();
    assert!(upgrade::load_plan(file.path(), &"a".repeat(64)).is_err());
    for mutation in 0..8 {
        let mut forged = p.clone();
        match mutation {
            0 => forged.changes[0].path = "LICENSE".into(),
            1 => forged.changes[0].decision = Decision::Import,
            2 => forged.source_catalog = Some(forged.target_catalog.clone()),
            3 => forged.changes[0].base_content = Some("invented base".into()),
            4 => forged.changes[0].proposed_content = Some("unreviewed content".into()),
            5 => forged.pin_changes.clear(),
            6 => forged.workflow_after.commit = "a".repeat(40),
            7 => forged.compatibility.clear(),
            _ => unreachable!(),
        }
        forged.plan_sha256 = forged.digest().unwrap();
        fs::write(file.path(), serde_json::to_vec(&forged).unwrap()).unwrap();
        assert!(upgrade::load_plan(file.path(), &forged.plan_sha256).is_err());
    }
    let mut value = serde_json::to_value(&p).unwrap();
    value["workspace"]["packages"][0]["unknown"] = "DO-NOT-ECHO".into();
    fs::write(file.path(), serde_json::to_vec(&value).unwrap()).unwrap();
    let e = upgrade::load_plan(file.path(), &p.plan_sha256).unwrap_err();
    assert!(!e.to_string().contains("DO-NOT-ECHO"));
    let raw = serde_json::to_string(&p).unwrap().replace(
        "\"state_preimages\":{",
        "\"state_preimages\":{\".armorer/state.json\":null,",
    );
    fs::write(file.path(), raw).unwrap();
    assert!(upgrade::load_plan(file.path(), &p.plan_sha256).is_err());
}
/// Verify that stale configuration ownership manifest policy and inventory require new review.
#[test]
fn stale_configuration_ownership_manifest_policy_and_inventory_require_new_review() {
    for input in [
        "armorer.toml",
        "Cargo.toml",
        "Cargo.lock",
        "LICENSE",
        "src/bin/new.rs",
        "rust-toolchain.toml",
    ] {
        let root = fixture("library");
        let p = preview(root.path(), &[]);
        let path = root.path().join(input);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let text = fs::read_to_string(&path).unwrap_or_default();
        fs::write(
            &path,
            if input.ends_with(".rs") {
                "fn main() {}\n".into()
            } else {
                format!("{text}\n# changed\n")
            },
        )
        .unwrap();
        let before = snapshot(root.path());
        assert!(upgrade::validate_fresh(root.path(), &p).is_err(), "{input}");
        assert_eq!(snapshot(root.path()), before);
    }
    let root = fixture("library");
    let p = preview(root.path(), &[]);
    bootstrap(root.path());
    let before = snapshot(root.path());
    assert!(upgrade::validate_fresh(root.path(), &p).is_err());
    assert_eq!(snapshot(root.path()), before);
}
/// Verify that old unknown config or ownership versions are explicitly unsupported not downgraded.
#[test]
fn old_unknown_config_or_ownership_versions_are_explicitly_unsupported_not_downgraded() {
    for version in [0, 2] {
        let root = fixture("library");
        fs::write(
            root.path().join("armorer.toml"),
            fs::read_to_string(root.path().join("armorer.toml"))
                .unwrap()
                .replacen(
                    "schema_version = 1",
                    &format!("schema_version = {version}"),
                    1,
                ),
        )
        .unwrap();
        let before = snapshot(root.path());
        assert!(
            upgrade::inspect(
                root.path(),
                &root.path().join(POLICY),
                "bootstrap-v1",
                &[],
                true
            )
            .is_err()
        );
        assert_eq!(snapshot(root.path()), before);
    }
    let root = fixture("library");
    let p = armorer::plan::inspect(root.path(), "plan").unwrap();
    armorer::apply::apply(root.path(), &p, &p.plan_sha256).unwrap();
    let state = root.path().join(".armorer/state.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&state).unwrap()).unwrap();
    value["runtime_version"] = "0.0.9".into();
    fs::write(state, serde_json::to_vec(&value).unwrap()).unwrap();
    let before = snapshot(root.path());
    assert!(
        upgrade::inspect(
            root.path(),
            &root.path().join(POLICY),
            "bootstrap-v1",
            &[],
            true
        )
        .is_err()
    );
    assert_eq!(snapshot(root.path()), before);
}
/// Verify that cli requires catalog import scope and never feeds upgrade packets to old apply.
#[test]
fn cli_requires_catalog_import_scope_and_never_feeds_upgrade_packets_to_old_apply() {
    let root = fixture("library");
    let cli = || {
        let mut c = Command::new(env!("CARGO_BIN_EXE_armorer"));
        c.arg("--repository").arg(root.path());
        c
    };
    assert!(
        !cli()
            .args(["upgrade", "plan", "--policy"])
            .arg(root.path().join(POLICY))
            .output()
            .unwrap()
            .status
            .success()
    );
    let before = snapshot(root.path());
    let output = cli()
        .args(["upgrade", "plan", "--policy"])
        .arg(root.path().join(POLICY))
        .args(["--catalog", "bootstrap-v1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(snapshot(root.path()), before);
    let p: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    fs::write(file.path(), &output.stdout).unwrap();
    for prefix in [vec!["apply"], vec!["bootstrap", "apply"]] {
        let result = cli()
            .args(prefix)
            .arg("--plan")
            .arg(file.path())
            .args(["--expect-plan-sha256", p["plan_sha256"].as_str().unwrap()])
            .output()
            .unwrap();
        assert!(!result.status.success());
    }
    for imports in [
        vec!["LICENSE"],
        vec!["../escape"],
        vec!["armorer.lock", "armorer.lock"],
    ] {
        assert!(
            upgrade::inspect(
                root.path(),
                &root.path().join(POLICY),
                "bootstrap-v1",
                &imports.iter().map(|s| (*s).into()).collect::<Vec<_>>(),
                false
            )
            .is_err()
        );
    }
    assert_eq!(snapshot(root.path()), before);
}

/// Verify that all profiles apply repeat migrate ownership and restore exact reviewed previous bytes.
#[test]
fn all_profiles_apply_repeat_migrate_ownership_and_restore_exact_reviewed_previous_bytes() {
    for profile in ["library", "cli", "service"] {
        for owned in [false, true] {
            let root = fixture(profile);
            if owned {
                bootstrap(root.path());
            }
            let before = snapshot(root.path());
            let p = preview(root.path(), &[]);
            let receipt = upgrade::apply(root.path(), &p, &p.plan_sha256).unwrap();
            assert_eq!(receipt.outcome, "applied");
            assert!(receipt.configured);
            assert!(
                !receipt.ci_verified
                    && !receipt.release_rehearsed
                    && !receipt.published
                    && !receipt.provenance_verified
            );
            assert!(
                !root
                    .path()
                    .join(".armorer/bootstrap-state-v2.json")
                    .exists()
            );
            assert!(root.path().join(".armorer/upgrade-state-v1.json").exists());
            let after = snapshot(root.path());
            let stamps: BTreeMap<_, _> = after
                .keys()
                .filter(|p| p.as_str() != ".armorer/apply.lock")
                .map(|p| {
                    (
                        p.clone(),
                        fs::metadata(root.path().join(p))
                            .unwrap()
                            .modified()
                            .unwrap(),
                    )
                })
                .collect();
            assert_eq!(
                upgrade::apply(root.path(), &p, &p.plan_sha256)
                    .unwrap()
                    .outcome,
                "unchanged"
            );
            assert_eq!(snapshot(root.path()), after);
            for (path, stamp) in stamps {
                assert_eq!(
                    fs::metadata(root.path().join(path))
                        .unwrap()
                        .modified()
                        .unwrap(),
                    stamp
                );
            }
            let fresh = preview(root.path(), &[]);
            assert_eq!(fresh.source_format, upgrade::SourceFormat::UpgradeV1);
            assert_eq!(fresh.direction, "refresh");
            assert!(
                fresh
                    .changes
                    .iter()
                    .all(|c| c.decision == Decision::Unchanged)
            );
            assert!(bootstrap::inspect(root.path(), &root.path().join(POLICY)).is_err());
            let legacy = armorer::plan::inspect(root.path(), "plan").unwrap();
            assert!(armorer::apply::apply(root.path(), &legacy, &legacy.plan_sha256).is_err());
            assert!(upgrade::inspect_rollback(root.path(), &p, &p.plan_sha256, false).is_err());
            assert_eq!(snapshot(root.path()), after);
            let reverse = upgrade::inspect_rollback(root.path(), &p, &p.plan_sha256, true).unwrap();
            assert_eq!(snapshot(root.path()), after);
            assert!(
                reverse
                    .changes
                    .iter()
                    .any(|c| c.path == ".armorer/upgrade-state-v1.json")
            );
            let receipt =
                upgrade::apply_rollback(root.path(), &reverse, &reverse.plan_sha256).unwrap();
            assert_eq!(receipt.outcome, "rolled-back");
            assert!(!receipt.configured && !receipt.ci_verified && !receipt.published);
            let mut restored = snapshot(root.path());
            restored.remove(".armorer/apply.lock");
            let mut expected = before;
            expected.remove(".armorer/apply.lock");
            assert_eq!(restored, expected);
            let restored = snapshot(root.path());
            assert_eq!(
                upgrade::apply_rollback(root.path(), &reverse, &reverse.plan_sha256)
                    .unwrap()
                    .outcome,
                "unchanged"
            );
            assert_eq!(snapshot(root.path()), restored);
            assert!(!root.path().join("BUILD_EXECUTED").exists());
            assert!(!root.path().join("target").exists());
        }
    }
}
/// Verify that upgraded generated bases survive customizations and refreshes then configuration upgrade.
#[test]
fn upgraded_generated_bases_survive_customizations_and_refreshes_then_configuration_upgrade() {
    let root = fixture("cli");
    bootstrap(root.path());
    let ci = ".github/workflows/armorer-ci.yml";
    let customized = fs::read_to_string(root.path().join(ci)).unwrap()
        + "  bespoke:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo custom\n";
    fs::write(root.path().join(ci), &customized).unwrap();
    let policy = fs::read_to_string(root.path().join(".armorer/ci-policy.toml")).unwrap()
        + "# retained policy note\n";
    fs::write(root.path().join(".armorer/ci-policy.toml"), &policy).unwrap();
    let p = preview(root.path(), &[]);
    upgrade::apply(root.path(), &p, &p.plan_sha256).unwrap();
    let next = preview(root.path(), &[]);
    assert_eq!(change(&next, ci).decision, Decision::PreserveCustomization);
    assert_eq!(
        change(&next, ".armorer/ci-policy.toml").decision,
        Decision::PreserveCustomization
    );
    assert_eq!(
        change(&next, ci).base_content,
        change(&p, ci).candidate_content.clone().into()
    );
    upgrade::apply(root.path(), &next, &next.plan_sha256).unwrap();
    assert_eq!(
        fs::read_to_string(root.path().join(ci)).unwrap(),
        customized
    );
    assert_eq!(
        fs::read_to_string(root.path().join(".armorer/ci-policy.toml")).unwrap(),
        policy
    );
    let original = snapshot(root.path());
    let path = root.path().join("armorer.toml");
    fs::write(
        &path,
        format!(
            "{}\n# approved configuration upgrade\n",
            fs::read_to_string(&path).unwrap()
        ),
    )
    .unwrap();
    let before = snapshot(root.path());
    let p = preview(root.path(), &[]);
    assert_eq!(
        change(&p, "rust-toolchain.toml").decision,
        Decision::Unchanged
    );
    assert_eq!(change(&p, "armorer.lock").decision, Decision::Update);
    upgrade::apply(root.path(), &p, &p.plan_sha256).unwrap();
    let reverse = upgrade::inspect_rollback(root.path(), &p, &p.plan_sha256, true).unwrap();
    upgrade::apply_rollback(root.path(), &reverse, &reverse.plan_sha256).unwrap();
    assert_eq!(snapshot(root.path()), before);
    assert_ne!(
        fs::read(root.path().join("armorer.toml")).unwrap(),
        original["armorer.toml"]
    );
    // Reverse never edits caller configuration to conceal restored old lock binding.
    assert!(load_lock(root.path()).is_err());
}
/// Verify that declared import is explicit and reversible without upgrading its claim to authentication.
#[test]
fn declared_import_is_explicit_and_reversible_without_upgrading_its_claim_to_authentication() {
    let root = fixture("library");
    let legacy = armorer::plan::inspect(root.path(), "plan").unwrap();
    armorer::apply::apply(root.path(), &legacy, &legacy.plan_sha256).unwrap();
    let old = format!(
        "schema_version=1\nruntime_version=\"0.1.0\"\nconfig_sha256=\"{}\"\n[workflows]\nrepository=\"example/old\"\ncommit=\"{}\"\n[tools.cargo-deny]\nversion=\"0.19.0\"\nsha256=\"{}\"\n",
        "a".repeat(64),
        "b".repeat(40),
        "c".repeat(64)
    );
    fs::write(root.path().join("armorer.lock"), &old).unwrap();
    let before = snapshot(root.path());
    let p = preview(root.path(), &["armorer.lock"]);
    assert_eq!(change(&p, "armorer.lock").decision, Decision::Import);
    upgrade::apply(root.path(), &p, &p.plan_sha256).unwrap();
    assert!(load_lock(root.path()).is_ok());
    let reverse = upgrade::inspect_rollback(root.path(), &p, &p.plan_sha256, true).unwrap();
    upgrade::apply_rollback(root.path(), &reverse, &reverse.plan_sha256).unwrap();
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(
        fs::read_to_string(root.path().join("armorer.lock")).unwrap(),
        old
    );
    assert!(load_lock(root.path()).is_err());
}
/// Verify that edited completed files or state block reviewed rollback and replay without mutations.
#[test]
fn edited_completed_files_or_state_block_reviewed_rollback_and_replay_without_mutations() {
    for path in [
        ".armorer/ci-policy.toml",
        ".github/workflows/armorer-build.yml",
        ".github/workflows/armorer-ci.yml",
        "armorer.lock",
        "rust-toolchain.toml",
    ]
    .into_iter()
    .chain([".armorer/upgrade-state-v1.json"])
    {
        let root = fixture("library");
        let p = preview(root.path(), &[]);
        upgrade::apply(root.path(), &p, &p.plan_sha256).unwrap();
        let reverse = upgrade::inspect_rollback(root.path(), &p, &p.plan_sha256, true).unwrap();
        fs::write(root.path().join(path), "intervening edit").unwrap();
        let before = snapshot(root.path());
        assert!(
            upgrade::inspect_rollback(root.path(), &p, &p.plan_sha256, true).is_err(),
            "{path}"
        );
        assert!(
            upgrade::apply_rollback(root.path(), &reverse, &reverse.plan_sha256).is_err(),
            "{path}"
        );
        assert!(
            upgrade::apply(root.path(), &p, &p.plan_sha256).is_err(),
            "{path}"
        );
        assert_eq!(snapshot(root.path()), before);
    }
}
/// Verify that strict rollback packet requires independent approval and reconstructs every restore.
#[test]
fn strict_rollback_packet_requires_independent_approval_and_reconstructs_every_restore() {
    let root = fixture("service");
    bootstrap(root.path());
    let p = preview(root.path(), &[]);
    upgrade::apply(root.path(), &p, &p.plan_sha256).unwrap();
    let reverse = upgrade::inspect_rollback(root.path(), &p, &p.plan_sha256, true).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    fs::write(file.path(), serde_json::to_vec(&reverse).unwrap()).unwrap();
    upgrade::load_rollback_plan(file.path(), &reverse.plan_sha256).unwrap();
    assert!(upgrade::load_rollback_plan(file.path(), &"a".repeat(64)).is_err());
    assert!(upgrade::load_plan(file.path(), &reverse.plan_sha256).is_err());
    for mutation in 0..6 {
        let mut forged = reverse.clone();
        match mutation {
            0 => forged.allow_downgrade = false,
            1 => forged.changes[0].path = "LICENSE".into(),
            2 => forged.changes[0].proposed_content = Some("foreign bytes".into()),
            3 => forged.changes.reverse(),
            4 => forged.original_upgrade.target_catalog.tools_sha256 = "a".repeat(64),
            5 => forged.limitations.clear(),
            _ => unreachable!(),
        }
        forged.plan_sha256 = forged.digest().unwrap();
        fs::write(file.path(), serde_json::to_vec(&forged).unwrap()).unwrap();
        assert!(
            upgrade::load_rollback_plan(file.path(), &forged.plan_sha256).is_err(),
            "{mutation}"
        );
        let before = snapshot(root.path());
        assert!(upgrade::apply_rollback(root.path(), &forged, &forged.plan_sha256).is_err());
        assert_eq!(snapshot(root.path()), before);
    }
}

/// Verify that identical unowned targets stay unowned until individual explicit supported imports.
#[test]
fn identical_unowned_targets_stay_unowned_until_individual_explicit_supported_imports() {
    let root = fixture("library");
    let initial = preview(root.path(), &[]);
    for c in &initial.changes {
        let p = root.path().join(&c.path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, &c.candidate_content).unwrap();
    }
    let p = preview(root.path(), &[]);
    assert!(p.changes.iter().all(|c| c.decision == Decision::Unchanged));
    upgrade::apply(root.path(), &p, &p.plan_sha256).unwrap();
    let next = preview(root.path(), &[]);
    assert!(next.changes.iter().all(|c| c.base_content.is_none()));
    let imported = preview(
        root.path(),
        &[
            ".github/workflows/armorer-ci.yml",
            ".armorer/ci-policy.toml",
        ],
    );
    upgrade::apply(root.path(), &imported, &imported.plan_sha256).unwrap();
    let next = preview(root.path(), &[]);
    assert!(
        change(&next, ".github/workflows/armorer-ci.yml")
            .base_content
            .is_some()
    );
    assert!(
        change(&next, ".armorer/ci-policy.toml")
            .base_content
            .is_some()
    );
    assert!(change(&next, "armorer.lock").base_content.is_none());
}
/// Verify that symlinked upgrade controls and targets cannot be read adopted or overwritten.
#[cfg(unix)]
#[test]
fn symlinked_upgrade_controls_and_targets_cannot_be_read_adopted_or_overwritten() {
    use std::os::unix::fs::symlink;
    for path in [
        ".armorer/upgrade-state-v1.json",
        ".armorer/upgrade-journal-v1.json",
        "armorer.lock",
        ".github/workflows/armorer-ci.yml",
    ] {
        let root = fixture("library");
        let p = preview(root.path(), &[]);
        let outside = tempfile::NamedTempFile::new().unwrap();
        fs::write(outside.path(), "outside").unwrap();
        let link = root.path().join(path);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        symlink(outside.path(), &link).unwrap();
        assert!(
            upgrade::inspect(
                root.path(),
                &root.path().join(POLICY),
                "bootstrap-v1",
                &[],
                true
            )
            .is_err(),
            "{path}"
        );
        assert!(
            upgrade::apply(root.path(), &p, &p.plan_sha256).is_err(),
            "{path}"
        );
        assert!(
            upgrade::recover(root.path(), &p.plan_sha256).is_err(),
            "{path}"
        );
        assert_eq!(fs::read(outside.path()).unwrap(), b"outside");
        assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
    }
}

/// Verify that committed schemas and all profile review packets match strict runtime contracts.
#[test]
fn committed_schemas_and_all_profile_review_packets_match_strict_runtime_contracts() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    for kind in ["upgrade-plan", "upgrade-rollback"] {
        let output = Command::new(env!("CARGO_BIN_EXE_armorer"))
            .args(["schema", kind])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            output.stdout,
            fs::read(source.join(format!("schemas/{kind}-v1.json"))).unwrap()
        );
    }
    for profile in ["library", "cli", "service"] {
        let path = source.join("examples/upgrades").join(profile);
        let p: serde_json::Value =
            serde_json::from_slice(&fs::read(path.join("upgrade-plan.json")).unwrap()).unwrap();
        upgrade::load_plan(
            &path.join("upgrade-plan.json"),
            p["plan_sha256"].as_str().unwrap(),
        )
        .unwrap();
        let r: serde_json::Value =
            serde_json::from_slice(&fs::read(path.join("rollback-plan.json")).unwrap()).unwrap();
        upgrade::load_rollback_plan(
            &path.join("rollback-plan.json"),
            r["plan_sha256"].as_str().unwrap(),
        )
        .unwrap();
    }
}
/// Verify that independent patch reconstructs upgrade and reverse images including deleted ownership.
#[test]
fn independent_patch_reconstructs_upgrade_and_reverse_images_including_deleted_ownership() {
    use std::io::Write;
    use std::process::Stdio;
    let root = fixture("library");
    let p = preview(root.path(), &[]);
    upgrade::apply(root.path(), &p, &p.plan_sha256).unwrap();
    let reverse = upgrade::inspect_rollback(root.path(), &p, &p.plan_sha256, true).unwrap();
    let cases = p
        .changes
        .iter()
        .map(|c| {
            (
                &c.path,
                &c.before_content,
                &c.proposed_content,
                c.proposed_diff.as_deref().unwrap(),
            )
        })
        .chain(reverse.changes.iter().map(|c| {
            (
                &c.path,
                &c.before_content,
                &c.proposed_content,
                c.unified_diff.as_str(),
            )
        }));
    for (path, before, after, diff) in cases {
        let output = tempfile::tempdir().unwrap();
        let file = output.path().join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        if let Some(before) = before {
            fs::write(&file, before).unwrap();
        }
        if !diff.is_empty() {
            let mut patch = Command::new("patch")
                .current_dir(output.path())
                .args(["-p1", "--batch", "-E"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            patch
                .stdin
                .take()
                .unwrap()
                .write_all(diff.as_bytes())
                .unwrap();
            let result = patch.wait_with_output().unwrap();
            assert!(
                result.status.success(),
                "{path}: {}",
                String::from_utf8_lossy(&result.stdout)
            );
        }
        match after {
            Some(after) => assert_eq!(fs::read(file).unwrap(), after.as_bytes(), "{path}"),
            None => assert!(!file.exists(), "{path}"),
        }
    }
}

/// Verify that forged legacy generated toolchain base with matching self hash never becomes an import.
#[test]
fn forged_legacy_generated_toolchain_base_with_matching_self_hash_never_becomes_an_import() {
    use sha2::{Digest, Sha256};
    let root = fixture("library");
    let legacy = armorer::plan::inspect(root.path(), "plan").unwrap();
    armorer::apply::apply(root.path(), &legacy, &legacy.plan_sha256).unwrap();
    let path = root.path().join(".armorer/state.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let forged = "[toolchain]\nchannel=\"1.95.0\"\nprofile=\"default\"\n";
    value["managed"]["rust-toolchain.toml"]["content"] = forged.into();
    value["managed"]["rust-toolchain.toml"]["sha256"] =
        format!("{:x}", Sha256::digest(forged)).into();
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    fs::write(root.path().join("rust-toolchain.toml"), forged).unwrap();
    let before = snapshot(root.path());
    assert!(
        upgrade::inspect(
            root.path(),
            &root.path().join(POLICY),
            "bootstrap-v1",
            &["armorer.lock".into()],
            true
        )
        .is_err()
    );
    assert_eq!(snapshot(root.path()), before);
}
