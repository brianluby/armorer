//! Explicit upgrade transactions; frozen v1/v2 plans retain their write inventories.
use super::{
    JOURNAL, Plan, STATE,
    merge::Decision,
    plan,
    rollback::{self, RollbackPlan},
    state,
};
use crate::{
    Error, Result,
    apply::{decode, json, lock, optional_bytes, replace, sync_directory},
    bootstrap, ci_policy,
    config::load_config,
    digest,
    discovery::discover,
    safe_path,
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Write {
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
enum Approval {
    Upgrade { plan: Plan },
    Rollback { plan: RollbackPlan },
}
impl Approval {
    /// Return the approved forward plan represented by this transaction direction.
    fn original(&self) -> &Plan {
        match self {
            Self::Upgrade { plan } => plan,
            Self::Rollback { plan } => &plan.original_upgrade,
        }
    }
    /// Hash the canonical approved record bytes for independent digest comparison.
    fn digest(&self) -> &str {
        match self {
            Self::Upgrade { plan } => &plan.plan_sha256,
            Self::Rollback { plan } => &plan.plan_sha256,
        }
    }
    /// Reconstruct the approved record semantics and byte identities before accepting it.
    fn validate(&self, expected: &str) -> Result<()> {
        match self {
            Self::Upgrade { plan } => {
                plan::validate(plan, expected)?;
                eligible(plan)
            }
            Self::Rollback { plan } => rollback::validate(plan, expected),
        }
    }
    /// Return the direction-specific fixed operations from the approved packet.
    fn operations(&self) -> Result<Vec<Write>> {
        match self {
            Self::Upgrade { plan } => operations(plan),
            Self::Rollback { plan } => reverse_operations(&plan.original_upgrade),
        }
    }
    /// Read the exact completed state required by the selected transaction direction.
    fn complete(&self, root: &Path) -> Result<bool> {
        match self {
            Self::Upgrade { plan } => after_images(root, plan),
            Self::Rollback { plan } => before_images(root, &plan.original_upgrade),
        }
    }
    /// Validate the source and ownership context required by this transaction.
    fn context(&self, root: &Path) -> Result<()> {
        context(root, self.original(), matches!(self, Self::Upgrade { .. }))
    }
    /// Read the approved initial bytes and ownership before staging a transaction.
    fn initial(&self, root: &Path) -> Result<()> {
        match self {
            Self::Upgrade { plan } => {
                if !before_owner(root, plan)? {
                    return Err(Error::Transaction(
                        "upgrade ownership changed; review a new plan",
                    ));
                }
                plan::validate_fresh(root, plan)
            }
            Self::Rollback { plan } => {
                if !after_images(root, &plan.original_upgrade)? {
                    return Err(Error::Transaction(
                        "rollback requires exact final upgrade images and ownership",
                    ));
                }
                Ok(())
            }
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema_version: u32,
    approval: Approval,
    committed: bool,
    operations: Vec<Write>,
}
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
/// Report the completed local transaction while keeping CI, release and provenance gates separate.
fn receipt(p: &Approval, outcome: &'static str, configured: bool) -> Receipt {
    Receipt {
        schema_version: 1,
        plan_sha256: p.digest().into(),
        outcome,
        configured,
        ci_verified: false,
        release_rehearsed: false,
        published: false,
        provenance_verified: false,
    }
}
/// Compare the current optional file bytes with the approved image without adopting a mismatch.
fn matches(bytes: Option<&[u8]>, text: Option<&str>) -> bool {
    bytes == text.map(str::as_bytes)
}

/// Reconstruct fixed managed writes, exact former-owner deletion and new receipt.
fn operations(p: &Plan) -> Result<Vec<Write>> {
    let mut writes = Vec::new();
    for c in &p.changes {
        if c.decision == Decision::Conflict {
            return Err(Error::Transaction("unresolved three-way conflict"));
        }
        let after = c
            .proposed_content
            .clone()
            .ok_or(Error::Transaction("upgrade after image missing"))?;
        if c.before_content.as_deref() != Some(&after) {
            writes.push(Write {
                path: c.path.clone(),
                before: c.before_content.clone(),
                after: Some(after),
            });
        }
    }
    if let Some(path) = state::prior_path(&p.source_format) {
        let before = p
            .state_preimages
            .get(path)
            .cloned()
            .flatten()
            .ok_or(Error::Transaction("former ownership preimage missing"))?;
        writes.push(Write {
            path: path.into(),
            before: Some(before),
            after: None,
        });
    }
    writes.push(Write {
        path: STATE.into(),
        before: p.state_preimages.get(STATE).cloned().flatten(),
        after: Some(state::next_text(p)?),
    });
    Ok(writes)
}

/// Exact reverse inventory includes ownership deletion/restoration, never configuration.
pub(super) fn reverse_operations(p: &Plan) -> Result<Vec<Write>> {
    Ok(operations(p)?
        .into_iter()
        .rev()
        .map(|w| Write {
            path: w.path,
            before: w.after,
            after: w.before,
        })
        .collect())
}
/// Check whether each managed slot still matches an approved transaction image.
pub(super) fn eligible(p: &Plan) -> Result<()> {
    if p.recovery_required
        || p.compatibility.iter().any(|c| c.state == "blocked")
        || p.changes.iter().any(|c| c.decision == Decision::Conflict)
    {
        return Err(Error::Transaction(
            "upgrade compatibility or merge conflict requires review",
        ));
    }
    Ok(())
}
/// Capture the exact prior file and ownership images for bounded recovery.
fn before_images(root: &Path, p: &Plan) -> Result<bool> {
    if !before_owner(root, p)? {
        return Ok(false);
    }
    for c in &p.changes {
        if !matches(
            optional_bytes(root, &c.path)?.as_deref(),
            c.before_content.as_deref(),
        ) {
            return Ok(false);
        }
    }
    Ok(true)
}
/// Revalidate every recovery preimage before any prior byte is restored.
pub(super) fn rollback_fresh(root: &Path, p: &Plan) -> Result<()> {
    eligible(p)?;
    let root = root.canonicalize()?;
    guard(&root, false)?;
    context(&root, p, false)?;
    if !after_images(&root, p)? {
        return Err(Error::Transaction(
            "rollback requires exact final upgrade images and ownership",
        ));
    }
    Ok(())
}

/// Never mix unfinished journal versions. This gate reads only bounded regular files.
fn guard(root: &Path, own: bool) -> Result<()> {
    for path in [bootstrap::V1_JOURNAL, bootstrap::JOURNAL] {
        if optional_bytes(root, path)?.is_some() {
            return Err(Error::Transaction(
                "earlier transaction requires its explicit recovery command",
            ));
        }
    }
    if !own && optional_bytes(root, JOURNAL)?.is_some() {
        return Err(Error::Transaction(
            "unfinished upgrade requires explicit recovery",
        ));
    }
    Ok(())
}

/// Recheck independent config/discovery/preimages and current UTC policy. Runtime
/// state/managed file outputs are checked separately, allowing exact committed replay.
fn context(root: &Path, p: &Plan, validate_policy: bool) -> Result<()> {
    let (intent, bytes) = load_config(root)?;
    if digest(&bytes) != p.config_sha256
        || serde_json::to_value(&intent).map_err(|_| Error::Json)?
            != serde_json::to_value(&p.intent).map_err(|_| Error::Json)?
    {
        return Err(Error::Transaction("approved upgrade configuration changed"));
    }
    if serde_json::to_value(discover(root, &intent)?).map_err(|_| Error::Json)?
        != serde_json::to_value(&p.workspace).map_err(|_| Error::Json)?
    {
        return Err(Error::Transaction(
            "approved upgrade Cargo inputs or target paths changed",
        ));
    }
    for (path, hash) in &p.input_preimages {
        if optional_bytes(root, path)?.as_deref().map(digest) != *hash {
            return Err(Error::Transaction("upgrade input preimage changed"));
        }
    }
    if !validate_policy {
        return Ok(());
    }
    let today = ci_policy::today()?;
    ci_policy::parse_at(p.policy_content.as_bytes(), today)?;
    let policy = p
        .changes
        .iter()
        .find(|c| c.path == ".armorer/ci-policy.toml")
        .and_then(|c| c.proposed_content.as_ref())
        .ok_or(Error::Transaction("upgrade effective policy missing"))?;
    ci_policy::parse_at(policy.as_bytes(), today)?;
    Ok(())
}

/// Reconstruct the previous versioned ownership receipt from the approved packet.
fn before_owner(root: &Path, p: &Plan) -> Result<bool> {
    for (path, before) in &p.state_preimages {
        if !matches(optional_bytes(root, path)?.as_deref(), before.as_deref()) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Exact final ownership and every after byte are required; identical unowned
/// inputs remain absent from the reconstructed managed-base inventory.
fn after_images(root: &Path, p: &Plan) -> Result<bool> {
    if !matches(
        optional_bytes(root, STATE)?.as_deref(),
        Some(&state::next_text(p)?),
    ) {
        return Ok(false);
    }
    for path in [bootstrap::STATE, bootstrap::V1_STATE] {
        if optional_bytes(root, path)?.is_some() {
            return Ok(false);
        }
    }
    for c in &p.changes {
        if !matches(
            optional_bytes(root, &c.path)?.as_deref(),
            c.proposed_content.as_deref(),
        ) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Persist one approved operation with its original mode and durable journal progress.
fn write(root: &Path, w: &Write, after: bool) -> Result<()> {
    let image = if after { &w.after } else { &w.before };
    let current = optional_bytes(root, &w.path)?;
    match image {
        Some(image) => {
            bootstrap::transaction::ensure_parent(root, &w.path)?;
            replace(root, &w.path, image.as_bytes(), current.is_none())
        }
        None => {
            if current.is_some() {
                let path = safe_path(root, &w.path)?;
                fs::remove_file(&path)?;
                sync_directory(
                    path.parent()
                        .ok_or(Error::Transaction("invalid migration parent"))?,
                )?;
            }
            Ok(())
        }
    }
}
/// Remove the completed journal and sync its parent after the caller has verified recovery state.
fn remove_journal(root: &Path, expected: &[u8]) -> Result<()> {
    if optional_bytes(root, JOURNAL)?.as_deref() != Some(expected) {
        return Err(Error::Transaction(
            "upgrade journal changed; preserved for explicit recovery",
        ));
    }
    fs::remove_file(safe_path(root, JOURNAL)?)?;
    sync_directory(&safe_path(root, ".armorer")?)
}

/// Validate every before/after possibility before restoring any file. Neither
/// unknown paths nor intervening edits may be overwritten during rollback.
fn rollback(root: &Path, j: &Journal, bytes: &[u8]) -> Result<()> {
    // Include unchanged targets and every supported ownership slot, so a
    // cooperative intervening edit blocks the whole restoration before writes.
    let original = j.approval.original();
    for c in &original.changes {
        let current = optional_bytes(root, &c.path)?;
        if !matches(current.as_deref(), c.before_content.as_deref())
            && !matches(current.as_deref(), c.proposed_content.as_deref())
        {
            return Err(Error::Transaction(
                "upgrade recovery conflicts with intervening edit; journal preserved",
            ));
        }
    }
    let next = state::next_text(original)?;
    for (path, before) in &original.state_preimages {
        let after = if path == STATE {
            Some(next.as_str())
        } else {
            None
        };
        let current = optional_bytes(root, path)?;
        if !matches(current.as_deref(), before.as_deref()) && !matches(current.as_deref(), after) {
            return Err(Error::Transaction(
                "upgrade recovery conflicts with intervening ownership; journal preserved",
            ));
        }
    }
    if optional_bytes(root, JOURNAL)?.as_deref() != Some(bytes) {
        return Err(Error::Transaction(
            "upgrade journal changed before recovery",
        ));
    }
    for w in j.operations.iter().rev() {
        if !matches(
            optional_bytes(root, &w.path)?.as_deref(),
            w.before.as_deref(),
        ) {
            write(root, w, false)?;
        }
    }
    remove_journal(root, bytes)
}

/// Apply only the separately reviewed upgrade contract; no arbitrary target,
/// template, shell, network, publication or credential operation is supported.
pub fn apply(root: &Path, p: &Plan, expected: &str) -> Result<Receipt> {
    transaction(
        root,
        &Approval::Upgrade { plan: p.clone() },
        expected,
        |_| Ok(()),
        |root, bytes| replace(root, JOURNAL, bytes, false),
    )
}
/// Apply a separately reviewed reverse packet; restoration is never release readiness.
pub fn apply_rollback(root: &Path, p: &RollbackPlan, expected: &str) -> Result<Receipt> {
    transaction(
        root,
        &Approval::Rollback { plan: p.clone() },
        expected,
        |_| Ok(()),
        |root, bytes| replace(root, JOURNAL, bytes, false),
    )
}
/// Execute fixed approved writes, ownership transitions and commit checks under the shared lock.
fn transaction(
    root: &Path,
    p: &Approval,
    expected: &str,
    mut after_write: impl FnMut(usize) -> Result<()>,
    commit_marker: impl FnOnce(&Path, &[u8]) -> Result<()>,
) -> Result<Receipt> {
    if !cfg!(unix) {
        return Err(Error::Transaction(
            "durable upgrades require supported Unix host",
        ));
    }
    p.validate(expected)?;
    let root = root.canonicalize()?;
    guard(&root, false)?;
    p.context(&root)?;
    if !p.complete(&root)? {
        p.initial(&root)?;
    }
    let _guard = lock(&root)?;
    guard(&root, false)?;
    p.context(&root)?;
    if p.complete(&root)? {
        return Ok(receipt(
            p,
            "unchanged",
            matches!(p, Approval::Upgrade { .. }),
        ));
    }
    p.initial(&root)?;
    let mut j = Journal {
        schema_version: 1,
        approval: p.clone(),
        committed: false,
        operations: p.operations()?,
    };
    let initial = json(&j)?;
    replace(&root, JOURNAL, &initial, true)?;
    let result = (|| {
        for (index, w) in j.operations.iter().enumerate() {
            if !matches(
                optional_bytes(&root, &w.path)?.as_deref(),
                w.before.as_deref(),
            ) {
                return Err(Error::Transaction(
                    "upgrade target changed before replacement",
                ));
            }
            write(&root, w, true)?;
            after_write(index)?;
        }
        p.context(&root)?;
        if !p.complete(&root)? {
            return Err(Error::Transaction(
                "upgrade final inventory differs from approved bytes",
            ));
        }
        Ok(())
    })();
    if let Err(error) = result {
        if let Err(rollback) = rollback(&root, &j, &initial) {
            return Err(Error::TransactionRollback {
                message: "upgrade failed; explicit recovery required; journal preserved",
                cause: Box::new(error),
                rollback: Box::new(rollback),
            });
        }
        return Err(error);
    }
    j.committed = true;
    let committed = json(&j)?;
    if let Err(cause) = commit_marker(&root, &committed) {
        return Err(Error::TransactionCause {
            message: "upgrade commit marker failed; recovery required; final bytes and journal preserved",
            cause: Box::new(cause),
        });
    }
    remove_journal(&root, &committed)?;
    Ok(receipt(
        p,
        if matches!(p, Approval::Upgrade { .. }) {
            "applied"
        } else {
            "rolled-back"
        },
        matches!(p, Approval::Upgrade { .. }),
    ))
}

/// Reconstruct the complete fixed operation inventory before recovery. Historical
/// policy expiry cannot block restoring an already approved transaction's bytes.
fn validate_journal(j: &Journal, expected: &str) -> Result<()> {
    j.approval.validate(expected)?;
    if j.schema_version != 1
        || j.operations.is_empty()
        || j.operations.len() > bootstrap::TARGETS.len() + 2
        || j.operations != j.approval.operations()?
    {
        return Err(Error::Transaction(
            "upgrade recovery inventory differs from approved packet",
        ));
    }
    Ok(())
}
/// Recover only a matching journal whose complete inventory and current images remain valid.
pub fn recover(root: &Path, expected: &str) -> Result<Receipt> {
    if !cfg!(unix) {
        return Err(Error::Transaction(
            "durable upgrades require supported Unix host",
        ));
    }
    let root = root.canonicalize()?;
    guard(&root, true)?;
    let bytes =
        optional_bytes(&root, JOURNAL)?.ok_or(Error::Transaction("no upgrade recovery journal"))?;
    let j: Journal = decode(&bytes)?;
    validate_journal(&j, expected)?;
    let _guard = lock(&root)?;
    guard(&root, true)?;
    if optional_bytes(&root, JOURNAL)?.as_deref() != Some(bytes.as_slice()) {
        return Err(Error::Transaction(
            "upgrade journal changed while acquiring lock",
        ));
    }
    if j.committed {
        if !j.approval.complete(&root)? {
            return Err(Error::Transaction(
                "committed upgrade conflicts with intervening edit",
            ));
        }
        remove_journal(&root, &bytes)?;
        Ok(receipt(
            &j.approval,
            "commit-recovered",
            matches!(j.approval, Approval::Upgrade { .. })
                && context(&root, j.approval.original(), true).is_ok(),
        ))
    } else {
        rollback(&root, &j, &bytes)?;
        Ok(receipt(
            &j.approval,
            if matches!(j.approval, Approval::Upgrade { .. }) {
                "rolled-back"
            } else {
                "rollback-reverted"
            },
            false,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    const POLICY: &str = "schema_version = 1\n[licenses]\nallow = [\"MIT\"]\n[advisories]\nexceptions = []\n[sources]\nallow_git = []\n[bans]\nmultiple_versions = \"warn\"\ndeny = []\n";
    /// Create an isolated repository fixture with explicit inputs for the surrounding transaction tests.
    fn fixture(kind: &str) -> (tempfile::TempDir, Plan) {
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
            "[package]\nname=\"pilot\"\nversion=\"0.1.0\"\nedition=\"2024\"\n",
        )
        .unwrap();
        fs::write(root.path().join("armorer.toml"),"schema_version = 1\nrepository = \"example/pilot\"\ntoolchain = \"1.95.0\"\n[[deliverables]]\nid = \"library\"\nprofile = \"library\"\npackage = \"pilot\"\ntargets = [\"x86_64-unknown-linux-gnu\"]\nfeature_set = \"standard\"\n[feature_sets.standard]\ndefault_features = true\nfeatures = []\n[policy]\nlicense_file = \"LICENSE\"\nattestations = \"required\"\n").unwrap();
        match kind {
            "legacy" => {
                let p = crate::plan::inspect(root.path(), "plan").unwrap();
                crate::apply::apply(root.path(), &p, &p.plan_sha256).unwrap();
            }
            "bootstrap" => {
                let policy = tempfile::NamedTempFile::new().unwrap();
                fs::write(policy.path(), POLICY).unwrap();
                let p = bootstrap::inspect(root.path(), policy.path()).unwrap();
                bootstrap::apply(root.path(), &p, &p.plan_sha256).unwrap();
            }
            "upgrade" => {
                let p = preview(root.path());
                apply(root.path(), &p, &p.plan_sha256).unwrap();
            }
            "unmanaged" => {}
            _ => unreachable!(),
        }
        if kind != "unmanaged" {
            let file = root.path().join("armorer.toml");
            fs::write(
                &file,
                format!(
                    "{}\n# approved configuration upgrade\n",
                    fs::read_to_string(&file).unwrap()
                ),
            )
            .unwrap();
        }
        let p = preview(root.path());
        (root, p)
    }
    /// Derive a fresh fixture plan from its explicit policy and independently selected catalog.
    fn preview(root: &Path) -> Plan {
        plan::inspect_bytes(
            root,
            POLICY.as_bytes(),
            super::super::authority::select("bootstrap-v1").unwrap(),
            &[],
            false,
        )
        .unwrap()
    }
    /// Capture every regular fixture file as exact bytes for mutation and recovery comparisons.
    fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
        /// Recursively record relative fixture paths and bytes without running repository code.
        fn walk(root: &Path, dir: &Path, map: &mut BTreeMap<String, Vec<u8>>) {
            for entry in fs::read_dir(dir).unwrap() {
                let p = entry.unwrap().path();
                if p.is_dir() {
                    walk(root, &p, map)
                } else {
                    let relative = p.strip_prefix(root).unwrap().to_str().unwrap();
                    if relative != ".armorer/apply.lock" {
                        map.insert(relative.into(), fs::read(p).unwrap());
                    }
                }
            }
        }
        let mut map = BTreeMap::new();
        walk(root, root, &mut map);
        map
    }
    /// Persist the fixture packet and return its separately supplied expected digest.
    fn approval(root: &Path, p: Plan, reverse: bool) -> Approval {
        if reverse {
            apply(root, &p, &p.plan_sha256).unwrap();
            let r = rollback::inspect_rollback(root, &p, &p.plan_sha256, true).unwrap();
            Approval::Rollback { plan: r }
        } else {
            Approval::Upgrade { plan: p }
        }
    }
    /// Terminate the fixture child at the selected durable transaction boundary.
    fn interrupt(root: &Path, p: &Approval, index: usize) {
        assert!(
            std::panic::catch_unwind(|| {
                let _ = transaction(
                    root,
                    p,
                    p.digest(),
                    |written| {
                        if index == written {
                            panic!("simulated interruption");
                        }
                        Ok(())
                    },
                    |root, bytes| replace(root, JOURNAL, bytes, false),
                );
            })
            .is_err()
        );
    }
    /// Verify that io failures restore every write and owner deletion boundary in both directions.
    #[test]
    fn io_failures_restore_every_write_and_owner_deletion_boundary_in_both_directions() {
        for kind in ["unmanaged", "legacy", "bootstrap", "upgrade"] {
            for reverse in [false, true] {
                let (probe, plan) = fixture(kind);
                let approved = approval(probe.path(), plan, reverse);
                let count = approved.operations().unwrap().len();
                for index in 0..count {
                    let (root, plan) = fixture(kind);
                    let approved = approval(root.path(), plan, reverse);
                    let before = snapshot(root.path());
                    assert!(
                        transaction(
                            root.path(),
                            &approved,
                            approved.digest(),
                            |written| {
                                if written == index {
                                    return Err(Error::Transaction("injected write failure"));
                                }
                                Ok(())
                            },
                            |root, bytes| replace(root, JOURNAL, bytes, false)
                        )
                        .is_err(),
                        "{kind} {reverse} {index}"
                    );
                    assert_eq!(snapshot(root.path()), before, "{kind} {reverse} {index}");
                    transaction(
                        root.path(),
                        &approved,
                        approved.digest(),
                        |_| Ok(()),
                        |root, bytes| replace(root, JOURNAL, bytes, false),
                    )
                    .unwrap();
                    assert!(approved.complete(root.path()).unwrap());
                }
            }
        }
    }
    /// Verify that process interruption recovery restores prior owner and rejects wrong digest for each boundary.
    #[test]
    fn process_interruption_recovery_restores_prior_owner_and_rejects_wrong_digest_for_each_boundary()
     {
        for kind in ["unmanaged", "legacy", "bootstrap", "upgrade"] {
            for reverse in [false, true] {
                let (probe, plan) = fixture(kind);
                let approved = approval(probe.path(), plan, reverse);
                let count = approved.operations().unwrap().len();
                for index in 0..count {
                    let (root, plan) = fixture(kind);
                    let approved = approval(root.path(), plan, reverse);
                    let before = snapshot(root.path());
                    interrupt(root.path(), &approved, index);
                    let partial = snapshot(root.path());
                    assert!(recover(root.path(), &"a".repeat(64)).is_err());
                    assert!(
                        transaction(
                            root.path(),
                            &approved,
                            approved.digest(),
                            |_| Ok(()),
                            |root, bytes| replace(root, JOURNAL, bytes, false)
                        )
                        .is_err()
                    );
                    assert_eq!(snapshot(root.path()), partial);
                    let r = recover(root.path(), approved.digest()).unwrap();
                    assert!(!r.configured);
                    assert_eq!(
                        r.outcome,
                        if reverse {
                            "rollback-reverted"
                        } else {
                            "rolled-back"
                        }
                    );
                    assert_eq!(snapshot(root.path()), before, "{kind} {reverse} {index}");
                }
            }
        }
    }
    /// Verify that commit marker uncertainty preserves final bytes until explicit recovery in both directions.
    #[test]
    fn commit_marker_uncertainty_preserves_final_bytes_until_explicit_recovery_in_both_directions()
    {
        for reverse in [false, true] {
            for persisted in [false, true] {
                let (root, p) = fixture("legacy");
                let approved = approval(root.path(), p, reverse);
                let before = snapshot(root.path());
                assert!(
                    transaction(
                        root.path(),
                        &approved,
                        approved.digest(),
                        |_| Ok(()),
                        |root, bytes| {
                            if persisted {
                                replace(root, JOURNAL, bytes, false)?;
                            }
                            Err(Error::Transaction("sync failed"))
                        }
                    )
                    .unwrap_err()
                    .to_string()
                    .contains("recovery required")
                );
                assert!(approved.complete(root.path()).unwrap());
                let mut after = snapshot(root.path());
                after.remove(JOURNAL);
                let journal: Journal =
                    decode(&fs::read(root.path().join(JOURNAL)).unwrap()).unwrap();
                assert_eq!(journal.committed, persisted);
                let receipt = recover(root.path(), approved.digest()).unwrap();
                assert_eq!(receipt.configured, persisted && !reverse);
                assert_eq!(
                    snapshot(root.path()),
                    if persisted { after } else { before }
                );
            }
        }
    }
    /// Verify that intervening file owner or journal edits block recovery before any restoration.
    #[test]
    fn intervening_file_owner_or_journal_edits_block_recovery_before_any_restoration() {
        for reverse in [false, true] {
            for path in bootstrap::TARGETS
                .into_iter()
                .chain([STATE, bootstrap::V1_STATE, JOURNAL])
            {
                let (root, p) = fixture("legacy");
                let approved = approval(root.path(), p, reverse);
                let count = approved.operations().unwrap().len();
                interrupt(root.path(), &approved, count - 1);
                fs::write(root.path().join(path), "external edit").unwrap();
                let partial = snapshot(root.path());
                assert!(
                    recover(root.path(), approved.digest()).is_err(),
                    "{reverse} {path}"
                );
                assert_eq!(snapshot(root.path()), partial, "{reverse} {path}");
            }
        }
    }
    /// Verify that forged missing reordered redirected or reverse journals never restore unapproved paths.
    #[test]
    fn forged_missing_reordered_redirected_or_reverse_journals_never_restore_unapproved_paths() {
        for reverse in [false, true] {
            for mutation in 0..8 {
                let (root, p) = fixture("legacy");
                let approved = approval(root.path(), p, reverse);
                interrupt(root.path(), &approved, 0);
                let mut journal: Journal =
                    decode(&fs::read(root.path().join(JOURNAL)).unwrap()).unwrap();
                match mutation {
                    0 => journal.operations[0].path = "LICENSE".into(),
                    1 => {
                        journal.operations.pop();
                    }
                    2 => journal.operations.insert(0, journal.operations[0].clone()),
                    3 => journal.operations.swap(0, 1),
                    4 => journal.operations[0].after = Some("forged after".into()),
                    5 => journal.operations[0].before = Some("forged before".into()),
                    6 => journal.schema_version = 2,
                    7 => {
                        journal.approval = Approval::Upgrade {
                            plan: journal.approval.original().clone(),
                        };
                        if !reverse {
                            journal.approval = Approval::Rollback {
                                plan: RollbackPlan {
                                    schema_version: 1,
                                    contract: "armorer-upgrade-rollback".into(),
                                    runtime_version: env!("CARGO_PKG_VERSION").into(),
                                    plan_sha256: approved.digest().into(),
                                    allow_downgrade: true,
                                    original_upgrade: approved.original().clone(),
                                    limitations: vec![],
                                    changes: vec![],
                                },
                            };
                        }
                    }
                    _ => unreachable!(),
                }
                fs::write(root.path().join(JOURNAL), json(&journal).unwrap()).unwrap();
                let partial = snapshot(root.path());
                assert!(
                    recover(root.path(), approved.digest()).is_err(),
                    "{reverse} {mutation}"
                );
                assert_eq!(snapshot(root.path()), partial);
            }
        }
    }
    /// Verify that input changes before commit restore owned bytes and preserve new external inputs.
    #[test]
    fn input_changes_before_commit_restore_owned_bytes_and_preserve_new_external_inputs() {
        for reverse in [false, true] {
            let (root, p) = fixture("bootstrap");
            let approved = approval(root.path(), p, reverse);
            let mut before = snapshot(root.path());
            let last = approved.operations().unwrap().len() - 1;
            assert!(
                transaction(
                    root.path(),
                    &approved,
                    approved.digest(),
                    |index| {
                        if index == last {
                            fs::write(root.path().join("Cargo.lock"), "version = 4\n")?;
                        }
                        Ok(())
                    },
                    |root, bytes| replace(root, JOURNAL, bytes, false)
                )
                .is_err()
            );
            before.insert("Cargo.lock".into(), b"version = 4\n".to_vec());
            assert_eq!(snapshot(root.path()), before);
        }
    }
    /// Verify that apply failure after an intervening managed edit preserves every partial byte.
    #[test]
    fn apply_failure_after_an_intervening_managed_edit_preserves_every_partial_byte() {
        let (root, p) = fixture("unmanaged");
        let approved = Approval::Upgrade { plan: p };
        let error = transaction(
            root.path(),
            &approved,
            approved.digest(),
            |_| {
                fs::write(
                    root.path().join(bootstrap::TARGETS[0]),
                    "external policy edit",
                )?;
                Err(Error::Transaction("injected failure"))
            },
            |root, bytes| replace(root, JOURNAL, bytes, false),
        )
        .unwrap_err();
        assert!(error.to_string().contains("recovery required"));
        let partial = snapshot(root.path());
        assert!(recover(root.path(), approved.digest()).is_err());
        assert_eq!(snapshot(root.path()), partial);
    }
    /// Verify that unchanged targets and foreign owner presence also block whole recovery.
    #[test]
    fn unchanged_targets_and_foreign_owner_presence_also_block_whole_recovery() {
        for path in [
            bootstrap::TARGETS[0],
            bootstrap::TARGETS[1],
            bootstrap::V1_STATE,
        ] {
            let (root, p) = fixture("bootstrap");
            let approved = Approval::Upgrade { plan: p };
            let count = approved.operations().unwrap().len();
            interrupt(root.path(), &approved, count - 1);
            fs::write(root.path().join(path), "external edit").unwrap();
            let before = snapshot(root.path());
            assert!(recover(root.path(), approved.digest()).is_err(), "{path}");
            assert_eq!(snapshot(root.path()), before);
        }
    }
    /// Verify that committed recovery finalizes exact bytes without claiming current changed configuration.
    #[test]
    fn committed_recovery_finalizes_exact_bytes_without_claiming_current_changed_configuration() {
        let (root, p) = fixture("unmanaged");
        let approved = Approval::Upgrade { plan: p };
        assert!(
            transaction(
                root.path(),
                &approved,
                approved.digest(),
                |_| Ok(()),
                |root, bytes| {
                    replace(root, JOURNAL, bytes, false)?;
                    Err(Error::Transaction("sync failed"))
                }
            )
            .is_err()
        );
        let input = root.path().join("armorer.toml");
        fs::write(
            &input,
            format!(
                "{}\n# changed after commit\n",
                fs::read_to_string(&input).unwrap()
            ),
        )
        .unwrap();
        let mut before = snapshot(root.path());
        before.remove(JOURNAL);
        let receipt = recover(root.path(), approved.digest()).unwrap();
        assert_eq!(receipt.outcome, "commit-recovered");
        assert!(!receipt.configured);
        assert_eq!(snapshot(root.path()), before);
    }
    /// Verify that upgrade and rollback share existing kernel lock and refuse concurrent writers.
    #[test]
    fn upgrade_and_rollback_share_existing_kernel_lock_and_refuse_concurrent_writers() {
        for reverse in [false, true] {
            let (root, p) = fixture("bootstrap");
            let approved = approval(root.path(), p, reverse);
            let before = snapshot(root.path());
            let guard = lock(root.path()).unwrap();
            assert!(
                transaction(
                    root.path(),
                    &approved,
                    approved.digest(),
                    |_| Ok(()),
                    |root, bytes| replace(root, JOURNAL, bytes, false)
                )
                .is_err()
            );
            assert_eq!(snapshot(root.path()), before);
            drop(guard);
            transaction(
                root.path(),
                &approved,
                approved.digest(),
                |_| Ok(()),
                |root, bytes| replace(root, JOURNAL, bytes, false),
            )
            .unwrap();
        }
    }
}
