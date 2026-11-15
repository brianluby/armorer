//! Separately reviewed exact historical restoration, never authenticity fallback.
use super::{Plan, plan, transaction};
use crate::{
    Error, Result,
    apply::{decode, json},
    bootstrap,
    config::hex_digest,
    digest, read_small,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Restore {
    pub path: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub before_sha256: Option<String>,
    pub before_content: Option<String>,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub after_sha256: Option<String>,
    pub proposed_content: Option<String>,
    pub unified_diff: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RollbackPlan {
    #[schemars(transform = crate::schema_bounds::unsigned)]
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    pub contract: String,
    pub runtime_version: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub plan_sha256: String,
    pub allow_downgrade: bool,
    pub original_upgrade: Plan,
    pub limitations: Vec<String>,
    pub changes: Vec<Restore>,
}
/// State the remaining readiness and rollback limits without granting release authority.
fn limitations() -> Vec<String> {
    vec![
        "Restores exact previously approved local bytes and ownership; previous declared pins are not authenticated historical catalog roots.".into(),
        "Configuration, Cargo inputs and reviewed source policy are not reverted. Re-run check/plan and review lock/config compatibility before CI or release.".into(),
        "Local byte restoration does not establish configured readiness, CI, signing, publication, provenance or SLSA.".into(),
    ]
}
/// Reverse-image unified diff; deletions retarget the header to /dev/null even when
/// the deleted file was empty and the diff itself short-circuits.
fn reverse_diff(path: &str, before: Option<&str>, after: Option<&str>) -> String {
    let diff = bootstrap::plan::diff(path, before, after.unwrap_or(""));
    if after.is_none() {
        if diff.is_empty() {
            // Deletion of an empty file: diff short-circuits, keep the header.
            format!("--- a/{path}\n+++ /dev/null\n")
        } else {
            diff.replacen(&format!("+++ b/{path}\n"), "+++ /dev/null\n", 1)
        }
    } else {
        diff
    }
}

/// Derive exact reverse images from the completed approved forward plan.
fn changes(original: &Plan) -> Result<Vec<Restore>> {
    transaction::reverse_operations(original)?
        .into_iter()
        .map(|w| {
            Ok(Restore {
                before_sha256: w.before.as_deref().map(|s| digest(s.as_bytes())),
                after_sha256: w.after.as_deref().map(|s| digest(s.as_bytes())),
                unified_diff: reverse_diff(&w.path, w.before.as_deref(), w.after.as_deref()),
                path: w.path,
                before_content: w.before,
                proposed_content: w.after,
            })
        })
        .collect()
}
impl RollbackPlan {
    /// Independent approval includes the original approved packet and exact reverse images.
    pub fn digest(&self) -> Result<String> {
        let mut value = self.clone();
        value.plan_sha256.clear();
        let bytes = serde_json::to_vec(&value).map_err(|_| Error::Json)?;
        if bytes.len() > 1_048_576 {
            return Err(Error::Transaction(
                "rollback plan exceeds 1 MiB; no image was truncated",
            ));
        }
        Ok(digest(&bytes))
    }
}
/// Read-only reversal preview requires both original approval and explicit downgrade intent.
/// It accepts only the exact final upgrade receipt/files, never edited or foreign state.
pub fn inspect_rollback(
    root: &Path,
    original: &Plan,
    expected: &str,
    allow_downgrade: bool,
) -> Result<RollbackPlan> {
    plan::validate(original, expected)?;
    if !allow_downgrade {
        return Err(Error::Transaction(
            "historical restoration requires explicit allow-downgrade",
        ));
    }
    transaction::rollback_fresh(root, original)?;
    let mut plan = RollbackPlan {
        schema_version: 1,
        contract: "armorer-upgrade-rollback".into(),
        runtime_version: env!("CARGO_PKG_VERSION").into(),
        plan_sha256: String::new(),
        allow_downgrade,
        original_upgrade: original.clone(),
        limitations: limitations(),
        changes: changes(original)?,
    };
    plan.plan_sha256 = plan.digest()?;
    json(&plan)?;
    Ok(plan)
}
/// Reconstruct the approved record semantics and byte identities before accepting it.
pub(super) fn validate(plan: &RollbackPlan, expected: &str) -> Result<()> {
    if !hex_digest(expected, 64)
        || plan.plan_sha256 != expected
        || plan.digest()? != expected
        || plan.schema_version != 1
        || plan.contract != "armorer-upgrade-rollback"
        || plan.runtime_version != env!("CARGO_PKG_VERSION")
        || !plan.allow_downgrade
        || plan.limitations != limitations()
    {
        return Err(Error::Transaction(
            "rollback packet differs from independent approval or downgrade decision",
        ));
    }
    plan::validate(&plan.original_upgrade, &plan.original_upgrade.plan_sha256)?;
    transaction::eligible(&plan.original_upgrade)?;
    if serde_json::to_value(&plan.changes).map_err(|_| Error::Json)?
        != serde_json::to_value(changes(&plan.original_upgrade)?).map_err(|_| Error::Json)?
    {
        return Err(Error::Transaction(
            "rollback images differ from exact original approved operation inventory",
        ));
    }
    json(plan)?;
    Ok(())
}
/// Strict versioned load; supplying this packet to an upgrade or frozen apply is rejected.
pub fn load_rollback_plan(path: &Path, expected: &str) -> Result<RollbackPlan> {
    let plan: RollbackPlan = decode(&read_small(path)?)?;
    validate(&plan, expected)?;
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::reverse_diff;

    #[test]
    /// Deletion diffs retarget /dev/null even when the deleted file was empty.
    fn reverse_diff_covers_empty_and_nonempty_deletions() {
        assert_eq!(
            reverse_diff("gone", Some(""), None),
            "--- a/gone\n+++ /dev/null\n"
        );
        let diff = reverse_diff("gone", Some("line\n"), None);
        assert!(diff.starts_with("--- a/gone\n+++ /dev/null\n"), "{diff}");
        assert!(!diff.contains("+++ b/gone"));
        // Non-deletion images keep the ordinary two-sided diff.
        assert!(reverse_diff("kept", Some("a\n"), Some("b\n")).contains("+++ b/kept"));
    }
}
