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

/// Earlier writers must refuse an upgrade owner/journal instead of crossing versions.
pub(crate) fn guard_previous(root: &std::path::Path) -> crate::Result<()> {
    if crate::apply::optional_bytes(root, STATE)?.is_some()
        || crate::apply::optional_bytes(root, JOURNAL)?.is_some()
    {
        return Err(crate::Error::Transaction(
            "upgrade ownership or journal requires the upgrade command",
        ));
    }
    Ok(())
}
