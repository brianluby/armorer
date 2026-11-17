//! Reviewable upgrades and migration, separate from v1/v2 apply authorization.
pub mod authority;
pub mod merge;
mod plan;
mod rollback;
mod state;
mod transaction;
pub use plan::{Change, Plan, SourceFormat, inspect, load_plan, validate_fresh};
pub use rollback::{Restore, RollbackPlan, inspect_rollback, load_rollback_plan};
pub use transaction::{Receipt, apply, apply_rollback, recover};

pub(crate) const STATE: &str = ".armorer/upgrade-state-v1.json";
pub(crate) const JOURNAL: &str = ".armorer/upgrade-journal-v1.json";

/// Leftover version-one ownership is rejected explicitly, never silently
/// re-adopted (ADR 0012 rule 1). Every surviving read/write path calls this.
pub(crate) fn reject_v1(root: &std::path::Path) -> crate::Result<()> {
    if crate::filesystem::optional_bytes(root, ".armorer/state.json")?.is_some()
        || crate::filesystem::optional_bytes(root, ".armorer/journal.json")?.is_some()
    {
        return Err(crate::Error::Transaction(
            "unsupported version-one ownership requires explicit removal review",
        ));
    }
    Ok(())
}

/// The catalog recorded by the upgrade owner, validated against compiled authority.
pub(crate) fn owner_catalog_id(root: &std::path::Path) -> crate::Result<Option<String>> {
    match crate::filesystem::optional_bytes(root, STATE)? {
        Some(bytes) => Ok(Some(state::decode_state(&bytes)?.authority.id)),
        None => Ok(None),
    }
}

/// Earlier writers must refuse an upgrade owner/journal instead of crossing versions.
pub(crate) fn guard_previous(root: &std::path::Path) -> crate::Result<()> {
    reject_v1(root)?;
    if crate::filesystem::optional_bytes(root, STATE)?.is_some()
        || crate::filesystem::optional_bytes(root, JOURNAL)?.is_some()
    {
        return Err(crate::Error::Transaction(
            "upgrade ownership or journal requires the upgrade command",
        ));
    }
    Ok(())
}
