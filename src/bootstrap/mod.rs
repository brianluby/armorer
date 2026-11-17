//! Explicit version-two, approval-bound local provisioning.
pub(crate) mod plan;
pub(crate) mod transaction;

pub use plan::{Change, Plan, inspect, load_plan};
pub use transaction::{Receipt, apply, recover};

pub(crate) const VERSION: u32 = 2;
pub(crate) const STATE: &str = ".armorer/bootstrap-state-v2.json";
pub(crate) const JOURNAL: &str = ".armorer/bootstrap-journal-v2.json";
pub(crate) const TARGETS: [&str; 5] = [
    ".armorer/ci-policy.toml",
    ".github/workflows/armorer-build.yml",
    ".github/workflows/armorer-ci.yml",
    "armorer.lock",
    "rust-toolchain.toml",
];
