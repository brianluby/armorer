//! Exact fixed-target previews and independent catalog authority.
use super::{JOURNAL, STATE, TARGETS, V1_JOURNAL, V1_STATE, VERSION, transaction::load_state};
use crate::{
    Error, Result,
    apply::{decode, json, optional_bytes, text},
    catalog, ci_policy,
    config::{Config, hex_digest, load_config},
    digest,
    discovery::{Workspace, discover},
    plan::Finding,
    read_small,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

/// Separate authorization contract; every before/after byte and diff is approval-bound.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema_version: u32,
    pub runtime_version: String,
    pub mode: String,
    pub plan_sha256: String,
    pub config_sha256: String,
    pub catalog_sha256: String,
    pub state_sha256: Option<String>,
    pub intent: Config,
    pub workspace: Workspace,
    pub policy_content: String,
    pub input_preimages: BTreeMap<String, Option<String>>,
    pub preserved_inputs: BTreeMap<String, Option<String>>,
    pub recovery_required: bool,
    pub capability_state: String,
    pub findings: Vec<Finding>,
    pub changes: Vec<Change>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub path: String,
    pub disposition: String,
    pub owned: bool,
    pub before_sha256: Option<String>,
    pub before_content: Option<String>,
    pub after_sha256: String,
    pub proposed_content: String,
    /// Complete linear replacement hunk, with no omitted conflict lines.
    pub unified_diff: String,
}

/// Emit each exact line and patch's missing-final-newline annotation.
fn append_lines(output: &mut String, prefix: char, content: &str) {
    for line in content.split_inclusive('\n') {
        output.push(prefix);
        output.push_str(line);
        if !line.ends_with('\n') {
            output.push_str("\n\\ No newline at end of file\n");
        }
    }
}

/// Render complete hunks in linear space, preserving CRLF, Unicode and absence.
pub(crate) fn diff(path: &str, before: Option<&str>, after: &str) -> String {
    if before == Some(after) {
        return String::new();
    }
    let old = before.unwrap_or_default();
    let old_lines = old.split_inclusive('\n').count();
    let new_lines = after.split_inclusive('\n').count();
    let old_name = before.map_or_else(|| "/dev/null".into(), |_| format!("a/{path}"));
    let mut output = format!(
        "--- {old_name}\n+++ b/{path}\n@@ -{},{old_lines} +{},{new_lines} @@\n",
        usize::from(old_lines != 0),
        usize::from(new_lines != 0),
    );
    append_lines(&mut output, '-', old);
    append_lines(&mut output, '+', after);
    output
}

/// Exact compiled template set. Consuming input selects neither paths nor templates.
pub(super) fn generated(plan: &Plan) -> Result<BTreeMap<String, String>> {
    catalog::reviewed()?.bootstrap_files(
        &plan.config_sha256,
        &plan.intent.toolchain,
        &plan.policy_content,
    )
}

/// Ownership is checked before comparing the new candidate: deletion or an edit
/// to candidate bytes must not silently adopt or erase a customization.
pub(super) fn disposition(
    before: Option<&str>,
    after: &str,
    base: Option<&str>,
    legacy: bool,
) -> &'static str {
    if legacy || base.is_some_and(|base| before != Some(base)) {
        "conflict"
    } else if before == Some(after) {
        "unchanged"
    } else if base.is_some() {
        "update"
    } else if before.is_none() {
        "create"
    } else {
        "conflict"
    }
}

impl Plan {
    /// SHA-256 of compact typed JSON with only this digest field blank.
    /// Bounds the entire escaped view; a conflict is never truncated.
    pub fn digest(&self) -> Result<String> {
        let mut value = self.clone();
        value.plan_sha256.clear();
        let bytes = serde_json::to_vec(&value).map_err(|_| Error::Json)?;
        if bytes.len() > 1_048_576 {
            return Err(Error::Transaction("bootstrap plan exceeds 1 MiB"));
        }
        Ok(digest(&bytes))
    }
}

/// Generate a read-only preview using an explicitly chosen policy file.
/// No policy default, download, build script, tool install or consuming write occurs.
pub fn inspect(root: &Path, policy: &Path) -> Result<Plan> {
    let (_, bytes) = ci_policy::load(policy)?;
    inspect_bytes(root, &bytes)
}

/// The exact approved policy is self-contained; its external filename is not authority.
pub(super) fn inspect_bytes(root: &Path, policy: &[u8]) -> Result<Plan> {
    crate::upgrade::guard_previous(root)?;
    ci_policy::parse_at(policy, ci_policy::today()?)?;
    let (intent, config) = load_config(root)?;
    if TARGETS.contains(&intent.policy.license_file.as_str())
        || [
            STATE,
            JOURNAL,
            V1_STATE,
            V1_JOURNAL,
            ".armorer/apply.lock",
            "armorer.toml",
        ]
        .contains(&intent.policy.license_file.as_str())
    {
        return Err(Error::Transaction(
            "license input overlaps bootstrap control or generated files",
        ));
    }
    let workspace = discover(root, &intent)?;
    let state = load_state(root)?;
    let mut input_preimages = BTreeMap::new();
    input_preimages.insert(
        intent.policy.license_file.clone(),
        optional_bytes(root, &intent.policy.license_file)?
            .as_deref()
            .map(digest),
    );
    let mut preserved_inputs = BTreeMap::new();
    for path in ["rust-toolchain", V1_STATE] {
        let content = optional_bytes(root, path)?
            .map(|bytes| text(&bytes))
            .transpose()?;
        preserved_inputs.insert(path.into(), content);
    }
    let mut plan = Plan {
        schema_version: VERSION,
        runtime_version: env!("CARGO_PKG_VERSION").into(),
        mode: "bootstrap-plan".into(),
        plan_sha256: String::new(),
        config_sha256: digest(&config),
        catalog_sha256: catalog::TOOLS_SHA256.into(),
        state_sha256: state.as_ref().map(|(_, bytes)| digest(bytes)),
        intent,
        workspace,
        policy_content: text(policy)?,
        input_preimages,
        preserved_inputs,
        recovery_required: optional_bytes(root, JOURNAL)?.is_some() || optional_bytes(root, V1_JOURNAL)?.is_some(),
        capability_state: "unknown".into(),
        findings: vec![
            Finding { code: "github-capabilities-unchecked".into(), detail: "Authenticated GitHub capability preflight remains required.".into() },
            Finding { code: "release-runtime-unavailable".into(), detail: "Callers provide CI and manual unsigned builds; secure publication, final-byte attestations and Apple finalization remain required.".into() },
        ],
        changes: Vec::new(),
    };
    if plan.preserved_inputs[V1_STATE].is_some() {
        plan.findings.push(Finding { code: "legacy-ownership-migration-required".into(), detail: "Version-one ownership is preserved; bootstrap requires an explicit reviewed migration.".into() });
    }
    if !plan.workspace.cargo_lock_present {
        plan.findings.push(Finding { code: "cargo-lock-missing".into(), detail: "CI and builds require a committed Cargo.lock; discovery and provisioning never generate it.".into() });
    }
    if plan.input_preimages[&plan.intent.policy.license_file].is_none() {
        plan.findings.push(Finding {
            code: "license-file-missing".into(),
            detail: "The configured project license file is missing.".into(),
        });
    }
    if plan.workspace.cargo_config_present {
        plan.findings.push(Finding { code: "cargo-config-excluded".into(), detail: "Discovery excluded Cargo configuration; overrides require explicit migration review.".into() });
    }
    if plan.workspace.packages.iter().any(|p| {
        p.links.is_some()
            || p.targets
                .iter()
                .any(|t| t.kind.iter().any(|k| k == "custom-build"))
    }) {
        plan.findings.push(Finding { code: "native-build-review".into(), detail: "Native links and build scripts need target-specific dependency review before CI/build adoption.".into() });
    }
    plan.findings.sort_by(|a, b| a.code.cmp(&b.code));
    let legacy = plan.preserved_inputs["rust-toolchain"].is_some();
    for (path, proposed_content) in generated(&plan)? {
        let before_content = optional_bytes(root, &path)?
            .map(|bytes| text(&bytes))
            .transpose()?;
        let base = state
            .as_ref()
            .and_then(|(state, _)| state.managed.get(&path));
        let disposition = disposition(
            before_content.as_deref(),
            &proposed_content,
            base.map(|base| base.content.as_str()),
            path == "rust-toolchain.toml" && legacy,
        );
        plan.changes.push(Change {
            unified_diff: diff(&path, before_content.as_deref(), &proposed_content),
            before_sha256: before_content
                .as_deref()
                .map(|content| digest(content.as_bytes())),
            after_sha256: digest(proposed_content.as_bytes()),
            path,
            disposition: disposition.into(),
            owned: base.is_some(),
            before_content,
            proposed_content,
        });
    }
    plan.plan_sha256 = plan.digest()?;
    json(&plan)?;
    Ok(plan)
}

/// Load a strict bounded plan, requiring a digest obtained from independent review.
pub fn load_plan(path: &Path, expected: &str) -> Result<Plan> {
    let plan = decode(&read_small(path)?)?;
    validate(&plan, expected)?;
    Ok(plan)
}

/// Validate fixed identities and generated content without touching project inputs.
/// Current policy expiry is enforced by fresh inspection at apply, not by rollback.
pub(super) fn validate(plan: &Plan, expected: &str) -> Result<()> {
    if !hex_digest(expected, 64)
        || plan.plan_sha256 != expected
        || plan.digest()? != expected
        || plan.schema_version != VERSION
        || plan.runtime_version != env!("CARGO_PKG_VERSION")
        || plan.mode != "bootstrap-plan"
        || plan.catalog_sha256 != catalog::TOOLS_SHA256
        || !hex_digest(&plan.config_sha256, 64)
        || plan.capability_state != "unknown"
        || plan.changes.len() != TARGETS.len()
        || plan
            .state_sha256
            .as_ref()
            .is_some_and(|hash| !hex_digest(hash, 64))
    {
        return Err(Error::Transaction(
            "bootstrap plan does not match approved digest, catalog or runtime",
        ));
    }
    // Structural config rules are independent of existing project files.
    plan.intent.validate_shape()?;
    let generated = generated(plan)?;
    for (change, (path, after)) in plan.changes.iter().zip(generated) {
        if change.path != path
            || change.proposed_content != after
            || change.after_sha256 != digest(after.as_bytes())
            || change.before_sha256
                != change
                    .before_content
                    .as_deref()
                    .map(|s| digest(s.as_bytes()))
            || change.unified_diff != diff(&path, change.before_content.as_deref(), &after)
            || !matches!(
                change.disposition.as_str(),
                "create" | "update" | "conflict" | "unchanged"
            )
        {
            return Err(Error::Transaction(
                "unsupported bootstrap content, preimage or target",
            ));
        }
    }
    json(plan)?;
    Ok(())
}

/// Ignore only transaction output preimages for matching committed-plan replay.
pub(super) fn same_intent(original: &Plan, fresh: &Plan) -> Result<bool> {
    fn normalized(plan: &Plan) -> Result<String> {
        let mut value = plan.clone();
        value.state_sha256 = None;
        value.recovery_required = false;
        for change in &mut value.changes {
            change.before_content = None;
            change.before_sha256 = None;
            change.disposition.clear();
            change.unified_diff.clear();
            change.owned = false;
        }
        value.digest()
    }
    Ok(normalized(original)? == normalized(fresh)?)
}
