//! Reviewed first-run proposals for `armorer.toml` and a CI policy draft.
//!
//! A proposal comes from read-only discovery: an isolated manifest snapshot and
//! `cargo metadata --no-deps --offline --locked` for an exact toolchain. It runs
//! no repository code, resolves no dependencies, contacts no network service and
//! writes nothing. Creating the files requires the proposal digest supplied
//! separately; files are only ever created, never overwritten or deleted.

use crate::{
    Error, Result,
    config::{
        self, Attestations, Config, Deliverable, FeatureSet, Policy, Profile, TARGETS, hex_digest,
    },
    digest,
    discovery::{self, Package, Workspace},
    filesystem::{lock, optional_bytes, replace},
    plan::Finding,
    safe_path,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Contract name carried by every proposal document.
pub const CONTRACT: &str = "armorer-init-proposal";
/// Proposal contract version.
pub const SCHEMA_VERSION: u32 = 1;
/// The configuration file `init` proposes.
pub const CONFIG_PATH: &str = "armorer.toml";
/// The unreviewed policy starter `init` proposes. It is not a managed file.
pub const POLICY_DRAFT_PATH: &str = "armorer-policy.draft.toml";

/// Checked in order; the first regular file wins.
const LICENSE_CANDIDATES: [&str; 6] = [
    "LICENSE",
    "LICENSE.md",
    "LICENSE.txt",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "COPYING",
];
const LIBRARY_KINDS: [&str; 6] = ["lib", "rlib", "cdylib", "dylib", "staticlib", "proc-macro"];
const MAX_DELIVERABLES: usize = 128;
const FEATURE_SET: &str = "default";
const MAX_BYTES: usize = 1_048_576;

const CONFIG_HEADER: &str = "\
# Proposed by `armorer init` from read-only Cargo metadata. Review every value
# before use: remove targets you do not ship, and change a long-running
# binary's profile from \"cli\" to \"service\" where that applies.
";

/// The starter policy. Every decision is left empty for the owner; the empty
/// license allowlist is rejected by `ci_policy::parse_at` until it is completed.
const POLICY_DRAFT: &str = "\
# Policy DRAFT proposed by `armorer init`. This is not reviewed policy.
#
# `armorer bootstrap plan` rejects this draft as written because
# licenses.allow is empty. Armorer has no default dependency policy. Before use:
#   1. List the SPDX license identifiers you accept for your dependencies in
#      licenses.allow, for example allow = [\"<SPDX-ID>\"].
#   2. Review advisories.exceptions, sources.allow_git and bans.
#   3. Save the reviewed copy and pass it to
#      armorer bootstrap plan --policy <reviewed-file>
schema_version = 1

[licenses]
allow = []

[advisories]
exceptions = []

[sources]
allow_git = []

[bans]
multiple_versions = \"warn\"
deny = []
";

/// Review document for `armorer init`; its digest authorizes `init --write`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    #[schemars(extend("const" = CONTRACT))]
    pub contract: String,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    pub runtime_version: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub proposal_sha256: String,
    #[schemars(regex(
        pattern = "^[A-Za-z0-9][A-Za-z0-9_.-]{0,99}/[A-Za-z0-9][A-Za-z0-9_.-]{0,99}$(?![\\s\\S])"
    ))]
    pub repository: String,
    #[schemars(regex(
        pattern = "^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$(?![\\s\\S])"
    ))]
    pub toolchain: String,
    pub workspace: Workspace,
    #[schemars(length(min = 2, max = 2))]
    pub files: Vec<ProposedFile>,
    pub findings: Vec<Finding>,
}

/// One exact file `init --write` would create.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProposedFile {
    #[schemars(extend("enum" = [POLICY_DRAFT_PATH, CONFIG_PATH]))]
    pub path: String,
    pub content: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub sha256: String,
}

/// Local file creation only. The repository is not configured or provisioned,
/// and the policy draft is not reviewed.
#[derive(Debug, Serialize)]
pub struct Receipt {
    pub contract: &'static str,
    pub schema_version: u32,
    pub outcome: &'static str,
    pub proposal_sha256: String,
    pub written: Vec<String>,
    pub configured: bool,
    pub policy_reviewed: bool,
    pub ci_verified: bool,
    pub release_rehearsed: bool,
    pub published: bool,
    pub provenance_verified: bool,
}

impl Proposal {
    /// SHA-256 of compact typed JSON with only this digest field blank, bounded at 1 MiB.
    pub fn digest(&self) -> Result<String> {
        let mut value = self.clone();
        value.proposal_sha256.clear();
        let bytes = serde_json::to_vec(&value).map_err(|_| Error::Json)?;
        if bytes.len() > MAX_BYTES {
            return Err(Error::Invalid("init proposal exceeds 1 MiB".into()));
        }
        Ok(digest(&bytes))
    }
}

/// Dispatch the CLI form: a read-only proposal, or creation under an approved digest.
pub fn run(
    root: &Path,
    repository: &str,
    toolchain: Option<&str>,
    expected: Option<&str>,
) -> Result<serde_json::Value> {
    match expected {
        Some(expected) => serde_json::to_value(write(root, repository, toolchain, expected)?),
        None => serde_json::to_value(propose(root, repository, toolchain)?),
    }
    .map_err(|_| Error::Json)
}

/// Build a deterministic proposal from read-only discovery.
///
/// # Errors
/// Returns `Error::Invalid` for an invalid repository name, an existing
/// `armorer.toml`, a missing or inexact toolchain, unsupported package or binary
/// names, no proposable deliverable or more than 128 of them. Propagates path,
/// discovery and selection errors. Error messages never echo input values.
pub fn propose(root: &Path, repository: &str, toolchain: Option<&str>) -> Result<Proposal> {
    if !config::repository_name(repository) {
        return Err(Error::Invalid(
            "--repo-name must be a GitHub owner/name".into(),
        ));
    }
    let root = root.canonicalize()?;
    require_uninitialized(&root)?;
    let toolchain_differs = toolchain_file_differs(&root, toolchain)?;
    let toolchain = select_toolchain(&root, toolchain)?;
    let workspace = discovery::discover_workspace(&root, &toolchain)?;
    let selection = select_deliverables(&workspace.packages)?;
    let license = license_file(&root)?;
    let proposed = Config {
        schema_version: config::VERSION,
        repository: repository.to_owned(),
        toolchain: toolchain.clone(),
        manifest: "Cargo.toml".into(),
        deliverables: selection.deliverables,
        feature_sets: BTreeMap::from([(
            FEATURE_SET.to_owned(),
            FeatureSet {
                default_features: true,
                features: Vec::new(),
            },
        )]),
        policy: Policy {
            license_file: license.unwrap_or(LICENSE_CANDIDATES[0]).to_owned(),
            attestations: Attestations::Required,
        },
    };
    let config_text = render_config(&proposed)?;
    // Valid by construction; re-check with the loader and discovery selection rules anyway.
    let parsed =
        config::parse_config(config_text.as_bytes(), &root).map_err(|error| match error {
            Error::Toml | Error::TomlAt { .. } | Error::Invalid(_) => defect(),
            other => other,
        })?;
    if serde_json::to_value(&parsed).map_err(|_| Error::Json)?
        != serde_json::to_value(&proposed).map_err(|_| Error::Json)?
    {
        return Err(defect());
    }
    discovery::validate_selection(&parsed, &workspace.packages)?;

    let mut findings = vec![
        finding(
            "policy-draft-incomplete",
            "armorer-policy.draft.toml has an empty licenses.allow list; bootstrap plan rejects it until the owner lists the dependency licenses they accept.",
        ),
        finding(
            "targets-review",
            "Every supported target is proposed; remove the targets the project does not ship.",
        ),
    ];
    if parsed
        .deliverables
        .iter()
        .any(|d| d.profile == Profile::Cli)
    {
        findings.push(finding(
            "profile-review",
            "Every binary target is proposed as a cli deliverable; change a long-running binary's profile to service where that applies.",
        ));
    }
    if license.is_none() {
        findings.push(finding(
            "license-file-missing",
            "No license file was found; the proposal names LICENSE, which the owner must add or replace.",
        ));
    }
    if selection.skipped_targets > 0 {
        findings.push(Finding {
            code: "required-features-skipped".into(),
            detail: format!(
                "{} binary target(s) were skipped because default features do not enable their required features.",
                selection.skipped_targets
            ),
        });
    }
    if selection.skipped_packages > 0 {
        findings.push(Finding {
            code: "package-without-deliverable".into(),
            detail: format!(
                "{} workspace package(s) have no binary or library target enabled by default features; no deliverable was proposed for them.",
                selection.skipped_packages
            ),
        });
    }
    if toolchain_differs {
        findings.push(finding(
            "toolchain-file-differs",
            "An existing rust-toolchain.toml or legacy rust-toolchain file does not pin the proposed toolchain; bootstrap plan will report it as a conflict until the owner resolves it.",
        ));
    }
    if exists(&root, POLICY_DRAFT_PATH)? {
        findings.push(finding(
            "policy-draft-exists",
            "armorer-policy.draft.toml already exists; init --write never overwrites it, so move it aside before writing.",
        ));
    }
    findings.sort_by(|a, b| a.code.cmp(&b.code));

    let mut files = vec![
        proposed_file(CONFIG_PATH, config_text),
        proposed_file(POLICY_DRAFT_PATH, POLICY_DRAFT.to_owned()),
    ];
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut proposal = Proposal {
        contract: CONTRACT.into(),
        schema_version: SCHEMA_VERSION,
        runtime_version: env!("CARGO_PKG_VERSION").into(),
        proposal_sha256: String::new(),
        repository: parsed.repository,
        toolchain,
        workspace,
        files,
        findings,
    };
    proposal.proposal_sha256 = proposal.digest()?;
    if serde_json::to_vec_pretty(&proposal)
        .map_err(|_| Error::Json)?
        .len()
        > MAX_BYTES
    {
        return Err(Error::Invalid("init proposal exceeds 1 MiB".into()));
    }
    Ok(proposal)
}

/// Create the proposed files after recomputing the proposal and matching the approved digest.
///
/// The policy draft is created first and `armorer.toml` last, each only if
/// absent. Nothing is overwritten or deleted; if the second creation fails the
/// error says the draft remains.
///
/// # Errors
/// Returns `Error::Transaction` for a malformed or stale digest, an existing
/// destination, a concurrent Armorer transaction or a failed creation, and
/// propagates every [`propose`] error.
pub fn write(
    root: &Path,
    repository: &str,
    toolchain: Option<&str>,
    expected: &str,
) -> Result<Receipt> {
    if !cfg!(unix) {
        return Err(Error::Transaction(
            "durable transactions require a supported Unix host",
        ));
    }
    let stale = || {
        Error::Transaction(
            "init proposal does not match the approved digest; review a fresh proposal",
        )
    };
    if !hex_digest(expected, 64) {
        return Err(stale());
    }
    let root = root.canonicalize()?;
    // Check before locking so a refused write leaves no lock directory behind.
    let proposal = propose(&root, repository, toolchain)?;
    if proposal.proposal_sha256 != expected {
        return Err(stale());
    }
    require_destinations_absent(&root)?;
    let _guard = lock(&root)?;
    let proposal = propose(&root, repository, toolchain)?;
    if proposal.proposal_sha256 != expected || proposal.digest()? != expected {
        return Err(stale());
    }
    require_destinations_absent(&root)?;
    let content = |path: &str| {
        proposal
            .files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.content.as_bytes())
            .ok_or(Error::Transaction("init proposal file is missing"))
    };
    if let Err(cause) = replace(&root, POLICY_DRAFT_PATH, content(POLICY_DRAFT_PATH)?, true) {
        return Err(failed_creation(
            &root,
            POLICY_DRAFT_PATH,
            [
                "armorer-policy.draft.toml was not created; nothing was written or deleted",
                "armorer-policy.draft.toml exists but init could not confirm it wrote it durably (the write or its directory sync failed); nothing was deleted, so inspect it and move it aside before rerunning init",
                "armorer-policy.draft.toml may or may not exist; nothing was deleted, so inspect the workspace and move any draft aside before rerunning init",
            ],
            cause,
        ));
    }
    if let Err(cause) = replace(&root, CONFIG_PATH, content(CONFIG_PATH)?, true) {
        return Err(failed_creation(
            &root,
            CONFIG_PATH,
            [
                "armorer-policy.draft.toml was created but armorer.toml was not; nothing was deleted, so move the draft aside before rerunning init",
                "armorer-policy.draft.toml and armorer.toml exist but init could not confirm it wrote armorer.toml durably (the write or its directory sync failed); nothing was deleted, so inspect both files before any recovery",
                "armorer-policy.draft.toml was created and armorer.toml may or may not exist; nothing was deleted, so inspect both files before any recovery",
            ],
            cause,
        ));
    }
    Ok(Receipt {
        contract: "armorer-init-receipt",
        schema_version: SCHEMA_VERSION,
        outcome: "written",
        proposal_sha256: proposal.proposal_sha256,
        written: vec![POLICY_DRAFT_PATH.into(), CONFIG_PATH.into()],
        configured: false,
        policy_reviewed: false,
        ci_verified: false,
        release_rehearsed: false,
        published: false,
        provenance_verified: false,
    })
}

/// Classify a failed `replace` by checking whether the destination now exists.
///
/// `replace` renames the file into place and only then syncs the directory, so an
/// error can leave `relative` behind. `messages` is `[absent, present, unknown]`.
fn failed_creation(
    root: &Path,
    relative: &str,
    messages: [&'static str; 3],
    cause: Error,
) -> Error {
    let message = match exists(root, relative) {
        Ok(false) => messages[0],
        Ok(true) => messages[1],
        Err(_) => messages[2],
    };
    Error::TransactionCause {
        message,
        cause: Box::new(cause),
    }
}

fn defect() -> Error {
    Error::Invalid("init produced an armorer.toml that does not validate; this is an Armorer defect and nothing was proposed".into())
}

fn finding(code: &str, detail: &str) -> Finding {
    Finding {
        code: code.into(),
        detail: detail.into(),
    }
}

fn proposed_file(path: &str, content: String) -> ProposedFile {
    ProposedFile {
        path: path.into(),
        sha256: digest(content.as_bytes()),
        content,
    }
}

/// Report whether a plain relative path exists, refusing symlinks in it.
fn exists(root: &Path, relative: &str) -> Result<bool> {
    match std::fs::symlink_metadata(safe_path(root, relative)?) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// Refuse an existing (or symlinked) `armorer.toml`.
fn require_uninitialized(root: &Path) -> Result<()> {
    if exists(root, CONFIG_PATH)? {
        return Err(Error::Invalid(
            "repository is already initialized: armorer.toml exists; review or edit it instead of running init".into(),
        ));
    }
    Ok(())
}

/// Both files must be absent before any write; creation itself also refuses to clobber.
fn require_destinations_absent(root: &Path) -> Result<()> {
    for path in [POLICY_DRAFT_PATH, CONFIG_PATH] {
        if exists(root, path)? {
            return Err(Error::Transaction(
                "an init destination already exists; nothing was written",
            ));
        }
    }
    Ok(())
}

/// Use an explicit exact toolchain, else an exact `toolchain.channel` in `rust-toolchain.toml`.
/// Never defaults to `stable` or to Armorer's own toolchain.
fn select_toolchain(root: &Path, explicit: Option<&str>) -> Result<String> {
    if let Some(toolchain) = explicit {
        return if config::exact_version(toolchain) {
            Ok(toolchain.to_owned())
        } else {
            Err(Error::Invalid(
                "--toolchain must be an exact stable x.y.z version".into(),
            ))
        };
    }
    let missing = || {
        Error::Invalid(
            "rust-toolchain.toml has no exact stable x.y.z channel; pass --toolchain x.y.z".into(),
        )
    };
    let bytes = optional_bytes(root, "rust-toolchain.toml")?.ok_or_else(missing)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| missing())?;
    let value: toml::Value = toml::from_str(text).map_err(|_| missing())?;
    value
        .get("toolchain")
        .and_then(|toolchain| toolchain.get("channel"))
        .and_then(toml::Value::as_str)
        .filter(|channel| config::exact_version(channel))
        .map(str::to_owned)
        .ok_or_else(missing)
}

/// Report whether an existing toolchain file disagrees with the proposal.
///
/// A legacy `rust-toolchain` file always differs: bootstrap treats it as a
/// conflict. With an explicit toolchain, a readable `rust-toolchain.toml` whose
/// `toolchain.channel` is not exactly that version differs. Absent files, and
/// `rust-toolchain.toml` bytes that are not UTF-8 TOML, add nothing; without an
/// explicit toolchain the file is the toolchain's source and [`select_toolchain`]
/// handles it. Filesystem and unsafe-path errors propagate.
fn toolchain_file_differs(root: &Path, explicit: Option<&str>) -> Result<bool> {
    if exists(root, "rust-toolchain")? {
        return Ok(true);
    }
    let Some(toolchain) = explicit else {
        return Ok(false);
    };
    let Some(bytes) = optional_bytes(root, "rust-toolchain.toml")? else {
        return Ok(false);
    };
    // A file that is not UTF-8 TOML cannot pin anything, and bootstrap will conflict on it.
    let Some(value) = std::str::from_utf8(&bytes)
        .ok()
        .and_then(|text| toml::from_str::<toml::Value>(text).ok())
    else {
        return Ok(true);
    };
    Ok(value
        .get("toolchain")
        .and_then(|t| t.get("channel"))
        .and_then(toml::Value::as_str)
        != Some(toolchain))
}

/// The first candidate that is a regular file directly under the root.
fn license_file(root: &Path) -> Result<Option<&'static str>> {
    for name in LICENSE_CANDIDATES {
        match std::fs::symlink_metadata(root.join(name)) {
            Ok(metadata) if metadata.file_type().is_file() => return Ok(Some(name)),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(None)
}

/// Serialize with the `toml` crate so the text is valid by construction.
fn render_config(config: &Config) -> Result<String> {
    let body = toml::to_string(config)
        .map_err(|_| Error::Invalid("proposed armorer.toml could not be serialized".into()))?;
    Ok(format!("{CONFIG_HEADER}{body}"))
}

struct Selection {
    deliverables: Vec<Deliverable>,
    skipped_targets: usize,
    skipped_packages: usize,
}

/// One cli deliverable per default-enabled bin target; one library deliverable
/// for a package with a library target and no bin targets.
fn select_deliverables(packages: &[Package]) -> Result<Selection> {
    let mut ordered: Vec<&Package> = packages.iter().collect();
    ordered.sort_by(|a, b| a.name.cmp(&b.name));
    let mut ids = BTreeSet::new();
    let mut selection = Selection {
        deliverables: Vec::new(),
        skipped_targets: 0,
        skipped_packages: 0,
    };
    for package in ordered {
        let mut active = BTreeSet::new();
        discovery::enable_closure(package, true, &mut active);
        let enabled = |required: &[String]| required.iter().all(|f| active.contains(f));
        let mut bins: Vec<_> = package
            .targets
            .iter()
            .filter(|target| target.kind.iter().any(|kind| kind == "bin"))
            .collect();
        bins.sort_by(|a, b| a.name.cmp(&b.name));
        let before = selection.deliverables.len();
        if bins.is_empty() {
            let library = package.targets.iter().any(|target| {
                target
                    .kind
                    .iter()
                    .any(|kind| LIBRARY_KINDS.contains(&kind.as_str()))
                    && enabled(&target.required_features)
            });
            if library {
                selection.deliverables.push(deliverable(
                    unique_id(&package.name, &mut ids)?,
                    Profile::Library,
                    package,
                    None,
                )?);
            }
        } else {
            for bin in bins {
                if enabled(&bin.required_features) {
                    selection.deliverables.push(deliverable(
                        unique_id(&bin.name, &mut ids)?,
                        Profile::Cli,
                        package,
                        Some(&bin.name),
                    )?);
                } else {
                    selection.skipped_targets += 1;
                }
            }
        }
        if selection.deliverables.len() == before {
            selection.skipped_packages += 1;
        }
        if selection.deliverables.len() > MAX_DELIVERABLES {
            return Err(Error::Invalid(
                "workspace has more than 128 deliverables; write armorer.toml by hand".into(),
            ));
        }
    }
    if selection.deliverables.is_empty() {
        return Err(Error::Invalid(
            "no binary or library target is enabled by default features; write armorer.toml by hand".into(),
        ));
    }
    Ok(selection)
}

fn unsupported_name() -> Error {
    Error::Invalid(
        "a package or binary name is not supported as an armorer.toml identifier; write armorer.toml by hand".into(),
    )
}

fn deliverable(
    id: String,
    profile: Profile,
    package: &Package,
    binary: Option<&str>,
) -> Result<Deliverable> {
    if !config::identifier(&package.name) || binary.is_some_and(|b| !config::identifier(b)) {
        return Err(unsupported_name());
    }
    Ok(Deliverable {
        id,
        profile,
        package: package.name.clone(),
        binary: binary.map(str::to_owned),
        targets: TARGETS.iter().map(|t| (*t).to_owned()).collect(),
        feature_set: FEATURE_SET.into(),
    })
}

/// Use `base`, else the first free `base-2`, `base-3`, ...; fail closed if no valid id results.
fn unique_id(base: &str, ids: &mut BTreeSet<String>) -> Result<String> {
    let valid = |id: &str| config::identifier(id) && !id.contains("--");
    if !valid(base) {
        return Err(unsupported_name());
    }
    let mut candidate = base.to_owned();
    let mut suffix = 1_usize;
    while ids.contains(&candidate) {
        suffix += 1;
        candidate = format!("{base}-{suffix}");
        if !valid(&candidate) {
            return Err(unsupported_name());
        }
    }
    ids.insert(candidate.clone());
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::Target;

    fn target(name: &str, kind: &str, required: &[&str]) -> Target {
        Target {
            name: name.into(),
            kind: vec![kind.into()],
            required_features: required.iter().map(|f| (*f).to_owned()).collect(),
        }
    }

    fn package(name: &str, targets: Vec<Target>, features: &[(&str, &[&str])]) -> Package {
        Package {
            name: name.into(),
            version: "0.1.0".into(),
            license: None,
            rust_version: None,
            links: None,
            features: features
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.iter().map(|f| (*f).to_owned()).collect()))
                .collect(),
            targets,
        }
    }

    #[test]
    /// A failed creation reports whether the file is absent, present or unknowable.
    fn failed_creation_reports_the_actual_file_state() {
        let root = tempfile::tempdir().unwrap();
        let messages = ["absent", "present", "unknown"];
        let message = |relative: &str| match failed_creation(
            root.path(),
            relative,
            messages,
            Error::Transaction("simulated"),
        ) {
            Error::TransactionCause { message, .. } => message,
            _ => panic!("expected a transaction cause"),
        };
        assert_eq!(message("armorer.toml"), "absent");
        std::fs::write(root.path().join("armorer.toml"), "x").unwrap();
        assert_eq!(message("armorer.toml"), "present");
        assert_eq!(message("../escape"), "unknown");
    }

    #[test]
    /// Collisions take the first free numeric suffix; names that cannot become ids fail closed.
    fn unique_ids_are_deterministic_and_fail_closed() {
        let mut ids = BTreeSet::new();
        assert_eq!(unique_id("tool", &mut ids).unwrap(), "tool");
        assert_eq!(unique_id("tool", &mut ids).unwrap(), "tool-2");
        assert_eq!(unique_id("tool-2", &mut ids).unwrap(), "tool-2-2");
        assert_eq!(unique_id("tool", &mut ids).unwrap(), "tool-3");
        let too_long = "x".repeat(101);
        for bad in ["a--b", "-lead", "has.dot", "", too_long.as_str()] {
            assert!(matches!(unique_id(bad, &mut ids), Err(Error::Invalid(_))));
        }
        // A maximal name is valid alone but cannot take a suffix.
        let long = "y".repeat(100);
        assert_eq!(unique_id(&long, &mut ids).unwrap(), long);
        assert!(unique_id(&long, &mut ids).is_err());
        // A trailing hyphen would yield `--` on collision.
        unique_id("edge-", &mut ids).unwrap();
        assert!(unique_id("edge-", &mut ids).is_err());
    }

    #[test]
    /// Bins become cli deliverables, bin-less libraries become library deliverables and
    /// disabled required features are counted, never named.
    fn selection_follows_targets_and_default_features() {
        let packages = vec![
            package(
                "tools",
                vec![
                    target("beta", "bin", &[]),
                    target("alpha", "bin", &["extra"]),
                    target("tools", "lib", &[]),
                ],
                &[("default", &["extra"]), ("extra", &[])],
            ),
            package("core", vec![target("core", "lib", &[])], &[]),
            package(
                "gated",
                vec![target("gated", "bin", &["off"])],
                &[("off", &[])],
            ),
            package("examples-only", vec![target("demo", "example", &[])], &[]),
        ];
        let selection = select_deliverables(&packages).unwrap();
        let summary: Vec<_> = selection
            .deliverables
            .iter()
            .map(|d| {
                (
                    d.id.as_str(),
                    d.profile,
                    d.package.as_str(),
                    d.binary.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("core", Profile::Library, "core", None),
                ("alpha", Profile::Cli, "tools", Some("alpha")),
                ("beta", Profile::Cli, "tools", Some("beta")),
            ]
        );
        assert_eq!(selection.skipped_targets, 1);
        assert_eq!(selection.skipped_packages, 2);
        assert!(
            selection
                .deliverables
                .iter()
                .all(|d| d.targets.len() == TARGETS.len() && d.feature_set == FEATURE_SET)
        );
    }

    #[test]
    /// Collisions across packages are suffixed; unsupported names and oversize workspaces fail closed.
    fn selection_collisions_names_and_limits() {
        let packages = vec![
            package("a", vec![target("run", "bin", &[])], &[]),
            package("b", vec![target("run", "bin", &[])], &[]),
            package("run-2", vec![target("run-2", "lib", &[])], &[]),
        ];
        let ids: Vec<_> = select_deliverables(&packages)
            .unwrap()
            .deliverables
            .into_iter()
            .map(|d| d.id)
            .collect();
        assert_eq!(ids, ["run", "run-2", "run-2-2"]);
        let bad = vec![package("bad--name", vec![target("x", "lib", &[])], &[])];
        assert!(matches!(
            select_deliverables(&bad),
            Err(Error::Invalid(message)) if message.contains("not supported") && !message.contains("bad")
        ));
        let many: Vec<_> = (0..129)
            .map(|i| package(&format!("p{i:03}"), vec![target("lib", "lib", &[])], &[]))
            .collect();
        assert!(matches!(
            select_deliverables(&many),
            Err(Error::Invalid(message)) if message.contains("by hand")
        ));
        let none = vec![package("x", vec![target("t", "test", &[])], &[])];
        assert!(select_deliverables(&none).is_err());
    }

    #[test]
    /// The draft has the version-one shape, is rejected while empty and accepted once completed.
    fn policy_draft_is_rejected_until_licenses_are_listed() {
        let draft: crate::ci_policy::CiPolicy = toml::from_str(POLICY_DRAFT).unwrap();
        assert!(draft.licenses.allow.is_empty());
        assert!(crate::ci_policy::parse_at(POLICY_DRAFT.as_bytes(), 20_000).is_err());
        let completed = POLICY_DRAFT.replace("allow = []", "allow = [\"MIT\"]");
        crate::ci_policy::parse_at(completed.as_bytes(), 20_000).unwrap();
    }

    #[test]
    /// Rendered configuration round-trips through the loader's parser unchanged.
    fn rendered_config_round_trips() {
        let root = tempfile::tempdir().unwrap();
        let packages = vec![package("cli", vec![target("cli", "bin", &[])], &[])];
        let config = Config {
            schema_version: 1,
            repository: "example/cli".into(),
            toolchain: "1.95.0".into(),
            manifest: "Cargo.toml".into(),
            deliverables: select_deliverables(&packages).unwrap().deliverables,
            feature_sets: BTreeMap::from([(
                FEATURE_SET.to_owned(),
                FeatureSet {
                    default_features: true,
                    features: Vec::new(),
                },
            )]),
            policy: Policy {
                license_file: "LICENSE".into(),
                attestations: Attestations::Required,
            },
        };
        let text = render_config(&config).unwrap();
        assert!(text.starts_with("# Proposed by `armorer init`"));
        let parsed = config::parse_config(text.as_bytes(), root.path()).unwrap();
        assert_eq!(
            serde_json::to_value(parsed).unwrap(),
            serde_json::to_value(config).unwrap()
        );
    }

    #[test]
    /// Only an explicit or pinned exact stable version is accepted.
    fn toolchain_selection_never_guesses() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            select_toolchain(root.path(), Some("1.95.0")).unwrap(),
            "1.95.0"
        );
        for bad in ["stable", "1.95", "1.95.0-beta.1", "nightly-2026-01-01"] {
            assert!(select_toolchain(root.path(), Some(bad)).is_err());
        }
        assert!(select_toolchain(root.path(), None).is_err());
        let file = root.path().join("rust-toolchain.toml");
        for bad in [
            "[toolchain]\nchannel = \"stable\"\n",
            "[toolchain]\nchannel = \"1.95\"\n",
            "[toolchain]\nprofile = \"minimal\"\n",
            "not toml [",
        ] {
            std::fs::write(&file, bad).unwrap();
            assert!(select_toolchain(root.path(), None).is_err());
        }
        std::fs::write(&file, "[toolchain]\nchannel = \"1.94.1\"\n").unwrap();
        assert_eq!(select_toolchain(root.path(), None).unwrap(), "1.94.1");
        // An explicit flag wins over the file.
        assert_eq!(
            select_toolchain(root.path(), Some("1.95.0")).unwrap(),
            "1.95.0"
        );
    }

    #[test]
    /// Only an existing, readable toolchain file that does not pin the proposal differs.
    fn toolchain_file_difference() {
        let root = tempfile::tempdir().unwrap();
        assert!(!toolchain_file_differs(root.path(), Some("1.95.0")).unwrap());
        let file = root.path().join("rust-toolchain.toml");
        for (content, differs) in [
            ("[toolchain]\nchannel = \"1.95.0\"\n", false),
            ("[toolchain]\nchannel = \"1.94.0\"\n", true),
            ("[toolchain]\nchannel = \"stable\"\n", true),
            ("[toolchain]\nprofile = \"minimal\"\n", true),
            ("not toml [", true),
        ] {
            std::fs::write(&file, content).unwrap();
            assert_eq!(
                toolchain_file_differs(root.path(), Some("1.95.0")).unwrap(),
                differs,
                "{content}"
            );
            // Without a flag the file is the source, not a difference.
            assert!(!toolchain_file_differs(root.path(), None).unwrap());
        }
        std::fs::write(root.path().join("rust-toolchain"), "1.95.0\n").unwrap();
        assert!(toolchain_file_differs(root.path(), Some("1.95.0")).unwrap());
        assert!(toolchain_file_differs(root.path(), None).unwrap());
    }

    #[test]
    /// The first regular candidate wins; symlinks and directories are not license files.
    fn license_file_prefers_first_regular_candidate() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(license_file(root.path()).unwrap(), None);
        std::fs::write(root.path().join("COPYING"), "text").unwrap();
        std::fs::create_dir(root.path().join("LICENSE")).unwrap();
        assert_eq!(license_file(root.path()).unwrap(), Some("COPYING"));
        std::fs::write(root.path().join("LICENSE-MIT"), "text").unwrap();
        assert_eq!(license_file(root.path()).unwrap(), Some("LICENSE-MIT"));
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("LICENSE-MIT", root.path().join("LICENSE.md")).unwrap();
            assert_eq!(license_file(root.path()).unwrap(), Some("LICENSE-MIT"));
        }
    }
}
