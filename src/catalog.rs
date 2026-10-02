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
use std::{collections::BTreeMap, path::Path};

pub const WORKFLOW_REPOSITORY: &str = "brianluby/armorer-workflows";
pub const WORKFLOW_COMMIT: &str = "772ca83e386c883c88cc3b936d69f8cb3216c91e";
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
}

/// Authenticate exact bytes before structural interpretation. The expected identity
/// is deliberately not a function parameter or part of a downloaded document.
///
/// # Errors
/// Rejects oversized, substituted or malformed catalog bytes. No network or
/// consuming-repository operation occurs; upgrades require a reviewed CLI source change.
pub fn authenticate(bytes: &[u8]) -> Result<AuthenticatedCatalog> {
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
            commit: WORKFLOW_COMMIT.into(),
        },
        tools_sha256: TOOLS_SHA256,
        tools: catalog.tools,
    })
}

/// Load the reviewed catalog shipped with this CLI, without downloads or execution.
pub fn reviewed() -> Result<AuthenticatedCatalog> {
    authenticate(TOOLS)
}

impl AuthenticatedCatalog {
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
            .collect();
        let lock = Lock {
            schema_version: 1,
            config_sha256: config_sha256.into(),
            runtime_version: env!("CARGO_PKG_VERSION").into(),
            workflows: WorkflowPin {
                repository: WORKFLOW_REPOSITORY.into(),
                commit: WORKFLOW_COMMIT.into(),
            },
            tools,
        };
        toml::to_string_pretty(&lock).map_err(|_| Error::Toml)
    }

    /// Fixed bootstrap file set for an already validated configuration and policy.
    pub(crate) fn bootstrap_files(
        &self,
        config_sha256: &str,
        toolchain: &str,
        policy: &str,
    ) -> Result<BTreeMap<String, String>> {
        let mut files = self.caller_workflows();
        files.insert(".armorer/ci-policy.toml".into(), policy.into());
        files.insert(
            "armorer.lock".into(),
            self.lock_for_config_sha256(config_sha256)?,
        );
        files.insert("rust-toolchain.toml".into(), format!("[toolchain]\nchannel = \"{}\"\nprofile = \"minimal\"\ncomponents = [\"clippy\", \"rustfmt\"]\n", toolchain));
        Ok(files)
    }

    /// Fixed unprivileged callers. Existing custom workflows remain separate.
    /// The build caller is manual and unsigned; it has no publication capability.
    pub fn caller_workflows(&self) -> BTreeMap<String, String> {
        let mut callers = BTreeMap::new();
        callers.insert(".github/workflows/armorer-ci.yml".into(), format!(
            "name: Armorer CI\n\non:\n  pull_request:\n  push:\n  workflow_dispatch:\n\npermissions:\n  contents: read\n\nconcurrency:\n  group: armorer-ci-${{{{ github.workflow }}}}-${{{{ github.ref }}}}\n  cancel-in-progress: true\n\njobs:\n  verify:\n    uses: {WORKFLOW_REPOSITORY}/.github/workflows/rust-ci.yml@{WORKFLOW_COMMIT}\n"
        ));
        callers.insert(".github/workflows/armorer-build.yml".into(), format!(
            "name: Armorer unsigned build\n\non:\n  workflow_dispatch:\n\npermissions:\n  contents: read\n\nconcurrency:\n  group: armorer-build-${{{{ github.workflow }}}}-${{{{ github.ref }}}}\n  cancel-in-progress: false\n\njobs:\n  build:\n    uses: {WORKFLOW_REPOSITORY}/.github/workflows/rust-build.yml@{WORKFLOW_COMMIT}\n"
        ));
        callers
    }
}
