//! Bootstrap catalog authority compiled into the reviewed Armorer source.
//!
//! Authentication precedes parsing. Consuming files and API observations cannot
//! choose an expected hash, workflow revision or a moving version reference.
use crate::{
    Error, Result,
    config::{Lock, ToolPin, WorkflowPin, load_config},
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

impl AuthenticatedCatalog {
    /// Sorted, unique repository-relative paths this catalog renders and may own.
    /// Derived from the same compiled table as [`AuthenticatedCatalog::bootstrap_files`],
    /// so the declared set and the rendered set cannot diverge.
    pub fn managed_paths(&self) -> Vec<&'static str> {
        self.files
            .iter()
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

    /// The complete managed file set for an already validated configuration and
    /// policy, rendered entry by entry from this catalog's compiled table.
    pub(crate) fn bootstrap_files(
        &self,
        config_sha256: &str,
        toolchain: &str,
        policy: &str,
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
            let content = match file.source {
                Source::Caller(template) => self.render_caller(template),
                Source::Toolchain => {
                    render_template(TOOLCHAIN_TEMPLATE, &[("TOOLCHAIN", toolchain)])?
                }
                Source::Lock => self.lock_for_config_sha256(config_sha256)?,
                Source::Policy => policy.into(),
                Source::Fixed(template) => render_template(template, &[])?,
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
}

/// One repository-relative file a catalog renders and may own.
#[derive(Debug, Clone, Copy)]
struct ManagedFile {
    path: &'static str,
    source: Source,
}

/// The managed file set of `bootstrap-v2` (and the test-only `bootstrap-v1`), in
/// sorted path order. This table is the single source of truth for which files a
/// catalog manages: `bootstrap_files`, `caller_workflows` and `managed_paths` all
/// derive from it. Adding an entry changes every consumer's plan.
const BOOTSTRAP_FILES: &[ManagedFile] = &[
    ManagedFile {
        path: ".armorer/ci-policy.toml",
        source: Source::Policy,
    },
    ManagedFile {
        path: ".github/workflows/armorer-build.yml",
        source: Source::Caller(BUILD_CALLER_TEMPLATE),
    },
    ManagedFile {
        path: ".github/workflows/armorer-ci.yml",
        source: Source::Caller(CI_CALLER_TEMPLATE),
    },
    ManagedFile {
        path: "armorer.lock",
        source: Source::Lock,
    },
    ManagedFile {
        path: "rust-toolchain.toml",
        source: Source::Toolchain,
    },
];

/// Clearly fake, test-only workflow pin that gives `bootstrap-test-wide` a distinct
/// identity for `id_for_commit`. It names no real commit; production builds lack it.
#[cfg(feature = "test-predecessor")]
pub const WORKFLOW_COMMIT_TEST_WIDE: &str = "7e577e577e577e577e577e577e577e577e577e57";

#[cfg(feature = "test-predecessor")]
const MANAGED_NOTE_TEMPLATE: &str = include_str!("../catalogs/templates/armorer-managed.md.tmpl");

/// Test-only `bootstrap-test-wide` (ADR 0013): `bootstrap-v2` plus one fixed file,
/// proving that a catalog defines its own managed file set end to end.
#[cfg(feature = "test-predecessor")]
const TEST_WIDE_FILES: &[ManagedFile] = &[
    ManagedFile {
        path: ".armorer/ci-policy.toml",
        source: Source::Policy,
    },
    ManagedFile {
        path: ".github/armorer-managed.md",
        source: Source::Fixed(MANAGED_NOTE_TEMPLATE),
    },
    ManagedFile {
        path: ".github/workflows/armorer-build.yml",
        source: Source::Caller(BUILD_CALLER_TEMPLATE),
    },
    ManagedFile {
        path: ".github/workflows/armorer-ci.yml",
        source: Source::Caller(CI_CALLER_TEMPLATE),
    },
    ManagedFile {
        path: "armorer.lock",
        source: Source::Lock,
    },
    ManagedFile {
        path: "rust-toolchain.toml",
        source: Source::Toolchain,
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
                .bootstrap_files(&"a".repeat(64), "1.95.0", "")
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
            let rendered = catalog
                .bootstrap_files(&"a".repeat(64), "1.95.0", "")
                .unwrap();
            let declared = catalog.managed_paths();
            assert_eq!(
                rendered.keys().map(String::as_str).collect::<Vec<_>>(),
                declared,
                "{id}"
            );
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
        assert_eq!(wide.managed_paths().len(), 6);
        assert_eq!(v2.managed_paths().len(), 5);
        assert_eq!(all_managed_paths(), expected);
        let fixed = wide.fixed_files().unwrap();
        assert_eq!(fixed.len(), 1);
        assert_eq!(fixed[".github/armorer-managed.md"], MANAGED_NOTE_TEMPLATE);
        assert!(v2.fixed_files().unwrap().is_empty());
        assert_eq!(wide.workflow().commit, WORKFLOW_COMMIT_TEST_WIDE);
        let inputs = (&"b".repeat(64), "1.95.0", "policy");
        let mut narrow = wide.bootstrap_files(inputs.0, inputs.1, inputs.2).unwrap();
        narrow.remove(".github/armorer-managed.md");
        let v2_files = v2.bootstrap_files(inputs.0, inputs.1, inputs.2).unwrap();
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
                catalog.bootstrap_files(&"a".repeat(64), bad, "").is_err(),
                "accepted {bad:?}"
            );
        }
        assert!(
            catalog
                .bootstrap_files(&"a".repeat(64), "1.95.0", "")
                .is_ok()
        );
    }
}
