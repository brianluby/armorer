//! Generated bases and approved customized images have separate meanings.
use super::{
    Plan, SourceFormat,
    authority::{self, Authority},
    merge::{self, Decision},
};
use crate::{
    Error, Result,
    apply::{decode, json, text},
    bootstrap, catalog,
    config::hex_digest,
    digest,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Image {
    pub generated: String,
    pub generated_sha256: String,
    pub applied: String,
    pub applied_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct State {
    pub schema_version: u32,
    pub runtime_version: String,
    pub authority: Authority,
    pub last_plan_sha256: String,
    pub config_sha256: String,
    pub toolchain: String,
    pub generated_policy: String,
    pub managed: BTreeMap<String, Image>,
}

/// Qualify authority and every generated image against compiled immutable roots.
/// Prior policy expiry cannot block restoring exact recorded local bytes.
pub(super) fn decode_state(bytes: &[u8]) -> Result<State> {
    let state: State = decode(bytes)?;
    if state.schema_version != 1
        || state.runtime_version != env!("CARGO_PKG_VERSION")
        || state.authority != authority::select(&state.authority.id)?
        || !hex_digest(&state.last_plan_sha256, 64)
        || !hex_digest(&state.config_sha256, 64)
        || state.managed.len() > bootstrap::TARGETS.len()
        || !semver::Version::parse(&state.toolchain)
            .is_ok_and(|v| v.pre.is_empty() && v.build.is_empty())
    {
        return Err(Error::Transaction(
            "invalid upgrade ownership identity or version; no fallback",
        ));
    }
    let generated = catalog::reviewed()?.bootstrap_files(
        &state.config_sha256,
        &state.toolchain,
        &state.generated_policy,
    )?;
    for (path, image) in &state.managed {
        if generated.get(path) != Some(&image.generated)
            || digest(image.generated.as_bytes()) != image.generated_sha256
            || digest(image.applied.as_bytes()) != image.applied_sha256
        {
            return Err(Error::Transaction(
                "upgrade generated base or approved image contradicts independent catalog",
            ));
        }
        let allowed = match path.as_str() {
            "armorer.lock" | "rust-toolchain.toml" => image.applied == image.generated,
            ".github/workflows/armorer-ci.yml" => {
                merge::caller_extension(&image.applied, &image.generated, "verify")
            }
            ".github/workflows/armorer-build.yml" => {
                merge::caller_extension(&image.applied, &image.generated, "build")
            }
            ".armorer/ci-policy.toml" => true,
            _ => false,
        };
        if !allowed {
            return Err(Error::Transaction(
                "upgrade approved image modifies protected generated identity",
            ));
        }
    }
    Ok(state)
}

/// Reconstruct the next receipt solely from the approved source/base/merge packet.
pub(super) fn next(plan: &Plan) -> Result<State> {
    let source = super::plan::source_bytes(plan.state_preimages.clone())?;
    let mut managed = BTreeMap::new();
    for c in &plan.changes {
        if c.decision == Decision::Conflict {
            return Err(Error::Transaction("unresolved upgrade conflict"));
        }
        let owned = source.bases.contains_key(&c.path)
            || c.imported
            || matches!(c.decision, Decision::Create | Decision::Update);
        if owned {
            let applied = c
                .proposed_content
                .clone()
                .ok_or(Error::Transaction("upgrade after image missing"))?;
            managed.insert(
                c.path.clone(),
                Image {
                    generated: c.candidate_content.clone(),
                    generated_sha256: c.candidate_sha256.clone(),
                    applied,
                    applied_sha256: c
                        .after_sha256
                        .clone()
                        .ok_or(Error::Transaction("upgrade after digest missing"))?,
                },
            );
        }
    }
    let state = State {
        schema_version: 1,
        runtime_version: env!("CARGO_PKG_VERSION").into(),
        authority: plan.target_catalog.clone(),
        last_plan_sha256: plan.plan_sha256.clone(),
        config_sha256: plan.config_sha256.clone(),
        toolchain: plan.intent.toolchain.clone(),
        generated_policy: plan.policy_content.clone(),
        managed,
    };
    decode_state(&json(&state)?)?;
    Ok(state)
}

/// Reconstruct the next generated text from the independently selected catalog authority.
pub(super) fn next_text(plan: &Plan) -> Result<String> {
    text(&json(&next(plan)?)?)
}

/// Source formats remain explicit; migration removes only the exact original owner.
pub(super) fn prior_path(format: &SourceFormat) -> Option<&'static str> {
    match format {
        SourceFormat::LegacyV1 => Some(bootstrap::V1_STATE),
        SourceFormat::BootstrapV2 => Some(bootstrap::STATE),
        SourceFormat::Unmanaged | SourceFormat::UpgradeV1 => None,
    }
}
