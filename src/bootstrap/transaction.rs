//! Fixed-inventory durable transactions with exact recovery and shared v1 locking.
use super::{
    JOURNAL, STATE, TARGETS, V1_JOURNAL, V1_STATE, VERSION,
    plan::{self, Plan},
};
use crate::{
    Error, Result,
    apply::{decode, json, lock, optional_bytes, replace, sync_directory, text},
    catalog,
    config::{WorkflowPin, hex_digest},
    digest, safe_path,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Base {
    pub content: String,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct State {
    schema_version: u32,
    runtime_version: String,
    catalog_sha256: String,
    workflow: WorkflowPin,
    last_plan_sha256: String,
    pub managed: BTreeMap<String, Base>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Write {
    path: String,
    before: Option<String>,
    after: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema_version: u32,
    plan: Plan,
    committed: bool,
    operations: Vec<Write>,
}

/// Local setup only; hosted/release/provenance states require separate evidence.
#[derive(Debug, Serialize)]
pub struct Receipt {
    pub schema_version: u32,
    pub plan_sha256: String,
    pub outcome: &'static str,
    pub configured: bool,
    pub ci_verified: bool,
    pub release_rehearsed: bool,
    pub published: bool,
    pub provenance_verified: bool,
}

fn receipt(plan: &Plan, outcome: &'static str, configured: bool) -> Receipt {
    Receipt {
        schema_version: VERSION,
        plan_sha256: plan.plan_sha256.clone(),
        outcome,
        configured,
        ci_verified: false,
        release_rehearsed: false,
        published: false,
        provenance_verified: false,
    }
}

fn validate_state(state: &State) -> Result<()> {
    if state.schema_version != VERSION
        || state.runtime_version != env!("CARGO_PKG_VERSION")
        || state.catalog_sha256 != catalog::TOOLS_SHA256
        || state.workflow.repository != catalog::WORKFLOW_REPOSITORY
        || state.workflow.commit != catalog::WORKFLOW_COMMIT
        || !hex_digest(&state.last_plan_sha256, 64)
        || state.managed.len() > TARGETS.len()
        || state.managed.iter().any(|(path, base)| {
            !TARGETS.contains(&path.as_str()) || digest(base.content.as_bytes()) != base.sha256
        })
    {
        return Err(Error::Transaction(
            "incompatible bootstrap state or managed base; review migration explicitly",
        ));
    }
    Ok(())
}

pub(super) fn load_state(root: &Path) -> Result<Option<(State, Vec<u8>)>> {
    let Some(bytes) = optional_bytes(root, STATE)? else {
        return Ok(None);
    };
    let state = decode(&bytes)?;
    validate_state(&state)?;
    Ok(Some((state, bytes)))
}

/// Reconstruct the entire write inventory, including exact ownership state.
/// Recovery does not trust a journal's arbitrary paths, bytes or claimed state.
fn operations(plan: &Plan, state_bytes: Option<&str>) -> Result<Vec<Write>> {
    if state_bytes.map(|bytes| digest(bytes.as_bytes())) != plan.state_sha256 {
        return Err(Error::Transaction(
            "bootstrap ownership preimage differs from plan",
        ));
    }
    let state: Option<State> = state_bytes
        .map(|bytes| decode(bytes.as_bytes()))
        .transpose()?;
    if let Some(state) = &state {
        validate_state(state)?;
    }
    let mut managed = state.map(|state| state.managed).unwrap_or_default();
    let mut writes = Vec::new();
    for change in &plan.changes {
        let base = managed.get(&change.path);
        let legacy = change.path == "rust-toolchain.toml"
            && plan
                .preserved_inputs
                .get("rust-toolchain")
                .is_some_and(Option::is_some);
        if change.owned != base.is_some()
            || change.disposition
                != plan::disposition(
                    change.before_content.as_deref(),
                    &change.proposed_content,
                    base.map(|b| b.content.as_str()),
                    legacy,
                )
            || change.disposition == "conflict"
        {
            return Err(Error::Transaction(
                "bootstrap ownership or customization conflicts with approved change",
            ));
        }
        if change.disposition == "unchanged" {
            continue;
        }
        writes.push(Write {
            path: change.path.clone(),
            before: change.before_content.clone(),
            after: change.proposed_content.clone(),
        });
        managed.insert(
            change.path.clone(),
            Base {
                content: change.proposed_content.clone(),
                sha256: change.after_sha256.clone(),
            },
        );
    }
    let next = State {
        schema_version: VERSION,
        runtime_version: env!("CARGO_PKG_VERSION").into(),
        catalog_sha256: catalog::TOOLS_SHA256.into(),
        workflow: WorkflowPin {
            repository: catalog::WORKFLOW_REPOSITORY.into(),
            commit: catalog::WORKFLOW_COMMIT.into(),
        },
        last_plan_sha256: plan.plan_sha256.clone(),
        managed,
    };
    writes.push(Write {
        path: STATE.into(),
        before: state_bytes.map(str::to_owned),
        after: text(&json(&next)?)?,
    });
    Ok(writes)
}

fn guard_legacy(root: &Path) -> Result<()> {
    if optional_bytes(root, V1_STATE)?.is_some() {
        return Err(Error::Transaction(
            "version-one ownership requires an explicit reviewed migration",
        ));
    }
    if optional_bytes(root, V1_JOURNAL)?.is_some() {
        return Err(Error::Transaction(
            "unfinished version-one transaction requires version-one recovery",
        ));
    }
    Ok(())
}

/// Create only required parents, sync each creation, and recheck for symlinks.
/// Empty created directories are retained on rollback; unrelated directories are never removed.
fn ensure_parent(root: &Path, relative: &str) -> Result<()> {
    let parent = Path::new(relative)
        .parent()
        .ok_or(Error::Transaction("invalid bootstrap parent"))?;
    let mut current = std::path::PathBuf::new();
    for component in parent.components() {
        current.push(component);
        let relative = current
            .to_str()
            .ok_or(Error::Transaction("invalid bootstrap path"))?;
        let directory = safe_path(root, relative)?;
        match fs::create_dir(&directory) {
            Ok(()) => sync_directory(
                directory
                    .parent()
                    .ok_or(Error::Transaction("invalid directory parent"))?,
            )?,
            Err(error)
                if error.kind() == std::io::ErrorKind::AlreadyExists && directory.is_dir() => {}
            Err(error) => return Err(error.into()),
        }
    }
    safe_path(root, relative)?;
    Ok(())
}

fn matches(current: Option<&[u8]>, content: Option<&str>) -> bool {
    current == content.map(str::as_bytes)
}
fn remove_journal(root: &Path) -> Result<()> {
    fs::remove_file(safe_path(root, JOURNAL)?)?;
    sync_directory(&safe_path(root, ".armorer")?)
}

/// Check every target first; no partial rollback may erase an intervening edit.
fn rollback(root: &Path, journal: &Journal) -> Result<()> {
    for write in &journal.operations {
        let current = optional_bytes(root, &write.path)?;
        if !matches(current.as_deref(), write.before.as_deref())
            && !matches(current.as_deref(), Some(&write.after))
        {
            return Err(Error::Transaction(
                "bootstrap recovery conflicts with intervening edit; journal preserved",
            ));
        }
    }
    for write in journal.operations.iter().rev() {
        let current = optional_bytes(root, &write.path)?;
        if matches(current.as_deref(), write.before.as_deref()) {
            continue;
        }
        if let Some(before) = &write.before {
            replace(root, &write.path, before.as_bytes(), current.is_none())?;
        } else {
            let path = safe_path(root, &write.path)?;
            fs::remove_file(&path)?;
            sync_directory(
                path.parent()
                    .ok_or(Error::Transaction("invalid recovery parent"))?,
            )?;
        }
    }
    remove_journal(root)
}

fn replay_matches(root: &Path, plan: &Plan, state: &State) -> Result<bool> {
    for change in &plan.changes {
        let base = state.managed.get(&change.path);
        let required = change.owned || change.disposition != "unchanged";
        if base.is_some() != required
            || base.is_some_and(|base| {
                base.sha256 != change.after_sha256 || base.content != change.proposed_content
            })
            || !matches(
                optional_bytes(root, &change.path)?.as_deref(),
                Some(&change.proposed_content),
            )
        {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Apply fresh, independently approved fixed-target bytes under the shared kernel lock.
/// Matching committed replay does not rewrite files or ownership receipts.
pub fn apply(root: &Path, plan: &Plan, expected: &str) -> Result<Receipt> {
    transaction(
        root,
        plan,
        expected,
        |_| Ok(()),
        |root, bytes| replace(root, JOURNAL, bytes, false),
    )
}

fn transaction(
    root: &Path,
    plan: &Plan,
    expected: &str,
    mut after_write: impl FnMut(usize) -> Result<()>,
    commit_marker: impl FnOnce(&Path, &[u8]) -> Result<()>,
) -> Result<Receipt> {
    if !cfg!(unix) {
        return Err(Error::Transaction(
            "durable transactions require a supported Unix host",
        ));
    }
    plan::validate(plan, expected)?;
    if plan.recovery_required || plan.changes.iter().any(|c| c.disposition == "conflict") {
        return Err(Error::Transaction(
            "bootstrap conflict or unfinished transaction requires explicit resolution",
        ));
    }
    let root = root.canonicalize()?;
    guard_legacy(&root)?;
    let fresh = plan::inspect_bytes(&root, plan.policy_content.as_bytes())?;
    if !plan::same_intent(plan, &fresh)?
        || (fresh.digest()? != expected
            && load_state(&root)?.is_none_or(|(state, _)| state.last_plan_sha256 != expected))
    {
        return Err(Error::Transaction(
            "approved bootstrap inputs or preimages changed; review a new plan",
        ));
    }
    let _guard = lock(&root)?;
    guard_legacy(&root)?;
    if optional_bytes(&root, JOURNAL)?.is_some() {
        return Err(Error::Transaction(
            "unfinished bootstrap transaction requires explicit recovery",
        ));
    }
    let fresh = plan::inspect_bytes(&root, plan.policy_content.as_bytes())?;
    if !plan::same_intent(plan, &fresh)? {
        return Err(Error::Transaction(
            "bootstrap inputs changed while acquiring lock",
        ));
    }
    let state = load_state(&root)?;
    if let Some((state, _)) = &state
        && state.last_plan_sha256 == expected
    {
        if replay_matches(&root, plan, state)? {
            return Ok(receipt(plan, "unchanged", true));
        }
        return Err(Error::Transaction(
            "committed bootstrap receipt conflicts with files or ownership",
        ));
    }
    if fresh.digest()? != expected {
        return Err(Error::Transaction(
            "bootstrap preimages or ownership changed; review a new plan",
        ));
    }
    let state_text = state.as_ref().map(|(_, bytes)| text(bytes)).transpose()?;
    let mut journal = Journal {
        schema_version: VERSION,
        plan: plan.clone(),
        committed: false,
        operations: operations(plan, state_text.as_deref())?,
    };
    replace(&root, JOURNAL, &json(&journal)?, true)?;
    let result = (|| {
        for (index, write) in journal.operations.iter().enumerate() {
            if !matches(
                optional_bytes(&root, &write.path)?.as_deref(),
                write.before.as_deref(),
            ) {
                return Err(Error::Transaction(
                    "bootstrap target changed before atomic replacement",
                ));
            }
            ensure_parent(&root, &write.path)?;
            replace(
                &root,
                &write.path,
                write.after.as_bytes(),
                write.before.is_none(),
            )?;
            after_write(index)?;
        }
        let fresh = plan::inspect_bytes(&root, plan.policy_content.as_bytes())?;
        if !plan::same_intent(plan, &fresh)?
            || !replay_matches(
                &root,
                plan,
                &load_state(&root)?
                    .ok_or(Error::Transaction("bootstrap receipt missing"))?
                    .0,
            )?
        {
            return Err(Error::Transaction(
                "bootstrap inputs or inventory changed during transaction",
            ));
        }
        Ok(())
    })();
    if let Err(error) = result {
        if rollback(&root, &journal).is_err() {
            return Err(Error::Transaction(
                "bootstrap apply failed; explicit recovery required; journal preserved",
            ));
        }
        return Err(error);
    }
    journal.committed = true;
    if json(&journal)
        .and_then(|bytes| commit_marker(&root, &bytes))
        .is_err()
    {
        return Err(Error::Transaction(
            "bootstrap commit marker failed; explicit recovery required; journal preserved",
        ));
    }
    remove_journal(&root)?;
    Ok(receipt(plan, "applied", true))
}

/// Reconstruct all operations before allowing recovery; no custom path/content authority.
fn validate_journal(journal: &Journal, expected: &str) -> Result<()> {
    plan::validate(&journal.plan, expected)?;
    if journal.schema_version != VERSION
        || journal.plan.recovery_required
        || journal
            .plan
            .preserved_inputs
            .get(V1_STATE)
            .is_none_or(Option::is_some)
        || journal.operations.is_empty()
        || journal.operations.len() > TARGETS.len() + 1
        || journal
            .operations
            .last()
            .is_none_or(|write| write.path != STATE)
    {
        return Err(Error::Transaction("invalid bootstrap recovery journal"));
    }
    let before = journal
        .operations
        .last()
        .and_then(|write| write.before.as_deref());
    if journal.operations != operations(&journal.plan, before)? {
        return Err(Error::Transaction(
            "bootstrap recovery inventory differs from approved plan",
        ));
    }
    Ok(())
}

/// Roll back uncommitted exact bytes, or finalize a committed exact inventory.
/// Recovery can restore an expired-policy transaction; applying always checks current UTC.
pub fn recover(root: &Path, expected: &str) -> Result<Receipt> {
    if !cfg!(unix) {
        return Err(Error::Transaction(
            "durable transactions require a supported Unix host",
        ));
    }
    let root = root.canonicalize()?;
    guard_legacy(&root)?;
    let bytes = optional_bytes(&root, JOURNAL)?
        .ok_or(Error::Transaction("no bootstrap recovery journal"))?;
    let journal: Journal = decode(&bytes)?;
    validate_journal(&journal, expected)?;
    let _guard = lock(&root)?;
    guard_legacy(&root)?;
    if optional_bytes(&root, JOURNAL)?.as_deref() != Some(bytes.as_slice()) {
        return Err(Error::Transaction(
            "bootstrap journal changed while acquiring lock",
        ));
    }
    if journal.committed {
        for write in &journal.operations {
            if !matches(
                optional_bytes(&root, &write.path)?.as_deref(),
                Some(&write.after),
            ) {
                return Err(Error::Transaction(
                    "committed bootstrap inventory conflicts with intervening edit",
                ));
            }
        }
        for change in &journal.plan.changes {
            if !matches(
                optional_bytes(&root, &change.path)?.as_deref(),
                Some(&change.proposed_content),
            ) {
                return Err(Error::Transaction(
                    "committed bootstrap target conflicts with intervening edit",
                ));
            }
        }
        remove_journal(&root)?;
        Ok(receipt(&journal.plan, "commit-recovered", true))
    } else {
        rollback(&root, &journal)?;
        Ok(receipt(&journal.plan, "rolled-back", false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const POLICY: &str = "schema_version = 1\n[licenses]\nallow = [\"MIT\"]\n[advisories]\nexceptions = []\n[sources]\nallow_git = []\n[bans]\nmultiple_versions = \"warn\"\ndeny = []\n";
    fn fixture() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("src")).unwrap();
        fs::write(
            root.path().join("src/lib.rs"),
            "compile_error!(\"no builds\");",
        )
        .unwrap();
        fs::write(root.path().join("LICENSE"), "MIT").unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname = \"pilot\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        fs::write(root.path().join("armorer.toml"), "schema_version = 1\nrepository = \"example/pilot\"\ntoolchain = \"1.95.0\"\n[[deliverables]]\nid = \"library\"\nprofile = \"library\"\npackage = \"pilot\"\ntargets = [\"x86_64-unknown-linux-gnu\"]\nfeature_set = \"standard\"\n[feature_sets.standard]\ndefault_features = true\nfeatures = []\n[policy]\nlicense_file = \"LICENSE\"\nattestations = \"required\"\n").unwrap();
        root
    }
    fn inspect(root: &Path) -> Plan {
        plan::inspect_bytes(root, POLICY.as_bytes()).unwrap()
    }
    fn interrupt(root: &Path, plan: &Plan, index: usize) {
        assert!(
            std::panic::catch_unwind(|| {
                let _ = transaction(
                    root,
                    plan,
                    &plan.plan_sha256,
                    |written| {
                        if written == index {
                            panic!("simulated process interruption");
                        }
                        Ok(())
                    },
                    |root, bytes| replace(root, JOURNAL, bytes, false),
                );
            })
            .is_err()
        );
    }
    fn absent(root: &Path) {
        for path in TARGETS.into_iter().chain([STATE, JOURNAL]) {
            assert!(!root.join(path).exists(), "{path}");
        }
    }
    #[test]
    fn write_failures_restore_all_bytes_at_every_boundary_and_allow_retry() {
        for index in 0..=TARGETS.len() {
            let root = fixture();
            let plan = inspect(root.path());
            assert!(
                transaction(
                    root.path(),
                    &plan,
                    &plan.plan_sha256,
                    |written| {
                        if written == index {
                            return Err(Error::Transaction("injected I/O failure"));
                        }
                        Ok(())
                    },
                    |root, bytes| replace(root, JOURNAL, bytes, false)
                )
                .is_err()
            );
            absent(root.path());
            assert_eq!(
                apply(root.path(), &plan, &plan.plan_sha256)
                    .unwrap()
                    .outcome,
                "applied"
            );
        }
    }
    #[test]
    fn crash_recovery_restores_every_boundary_and_blocks_wrong_digest() {
        for index in 0..=TARGETS.len() {
            let root = fixture();
            let plan = inspect(root.path());
            interrupt(root.path(), &plan, index);
            assert!(apply(root.path(), &plan, &plan.plan_sha256).is_err());
            assert!(recover(root.path(), &"a".repeat(64)).is_err());
            assert_eq!(
                recover(root.path(), &plan.plan_sha256).unwrap().outcome,
                "rolled-back"
            );
            absent(root.path());
            assert_eq!(
                apply(root.path(), &plan, &plan.plan_sha256)
                    .unwrap()
                    .outcome,
                "applied"
            );
        }
    }
    #[test]
    fn commit_marker_failures_before_and_after_persist_are_recoverable() {
        for persisted in [false, true] {
            let root = fixture();
            let plan = inspect(root.path());
            assert!(
                transaction(
                    root.path(),
                    &plan,
                    &plan.plan_sha256,
                    |_| Ok(()),
                    |root, bytes| {
                        if persisted {
                            replace(root, JOURNAL, bytes, false)?;
                        }
                        Err(Error::Transaction("commit marker sync failed"))
                    }
                )
                .unwrap_err()
                .to_string()
                .contains("recovery required")
            );
            for change in &plan.changes {
                assert_eq!(
                    fs::read(root.path().join(&change.path)).unwrap(),
                    change.proposed_content.as_bytes()
                );
            }
            let journal: Journal = decode(&fs::read(root.path().join(JOURNAL)).unwrap()).unwrap();
            assert_eq!(journal.committed, persisted);
            let recovered = recover(root.path(), &plan.plan_sha256).unwrap();
            assert_eq!(recovered.configured, persisted);
            assert_eq!(
                recovered.outcome,
                if persisted {
                    "commit-recovered"
                } else {
                    "rolled-back"
                }
            );
            assert_eq!(
                apply(root.path(), &plan, &plan.plan_sha256)
                    .unwrap()
                    .outcome,
                if persisted { "unchanged" } else { "applied" }
            );
        }
    }
    #[test]
    fn intervening_edits_block_rollback_before_any_file_is_restored() {
        for path in TARGETS.into_iter().chain([STATE]) {
            let root = fixture();
            let plan = inspect(root.path());
            interrupt(root.path(), &plan, TARGETS.len());
            fs::write(root.path().join(path), "external edit").unwrap();
            let snapshot: Vec<_> = TARGETS
                .into_iter()
                .chain([STATE, JOURNAL])
                .map(|path| (path, fs::read(root.path().join(path)).unwrap()))
                .collect();
            assert!(recover(root.path(), &plan.plan_sha256).is_err(), "{path}");
            for (path, bytes) in snapshot {
                assert_eq!(fs::read(root.path().join(path)).unwrap(), bytes);
            }
        }
    }
    #[test]
    fn edited_file_on_apply_failure_preserves_journal_for_explicit_recovery() {
        let root = fixture();
        let plan = inspect(root.path());
        let error = transaction(
            root.path(),
            &plan,
            &plan.plan_sha256,
            |_| {
                fs::write(root.path().join(TARGETS[0]), "external policy edit")?;
                Err(Error::Transaction("injected failure"))
            },
            |root, bytes| replace(root, JOURNAL, bytes, false),
        )
        .unwrap_err();
        assert!(error.to_string().contains("recovery required"));
        assert!(root.path().join(JOURNAL).exists());
        assert_eq!(
            fs::read(root.path().join(TARGETS[0])).unwrap(),
            b"external policy edit"
        );
    }
    #[test]
    fn source_inputs_changed_during_apply_roll_back_before_commit() {
        let root = fixture();
        let plan = inspect(root.path());
        assert!(
            transaction(
                root.path(),
                &plan,
                &plan.plan_sha256,
                |index| {
                    if index == TARGETS.len() {
                        fs::write(root.path().join("Cargo.lock"), "version = 4\n")?;
                    }
                    Ok(())
                },
                |root, bytes| replace(root, JOURNAL, bytes, false)
            )
            .is_err()
        );
        absent(root.path());
        assert_eq!(
            fs::read(root.path().join("Cargo.lock")).unwrap(),
            b"version = 4\n"
        );
    }
    #[test]
    fn recovery_rejects_redirection_missing_extra_reordered_or_forged_state() {
        for mutation in 0..7 {
            let root = fixture();
            let plan = inspect(root.path());
            interrupt(root.path(), &plan, 0);
            let mut journal: Journal =
                decode(&fs::read(root.path().join(JOURNAL)).unwrap()).unwrap();
            match mutation {
                0 => journal.operations[0].path = "LICENSE".into(),
                1 => {
                    journal.operations.remove(1);
                }
                2 => journal.operations.insert(0, journal.operations[0].clone()),
                3 => journal.operations.swap(0, 1),
                4 => journal.operations.last_mut().unwrap().after = "{}".into(),
                5 => journal.operations[0].before = Some("forged preimage".into()),
                6 => journal.operations[0].after = "forged policy".into(),
                _ => unreachable!(),
            }
            let bytes = json(&journal).unwrap();
            fs::write(root.path().join(JOURNAL), &bytes).unwrap();
            assert!(
                recover(root.path(), &plan.plan_sha256).is_err(),
                "mutation {mutation}"
            );
            assert_eq!(fs::read(root.path().join("LICENSE")).unwrap(), b"MIT");
            assert_eq!(fs::read(root.path().join(JOURNAL)).unwrap(), bytes);
            assert_eq!(
                fs::read(root.path().join(TARGETS[0])).unwrap(),
                POLICY.as_bytes()
            );
        }
    }
    #[test]
    fn owned_policy_update_preserves_mode_and_rolls_back_prior_receipt() {
        let root = fixture();
        let first = inspect(root.path());
        apply(root.path(), &first, &first.plan_sha256).unwrap();
        let old_state = fs::read(root.path().join(STATE)).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                root.path().join(TARGETS[0]),
                fs::Permissions::from_mode(0o640),
            )
            .unwrap();
        }
        let policy = POLICY.replace("[\"MIT\"]", "[\"MIT\", \"Apache-2.0\"]");
        let next = plan::inspect_bytes(root.path(), policy.as_bytes()).unwrap();
        assert_eq!(next.changes[0].disposition, "update");
        for index in 0..2 {
            assert!(
                transaction(
                    root.path(),
                    &next,
                    &next.plan_sha256,
                    |written| {
                        if index == written {
                            return Err(Error::Transaction("update failed"));
                        }
                        Ok(())
                    },
                    |root, bytes| replace(root, JOURNAL, bytes, false)
                )
                .is_err()
            );
            assert_eq!(
                fs::read(root.path().join(TARGETS[0])).unwrap(),
                POLICY.as_bytes()
            );
            assert_eq!(fs::read(root.path().join(STATE)).unwrap(), old_state);
        }
        apply(root.path(), &next, &next.plan_sha256).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(root.path().join(TARGETS[0]))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o640
            );
        }
    }
    #[test]
    fn committed_recovery_also_rechecks_unchanged_unowned_targets() {
        let root = fixture();
        let initial = inspect(root.path());
        let toolchain = initial.changes.last().unwrap();
        fs::write(
            root.path().join(&toolchain.path),
            &toolchain.proposed_content,
        )
        .unwrap();
        let plan = inspect(root.path());
        interrupt(root.path(), &plan, TARGETS.len() - 1);
        let mut journal: Journal = decode(&fs::read(root.path().join(JOURNAL)).unwrap()).unwrap();
        journal.committed = true;
        fs::write(root.path().join(JOURNAL), json(&journal).unwrap()).unwrap();
        fs::write(root.path().join(&toolchain.path), "user customization").unwrap();
        assert!(recover(root.path(), &plan.plan_sha256).is_err());
        assert!(root.path().join(JOURNAL).exists());
    }
}
