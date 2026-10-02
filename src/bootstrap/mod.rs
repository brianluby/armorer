//! Explicit version-two, approval-bound local provisioning.
//! Version-one plans never gain these mutation targets implicitly.
mod plan;
mod transaction;

pub use plan::{Change, Plan, inspect, load_plan};
pub use transaction::{Receipt, apply, recover};

use crate::{Error, Result, apply::optional_bytes};
use std::path::Path;

pub(crate) const VERSION: u32 = 2;
pub(crate) const STATE: &str = ".armorer/bootstrap-state-v2.json";
pub(crate) const JOURNAL: &str = ".armorer/bootstrap-journal-v2.json";
pub(crate) const V1_STATE: &str = ".armorer/state.json";
pub(crate) const V1_JOURNAL: &str = ".armorer/journal.json";
pub(crate) const TARGETS: [&str; 5] = [
    ".armorer/ci-policy.toml",
    ".github/workflows/armorer-build.yml",
    ".github/workflows/armorer-ci.yml",
    "armorer.lock",
    "rust-toolchain.toml",
];

/// The updated v1 writer refuses files governed by a v2 owner or journal.
pub(crate) fn guard_v1(root: &Path) -> Result<()> {
    if optional_bytes(root, STATE)?.is_some() || optional_bytes(root, JOURNAL)?.is_some() {
        return Err(Error::Transaction(
            "version-two bootstrap state or recovery requires the bootstrap command",
        ));
    }
    Ok(())
}
