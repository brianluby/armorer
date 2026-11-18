//! ADR 0013: the managed file set is a property of the compiled catalog entry.
//! These tests drive a second, test-only file set (`bootstrap-test-wide`, which is
//! `bootstrap-v2` plus one fixed file) through the real bootstrap, upgrade,
//! rollback, recovery and plan-loading paths.
use armorer::{
    bootstrap, catalog,
    plan::Disposition,
    upgrade::{self, merge::Decision},
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path, process::Command};

const POLICY: &str = "ci-policy.reviewed.toml";
const WIDE: &str = "bootstrap-test-wide";
const EXTRA: &str = ".github/armorer-managed.md";
const EXTRA_CONTENT: &str = include_str!("../catalogs/templates/armorer-managed.md.tmpl");
const BOOTSTRAP_STATE: &str = ".armorer/bootstrap-state-v2.json";
const BOOTSTRAP_JOURNAL: &str = ".armorer/bootstrap-journal-v2.json";
const UPGRADE_STATE: &str = ".armorer/upgrade-state-v1.json";
const UPGRADE_JOURNAL: &str = ".armorer/upgrade-journal-v1.json";

/// Copy the committed library profile into isolated storage, with a build canary.
fn fixture() -> tempfile::TempDir {
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
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/bootstrap-v2/library"),
        root.path(),
    );
    fs::write(
        root.path().join("build.rs"),
        "fn main(){std::fs::write(\"BUILD_EXECUTED\",\"bad\").unwrap();panic!(\"no builds\");}",
    )
    .unwrap();
    root
}

/// Every regular file as exact bytes, ignoring the persistent kernel lock.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
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
    map.remove(".armorer/apply.lock");
    map
}

/// Provision bootstrap-v2 (five files) through the approved bootstrap contract.
fn bootstrap(root: &Path) {
    let p = bootstrap::inspect(root, &root.join(POLICY)).unwrap();
    assert_eq!(p.catalog_id, "bootstrap-v2");
    assert_eq!(p.changes.len(), 5);
    bootstrap::apply(root, &p, &p.plan_sha256).unwrap();
}

/// Preview an upgrade to the named compiled catalog.
fn preview(root: &Path, id: &str, allow_downgrade: bool) -> armorer::Result<upgrade::Plan> {
    upgrade::inspect(root, &root.join(POLICY), id, &[], allow_downgrade)
}

/// Paths of a plan's change list in plan order (not re-sorted, so comparing against the
/// sorted declared set also asserts that the plan itself is ordered).
fn paths(changes: impl IntoIterator<Item = String>) -> Vec<String> {
    changes.into_iter().collect()
}

/// Managed paths the named catalog declares.
fn managed(id: &str) -> Vec<String> {
    catalog::reviewed_for(id)
        .unwrap()
        .managed_paths()
        .into_iter()
        .map(str::to_owned)
        .collect()
}

/// Paths owned by the upgrade ownership receipt.
fn owned(root: &Path, state: &str) -> Vec<String> {
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(state)).unwrap()).unwrap();
    value["managed"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

/// Bootstrap v2 then apply the reviewed v2 -> wide upgrade; returns the approved plan.
fn upgraded_to_wide(root: &Path) -> upgrade::Plan {
    bootstrap(root);
    let p = preview(root, WIDE, false).unwrap();
    assert_eq!(
        upgrade::apply(root, &p, &p.plan_sha256).unwrap().outcome,
        "applied"
    );
    p
}

#[test]
fn declared_set_equals_rendered_set_for_every_compiled_catalog() {
    let mut union = std::collections::BTreeSet::new();
    for id in catalog::compiled_ids() {
        let declared = managed(id);
        let mut sorted = declared.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(declared, sorted, "{id}: sorted and unique");
        // An unmanaged repository's upgrade preview is exactly the rendered set.
        let root = fixture();
        let p = preview(root.path(), id, false).unwrap();
        assert_eq!(
            paths(p.changes.iter().map(|c| c.path.clone())),
            declared,
            "{id}"
        );
        assert!(p.changes.iter().all(|c| c.decision == Decision::Create));
        union.extend(declared);
    }
    assert_eq!(
        catalog::all_managed_paths(),
        union.iter().map(String::as_str).collect::<Vec<_>>()
    );
    assert_eq!(managed("bootstrap-v2").len(), 5);
    assert_eq!(managed(WIDE).len(), 6);
    assert!(managed(WIDE).contains(&EXTRA.to_owned()));
}

#[test]
fn upgrade_v2_to_wide_creates_the_sixth_file_and_replays_unchanged() {
    let root = fixture();
    bootstrap(root.path());
    let before = snapshot(root.path());
    let p = preview(root.path(), WIDE, false).unwrap();
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(p.direction, "upgrade");
    assert_eq!(p.source_catalog.as_ref().unwrap().id, "bootstrap-v2");
    assert_eq!(p.target_catalog.id, WIDE);
    assert_eq!(p.target_catalog.revision, 3);
    assert_eq!(p.changes.len(), 6);
    assert_eq!(
        paths(p.changes.iter().map(|c| c.path.clone())),
        managed(WIDE)
    );
    let creates: Vec<_> = p
        .changes
        .iter()
        .filter(|c| c.decision == Decision::Create)
        .collect();
    assert_eq!(creates.len(), 1);
    let extra = creates[0];
    assert_eq!(extra.path, EXTRA);
    assert!(extra.base_content.is_none() && extra.before_content.is_none());
    assert_eq!(extra.proposed_content.as_deref(), Some(EXTRA_CONTENT));
    assert!(p.changes.iter().all(|c| c.decision != Decision::Conflict));
    // The saved packet reloads against its independently supplied digest.
    let file = tempfile::NamedTempFile::new().unwrap();
    fs::write(file.path(), serde_json::to_vec(&p).unwrap()).unwrap();
    upgrade::load_plan(file.path(), &p.plan_sha256).unwrap();

    assert_eq!(
        upgrade::apply(root.path(), &p, &p.plan_sha256)
            .unwrap()
            .outcome,
        "applied"
    );
    assert!(!root.path().join(BOOTSTRAP_STATE).exists());
    assert_eq!(owned(root.path(), UPGRADE_STATE), managed(WIDE));
    assert_eq!(
        fs::read_to_string(root.path().join(EXTRA)).unwrap(),
        EXTRA_CONTENT
    );
    let after = snapshot(root.path());
    assert_eq!(
        upgrade::apply(root.path(), &p, &p.plan_sha256)
            .unwrap()
            .outcome,
        "unchanged"
    );
    assert_eq!(snapshot(root.path()), after);
    // Read-only bootstrap inspection (check) follows the upgrade owner's catalog.
    let check = bootstrap::inspect(root.path(), &root.path().join(POLICY)).unwrap();
    assert_eq!(check.catalog_id, WIDE);
    assert_eq!(check.changes.len(), 6);
    assert!(
        check
            .changes
            .iter()
            .all(|c| c.disposition == Disposition::Unchanged)
    );
    assert!(check.drift().is_empty());
    assert!(!root.path().join("BUILD_EXECUTED").exists());
}

#[test]
fn reviewed_rollback_restores_the_five_file_state_and_removes_the_sixth_file() {
    let root = fixture();
    bootstrap(root.path());
    let five = snapshot(root.path());
    let p = preview(root.path(), WIDE, false).unwrap();
    upgrade::apply(root.path(), &p, &p.plan_sha256).unwrap();
    let reverse = upgrade::inspect_rollback(root.path(), &p, &p.plan_sha256, true).unwrap();
    assert!(
        reverse
            .changes
            .iter()
            .any(|c| c.path == EXTRA && c.proposed_content.is_none())
    );
    assert_eq!(
        upgrade::apply_rollback(root.path(), &reverse, &reverse.plan_sha256)
            .unwrap()
            .outcome,
        "rolled-back"
    );
    assert!(!root.path().join(EXTRA).exists());
    assert!(!root.path().join(UPGRADE_STATE).exists());
    assert_eq!(snapshot(root.path()), five);
    assert_eq!(owned(root.path(), BOOTSTRAP_STATE), managed("bootstrap-v2"));
    let check = bootstrap::inspect(root.path(), &root.path().join(POLICY)).unwrap();
    assert_eq!(check.catalog_id, "bootstrap-v2");
    assert_eq!(check.changes.len(), 5);
    assert!(check.drift().is_empty());
}

#[test]
fn refresh_on_the_wide_catalog_is_six_to_six_and_all_unchanged() {
    let root = fixture();
    upgraded_to_wide(root.path());
    let before = snapshot(root.path());
    let refresh = preview(root.path(), WIDE, false).unwrap();
    assert_eq!(refresh.direction, "refresh");
    assert_eq!(refresh.source_catalog.as_ref().unwrap().id, WIDE);
    assert_eq!(refresh.changes.len(), 6);
    assert!(
        refresh
            .changes
            .iter()
            .all(|c| c.decision == Decision::Unchanged)
    );
    assert_eq!(
        upgrade::apply(root.path(), &refresh, &refresh.plan_sha256)
            .unwrap()
            .outcome,
        "applied"
    );
    // Only the ownership receipt's plan binding moves; no managed byte changes.
    let mut after = snapshot(root.path());
    let mut expected = before;
    after.remove(UPGRADE_STATE);
    expected.remove(UPGRADE_STATE);
    assert_eq!(after, expected);
    assert_eq!(owned(root.path(), UPGRADE_STATE), managed(WIDE));
}

/// An owned file the target catalog does not render has no defined merge outcome,
/// so it is a blocking conflict: never a silent delete or a silent ownership release.
#[test]
fn downgrade_to_a_narrower_catalog_blocks_on_the_owned_unrendered_file() {
    let root = fixture();
    upgraded_to_wide(root.path());
    let before = snapshot(root.path());
    assert!(preview(root.path(), "bootstrap-v2", false).is_err());
    let down = preview(root.path(), "bootstrap-v2", true).unwrap();
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(down.direction, "downgrade");
    assert_eq!(down.source_catalog.as_ref().unwrap().id, WIDE);
    assert_eq!(down.changes.len(), 6);
    assert_eq!(
        paths(down.changes.iter().map(|c| c.path.clone())),
        managed(WIDE)
    );
    let extra = down.changes.iter().find(|c| c.path == EXTRA).unwrap();
    assert_eq!(extra.decision, Decision::Conflict);
    assert_eq!(extra.base_content.as_deref(), Some(EXTRA_CONTENT));
    assert_eq!(extra.before_content.as_deref(), Some(EXTRA_CONTENT));
    assert!(extra.proposed_content.is_none() && extra.after_sha256.is_none());
    assert!(extra.candidate_content.is_empty());
    assert!(extra.candidate_diff.contains("+++ /dev/null\n"));
    // Every rendered file still follows the ordinary merge rules.
    assert!(
        down.changes
            .iter()
            .filter(|c| c.path != EXTRA)
            .all(|c| matches!(c.decision, Decision::Unchanged | Decision::Update))
    );
    assert!(
        down.compatibility
            .iter()
            .any(|c| c.state == "blocked" && c.code == "three-way-conflict")
    );
    // The packet is self-consistent and reconstructs, but it cannot be applied.
    let file = tempfile::NamedTempFile::new().unwrap();
    fs::write(file.path(), serde_json::to_vec(&down).unwrap()).unwrap();
    upgrade::load_plan(file.path(), &down.plan_sha256).unwrap();
    upgrade::validate_fresh(root.path(), &down).unwrap();
    assert!(upgrade::apply(root.path(), &down, &down.plan_sha256).is_err());
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(
        fs::read_to_string(root.path().join(EXTRA)).unwrap(),
        EXTRA_CONTENT
    );
    assert_eq!(owned(root.path(), UPGRADE_STATE), managed(WIDE));
}

/// Rename one managed entry of a state document to the wide catalog's extra path.
fn retarget(value: &mut serde_json::Value, from: &str, entry: serde_json::Value) {
    let managed = value["managed"].as_object_mut().unwrap();
    managed.remove(from).unwrap();
    managed.insert(EXTRA.into(), entry);
}

#[test]
fn states_listing_a_path_outside_their_catalog_set_are_rejected() {
    let digest = format!("{:x}", Sha256::digest(EXTRA_CONTENT.as_bytes()));
    // Bootstrap-v2 ownership: a renamed entry, then an added sixth entry.
    for extra_entry in [false, true] {
        let root = fixture();
        bootstrap(root.path());
        let path = root.path().join(BOOTSTRAP_STATE);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let entry = serde_json::json!({ "content": EXTRA_CONTENT, "sha256": digest });
        if extra_entry {
            value["managed"][EXTRA] = entry;
        } else {
            retarget(&mut value, "rust-toolchain.toml", entry);
        }
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        let before = snapshot(root.path());
        let error = bootstrap::inspect(root.path(), &root.path().join(POLICY)).unwrap_err();
        assert!(
            error.to_string().contains("incompatible bootstrap state"),
            "{error}"
        );
        assert!(preview(root.path(), WIDE, false).is_err());
        assert_eq!(snapshot(root.path()), before);
    }
    // Upgrade ownership on bootstrap-v2 cannot own the wide catalog's extra file.
    for extra_entry in [false, true] {
        let root = fixture();
        bootstrap(root.path());
        let p = preview(root.path(), "bootstrap-v2", false).unwrap();
        upgrade::apply(root.path(), &p, &p.plan_sha256).unwrap();
        let path = root.path().join(UPGRADE_STATE);
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let entry = serde_json::json!({
            "generated": EXTRA_CONTENT,
            "generated_sha256": digest,
            "applied": EXTRA_CONTENT,
            "applied_sha256": digest,
        });
        if extra_entry {
            value["managed"][EXTRA] = entry;
        } else {
            retarget(&mut value, "rust-toolchain.toml", entry);
        }
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        let before = snapshot(root.path());
        assert!(preview(root.path(), "bootstrap-v2", false).is_err());
        assert!(preview(root.path(), WIDE, false).is_err());
        assert!(bootstrap::inspect(root.path(), &root.path().join(POLICY)).is_err());
        assert_eq!(snapshot(root.path()), before);
    }
}

#[test]
fn bootstrap_journal_listing_a_path_outside_its_catalog_set_is_rejected() {
    let root = fixture();
    let plan = bootstrap::inspect(root.path(), &root.path().join(POLICY)).unwrap();
    let mut operations: Vec<serde_json::Value> = plan
        .changes
        .iter()
        .map(|c| serde_json::json!({ "path": c.path, "before": null, "after": c.proposed_content }))
        .collect();
    operations.push(serde_json::json!({ "path": EXTRA, "before": null, "after": EXTRA_CONTENT }));
    operations.push(serde_json::json!({ "path": BOOTSTRAP_STATE, "before": null, "after": "{}" }));
    for replace in [false, true] {
        let mut ops = operations.clone();
        if replace {
            // Same count as a genuine inventory, but one path is outside the set.
            ops.remove(0);
        }
        let journal = serde_json::json!({
            "schema_version": 2,
            "plan": plan,
            "committed": false,
            "operations": ops,
        });
        fs::create_dir_all(root.path().join(".armorer")).unwrap();
        fs::write(
            root.path().join(BOOTSTRAP_JOURNAL),
            serde_json::to_vec(&journal).unwrap(),
        )
        .unwrap();
        let before = snapshot(root.path());
        let error = bootstrap::recover(root.path(), &plan.plan_sha256).unwrap_err();
        let expected = if replace {
            "bootstrap recovery inventory differs from approved plan"
        } else {
            "invalid bootstrap recovery journal"
        };
        assert!(error.to_string().contains(expected), "{error}");
        assert_eq!(snapshot(root.path()), before);
    }
}

#[test]
fn upgrade_journal_listing_a_path_outside_its_catalog_set_is_rejected() {
    let root = fixture();
    let p = preview(root.path(), "bootstrap-v2", false).unwrap();
    let mut operations: Vec<serde_json::Value> = p
        .changes
        .iter()
        .map(|c| serde_json::json!({ "path": c.path, "before": null, "after": c.proposed_content }))
        .collect();
    // bootstrap-v2 bounds an inventory at five files plus two ownership slots.
    operations.push(serde_json::json!({ "path": EXTRA, "before": null, "after": EXTRA_CONTENT }));
    operations.push(serde_json::json!({ "path": EXTRA, "before": null, "after": EXTRA_CONTENT }));
    operations.push(serde_json::json!({ "path": UPGRADE_STATE, "before": null, "after": "{}" }));
    let journal = serde_json::json!({
        "schema_version": 1,
        "approval": { "operation": "upgrade", "plan": p },
        "committed": false,
        "operations": operations,
    });
    fs::create_dir_all(root.path().join(".armorer")).unwrap();
    fs::write(
        root.path().join(UPGRADE_JOURNAL),
        serde_json::to_vec(&journal).unwrap(),
    )
    .unwrap();
    let before = snapshot(root.path());
    let error = upgrade::recover(root.path(), &p.plan_sha256).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("upgrade recovery inventory differs from approved packet"),
        "{error}"
    );
    assert_eq!(snapshot(root.path()), before);
}

#[test]
fn bootstrap_plans_must_list_exactly_their_catalog_set() {
    let root = fixture();
    let plan = bootstrap::inspect(root.path(), &root.path().join(POLICY)).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    for mutation in ["missing", "extra", "reordered"] {
        let mut forged = plan.clone();
        match mutation {
            "missing" => {
                forged.changes.pop();
            }
            "extra" => {
                let mut c = forged.changes[0].clone();
                c.path = EXTRA.into();
                forged.changes.insert(1, c);
            }
            "reordered" => forged.changes.swap(0, 1),
            _ => unreachable!(),
        }
        forged.plan_sha256 = forged.digest().unwrap();
        fs::write(file.path(), serde_json::to_vec(&forged).unwrap()).unwrap();
        let error = bootstrap::load_plan(file.path(), &forged.plan_sha256).unwrap_err();
        let expected = if mutation == "reordered" {
            "unsupported bootstrap content, preimage or target"
        } else {
            "bootstrap plan does not match approved digest, catalog or runtime"
        };
        assert!(error.to_string().contains(expected), "{mutation}: {error}");
    }
    // A genuine five-file list is rejected when relabeled to the six-file catalog.
    let mut relabeled = plan.clone();
    relabeled.catalog_id = WIDE.into();
    relabeled.plan_sha256 = relabeled.digest().unwrap();
    fs::write(file.path(), serde_json::to_vec(&relabeled).unwrap()).unwrap();
    let error = bootstrap::load_plan(file.path(), &relabeled.plan_sha256).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("bootstrap plan does not match approved digest, catalog or runtime"),
        "{error}"
    );
}

#[test]
fn upgrade_plans_must_list_exactly_their_source_and_target_scope() {
    let root = fixture();
    bootstrap(root.path());
    let file = tempfile::NamedTempFile::new().unwrap();
    for target in ["bootstrap-v2", WIDE] {
        let p = preview(root.path(), target, false).unwrap();
        for mutation in ["missing", "extra", "reordered"] {
            let mut forged = p.clone();
            match mutation {
                "missing" => {
                    forged.changes.retain(|c| c.path != "armorer.lock");
                }
                "extra" => {
                    let mut c = forged.changes[0].clone();
                    c.path = if target == WIDE {
                        ".github/unlisted.md".into()
                    } else {
                        EXTRA.into()
                    };
                    forged.changes.insert(1, c);
                }
                "reordered" => forged.changes.swap(0, 1),
                _ => unreachable!(),
            }
            forged.plan_sha256 = forged.digest().unwrap();
            fs::write(file.path(), serde_json::to_vec(&forged).unwrap()).unwrap();
            let error = upgrade::load_plan(file.path(), &forged.plan_sha256).unwrap_err();
            let expected = match (target, mutation) {
                // Within the compiled bound, but not this source-and-target scope.
                ("bootstrap-v2", "extra") => "generated target set differs from fixed contract",
                (_, "reordered") => "upgrade merge or import differs from stored generated base",
                _ => "upgrade plan does not match independently approved digest or authority",
            };
            assert!(
                error.to_string().contains(expected),
                "{target} {mutation}: {error}"
            );
        }
    }
}

/// The test build (feature `test-predecessor`) selects the wide catalog; the
/// production binary's refusal is asserted in the development workflow.
#[test]
fn test_build_cli_plans_the_wide_catalog() {
    let root = fixture();
    let output = Command::new(env!("CARGO_BIN_EXE_armorer"))
        .arg("--repository")
        .arg(root.path())
        .args(["upgrade", "plan", "--policy"])
        .arg(root.path().join(POLICY))
        .args(["--catalog", WIDE])
        .output()
        .unwrap();
    assert!(output.status.success());
    let plan: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(plan["target_catalog"]["id"], WIDE);
    assert_eq!(plan["changes"].as_array().unwrap().len(), 6);
}

#[test]
fn upgrade_state_fixed_file_is_accepted_only_when_unmodified() {
    let root = fixture();
    upgraded_to_wide(root.path());
    // Unmodified: the wide state decodes and previews again.
    assert!(preview(root.path(), WIDE, false).is_ok());
    // Customized: the fixed file's approved image differs from the generated bytes.
    let path = root.path().join(UPGRADE_STATE);
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let custom = format!("{EXTRA_CONTENT}# customized\n");
    value["managed"][EXTRA]["applied"] = custom.clone().into();
    value["managed"][EXTRA]["applied_sha256"] =
        format!("{:x}", Sha256::digest(custom.as_bytes())).into();
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    let before = snapshot(root.path());
    assert!(preview(root.path(), WIDE, false).is_err());
    assert!(preview(root.path(), "bootstrap-v2", true).is_err());
    assert_eq!(snapshot(root.path()), before);
}
