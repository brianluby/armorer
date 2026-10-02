//! Read-only exact base/current/candidate previews and explicit import scope.
use super::{
    authority::{self, Authority},
    merge::{self, Decision},
};
use crate::{
    Error, Result,
    apply::{decode, json, optional_bytes, text},
    bootstrap, catalog, ci_policy,
    config::{Config, Lock, ToolPin, hex_digest, identifier, load_config, repository_name},
    digest,
    discovery::{Workspace, discover},
    read_small,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SourceFormat {
    Unmanaged,
    LegacyV1,
    BootstrapV2,
    UpgradeV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Compatibility {
    pub state: String,
    pub code: String,
    pub detail: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PinChange {
    pub name: String,
    /// Syntax-checked observations, never independent catalog trust roots.
    pub before: Option<ToolPin>,
    pub after: Option<ToolPin>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub path: String,
    pub decision: Decision,
    pub imported: bool,
    pub base_sha256: Option<String>,
    pub base_content: Option<String>,
    pub before_sha256: Option<String>,
    pub before_content: Option<String>,
    pub candidate_sha256: String,
    pub candidate_content: String,
    pub after_sha256: Option<String>,
    pub proposed_content: Option<String>,
    pub candidate_diff: String,
    pub proposed_diff: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema_version: u32,
    pub contract: String,
    pub runtime_version: String,
    pub plan_sha256: String,
    pub source_format: SourceFormat,
    pub source_catalog: Option<Authority>,
    pub target_catalog: Authority,
    pub direction: String,
    pub allow_downgrade: bool,
    pub import_files: Vec<String>,
    pub state_preimages: BTreeMap<String, Option<String>>,
    pub input_preimages: BTreeMap<String, Option<String>>,
    pub config_sha256: String,
    pub intent: Config,
    pub workspace: Workspace,
    pub policy_content: String,
    /// UTC epoch day used for the reviewed policy/compatibility snapshot, not a live trust root.
    pub policy_review_day: u64,
    pub recovery_required: bool,
    pub compatibility: Vec<Compatibility>,
    pub pin_changes: Vec<PinChange>,
    pub workflow_before: Option<crate::config::WorkflowPin>,
    pub workflow_after: crate::config::WorkflowPin,
    pub changes: Vec<Change>,
}

fn note(state: &str, code: &str, detail: &str) -> Compatibility {
    Compatibility {
        state: state.into(),
        code: code.into(),
        detail: detail.into(),
    }
}
fn hash(content: Option<&str>) -> Option<String> {
    content.map(|s| digest(s.as_bytes()))
}
fn imports(files: &[String]) -> Result<()> {
    if files.len() > bootstrap::TARGETS.len()
        || files.iter().collect::<BTreeSet<_>>().len() != files.len()
        || files
            .iter()
            .any(|path| !bootstrap::TARGETS.contains(&path.as_str()))
    {
        return Err(Error::Transaction(
            "import decisions must be unique fixed bootstrap targets",
        ));
    }
    Ok(())
}

/// Recognize only the strict supported v1 declaration format; this does not
/// authenticate before pins or bind them to the newly changed configuration.
fn declared_lock(content: &str) -> Option<Lock> {
    let lock: Lock = toml::from_str(content).ok()?;
    let version =
        |s: &str| semver::Version::parse(s).is_ok_and(|v| v.pre.is_empty() && v.build.is_empty());
    (lock.schema_version == 1
        && lock.runtime_version == "0.1.0"
        && hex_digest(&lock.config_sha256, 64)
        && repository_name(&lock.workflows.repository)
        && hex_digest(&lock.workflows.commit, 40)
        && !lock.tools.is_empty()
        && lock.tools.len() <= 128
        && lock.tools.iter().all(|(name, pin)| {
            identifier(name) && version(&pin.version) && hex_digest(&pin.sha256, 64)
        }))
    .then_some(lock)
}

pub(super) struct Ownership {
    pub format: SourceFormat,
    pub catalog: Option<Authority>,
    pub bases: BTreeMap<String, String>,
    pub preimages: BTreeMap<String, Option<String>>,
}

/// Qualify generated v2 lock/workflow bases against the independent source,
/// rather than trusting only a state's self-declared catalog hash and base hash.
fn known_bases(bases: &BTreeMap<String, String>) -> Result<()> {
    let catalog = catalog::reviewed()?;
    let callers = catalog.caller_workflows();
    for (path, base) in bases {
        let matches = match path.as_str() {
            ".github/workflows/armorer-ci.yml" | ".github/workflows/armorer-build.yml" => {
                callers.get(path) == Some(base)
            }
            "armorer.lock" => declared_lock(base).is_some_and(|lock| {
                catalog
                    .lock_for_config_sha256(&lock.config_sha256)
                    .is_ok_and(|text| text == *base)
            }),
            "rust-toolchain.toml" => toml::from_str::<toml::Value>(base)
                .ok()
                .and_then(|v| {
                    v.get("toolchain")?
                        .get("channel")?
                        .as_str()
                        .map(str::to_owned)
                })
                .is_some_and(|channel| {
                    semver::Version::parse(&channel)
                        .is_ok_and(|v| v.pre.is_empty() && v.build.is_empty())
                        && catalog
                            .bootstrap_files(&"0".repeat(64), &channel, "")
                            .is_ok_and(|files| files["rust-toolchain.toml"] == *base)
                }),
            ".armorer/ci-policy.toml" => true, // owner-selected policy, not publisher/catalog authority
            _ => false,
        };
        if !matches {
            return Err(Error::Transaction(
                "generated base contradicts independently reviewed catalog; no import fallback",
            ));
        }
    }
    Ok(())
}

/// Exact state preimages are decoded before inspecting Cargo; import flags do
/// not override unknown versions, ambiguous ownership or failed authenticity.
pub(super) fn source_bytes(preimages: BTreeMap<String, Option<String>>) -> Result<Ownership> {
    if preimages.len() != 3
        || !preimages.contains_key(bootstrap::V1_STATE)
        || !preimages.contains_key(bootstrap::STATE)
        || !preimages.contains_key(super::STATE)
    {
        return Err(Error::Transaction(
            "upgrade ownership inventory differs from supported versions",
        ));
    }
    let legacy = preimages[bootstrap::V1_STATE]
        .as_ref()
        .map(|b| crate::apply::state_from_bytes(b.as_bytes()))
        .transpose()?;
    let modern = preimages[bootstrap::STATE]
        .as_ref()
        .map(|b| bootstrap::transaction::state_from_bytes(b.as_bytes()))
        .transpose()?;
    let upgraded = preimages[super::STATE]
        .as_ref()
        .map(|b| super::state::decode_state(b.as_bytes()))
        .transpose()?;
    if upgraded.is_some() && (legacy.is_some() || modern.is_some()) {
        return Err(Error::Transaction(
            "multiple active ownership versions require recovery",
        ));
    }
    if legacy.is_some() && modern.is_some() {
        return Err(Error::Transaction(
            "conflicting ownership versions require explicit recovery",
        ));
    }
    let (format, catalog, bases) = if let Some(state) = upgraded {
        (
            SourceFormat::UpgradeV1,
            Some(state.authority.clone()),
            state
                .managed
                .into_iter()
                .map(|(p, b)| (p, b.generated))
                .collect(),
        )
    } else if let Some(state) = modern {
        let bases = state
            .managed
            .into_iter()
            .map(|(p, b)| (p, b.content))
            .collect();
        known_bases(&bases)?;
        (
            SourceFormat::BootstrapV2,
            Some(authority::select("bootstrap-v1")?),
            bases,
        )
    } else if let Some(state) = legacy {
        let bases = state
            .managed
            .into_iter()
            .map(|(p, b)| (p, b.content))
            .collect();
        known_bases(&bases)?;
        (SourceFormat::LegacyV1, None, bases)
    } else {
        (SourceFormat::Unmanaged, None, BTreeMap::new())
    };
    Ok(Ownership {
        format,
        catalog,
        bases,
        preimages,
    })
}

/// Read exact ownership independently of upgrade/import flags. Authentication or
/// version failure never falls back to unowned import.
fn source(root: &Path) -> Result<Ownership> {
    let mut preimages = BTreeMap::new();
    for path in [bootstrap::V1_STATE, bootstrap::STATE, super::STATE] {
        preimages.insert(
            path.into(),
            optional_bytes(root, path)?.map(|b| text(&b)).transpose()?,
        );
    }
    source_bytes(preimages)
}

fn import_decision(path: &str, current: Option<&str>, candidate: &str) -> Decision {
    let Some(current) = current else {
        return Decision::Conflict;
    };
    if current == candidate {
        return Decision::Import;
    }
    match path {
        "armorer.lock" if declared_lock(current).is_some() => Decision::Import,
        ".github/workflows/armorer-ci.yml"
            if merge::caller_extension(current, candidate, "verify") =>
        {
            Decision::Import
        }
        ".github/workflows/armorer-build.yml"
            if merge::caller_extension(current, candidate, "build") =>
        {
            Decision::Import
        }
        _ => Decision::Conflict,
    }
}

/// Recompute the merge rather than accepting caller-supplied resolved bytes.
fn change(
    path: String,
    base: Option<String>,
    current: Option<String>,
    candidate: String,
    imported: bool,
) -> Change {
    let mut decision = if imported {
        import_decision(&path, current.as_deref(), &candidate)
    } else {
        merge::decide(base.as_deref(), current.as_deref(), &candidate)
    };
    let preserves = decision == Decision::PreserveCustomization
        || (imported && path.ends_with(".yml") && decision == Decision::Import);
    let mut proposed = if decision == Decision::Conflict {
        None
    } else if preserves {
        current.clone()
    } else {
        Some(candidate.clone())
    };
    if preserves {
        let accepted = match path.as_str() {
            "armorer.lock" | "rust-toolchain.toml" => current.as_deref() == Some(&candidate),
            ".github/workflows/armorer-ci.yml" => current
                .as_deref()
                .is_some_and(|s| merge::caller_extension(s, &candidate, "verify")),
            ".github/workflows/armorer-build.yml" => current
                .as_deref()
                .is_some_and(|s| merge::caller_extension(s, &candidate, "build")),
            ".armorer/ci-policy.toml" => true,
            _ => false,
        };
        if !accepted {
            decision = Decision::Conflict;
            proposed = None;
        }
    }
    Change {
        base_sha256: hash(base.as_deref()),
        before_sha256: hash(current.as_deref()),
        candidate_sha256: digest(candidate.as_bytes()),
        after_sha256: hash(proposed.as_deref()),
        candidate_diff: bootstrap::plan::diff(&path, current.as_deref(), &candidate),
        proposed_diff: proposed
            .as_deref()
            .map(|s| bootstrap::plan::diff(&path, current.as_deref(), s)),
        path,
        decision,
        imported,
        base_content: base,
        before_content: current,
        candidate_content: candidate,
        proposed_content: proposed,
    }
}

impl Plan {
    /// Full bounded canonical approval digest, retaining exact three-way images.
    pub fn digest(&self) -> Result<String> {
        let mut value = self.clone();
        value.plan_sha256.clear();
        let bytes = serde_json::to_vec(&value).map_err(|_| Error::Json)?;
        if bytes.len() > 1_048_576 {
            return Err(Error::Transaction(
                "upgrade plan exceeds 1 MiB; no image was truncated",
            ));
        }
        Ok(digest(&bytes))
    }
}

/// Preview fixed local upgrades with an explicitly selected catalog and project
/// policy. Imports are named individually; no consuming writes, builds or network.
pub fn inspect(
    root: &Path,
    policy: &Path,
    catalog_id: &str,
    import_files: &[String],
    allow_downgrade: bool,
) -> Result<Plan> {
    let target = authority::select(catalog_id)?;
    imports(import_files)?;
    let (_, policy) = ci_policy::load(policy)?;
    inspect_bytes(root, &policy, target, import_files, allow_downgrade)
}

pub(super) fn inspect_bytes(
    root: &Path,
    policy: &[u8],
    target: Authority,
    import_files: &[String],
    allow_downgrade: bool,
) -> Result<Plan> {
    let policy_review_day = ci_policy::today()?;
    ci_policy::parse_at(policy, policy_review_day)?;
    let (intent, config) = load_config(root)?;
    if bootstrap::TARGETS.contains(&intent.policy.license_file.as_str())
        || [
            bootstrap::STATE,
            bootstrap::V1_STATE,
            bootstrap::JOURNAL,
            bootstrap::V1_JOURNAL,
            super::STATE,
            super::JOURNAL,
            ".armorer/apply.lock",
            "armorer.toml",
            "rust-toolchain",
        ]
        .contains(&intent.policy.license_file.as_str())
    {
        return Err(Error::Transaction(
            "license input overlaps migration control or generated targets",
        ));
    }
    let Ownership {
        format: source_format,
        catalog: source_catalog,
        bases,
        preimages: state_preimages,
    } = source(root)?;
    let workspace = discover(root, &intent)?;
    let current_revision = source_catalog.as_ref().map_or(0, |source| source.revision);
    let direction =
        authority::direction(current_revision, target.revision, allow_downgrade)?.into();
    let config_sha256 = digest(&config);
    let generated = catalog::reviewed()?.bootstrap_files(
        &config_sha256,
        &intent.toolchain,
        std::str::from_utf8(policy).map_err(|_| Error::Toml)?,
    )?;
    let mut input_preimages = BTreeMap::new();
    for path in [&intent.policy.license_file, &"rust-toolchain".into()] {
        input_preimages.insert(
            path.clone(),
            optional_bytes(root, path)?.as_deref().map(digest),
        );
    }
    let mut changes = Vec::new();
    for (path, candidate) in generated {
        let current = optional_bytes(root, &path)?.map(|b| text(&b)).transpose()?;
        let requested = import_files.contains(&path);
        if requested && bases.contains_key(&path) {
            return Err(Error::Transaction(
                "already-owned files cannot be reclassified as imports",
            ));
        }
        let mut c = change(
            path.clone(),
            bases.get(&path).cloned(),
            current,
            candidate,
            requested,
        );
        if path == "rust-toolchain.toml" && input_preimages["rust-toolchain"].is_some() {
            c.decision = Decision::Conflict;
            c.proposed_content = None;
            c.after_sha256 = None;
            c.proposed_diff = None;
        }
        changes.push(c);
    }
    let Metadata {
        compatibility,
        pin_changes,
        workflow_before,
        workflow_after,
    } = metadata(
        &source_format,
        &workspace,
        &input_preimages,
        &intent,
        &changes,
        policy_review_day,
    )?;
    let recovery_required = [bootstrap::JOURNAL, bootstrap::V1_JOURNAL, super::JOURNAL]
        .into_iter()
        .try_fold(false, |found, path| {
            let present = optional_bytes(root, path)?.is_some();
            Ok::<_, Error>(found || present)
        })?;
    let mut import_files = import_files.to_vec();
    import_files.sort();
    let mut plan = Plan {
        schema_version: 1,
        contract: "armorer-upgrade-plan".into(),
        runtime_version: env!("CARGO_PKG_VERSION").into(),
        plan_sha256: String::new(),
        source_format,
        source_catalog,
        target_catalog: target,
        direction,
        allow_downgrade,
        import_files,
        state_preimages,
        input_preimages,
        config_sha256,
        intent,
        workspace,
        policy_content: text(policy)?,
        policy_review_day,
        recovery_required,
        compatibility,
        pin_changes,
        workflow_before,
        workflow_after,
        changes,
    };
    plan.plan_sha256 = plan.digest()?;
    json(&plan)?;
    Ok(plan)
}

struct Metadata {
    compatibility: Vec<Compatibility>,
    pin_changes: Vec<PinChange>,
    workflow_before: Option<crate::config::WorkflowPin>,
    workflow_after: crate::config::WorkflowPin,
}

fn metadata(
    source_format: &SourceFormat,
    workspace: &Workspace,
    input_preimages: &BTreeMap<String, Option<String>>,
    intent: &Config,
    changes: &[Change],
    policy_review_day: u64,
) -> Result<Metadata> {
    let mut compatibility = vec![
        note(
            "manual-review",
            "github-capabilities-unchecked",
            "Local migration cannot establish GitHub capabilities, CI/release evidence or SLSA.",
        ),
        note(
            "supported",
            "target-catalog-authenticated",
            "Target workflow/tool identities are independently compiled immutable authority, not before-file claims.",
        ),
    ];
    if *source_format == SourceFormat::LegacyV1 {
        compatibility.push(note("manual-review","legacy-ownership-migration","V1 owns only the recorded toolchain; moving ownership to the upgrade writer needs the separate reviewed transaction."));
    }
    if *source_format == SourceFormat::Unmanaged {
        compatibility.push(note("manual-review","unowned-inputs","Unowned files stay unowned unless a specific supported import is approved; before pins are declarations only."));
    }
    if changes.iter().any(|c| c.decision == Decision::Conflict) {
        compatibility.push(note("blocked","three-way-conflict","Generated bases, current edits or import scope conflict; every original byte remains preserved."));
    }
    if !workspace.cargo_lock_present {
        compatibility.push(note(
            "manual-review",
            "cargo-lock-missing",
            "Locked CI requires Cargo.lock; discovery never creates it.",
        ));
    }
    if workspace.packages.iter().any(|p| {
        p.links.is_some()
            || p.targets
                .iter()
                .any(|t| t.kind.iter().any(|kind| kind == "custom-build"))
    }) {
        compatibility.push(note("manual-review","native-build-review","Declared native links/build scripts require target-specific dependency review before CI/build execution."));
    }
    if input_preimages[&intent.policy.license_file].is_none() {
        compatibility.push(note(
            "manual-review",
            "license-file-missing",
            "The configured project license file is missing.",
        ));
    }
    if workspace.cargo_config_present {
        compatibility.push(note(
            "manual-review",
            "cargo-config-excluded",
            "Excluded Cargo overrides require explicit review before CI/build execution.",
        ));
    }
    let result_policy = changes
        .iter()
        .find(|c| c.path == ".armorer/ci-policy.toml")
        .and_then(|c| c.proposed_content.as_ref());
    let today = policy_review_day;
    if result_policy.is_some_and(|content| ci_policy::parse_at(content.as_bytes(), today).is_err())
    {
        compatibility.push(note("blocked","result-policy-invalid","Preserved project policy must remain structurally supported and valid under current UTC expiry rules."));
    }
    let current_lock = changes
        .iter()
        .find(|c| c.path == "armorer.lock")
        .and_then(|c| c.before_content.as_deref())
        .and_then(declared_lock);
    let target_lock = declared_lock(
        changes
            .iter()
            .find(|c| c.path == "armorer.lock")
            .ok_or(Error::Transaction("lock preview missing"))?
            .candidate_content
            .as_str(),
    )
    .ok_or(Error::Transaction("compiled lock invalid"))?;
    if changes
        .iter()
        .any(|c| c.path == "armorer.lock" && c.before_content.is_some())
    {
        compatibility.push(note("manual-review","before-pins-not-authentication","Displayed before-pin syntax and hashes are observations; they cannot authenticate a historical catalog or choose new roots."));
    }
    let mut names: BTreeSet<String> = target_lock.tools.keys().cloned().collect();
    if let Some(lock) = &current_lock {
        names.extend(lock.tools.keys().cloned());
    }
    let pin_changes = names
        .into_iter()
        .filter_map(|name| {
            let before = current_lock
                .as_ref()
                .and_then(|lock| lock.tools.get(&name))
                .cloned();
            let after = target_lock.tools.get(&name).cloned();
            if before.as_ref().map(|p| (&p.version, &p.sha256))
                == after.as_ref().map(|p| (&p.version, &p.sha256))
            {
                None
            } else {
                Some(PinChange {
                    name,
                    before,
                    after,
                })
            }
        })
        .collect();
    compatibility.sort_by(|a, b| a.code.cmp(&b.code));
    Ok(Metadata {
        compatibility,
        pin_changes,
        workflow_before: current_lock.map(|l| l.workflows),
        workflow_after: target_lock.workflows,
    })
}

/// Strict bounded load for independent review. It is never a v1/v2 apply input.
pub fn load_plan(path: &Path, expected: &str) -> Result<Plan> {
    let plan: Plan = decode(&read_small(path)?)?;
    validate(&plan, expected)?;
    Ok(plan)
}

pub(super) fn validate(plan: &Plan, expected: &str) -> Result<()> {
    if !hex_digest(expected, 64)
        || plan.plan_sha256 != expected
        || plan.digest()? != expected
        || plan.schema_version != 1
        || plan.contract != "armorer-upgrade-plan"
        || plan.runtime_version != env!("CARGO_PKG_VERSION")
        || plan.target_catalog != authority::select(&plan.target_catalog.id)?
        || plan.changes.len() != bootstrap::TARGETS.len()
    {
        return Err(Error::Transaction(
            "upgrade plan does not match independently approved digest or authority",
        ));
    }
    imports(&plan.import_files)?;
    plan.intent.validate_shape()?;
    let origin = source_bytes(plan.state_preimages.clone())?;
    if plan.source_format != origin.format
        || plan.source_catalog != origin.catalog
        || plan.direction
            != authority::direction(
                origin.catalog.as_ref().map_or(0, |c| c.revision),
                plan.target_catalog.revision,
                plan.allow_downgrade,
            )?
        || !hex_digest(&plan.config_sha256, 64)
        || plan.input_preimages.len() != 2
        || !plan.input_preimages.contains_key("rust-toolchain")
        || !plan
            .input_preimages
            .contains_key(&plan.intent.policy.license_file)
        || plan
            .input_preimages
            .values()
            .flatten()
            .any(|hash| !hex_digest(hash, 64))
    {
        return Err(Error::Transaction(
            "upgrade ownership, direction or input inventory differs from contract",
        ));
    }
    let generated = catalog::reviewed()?.bootstrap_files(
        &plan.config_sha256,
        &plan.intent.toolchain,
        &plan.policy_content,
    )?;
    for (c, (path, candidate)) in plan.changes.iter().zip(generated) {
        let requested = plan.import_files.contains(&path);
        if requested && origin.bases.contains_key(&path) {
            return Err(Error::Transaction(
                "owned target cannot be reclassified as an import",
            ));
        }
        let mut derived = change(
            path.clone(),
            origin.bases.get(&path).cloned(),
            c.before_content.clone(),
            candidate.clone(),
            requested,
        );
        if path == "rust-toolchain.toml" && plan.input_preimages["rust-toolchain"].is_some() {
            derived.decision = Decision::Conflict;
            derived.proposed_content = None;
            derived.after_sha256 = None;
            derived.proposed_diff = None;
        }
        if serde_json::to_value(c).map_err(|_| Error::Json)?
            != serde_json::to_value(&derived).map_err(|_| Error::Json)?
        {
            return Err(Error::Transaction(
                "upgrade merge or import differs from stored generated base",
            ));
        }
        if c.path != path
            || c.candidate_content != candidate
            || c.candidate_sha256 != digest(candidate.as_bytes())
            || c.before_sha256 != hash(c.before_content.as_deref())
            || c.base_sha256 != hash(c.base_content.as_deref())
            || c.after_sha256 != hash(c.proposed_content.as_deref())
            || c.candidate_diff
                != bootstrap::plan::diff(&path, c.before_content.as_deref(), &candidate)
            || c.proposed_diff
                != c.proposed_content
                    .as_deref()
                    .map(|s| bootstrap::plan::diff(&path, c.before_content.as_deref(), s))
        {
            return Err(Error::Transaction(
                "upgrade preview images or target differ from fixed contract",
            ));
        }
    }
    // Historical recovery replays the policy snapshot at its reviewed UTC day;
    // new applies separately enforce the current independently observed UTC day.
    if plan.policy_review_day > 2_932_896 {
        return Err(Error::Transaction("unsupported policy review epoch day"));
    }
    ci_policy::parse_at(plan.policy_content.as_bytes(), plan.policy_review_day)?;
    let Metadata {
        compatibility,
        pin_changes: pins,
        workflow_before: before,
        workflow_after: after,
    } = metadata(
        &plan.source_format,
        &plan.workspace,
        &plan.input_preimages,
        &plan.intent,
        &plan.changes,
        plan.policy_review_day,
    )?;
    if serde_json::to_value(&plan.compatibility).map_err(|_| Error::Json)?
        != serde_json::to_value(compatibility).map_err(|_| Error::Json)?
        || serde_json::to_value(&plan.pin_changes).map_err(|_| Error::Json)?
            != serde_json::to_value(pins).map_err(|_| Error::Json)?
        || serde_json::to_value(&plan.workflow_before).map_err(|_| Error::Json)?
            != serde_json::to_value(before).map_err(|_| Error::Json)?
        || serde_json::to_value(&plan.workflow_after).map_err(|_| Error::Json)?
            != serde_json::to_value(after).map_err(|_| Error::Json)?
    {
        return Err(Error::Transaction(
            "upgrade pin, workflow or compatibility display differs from reconstructed inputs",
        ));
    }
    json(plan)?;
    Ok(())
}

/// Recheck a saved review packet against current source, ownership and preimages.
/// This is read-only and never expands v1/v2 apply authorization.
pub fn validate_fresh(root: &Path, plan: &Plan) -> Result<()> {
    let fresh = inspect_bytes(
        root,
        plan.policy_content.as_bytes(),
        authority::select(&plan.target_catalog.id)?,
        &plan.import_files,
        plan.allow_downgrade,
    )?;
    if plan.digest()? != fresh.digest()? || plan.plan_sha256 != fresh.plan_sha256 {
        return Err(Error::Transaction(
            "saved upgrade inputs, generated bases or preimages changed; review a new plan",
        ));
    }
    Ok(())
}
