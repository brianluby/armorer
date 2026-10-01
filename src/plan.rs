//! Deterministic previews. No consumer files are written by this module.

use crate::{
    Result,
    config::{Config, VERSION, load_config, load_lock},
    digest,
    discovery::{Workspace, discover},
    read_small, safe_path,
};
use schemars::JsonSchema;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize, JsonSchema)]
pub struct Plan {
    pub schema_version: u32,
    pub mode: &'static str,
    pub repository: String,
    pub intent: Config,
    pub state: &'static str,
    pub config_sha256: String,
    pub lock_sha256: Option<String>,
    pub capability_state: &'static str,
    pub workspace: Workspace,
    pub changes: Vec<Change>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Change {
    pub path: &'static str,
    pub disposition: &'static str,
    pub before_sha256: Option<String>,
    pub after_sha256: String,
    pub proposed_content: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Finding {
    pub code: &'static str,
    pub detail: &'static str,
}

fn finding(code: &'static str, detail: &'static str) -> Finding {
    Finding { code, detail }
}

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
        path: "rust-toolchain.toml",
        disposition,
        before_sha256: current.as_deref().map(digest),
        after_sha256: digest(proposed_content.as_bytes()),
        proposed_content,
    })
}

/// Inspect configuration and preview the first managed file. Missing capabilities fail closed.
pub fn inspect(root: &Path, mode: &'static str) -> Result<Plan> {
    let (config, bytes) = load_config(root)?;
    let config_sha256 = digest(&bytes);
    let lock = load_lock(root, &config_sha256)?;
    let lock_sha256 = if lock.is_some() {
        Some(digest(&read_small(&safe_path(root, "armorer.lock")?)?))
    } else {
        None
    };
    let workspace = discover(root, &config)?;
    let change = toolchain_change(root, &config)?;
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
        findings.push(finding("toolchain-conflict", "Existing toolchain customization is preserved; resolve the conflict before future apply."));
    }
    findings.sort_by_key(|f| f.code);
    Ok(Plan {
        schema_version: VERSION,
        mode,
        repository: config.repository.clone(),
        intent: config,
        state: "configuration-valid",
        config_sha256,
        lock_sha256,
        capability_state: "unknown",
        workspace,
        changes: vec![change],
        findings,
    })
}
