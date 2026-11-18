//! Bootstrap catalog authority compiled into the reviewed Armorer source.
//!
//! Authentication precedes parsing. Consuming files and API observations cannot
//! choose an expected hash, workflow revision or a moving version reference.
use crate::{
    Error, Result,
    config::{Config, Lock, RepositoryFiles, ToolPin, WorkflowPin, load_config, repository_name},
    digest,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub const WORKFLOW_REPOSITORY: &str = "brianluby/armorer-workflows";
/// Test-only predecessor pin (ADR 0012 rule 3); production builds cannot select it.
#[cfg(feature = "test-predecessor")]
pub const WORKFLOW_COMMIT: &str = "772ca83e386c883c88cc3b936d69f8cb3216c91e";
/// Accepted workflows head pinned by the separately reviewed bootstrap-v2 successor.
/// Settled at workflows main `e5e9c0d31baff87e357046d81068b646a39a9f23`: `pins/tools.json`
/// is byte-identical to bootstrap-v1 there (and at `b94969c`), and the bootstrap callers
/// rust-ci.yml/rust-build.yml are unchanged between them, so v2 inherits the PR #23
/// hardening. Both entries authenticate the same embedded digest; only the workflow pin moves.
pub const WORKFLOW_COMMIT_V2: &str = "e5e9c0d31baff87e357046d81068b646a39a9f23";
/// Runtime contract frozen by the reviewed bootstrap catalogs and recorded in every
/// lock they render. Patch releases keep this value, so a catalog id renders the
/// same lock bytes across a patch line; see `crate::runtime`.
pub const RUNTIME_VERSION: &str = "0.1.0";
pub const TOOLS_SHA256: &str = "e21e6cbd2cdd1125dbb9817116b1f46d043807b4d92128abc9dc07256dbb4b13";
const TOOLS: &[u8] = include_bytes!("../catalogs/bootstrap-tools-v1.json");
/// UTC date the bootstrap-v2 catalog pins were last reviewed. Source: the catalog source
/// receipt `catalogs/bootstrap-tools-v1-source.json` (`verified_on`), restated in
/// `docs/bootstrap-catalog.md`; it equals the author date of the last commit that changed
/// the catalog JSON. Update it in the same reviewed change as any catalog update.
pub const CATALOG_REVIEWED_ON: &str = "2026-10-02";
/// Days after `CATALOG_REVIEWED_ON` before the pins are reported stale.
pub const REVIEW_WINDOW_DAYS: u64 = 180;

/// Catalog review age against the window. Kept outside plans so digests stay time-free.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Freshness {
    pub reviewed_on: &'static str,
    pub age_days: u64,
    pub window_days: u64,
    pub stale: bool,
}

/// Pure freshness for a UTC day number (days since the Unix epoch). A clock earlier
/// than the review date is age 0, never stale.
///
/// # Errors
/// Fails only if the compiled review date is malformed (guarded by a unit test).
pub fn freshness_at(today_utc_day: u64) -> Result<Freshness> {
    let reviewed = crate::ci_policy::utc_day(CATALOG_REVIEWED_ON)?;
    let age_days = today_utc_day.saturating_sub(reviewed);
    Ok(Freshness {
        reviewed_on: CATALOG_REVIEWED_ON,
        age_days,
        window_days: REVIEW_WINDOW_DAYS,
        stale: age_days > REVIEW_WINDOW_DAYS,
    })
}

/// Freshness against the UTC wall clock.
///
/// # Errors
/// Fails if the clock precedes the Unix epoch.
pub fn freshness_now() -> Result<Freshness> {
    freshness_at(crate::ci_policy::today()?)
}

/// The single stderr line printed when the catalog is stale; `None` when fresh.
#[must_use]
pub fn stale_warning(freshness: &Freshness) -> Option<String> {
    freshness.stale.then(|| {
        format!(
            "armorer: warning: catalog pins were last reviewed {} days ago (window {}); update Armorer or review the catalog",
            freshness.age_days, freshness.window_days
        )
    })
}

/// Add `catalog-review-stale` to a release blocker list when stale; result stays sorted.
#[must_use]
pub fn with_review_blocker(mut blockers: Vec<String>, freshness: &Freshness) -> Vec<String> {
    if freshness.stale {
        blockers.push("catalog-review-stale".into());
        blockers.sort();
        blockers.dedup();
    }
    blockers
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ToolCatalog {
    schema_version: u32,
    tools: BTreeMap<String, Tool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub version: String,
    pub binary: String,
    pub platforms: BTreeMap<String, Distribution>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Distribution {
    pub url: String,
    pub sha256: String,
    pub format: String,
}

/// Only constructible by comparison with the independently compiled trust anchor.
/// This proves catalog membership in trusted CLI source, not Sigstore/Apple/L2.
#[derive(Debug, Serialize)]
pub struct AuthenticatedCatalog {
    schema_version: u32,
    workflow: WorkflowPin,
    tools_sha256: &'static str,
    tools: BTreeMap<String, Tool>,
    /// Compiled managed-file table of this catalog entry; not part of the printed catalog.
    #[serde(skip)]
    files: &'static [ManagedFile],
}

/// Authenticate exact bytes before structural interpretation. The expected identity
/// is deliberately not a function parameter or part of a downloaded document.
///
/// # Errors
/// Rejects oversized, substituted or malformed catalog bytes. No network or
/// consuming-repository operation occurs; upgrades require a reviewed CLI source change.
pub fn authenticate(bytes: &[u8]) -> Result<AuthenticatedCatalog> {
    authenticate_with(bytes, WORKFLOW_COMMIT_V2, BOOTSTRAP_FILES)
}

/// Hash-before-parse against the compiled digest, binding the given compiled workflow
/// pin and managed-file table.
fn authenticate_with(
    bytes: &[u8],
    workflow_commit: &str,
    files: &'static [ManagedFile],
) -> Result<AuthenticatedCatalog> {
    if bytes.len() > 1_048_576 || digest(bytes) != TOOLS_SHA256 {
        return Err(Error::Invalid(
            "bootstrap catalog authentication failed".into(),
        ));
    }
    let catalog: ToolCatalog = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
    if catalog.schema_version != 1 {
        return Err(Error::Invalid("unsupported bootstrap catalog".into()));
    }
    Ok(AuthenticatedCatalog {
        schema_version: 1,
        workflow: WorkflowPin {
            repository: WORKFLOW_REPOSITORY.into(),
            commit: workflow_commit.into(),
        },
        tools_sha256: TOOLS_SHA256,
        tools: catalog.tools,
        files,
    })
}

/// Load the test-only predecessor catalog (ADR 0012 rule 3). Not compiled into
/// production builds; bootstrap-v2 is the only production authority.
#[cfg(feature = "test-predecessor")]
pub fn reviewed() -> Result<AuthenticatedCatalog> {
    reviewed_for("bootstrap-v1")
}

/// Load the reviewed bootstrap-v2 successor: same authenticated tool bytes, accepted workflows head.
pub fn reviewed_v2() -> Result<AuthenticatedCatalog> {
    reviewed_for("bootstrap-v2")
}

/// Every compiled catalog id. Production builds compile only `bootstrap-v2`; the
/// test-only entries exist solely behind the `test-predecessor` feature.
const COMPILED: &[&str] = &[
    #[cfg(feature = "test-predecessor")]
    "bootstrap-v1",
    "bootstrap-v2",
    #[cfg(feature = "test-predecessor")]
    "bootstrap-test-wide",
];

/// Compiled registry entry: workflow pin and managed-file table. Unknown ids have none.
fn registry(id: &str) -> Option<(&'static str, &'static [ManagedFile])> {
    match id {
        #[cfg(feature = "test-predecessor")]
        "bootstrap-v1" => Some((WORKFLOW_COMMIT, BOOTSTRAP_FILES)),
        "bootstrap-v2" => Some((WORKFLOW_COMMIT_V2, BOOTSTRAP_FILES)),
        #[cfg(feature = "test-predecessor")]
        "bootstrap-test-wide" => Some((WORKFLOW_COMMIT_TEST_WIDE, TEST_WIDE_FILES)),
        _ => None,
    }
}

/// Ids of every catalog compiled into this build, in registry order.
pub fn compiled_ids() -> &'static [&'static str] {
    COMPILED
}

/// Select the compiled catalog for a registry id. Unknown ids never fall back.
pub fn reviewed_for(id: &str) -> Result<AuthenticatedCatalog> {
    let (commit, files) =
        registry(id).ok_or_else(|| Error::Invalid("unknown bootstrap catalog authority".into()))?;
    authenticate_with(TOOLS, commit, files)
}

/// Map a compiled workflow pin back to its registry id. Unknown commits have no authority.
/// Every compiled entry has a distinct pin, so the mapping is unambiguous.
pub(crate) fn id_for_commit(commit: &str) -> Option<&'static str> {
    COMPILED
        .iter()
        .copied()
        .find(|id| registry(id).is_some_and(|(pin, _)| pin == commit))
}

/// Sorted, unique union of the managed paths of every compiled catalog. This is a
/// safe upper bound for guards that run before any catalog governs the document.
pub fn all_managed_paths() -> Vec<&'static str> {
    COMPILED
        .iter()
        .filter_map(|id| registry(id))
        .flat_map(|(_, files)| files.iter().map(|file| file.path))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Configuration inputs that select and render the optional repository files.
/// Every other managed file ignores them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Selection<'a> {
    /// `owner/name` substituted into `SECURITY.md`.
    pub repository: &'a str,
    /// Which optional files are switched on, and the code owners.
    pub files: &'a RepositoryFiles,
}

impl<'a> From<&'a Config> for Selection<'a> {
    fn from(config: &'a Config) -> Self {
        Self {
            repository: &config.repository,
            files: &config.repository_files,
        }
    }
}

impl AuthenticatedCatalog {
    /// The MAXIMAL set: sorted, unique repository-relative paths this catalog can
    /// ever render and may own, for any configuration. Derived from the same compiled
    /// table as [`AuthenticatedCatalog::bootstrap_files`], so the declared set and the
    /// rendered set cannot diverge. Documents that carry no configuration are bounded
    /// by this set.
    pub fn managed_paths(&self) -> Vec<&'static str> {
        self.files
            .iter()
            .map(|file| file.path)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// The SELECTED set: sorted, unique paths this catalog renders for a configuration
    /// with these optional-file switches. Always a subset of
    /// [`AuthenticatedCatalog::managed_paths`]; with every switch off it is the set of
    /// unconditional entries.
    pub fn selected_paths(&self, files: &RepositoryFiles) -> Vec<&'static str> {
        self.files
            .iter()
            .filter(|file| file.when.selected(files))
            .map(|file| file.path)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Read the embedded catalog tool entries after the catalog authority has been authenticated.
    pub fn tools(&self) -> &BTreeMap<String, Tool> {
        &self.tools
    }

    /// Render a compatible v1 lock for the exact locally validated configuration.
    /// All three distributions of each tool have explicit target-qualified IDs,
    /// avoiding an ambiguous single-platform digest for multi-target consumers.
    /// No moving pins, caller URLs, installation, adoption or mutation is performed.
    ///
    /// # Errors
    /// Propagates read-only configuration validation and serialization failures.
    pub fn render_lock(&self, root: &Path) -> Result<String> {
        let (_, bytes) = load_config(root)?;
        self.lock_for_config_sha256(&digest(&bytes))
    }

    /// Render fixed pins for a digest already bound to the approved configuration.
    pub(crate) fn lock_for_config_sha256(&self, config_sha256: &str) -> Result<String> {
        let tools = self
            .tools
            .iter()
            .flat_map(|(name, tool)| {
                tool.platforms.iter().map(move |(target, distribution)| {
                    (
                        format!("{name}--{target}"),
                        ToolPin {
                            version: tool.version.clone(),
                            sha256: distribution.sha256.clone(),
                        },
                    )
                })
            })
            .collect::<BTreeMap<_, _>>();
        if tools.len()
            != self
                .tools
                .values()
                .map(|t| t.platforms.len())
                .sum::<usize>()
        {
            return Err(Error::Invalid("duplicate tool pin id in catalog".into()));
        }
        let lock = Lock {
            schema_version: 1,
            config_sha256: config_sha256.into(),
            runtime_version: RUNTIME_VERSION.into(),
            workflows: self.workflow.clone(),
            tools,
        };
        toml::to_string_pretty(&lock).map_err(|_| Error::Toml)
    }

    /// The SELECTED managed file set for an already validated configuration and
    /// policy, rendered entry by entry from this catalog's compiled table. Its keys
    /// are exactly [`AuthenticatedCatalog::selected_paths`] for `selection.files`.
    pub(crate) fn bootstrap_files(
        &self,
        config_sha256: &str,
        toolchain: &str,
        policy: &str,
        selection: Selection<'_>,
    ) -> Result<BTreeMap<String, String>> {
        // Shared boundary: enforce the exact-stable contract here, not only at call sites.
        if !semver::Version::parse(toolchain).is_ok_and(|v| v.pre.is_empty() && v.build.is_empty())
        {
            return Err(Error::Invalid(
                "toolchain must be an exact stable x.y.z version".into(),
            ));
        }
        let mut files = BTreeMap::new();
        for file in self.files {
            if !file.when.selected(selection.files) {
                continue;
            }
            let content = match file.source {
                Source::Caller(template) => self.render_caller(template),
                Source::Toolchain => {
                    render_template(TOOLCHAIN_TEMPLATE, &[("TOOLCHAIN", toolchain)])?
                }
                Source::Lock => self.lock_for_config_sha256(config_sha256)?,
                Source::Policy => policy.into(),
                Source::Fixed(template) => render_template(template, &[])?,
                Source::SecurityPolicy(template) => {
                    render_security_policy(template, selection.repository)?
                }
                Source::Dependabot(template) => render_template(
                    template,
                    &[("WORKFLOW_REPOSITORY", self.workflow.repository.as_str())],
                )?,
                Source::CodeOwners => render_code_owners(selection.files)?,
            };
            if files.insert(file.path.to_owned(), content).is_some() {
                return Err(Error::Invalid("duplicate managed path in catalog".into()));
            }
        }
        Ok(files)
    }

    /// Fixed unprivileged callers. Existing custom workflows remain separate.
    /// The build caller is manual and unsigned; it has no publication capability.
    /// CI cancels superseded runs only for pull requests, so every pushed commit
    /// keeps a completed run.
    ///
    /// # Panics
    /// Never on user input: the templates and the workflow pin are compile-time
    /// constants, and `compiled_templates_render_for_every_catalog` renders every
    /// compiled catalog. A failure here is a build defect and fails closed.
    pub fn caller_workflows(&self) -> BTreeMap<String, String> {
        self.files
            .iter()
            .filter_map(|file| match file.source {
                Source::Caller(template) => {
                    Some((file.path.to_owned(), self.render_caller(template)))
                }
                _ => None,
            })
            .collect()
    }

    /// Managed files whose bytes depend only on this catalog (no placeholders at all).
    pub(crate) fn fixed_files(&self) -> Result<BTreeMap<String, String>> {
        self.files
            .iter()
            .filter_map(|file| match file.source {
                Source::Fixed(template) => {
                    Some(render_template(template, &[]).map(|text| (file.path.to_owned(), text)))
                }
                _ => None,
            })
            .collect()
    }

    /// Render one caller template with this catalog's compiled workflow pin.
    fn render_caller(&self, template: &str) -> String {
        let values = [
            ("WORKFLOW_REPOSITORY", self.workflow.repository.as_str()),
            ("WORKFLOW_COMMIT", self.workflow.commit.as_str()),
        ];
        let Ok(text) = render_template(template, &values) else {
            unreachable!("compiled caller template must render with compiled pins");
        };
        text
    }

    /// The compiled workflow identity bound to this authenticated catalog.
    pub(crate) fn workflow(&self) -> &WorkflowPin {
        &self.workflow
    }

    /// Decide whether `base`, recorded for an optional entry by a document that does
    /// not carry its configuration, is exactly what this catalog renders for some valid
    /// configuration. Returns `None` when `path` is not one of this catalog's optional
    /// entries, so callers keep their own rule for every other path.
    pub(crate) fn renders_optional_base(&self, path: &str, base: &str) -> Option<bool> {
        let file = self.files.iter().find(|file| file.path == path)?;
        let renders = match file.source {
            Source::SecurityPolicy(template) => template
                .split_once(REPOSITORY_PLACEHOLDER)
                .and_then(|(prefix, suffix)| base.strip_prefix(prefix)?.strip_suffix(suffix))
                .is_some_and(|repository| {
                    render_security_policy(template, repository).is_ok_and(|text| text == base)
                }),
            Source::Dependabot(template) => render_template(
                template,
                &[("WORKFLOW_REPOSITORY", self.workflow.repository.as_str())],
            )
            .is_ok_and(|text| text == base),
            Source::CodeOwners => base
                .lines()
                .find_map(|line| line.strip_prefix("* "))
                .is_some_and(|owners| {
                    let files = RepositoryFiles {
                        code_owners: owners.split(' ').map(str::to_owned).collect(),
                        ..RepositoryFiles::default()
                    };
                    render_code_owners(&files).is_ok_and(|text| text == base)
                }),
            Source::Caller(_)
            | Source::Toolchain
            | Source::Lock
            | Source::Policy
            | Source::Fixed(_) => return None,
        };
        Some(renders)
    }
}

const REPOSITORY_PLACEHOLDER: &str = "%%REPOSITORY%%";

/// Render the security policy for a syntactically valid `owner/name`.
fn render_security_policy(template: &str, repository: &str) -> Result<String> {
    if !repository_name(repository) {
        return Err(Error::Invalid("repository must be owner/name".into()));
    }
    render_template(template, &[("REPOSITORY", repository)])
}

/// Paths `.github/CODEOWNERS` assigns to the configured owners after the catch-all
/// `*` rule: the managed control files and the security policy.
const CODE_OWNER_PATHS: [&str; 6] = [
    "/.github/",
    "/.armorer/",
    "/armorer.toml",
    "/armorer.lock",
    "/rust-toolchain.toml",
    "/SECURITY.md",
];

/// Render `.github/CODEOWNERS` from a validated, non-empty owner list. The variable
/// owner count makes this code, not a placeholder template. Owners are revalidated
/// and, like template values, may contain no control character.
fn render_code_owners(files: &RepositoryFiles) -> Result<String> {
    let invalid = || Error::Invalid("invalid code owners".into());
    files.validate().map_err(|_| invalid())?;
    if files.code_owners.is_empty()
        || files
            .code_owners
            .iter()
            .any(|owner| owner.chars().any(char::is_control))
    {
        return Err(invalid());
    }
    let owners = files.code_owners.join(" ");
    let mut text = String::from(
        "# Managed by Armorer from repository_files.code_owners in armorer.toml; edit that list instead of this file.\n\
         # Code-owner review takes effect only when a ruleset or branch protection requires it.\n",
    );
    text.push_str(&format!("* {owners}\n"));
    for path in CODE_OWNER_PATHS {
        text.push_str(&format!("{path} {owners}\n"));
    }
    Ok(text)
}

const CI_CALLER_TEMPLATE: &str = include_str!("../catalogs/templates/armorer-ci.yml.tmpl");
const BUILD_CALLER_TEMPLATE: &str = include_str!("../catalogs/templates/armorer-build.yml.tmpl");
const TOOLCHAIN_TEMPLATE: &str = include_str!("../catalogs/templates/rust-toolchain.toml.tmpl");

/// How a managed file's bytes are produced. Consuming input selects neither a path
/// nor a source; both are compiled into the catalog entry.
#[derive(Debug, Clone, Copy)]
enum Source {
    /// Unprivileged caller workflow rendered with the catalog's workflow pin.
    Caller(&'static str),
    /// Toolchain file rendered from the exact stable toolchain.
    Toolchain,
    /// Target-qualified lock bound to the approved configuration digest.
    Lock,
    /// Owner-supplied project policy bytes, carried verbatim.
    Policy,
    /// Fixed template text with no placeholders. Only the test-only wide catalog
    /// uses it today, so production builds construct no such entry.
    #[cfg_attr(not(feature = "test-predecessor"), allow(dead_code))]
    Fixed(&'static str),
    /// Security policy template with the configured repository substituted.
    SecurityPolicy(&'static str),
    /// Dependency-update template with the catalog's workflow repository substituted.
    Dependabot(&'static str),
    /// Code owners rendered in code from the configured owner list.
    CodeOwners,
}

/// When a catalog entry is rendered. Unconditional entries form the set every
/// configuration gets; each optional entry is rendered only when its switch in
/// `[repository_files]` selects it, and every switch is off by default.
#[derive(Debug, Clone, Copy)]
enum When {
    Always,
    SecurityPolicy,
    Dependabot,
    CodeOwners,
}

impl When {
    /// Whether this entry belongs to the selected set for these switches.
    fn selected(self, files: &RepositoryFiles) -> bool {
        match self {
            Self::Always => true,
            Self::SecurityPolicy => files.security_policy,
            Self::Dependabot => files.dependabot,
            Self::CodeOwners => !files.code_owners.is_empty(),
        }
    }
}

/// One repository-relative file a catalog renders and may own.
#[derive(Debug, Clone, Copy)]
struct ManagedFile {
    path: &'static str,
    source: Source,
    when: When,
}

const SECURITY_POLICY_TEMPLATE: &str = include_str!("../catalogs/templates/SECURITY.md.tmpl");
const DEPENDABOT_TEMPLATE: &str = include_str!("../catalogs/templates/dependabot.yml.tmpl");

/// The managed file set of `bootstrap-v2` (and the test-only `bootstrap-v1`), in
/// sorted path order. This table is the single source of truth for which files a
/// catalog manages: `bootstrap_files`, `caller_workflows`, `managed_paths` (the
/// maximal set) and `selected_paths` all derive from it. Adding an unconditional
/// entry changes every consumer's plan; the optional entries are off by default, so
/// they change no plan until a configuration selects them.
const BOOTSTRAP_FILES: &[ManagedFile] = &[
    ManagedFile {
        path: ".armorer/ci-policy.toml",
        source: Source::Policy,
        when: When::Always,
    },
    ManagedFile {
        path: ".github/CODEOWNERS",
        source: Source::CodeOwners,
        when: When::CodeOwners,
    },
    ManagedFile {
        path: ".github/dependabot.yml",
        source: Source::Dependabot(DEPENDABOT_TEMPLATE),
        when: When::Dependabot,
    },
    ManagedFile {
        path: ".github/workflows/armorer-build.yml",
        source: Source::Caller(BUILD_CALLER_TEMPLATE),
        when: When::Always,
    },
    ManagedFile {
        path: ".github/workflows/armorer-ci.yml",
        source: Source::Caller(CI_CALLER_TEMPLATE),
        when: When::Always,
    },
    ManagedFile {
        path: "SECURITY.md",
        source: Source::SecurityPolicy(SECURITY_POLICY_TEMPLATE),
        when: When::SecurityPolicy,
    },
    ManagedFile {
        path: "armorer.lock",
        source: Source::Lock,
        when: When::Always,
    },
    ManagedFile {
        path: "rust-toolchain.toml",
        source: Source::Toolchain,
        when: When::Always,
    },
];

/// Clearly fake, test-only workflow pin that gives `bootstrap-test-wide` a distinct
/// identity for `id_for_commit`. It names no real commit; production builds lack it.
#[cfg(feature = "test-predecessor")]
pub const WORKFLOW_COMMIT_TEST_WIDE: &str = "7e577e577e577e577e577e577e577e577e577e57";

#[cfg(feature = "test-predecessor")]
const MANAGED_NOTE_TEMPLATE: &str = include_str!("../catalogs/templates/armorer-managed.md.tmpl");

/// Test-only `bootstrap-test-wide` (ADR 0013): `bootstrap-v2` plus one fixed file,
/// proving that a catalog defines its own managed file set end to end. It carries the
/// same optional entries as `bootstrap-v2`, so it stays exactly one file wider.
#[cfg(feature = "test-predecessor")]
const TEST_WIDE_FILES: &[ManagedFile] = &[
    ManagedFile {
        path: ".armorer/ci-policy.toml",
        source: Source::Policy,
        when: When::Always,
    },
    ManagedFile {
        path: ".github/CODEOWNERS",
        source: Source::CodeOwners,
        when: When::CodeOwners,
    },
    ManagedFile {
        path: ".github/armorer-managed.md",
        source: Source::Fixed(MANAGED_NOTE_TEMPLATE),
        when: When::Always,
    },
    ManagedFile {
        path: ".github/dependabot.yml",
        source: Source::Dependabot(DEPENDABOT_TEMPLATE),
        when: When::Dependabot,
    },
    ManagedFile {
        path: ".github/workflows/armorer-build.yml",
        source: Source::Caller(BUILD_CALLER_TEMPLATE),
        when: When::Always,
    },
    ManagedFile {
        path: ".github/workflows/armorer-ci.yml",
        source: Source::Caller(CI_CALLER_TEMPLATE),
        when: When::Always,
    },
    ManagedFile {
        path: "SECURITY.md",
        source: Source::SecurityPolicy(SECURITY_POLICY_TEMPLATE),
        when: When::SecurityPolicy,
    },
    ManagedFile {
        path: "armorer.lock",
        source: Source::Lock,
        when: When::Always,
    },
    ManagedFile {
        path: "rust-toolchain.toml",
        source: Source::Toolchain,
        when: When::Always,
    },
];

/// Render a template with fixed `%%NAME%%` placeholders in one left-to-right pass.
/// Substituted values are never rescanned, so a value cannot introduce or complete
/// a placeholder. Every `%%` opens a placeholder whose name is one or more of `A-Z`
/// and `_`, closed by `%%`; anything else after `%%` is an error. A lone `%` is
/// literal text. Errors never echo values.
fn render_template(template: &str, values: &[(&str, &str)]) -> Result<String> {
    let invalid = || Error::Invalid("invalid file template".into());
    if values
        .iter()
        .any(|(_, v)| v.contains("%%") || v.chars().any(char::is_control))
    {
        return Err(invalid());
    }
    let mut used = vec![false; values.len()];
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(at) = rest.find("%%") {
        out.push_str(&rest[..at]);
        let after = &rest[at + 2..];
        let len = after
            .find(|c: char| !(c.is_ascii_uppercase() || c == '_'))
            .unwrap_or(after.len());
        let (name, tail) = after.split_at(len);
        if name.is_empty() {
            return Err(invalid());
        }
        let tail = tail.strip_prefix("%%").ok_or_else(invalid)?;
        let index = values
            .iter()
            .position(|(n, _)| *n == name)
            .ok_or_else(invalid)?;
        used[index] = true;
        out.push_str(values[index].1);
        rest = tail;
    }
    out.push_str(rest);
    if used.contains(&false) {
        return Err(invalid());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const V2_GOLDENS: &[(&str, &str)] = &[
        (
            ".github/workflows/armorer-build.yml",
            "30ff9cbd05cd26659cf7f28274f236762ba2d3fc6518b57cfa7a0d47cc12b715",
        ),
        (
            ".github/workflows/armorer-ci.yml",
            "edfb0ff48f128193445b0f35e2022913a2e0adb71007fb59d8d535cdf4f392c5",
        ),
        (
            "rust-toolchain.toml",
            "24ef3b9d3edbd850aa386cb0a98e10450b0030991a4537cb359f54d49dbbb33a",
        ),
    ];
    const V1_GOLDENS: &[(&str, &str)] = &[
        (
            ".github/workflows/armorer-build.yml",
            "d54919c9b41be91ffb665eb631a27aeed4ebb482339a64977a968798645f1ed1",
        ),
        (
            ".github/workflows/armorer-ci.yml",
            "2478570952c760df003bd956521b126d936bfc28098797082b2ad30767c005e8",
        ),
        (
            "rust-toolchain.toml",
            "24ef3b9d3edbd850aa386cb0a98e10450b0030991a4537cb359f54d49dbbb33a",
        ),
    ];

    /// Rendered bytes of every template file are frozen for each compiled catalog; a
    /// template edit changes every consumer's bytes and must update these on purpose.
    /// Digests were captured from the former inline `format!` renderers.
    #[test]
    fn compiled_templates_render_for_every_catalog() {
        for (catalog, goldens) in [
            (reviewed().unwrap(), V1_GOLDENS),
            (reviewed_v2().unwrap(), V2_GOLDENS),
        ] {
            let mut files = catalog.caller_workflows();
            let all = catalog
                .bootstrap_files(&"a".repeat(64), "1.95.0", "", off())
                .unwrap();
            files.insert(
                "rust-toolchain.toml".into(),
                all["rust-toolchain.toml"].clone(),
            );
            assert_eq!(files.len(), goldens.len());
            for (path, expected) in goldens {
                assert_eq!(digest(files[*path].as_bytes()), *expected, "{path}");
            }
        }
    }

    #[test]
    fn renderer_rejects_unknown_unused_and_unsafe_values() {
        assert_eq!(
            render_template("a %%X%% b", &[("X", "1")]).unwrap(),
            "a 1 b"
        );
        assert!(render_template("a %%Y%%", &[("X", "1")]).is_err());
        assert!(render_template("a %%X", &[("X", "1")]).is_err());
        assert!(render_template("a", &[("X", "1")]).is_err());
        for bad in ["a%%b", "a\nb", "a\u{7}b", "a\rb"] {
            assert!(render_template("%%X%%", &[("X", bad)]).is_err(), "{bad:?}");
        }
        let err = render_template("%%X%%", &[("X", "se%%cret")]).unwrap_err();
        assert!(!err.to_string().contains("secret"));
    }

    #[test]
    fn renderer_never_reexpands_values_and_passes_literals_through() {
        let out = render_template("%%A%%|%%B%%", &[("A", "%B"), ("B", "x")]).unwrap();
        assert_eq!(out, "%B|x");
        let out = render_template("%%A%%|%%B%%", &[("A", "B%"), ("B", "%x")]).unwrap();
        assert_eq!(out, "B%|%x");
        let lit = "g: ${{ github.ref }} ${{ a == 'b' }} me@example.com\n";
        assert_eq!(render_template(lit, &[]).unwrap(), lit);
        assert_eq!(
            render_template("a@%%A%%@b", &[("A", "1")]).unwrap(),
            "a@1@b"
        );
    }

    #[test]
    fn renderer_allows_lone_percent_but_rejects_malformed_placeholders() {
        assert_eq!(render_template("100% a%b", &[]).unwrap(), "100% a%b");
        for bad in ["%%", "%%%%", "%%A", "%%a%%", "%%A_1%%", "%%A %%", "x%%"] {
            assert!(render_template(bad, &[("A", "1")]).is_err(), "{bad:?}");
        }
    }

    /// The settled v2 successor pins workflows main e5e9c0d and authenticates the same
    /// embedded tool bytes as frozen v1; only the workflow pin moves, never the digest.
    #[test]
    fn v2_successor_pins_the_settled_commit_and_authenticates_the_embedded_bytes() {
        assert_eq!(WORKFLOW_COMMIT, "772ca83e386c883c88cc3b936d69f8cb3216c91e");
        assert_eq!(
            WORKFLOW_COMMIT_V2,
            "e5e9c0d31baff87e357046d81068b646a39a9f23"
        );
        assert_eq!(digest(TOOLS), TOOLS_SHA256);
        let v1 = reviewed().unwrap();
        let v2 = reviewed_v2().unwrap();
        assert_eq!(v1.workflow().commit, WORKFLOW_COMMIT);
        assert_eq!(v2.workflow().commit, WORKFLOW_COMMIT_V2);
        assert_eq!(v1.tools_sha256, TOOLS_SHA256);
        assert_eq!(v2.tools_sha256, TOOLS_SHA256);
        assert_eq!(v2.workflow().repository, WORKFLOW_REPOSITORY);
        // Registry ids map back to their compiled pins; unknown commits have no authority.
        assert_eq!(id_for_commit(WORKFLOW_COMMIT), Some("bootstrap-v1"));
        assert_eq!(id_for_commit(WORKFLOW_COMMIT_V2), Some("bootstrap-v2"));
        assert_eq!(
            id_for_commit(WORKFLOW_COMMIT_TEST_WIDE),
            Some("bootstrap-test-wide")
        );
        assert_eq!(id_for_commit(&"a".repeat(40)), None);
        // Substituted or reformatted bytes fail successor authentication too.
        let mut bad = TOOLS.to_vec();
        bad.push(b'\n');
        assert!(authenticate_with(&bad, WORKFLOW_COMMIT_V2, BOOTSTRAP_FILES).is_err());
    }

    /// Each compiled table is sorted and unique, its declared paths are exactly the
    /// keys it renders, and every compiled entry has its own workflow identity.
    #[test]
    fn every_compiled_table_declares_exactly_what_it_renders() {
        let mut pins = BTreeSet::new();
        for id in compiled_ids() {
            let (pin, files) = registry(id).unwrap();
            assert!(pins.insert(pin), "{id}: workflow pin reused");
            assert_eq!(id_for_commit(pin), Some(*id));
            assert!(
                files.windows(2).all(|pair| pair[0].path < pair[1].path),
                "{id}: table must be sorted and unique"
            );
            let catalog = reviewed_for(id).unwrap();
            // Every switch on renders the maximal set; every switch off the selected
            // default set. Both come from the same table as the declared paths.
            let on = all_on();
            let rendered = catalog
                .bootstrap_files(
                    &"a".repeat(64),
                    "1.95.0",
                    "",
                    Selection {
                        repository: "example/pilot",
                        files: &on,
                    },
                )
                .unwrap();
            let declared = catalog.managed_paths();
            assert_eq!(
                rendered.keys().map(String::as_str).collect::<Vec<_>>(),
                declared,
                "{id}"
            );
            assert_eq!(catalog.selected_paths(&on), declared, "{id}");
            let default = catalog
                .bootstrap_files(&"a".repeat(64), "1.95.0", "", off())
                .unwrap();
            assert_eq!(
                default.keys().map(String::as_str).collect::<Vec<_>>(),
                catalog.selected_paths(&RepositoryFiles::default()),
                "{id}"
            );
            for optional in OPTIONAL {
                assert!(declared.contains(&optional), "{id}: {optional}");
                assert!(!default.contains_key(optional), "{id}: {optional}");
            }
            for path in catalog.caller_workflows().keys() {
                assert!(declared.contains(&path.as_str()), "{id}: {path}");
            }
            for path in &declared {
                assert!(all_managed_paths().contains(path), "{id}: {path}");
            }
        }
        assert!(reviewed_for("bootstrap-v3").is_err());
    }

    /// The test-only wide catalog is bootstrap-v2 plus exactly one fixed file.
    #[test]
    fn test_wide_catalog_is_bootstrap_v2_plus_one_fixed_file() {
        let v2 = reviewed_v2().unwrap();
        let wide = reviewed_for("bootstrap-test-wide").unwrap();
        let mut expected = v2.managed_paths();
        expected.push(".github/armorer-managed.md");
        expected.sort_unstable();
        assert_eq!(wide.managed_paths(), expected);
        // Maximal sets: the unconditional files plus the three optional entries.
        assert_eq!(wide.managed_paths().len(), 9);
        assert_eq!(v2.managed_paths().len(), 8);
        // Selected sets with every switch off: exactly the former six and five.
        assert_eq!(wide.selected_paths(&OFF).len(), 6);
        assert_eq!(v2.selected_paths(&OFF).len(), 5);
        assert_eq!(all_managed_paths(), expected);
        let fixed = wide.fixed_files().unwrap();
        assert_eq!(fixed.len(), 1);
        assert_eq!(fixed[".github/armorer-managed.md"], MANAGED_NOTE_TEMPLATE);
        assert!(v2.fixed_files().unwrap().is_empty());
        assert_eq!(wide.workflow().commit, WORKFLOW_COMMIT_TEST_WIDE);
        let on = all_on();
        for files in [&OFF, &on] {
            let selection = Selection {
                repository: "example/pilot",
                files,
            };
            let inputs = (&"b".repeat(64), "1.95.0", "policy");
            let mut narrow = wide
                .bootstrap_files(inputs.0, inputs.1, inputs.2, selection)
                .unwrap();
            narrow.remove(".github/armorer-managed.md");
            let v2_files = v2
                .bootstrap_files(inputs.0, inputs.1, inputs.2, selection)
                .unwrap();
            assert_eq!(
                narrow.keys().collect::<Vec<_>>(),
                v2_files.keys().collect::<Vec<_>>()
            );
            // Only the workflow pin differs between the shared files.
            for (path, text) in &v2_files {
                assert_eq!(
                    narrow[path],
                    text.replace(WORKFLOW_COMMIT_V2, WORKFLOW_COMMIT_TEST_WIDE),
                    "{path}"
                );
            }
        }
    }

    #[test]
    /// A catalog renders the frozen runtime contract, so its lock bytes do not move
    /// with patch releases of the running binary.
    fn rendered_lock_records_the_frozen_runtime_contract() {
        let text = reviewed_v2()
            .unwrap()
            .lock_for_config_sha256(&"a".repeat(64))
            .unwrap();
        let lock: Lock = toml::from_str(&text).unwrap();
        assert_eq!(lock.runtime_version, RUNTIME_VERSION);
        assert_eq!(RUNTIME_VERSION, "0.1.0");
        assert!(crate::runtime::compatible(RUNTIME_VERSION));
    }

    #[test]
    /// The shared boundary rejects non-exact toolchains before TOML interpolation.
    fn bootstrap_files_reject_non_exact_toolchain() {
        let catalog = reviewed().unwrap();
        for bad in ["1.95", "1.95.0-rc.1", "1.95.0\nx = \"y", "stable"] {
            assert!(
                catalog
                    .bootstrap_files(&"a".repeat(64), bad, "", off())
                    .is_err(),
                "accepted {bad:?}"
            );
        }
        assert!(
            catalog
                .bootstrap_files(&"a".repeat(64), "1.95.0", "", off())
                .is_ok()
        );
    }

    fn day() -> u64 {
        crate::ci_policy::utc_day(CATALOG_REVIEWED_ON).unwrap()
    }

    #[test]
    fn freshness_is_stale_only_after_the_window() {
        let at = freshness_at(day() + REVIEW_WINDOW_DAYS).unwrap();
        assert_eq!(at.age_days, REVIEW_WINDOW_DAYS);
        assert!(!at.stale);
        let past = freshness_at(day() + REVIEW_WINDOW_DAYS + 1).unwrap();
        assert_eq!(past.age_days, REVIEW_WINDOW_DAYS + 1);
        assert!(past.stale);
        assert_eq!(past.reviewed_on, CATALOG_REVIEWED_ON);
        assert_eq!(past.window_days, 180);
    }

    #[test]
    fn clock_before_the_review_date_is_age_zero_and_fresh() {
        let early = freshness_at(day() - 40).unwrap();
        assert_eq!((early.age_days, early.stale), (0, false));
        assert!(!freshness_at(0).unwrap().stale);
    }

    #[test]
    fn review_date_constant_is_a_sane_calendar_date() {
        // Shape and calendar validity come from the shared strict parser.
        let reviewed = crate::ci_policy::utc_day(CATALOG_REVIEWED_ON)
            .expect("CATALOG_REVIEWED_ON must be a valid YYYY-MM-DD date");
        // Sanity bounds derived from the constant: not before the catalog source receipt
        // (2026-10-02) and no more than a window-length past it as a typo guard.
        let receipt = crate::ci_policy::utc_day("2026-10-02").unwrap();
        assert!(
            reviewed >= receipt,
            "CATALOG_REVIEWED_ON predates the catalog receipt"
        );
        assert!(
            reviewed <= receipt + 10 * REVIEW_WINDOW_DAYS,
            "CATALOG_REVIEWED_ON is implausibly far after the catalog receipt"
        );
    }

    #[test]
    fn stale_decisions_are_pure() {
        let fresh = freshness_at(day()).unwrap();
        let stale = freshness_at(day() + REVIEW_WINDOW_DAYS + 5).unwrap();
        assert_eq!(stale_warning(&fresh), None);
        assert_eq!(
            stale_warning(&stale).unwrap(),
            "armorer: warning: catalog pins were last reviewed 185 days ago (window 180); update Armorer or review the catalog"
        );
        let blockers = vec!["a-gate".to_string(), "z-gate".to_string()];
        assert_eq!(with_review_blocker(blockers.clone(), &fresh), blockers);
        assert_eq!(
            with_review_blocker(blockers, &stale),
            ["a-gate", "catalog-review-stale", "z-gate"]
        );
    }

    /// The three optional entries every compiled table carries.
    const OPTIONAL: [&str; 3] = [
        ".github/CODEOWNERS",
        ".github/dependabot.yml",
        "SECURITY.md",
    ];

    /// Every optional switch off: the configuration every existing repository has.
    static OFF: RepositoryFiles = RepositoryFiles {
        security_policy: false,
        dependabot: false,
        code_owners: Vec::new(),
    };

    /// Rendering inputs with every optional file switched off.
    fn off() -> Selection<'static> {
        Selection {
            repository: "example/pilot",
            files: &OFF,
        }
    }

    /// Every optional file switched on, with two owners.
    fn all_on() -> RepositoryFiles {
        RepositoryFiles {
            security_policy: true,
            dependabot: true,
            code_owners: vec!["@brianluby".into(), "@example-org/platform".into()],
        }
    }

    /// Golden digests of the two optional templates for repository `example/pilot`;
    /// a template edit changes every opted-in consumer's bytes and must update these.
    const OPTIONAL_GOLDENS: &[(&str, &str)] = &[
        (
            ".github/dependabot.yml",
            "9338749f7825aee7e368c1fdadb92a73cfb60eac0a558d96fd7644a7f2b37741",
        ),
        (
            "SECURITY.md",
            "ec5608b9a346775561f0535f1ea77b4d2c1f71512b825c47fd09a3b392302601",
        ),
    ];

    #[test]
    fn optional_templates_render_their_golden_bytes() {
        let on = all_on();
        for catalog in [reviewed().unwrap(), reviewed_v2().unwrap()] {
            let files = catalog
                .bootstrap_files(
                    &"a".repeat(64),
                    "1.95.0",
                    "",
                    Selection {
                        repository: "example/pilot",
                        files: &on,
                    },
                )
                .unwrap();
            for (path, expected) in OPTIONAL_GOLDENS {
                assert_eq!(digest(files[*path].as_bytes()), *expected, "{path}");
            }
            assert!(
                files["SECURITY.md"]
                    .contains("https://github.com/example/pilot/security/advisories/new\n")
            );
            assert!(
                files[".github/dependabot.yml"]
                    .contains("- dependency-name: brianluby/armorer-workflows*\n")
            );
        }
    }

    #[test]
    fn code_owners_render_exactly_from_the_configured_list() {
        let text = render_code_owners(&all_on()).unwrap();
        assert_eq!(
            text,
            "# Managed by Armorer from repository_files.code_owners in armorer.toml; edit that list instead of this file.\n\
             # Code-owner review takes effect only when a ruleset or branch protection requires it.\n\
             * @brianluby @example-org/platform\n\
             /.github/ @brianluby @example-org/platform\n\
             /.armorer/ @brianluby @example-org/platform\n\
             /armorer.toml @brianluby @example-org/platform\n\
             /armorer.lock @brianluby @example-org/platform\n\
             /rust-toolchain.toml @brianluby @example-org/platform\n\
             /SECURITY.md @brianluby @example-org/platform\n"
        );
        for owners in [
            vec![],
            vec!["@a\nb".to_owned()],
            vec!["@a\u{7}".to_owned()],
            vec!["owner".to_owned()],
            vec!["@a".to_owned(), "@A".to_owned()],
        ] {
            let files = RepositoryFiles {
                code_owners: owners,
                ..RepositoryFiles::default()
            };
            let error = render_code_owners(&files).unwrap_err();
            assert_eq!(
                error.to_string(),
                "invalid configuration: invalid code owners"
            );
        }
    }

    /// Bases recorded without their configuration are accepted only when they are
    /// exactly what the catalog renders for some valid configuration.
    #[test]
    fn optional_bases_are_recognized_only_in_their_rendered_shape() {
        let catalog = reviewed_v2().unwrap();
        let on = all_on();
        let files = catalog
            .bootstrap_files(
                &"a".repeat(64),
                "1.95.0",
                "",
                Selection {
                    repository: "example/pilot",
                    files: &on,
                },
            )
            .unwrap();
        for path in OPTIONAL {
            assert_eq!(
                catalog.renders_optional_base(path, &files[path]),
                Some(true)
            );
            let edited = format!("{}# edit\n", files[path]);
            assert_eq!(catalog.renders_optional_base(path, &edited), Some(false));
            assert_eq!(catalog.renders_optional_base(path, ""), Some(false));
        }
        let other = files["SECURITY.md"].replace("example/pilot", "other/repo");
        assert_eq!(
            catalog.renders_optional_base("SECURITY.md", &other),
            Some(true)
        );
        let bad = files["SECURITY.md"].replace("example/pilot", "../x");
        assert_eq!(
            catalog.renders_optional_base("SECURITY.md", &bad),
            Some(false)
        );
        let owners = files[".github/CODEOWNERS"].replace("@example-org/platform", "owner");
        assert_eq!(
            catalog.renders_optional_base(".github/CODEOWNERS", &owners),
            Some(false)
        );
        assert_eq!(catalog.renders_optional_base("armorer.lock", ""), None);
        assert_eq!(catalog.renders_optional_base("unknown", ""), None);
        assert_eq!(
            SECURITY_POLICY_TEMPLATE
                .matches(REPOSITORY_PLACEHOLDER)
                .count(),
            1
        );
    }
}
