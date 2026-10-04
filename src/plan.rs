//! Deterministic previews. No consumer files are written by this module.

use crate::{
    Result,
    config::{Config, VERSION, load_config, load_lock},
    digest,
    discovery::{Workspace, discover},
    read_small, safe_path,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub runtime_version: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub plan_sha256: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub state_sha256: Option<String>,
    pub input_preimages: BTreeMap<String, Option<String>>,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    pub mode: String,
    pub repository: String,
    pub intent: Config,
    pub state: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub config_sha256: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub lock_sha256: Option<String>,
    pub capability_state: String,
    pub workspace: Workspace,
    pub changes: Vec<Change>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub path: String,
    pub disposition: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub before_sha256: Option<String>,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub after_sha256: String,
    pub proposed_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub code: String,
    pub detail: String,
}

fn finding(code: &str, detail: &str) -> Finding {
    Finding {
        code: code.into(),
        detail: detail.into(),
    }
}

/// Preview the pinned toolchain file without writing it.
///
/// A missing file is `create`, identical bytes are `unchanged`, and differing
/// bytes are `conflict`. A legacy `rust-toolchain` file always causes a conflict.
/// Returns proposed content and digests, propagating path, I/O, and size errors.
fn toolchain_change(root: &Path, config: &Config) -> Result<Change> {
    let path = safe_path(root, "rust-toolchain.toml")?;
    let proposed_content = format!(
        "[toolchain]\nchannel = \"{}\"\nprofile = \"minimal\"\ncomponents = [\"clippy\", \"rustfmt\"]\n",
        config.toolchain
    );
    let current = if path.try_exists()? {
        Some(read_small(&path)?)
    } else {
        None
    };
    let legacy = safe_path(root, "rust-toolchain")?.try_exists()?;
    let disposition = match &current {
        _ if legacy => "conflict",
        None => "create",
        Some(bytes) if *bytes == proposed_content.as_bytes() => "unchanged",
        Some(_) => "conflict",
    };
    Ok(Change {
        path: "rust-toolchain.toml".into(),
        disposition: disposition.into(),
        before_sha256: current.as_deref().map(digest),
        after_sha256: digest(proposed_content.as_bytes()),
        proposed_content,
    })
}

/// Inspect configuration and preview the first managed file. Missing capabilities fail closed.
///
/// `mode` is copied into the plan without changing inspection behavior. Returns
/// input digests, discovered workspace selections, a toolchain-file preview,
/// and findings sorted by code. Missing prerequisites and toolchain conflicts
/// become findings; a successful return does not indicate release readiness.
/// Repository files are preserved; discovery uses temporary files and subprocesses.
///
/// # Errors
/// Propagates configuration and lock validation, discovery, path, I/O, and
/// size-limit errors. An absent lock is a finding, but an invalid lock is an error.
pub fn inspect(root: &Path, mode: &str) -> Result<Plan> {
    let (config, bytes) = load_config(root)?;
    let config_sha256 = digest(&bytes);
    let lock = load_lock(root, &config_sha256)?;
    let lock_sha256 = lock.as_ref().map(|(_, bytes)| digest(bytes));
    let workspace = discover(root, &config)?;
    let ownership = crate::apply::load_state(root)?;
    let state_sha256 = ownership.as_ref().map(|(_, bytes)| digest(bytes));
    let mut change = toolchain_change(root, &config)?;
    if change.disposition == "conflict"
        && !safe_path(root, "rust-toolchain")?.try_exists()?
        && ownership
            .as_ref()
            .and_then(|(state, _)| state.managed.get(&change.path))
            .is_some_and(|base| Some(base.sha256.clone()) == change.before_sha256)
    {
        change.disposition = "update".into();
    }
    let mut input_preimages = BTreeMap::new();
    for relative in [&config.policy.license_file, &"rust-toolchain".to_owned()] {
        let path = safe_path(root, relative)?;
        input_preimages.insert(
            relative.clone(),
            if path.try_exists()? {
                Some(digest(&read_small(&path)?))
            } else {
                None
            },
        );
    }
    let mut findings = vec![
        finding(
            "github-capabilities-unchecked",
            "Repository visibility, plan eligibility, immutable releases, rulesets and environments require a later authenticated preflight.",
        ),
        finding(
            "release-runtime-unavailable",
            "No reviewed reusable workflow/runtime catalog is shipped in this slice. Secure release readiness is blocked.",
        ),
        finding(
            "license-policy-unreviewed",
            "License-file presence is inspected; project-specific dependency license policy requires review.",
        ),
    ];
    if lock.is_none() {
        findings.push(finding(
            "reviewed-lock-missing",
            "armorer.lock requires reviewed workflow and tool pins; no pins are invented.",
        ));
    } else {
        findings.push(finding("pins-not-authenticated", "Pin syntax and config binding are valid; upstream identities, distribution bytes and catalog approval have not been verified."));
    }
    if !workspace.cargo_lock_present {
        findings.push(finding(
            "cargo-lock-missing",
            "Locked CI/builds require a committed Cargo.lock; discovery does not create one.",
        ));
    }
    if !safe_path(root, &config.policy.license_file)?.is_file() {
        findings.push(finding(
            "license-file-missing",
            "The configured project license file is missing.",
        ));
    }
    if workspace.cargo_config_present {
        findings.push(finding("cargo-config-excluded", "Repository Cargo configuration was excluded from discovery; overrides need explicit migration review."));
    }
    if workspace.packages.iter().any(|p| {
        p.links.is_some()
            || p.targets
                .iter()
                .any(|t| t.kind.iter().any(|k| k == "custom-build"))
    }) {
        findings.push(finding("native-build-review", "Declared native links/build scripts require target-specific tool and system dependency review."));
    }
    if change.disposition == "conflict" {
        findings.push(finding(
            "toolchain-conflict",
            "Existing toolchain customization is preserved; resolve the conflict before apply.",
        ));
    }
    if safe_path(root, ".armorer/journal.json")?.try_exists()? {
        findings.push(finding(
            "transaction-recovery-required",
            "An unfinished transaction must be explicitly recovered before applying another plan.",
        ));
    }
    findings.sort_by(|a, b| a.code.cmp(&b.code));
    let mut plan = Plan {
        runtime_version: env!("CARGO_PKG_VERSION").into(),
        plan_sha256: String::new(),
        state_sha256,
        input_preimages,
        schema_version: VERSION,
        mode: mode.into(),
        repository: config.repository.clone(),
        intent: config,
        state: "configuration-valid".into(),
        config_sha256,
        lock_sha256,
        capability_state: "unknown".into(),
        workspace,
        changes: vec![change],
        findings,
    };
    plan.plan_sha256 = plan.digest()?;
    if serde_json::to_vec_pretty(&plan)
        .map_err(|_| crate::Error::Json)?
        .len()
        > 1_048_576
    {
        return Err(crate::Error::Transaction("plan exceeds 1 MiB"));
    }
    Ok(plan)
}

impl Plan {
    /// Hash compact typed JSON with the digest field blank. Formatting is irrelevant.
    pub fn digest(&self) -> Result<String> {
        let mut canonical = self.clone();
        canonical.plan_sha256.clear();
        let bytes = serde_json::to_vec(&canonical).map_err(|_| crate::Error::Json)?;
        if bytes.len() > 1_048_576 {
            return Err(crate::Error::Transaction("plan exceeds 1 MiB"));
        }
        Ok(digest(&bytes))
    }
}
