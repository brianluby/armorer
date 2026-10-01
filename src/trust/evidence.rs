//! Per-artifact producer records, platform-evidence references and byte chains.
use super::capability::{CapabilityId, Enforcement};
use super::inventory::{AssetRole, ReleaseInventory};
use super::{ByteIdentity, InputIdentity, Review, RunIdentity, require};
use crate::{
    Result,
    config::{Config, Profile, TARGETS, identifier},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub deliverable_id: String,
    pub profile: Profile,
    pub package: String,
    pub package_version: String,
    pub binary: Option<String>,
    pub target: String,
    pub feature_set: String,
    pub default_features: bool,
    pub features: Vec<String>,
    pub toolchain: String,
}

impl Selection {
    /// The source-resolved package version must also be checked by the builder.
    pub fn validate_against(&self, config: &Config) -> Result<()> {
        let d = config
            .deliverables
            .iter()
            .find(|d| d.id == self.deliverable_id)
            .ok_or_else(|| crate::Error::Invalid("unknown-evidence-selection".into()))?;
        let f = config
            .feature_sets
            .get(&d.feature_set)
            .ok_or_else(|| crate::Error::Invalid("unknown-evidence-feature-set".into()))?;
        require(
            self.profile == d.profile
                && self.package == d.package
                && self.binary == d.binary
                && d.targets.contains(&self.target)
                && TARGETS.contains(&self.target.as_str())
                && self.feature_set == d.feature_set
                && self.default_features == f.default_features
                && self.features == f.features
                && self.toolchain == config.toolchain
                && super::stable_version(&self.package_version),
            "evidence-selection-mismatch",
        )
    }
    pub fn key(&self) -> String {
        format!(
            "{}--{}--{}",
            self.deliverable_id, self.target, self.feature_set
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Passed,
    Failed,
    Unknown,
    Unsupported,
    Error,
    Skipped,
    NotTested,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum StepKind {
    Build,
    Sign,
    Notarize,
    Package,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildStep {
    pub kind: StepKind,
    pub inputs: Vec<ByteIdentity>,
    pub output: ByteIdentity,
    pub run: RunIdentity,
    pub started_at: u64,
    pub finished_at: u64,
    pub outcome: Outcome,
    /// Digest of separately retained platform evidence, not a trust boolean.
    /// #8/#9 must authenticate these bytes and their subject/source identities.
    pub platform_evidence: Vec<ByteIdentity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum InputKind {
    Tool,
    Action,
    Helper,
    Database,
    QueryPack,
    Schema,
    Policy,
    NativeInput,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToolEvidence {
    pub name: String,
    pub kind: InputKind,
    pub version: String,
    pub bytes: ByteIdentity,
    pub authentication_record: ByteIdentity,
    pub observed_at: u64,
    pub max_age_seconds: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Exception {
    pub id: String,
    pub capability: CapabilityId,
    pub tool: String,
    pub rule: String,
    pub tool_version: String,
    pub subject: ByteIdentity,
    pub policy: ByteIdentity,
    pub review: Review,
    pub positive_control: ByteIdentity,
}

impl Exception {
    pub fn validate_at(&self, now: u64) -> Result<()> {
        // Trust/byte/source/signer/trigger/publication requirements cannot be waived.
        require(
            matches!(
                self.capability,
                CapabilityId::Codeql
                    | CapabilityId::CargoVet
                    | CapabilityId::DependencyPolicy
                    | CapabilityId::NativeSbom
            ) && identifier(&self.id)
                && identifier(&self.tool)
                && identifier(&self.rule)
                && !self.tool_version.is_empty(),
            "non-waivable-or-invalid-exception",
        )?;
        self.subject.validate()?;
        self.policy.validate()?;
        self.review.validate_at(now)?;
        self.positive_control.validate()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub capability: CapabilityId,
    pub scope: String,
    pub tested_subject: ByteIdentity,
    pub omissions: Vec<String>,
    pub outcome: Outcome,
    pub enforcement: Enforcement,
    pub exception_ids: Vec<String>,
}

/// Apple values remain producer assertions until independently checked on macOS.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AppleAssertions {
    pub team_id: String,
    pub certificate_sha256: String,
    pub hardened_runtime: bool,
    pub secure_timestamp: bool,
    pub notarization_submission_id: String,
    pub notarization_outcome: Outcome,
    pub notarization_log: ByteIdentity,
    pub ticket_mode: AppleTicketMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AppleTicketMode {
    OnlineStandaloneMachO,
    StapledPackage,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactEvidence {
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    pub inputs: InputIdentity,
    pub selection: Selection,
    pub catalog: ByteIdentity,
    pub runner_label: String,
    /// Mutable hosted image identity, never a reproducibility guarantee.
    pub runner_image: String,
    pub recorded_at: u64,
    pub tools: Vec<ToolEvidence>,
    pub coverage: Vec<Coverage>,
    pub exceptions: Vec<Exception>,
    pub steps: Vec<BuildStep>,
    pub apple_assertions: Option<AppleAssertions>,
}

impl ArtifactEvidence {
    /// Validate a successful complete chain against separately established context.
    /// Failed/partial records deserialize but cannot satisfy this release gate.
    pub fn validate_release_chain(
        &self,
        config: &Config,
        inventory: &ReleaseInventory,
        expected_inputs: &InputIdentity,
        expected_catalog: &ByteIdentity,
        expected_apple_team: Option<&str>,
        now: u64,
    ) -> Result<()> {
        require(
            self.schema_version == 1
                && self.inputs == *expected_inputs
                && self.inputs == inventory.inputs
                && self.catalog == *expected_catalog,
            "evidence-input-mismatch",
        )?;
        self.inputs.validate()?;
        self.catalog.validate()?;
        self.selection.validate_against(config)?;
        let runner = match self.selection.target.as_str() {
            "x86_64-unknown-linux-gnu" => "ubuntu-24.04",
            "aarch64-unknown-linux-gnu" => "ubuntu-24.04-arm",
            "aarch64-apple-darwin" => "macos-15",
            _ => return Err(crate::Error::Invalid("unsupported-evidence-target".into())),
        };
        require(
            self.runner_label == runner
                && !self.runner_image.trim().is_empty()
                && self.recorded_at > 0
                && self.recorded_at <= now,
            "invalid-runner-or-evidence-time",
        )?;
        require(
            !self.tools.is_empty()
                && self.tools.len() <= 128
                && !self.coverage.is_empty()
                && self.coverage.len() <= 128
                && self.exceptions.len() <= 128,
            "evidence-coverage-missing",
        )?;
        let mut tools = BTreeSet::new();
        for tool in &self.tools {
            tool.bytes.validate()?;
            tool.authentication_record.validate()?;
            require(
                identifier(&tool.name)
                    && tools.insert(&tool.name)
                    && !tool.version.trim().is_empty()
                    && tool.observed_at > 0
                    && tool.observed_at <= self.recorded_at,
                "invalid-tool-evidence",
            )?;
            if tool.kind == InputKind::Database {
                require(tool.max_age_seconds.is_some(), "database-freshness-missing")?;
            }
            if let Some(age) = tool.max_age_seconds {
                require(
                    age > 0 && now - tool.observed_at <= age,
                    "stale-tool-or-database",
                )?;
            }
        }
        let mut exceptions = BTreeSet::new();
        for e in &self.exceptions {
            e.validate_at(now)?;
            require(exceptions.insert(&e.id), "duplicate-exception")?;
            require(
                self.tools
                    .iter()
                    .any(|t| t.name == e.tool && t.version == e.tool_version),
                "exception-tool-mismatch",
            )?;
        }
        let mut coverage_ids = BTreeSet::new();
        for c in &self.coverage {
            c.tested_subject.validate()?;
            require(
                coverage_ids.insert(c.capability)
                    && !c.scope.trim().is_empty()
                    && c.exception_ids.iter().all(|id| {
                        self.exceptions.iter().any(|e| {
                            e.id == *id
                                && e.capability == c.capability
                                && e.subject == c.tested_subject
                        })
                    }),
                "invalid-coverage-or-exception",
            )?;
            require(
                c.enforcement != Enforcement::Enforced || c.outcome == Outcome::Passed,
                "false-enforced-outcome",
            )?;
        }
        let mac = self.selection.profile != Profile::Library
            && self.selection.target == "aarch64-apple-darwin";
        let required: &[StepKind] = if mac {
            &[
                StepKind::Build,
                StepKind::Sign,
                StepKind::Notarize,
                StepKind::Package,
            ]
        } else {
            &[StepKind::Build, StepKind::Package]
        };
        require(
            self.steps.len() == required.len(),
            "incomplete-transformation-chain",
        )?;
        let mut previous: Option<&BuildStep> = None;
        for (s, kind) in self.steps.iter().zip(required) {
            s.run.validate()?;
            s.output.validate()?;
            require(
                s.kind == *kind
                    && s.outcome == Outcome::Passed
                    && s.run.id == self.inputs.run.id
                    && s.run.attempt == self.inputs.run.attempt
                    && s.started_at > 0
                    && s.started_at <= s.finished_at
                    && s.finished_at <= self.recorded_at
                    && !s.platform_evidence.is_empty(),
                "invalid-step-context-or-outcome",
            )?;
            for input in &s.inputs {
                input.validate()?;
            }
            for reference in &s.platform_evidence {
                reference.validate()?;
            }
            if let Some(p) = previous {
                require(
                    s.inputs == [p.output.clone()] && p.finished_at <= s.started_at,
                    "transformation-byte-chain-mismatch",
                )?;
                if s.kind == StepKind::Sign {
                    require(s.output != p.output, "signing-byte-identity-unchanged")?;
                }
            } else {
                require(!s.inputs.is_empty(), "build-input-evidence-missing")?;
            }
            previous = Some(s);
        }
        let final_asset = inventory
            .assets
            .iter()
            .find(|a| {
                a.role == AssetRole::Distributable
                    && a.name
                        == format!(
                            "{}{}",
                            self.selection.key(),
                            if self.selection.profile == Profile::Library {
                                ".crate"
                            } else {
                                ".tar.gz"
                            }
                        )
            })
            .ok_or_else(|| crate::Error::Invalid("final-artifact-missing".into()))?;
        require(
            previous.is_some_and(|s| s.output == final_asset.bytes),
            "final-distributed-byte-mismatch",
        )?;
        if mac {
            let a = self
                .apple_assertions
                .as_ref()
                .ok_or_else(|| crate::Error::Invalid("apple-evidence-missing".into()))?;
            a.notarization_log.validate()?;
            require(
                expected_apple_team == Some(a.team_id.as_str())
                    && a.team_id.len() == 10
                    && crate::config::hex_digest(&a.certificate_sha256, 64)
                    && a.hardened_runtime
                    && a.secure_timestamp
                    && identifier(&a.notarization_submission_id)
                    && a.notarization_outcome == Outcome::Passed,
                "apple-assertions-mismatch",
            )?;
        } else {
            require(self.apple_assertions.is_none(), "unexpected-apple-evidence")?;
        }
        Ok(())
    }
}

/// Independently reviewed evidence expectations, distinct from producer records.
/// Pin bytes and reviewed exceptions come from authenticated catalog/policy input.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRequirements {
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    pub inputs: InputIdentity,
    pub selection: Selection,
    pub catalog: ByteIdentity,
    pub max_age_seconds: u64,
    pub steps: std::collections::BTreeMap<StepKind, super::WorkflowIdentity>,
    pub tools: Vec<ToolEvidence>,
    pub required_coverage: Vec<Coverage>,
    pub allowed_exceptions: Vec<Exception>,
    pub apple_team: Option<String>,
    pub review: Review,
}

impl ArtifactEvidence {
    /// Apply independently supplied workflow, pin, freshness and evidence scope.
    /// These requirements must never be synthesized from this producer record.
    pub fn validate_against_requirements(
        &self,
        config: &Config,
        inventory: &ReleaseInventory,
        required: &EvidenceRequirements,
        now: u64,
    ) -> Result<()> {
        require(
            required.schema_version == 1 && required.max_age_seconds > 0,
            "invalid-evidence-requirements",
        )?;
        required.review.validate_at(now)?;
        self.validate_release_chain(
            config,
            inventory,
            &required.inputs,
            &required.catalog,
            required.apple_team.as_deref(),
            now,
        )?;
        require(
            self.selection == required.selection
                && now - self.recorded_at <= required.max_age_seconds,
            "evidence-selection-or-freshness-mismatch",
        )?;
        require(
            self.steps.len() == required.steps.len(),
            "evidence-step-policy-mismatch",
        )?;
        for step in &self.steps {
            require(
                required.steps.get(&step.kind) == Some(&step.run.workflow),
                "evidence-workflow-not-allowed",
            )?;
        }
        require(
            self.tools.len() == required.tools.len(),
            "evidence-tool-set-mismatch",
        )?;
        for t in &self.tools {
            let expected = required
                .tools
                .iter()
                .find(|e| e.name == t.name)
                .ok_or_else(|| crate::Error::Invalid("evidence-tool-not-allowed".into()))?;
            require(
                t.kind == expected.kind
                    && t.version == expected.version
                    && t.bytes == expected.bytes
                    && t.authentication_record == expected.authentication_record
                    && t.max_age_seconds == expected.max_age_seconds,
                "evidence-tool-pin-mismatch",
            )?;
        }
        for expected in &required.required_coverage {
            require(
                expected.outcome == Outcome::Passed
                    && expected.enforcement == Enforcement::Enforced,
                "invalid-required-coverage-policy",
            )?;
            let found = self
                .coverage
                .iter()
                .find(|c| c.capability == expected.capability)
                .ok_or_else(|| {
                    crate::Error::Invalid("required-evidence-coverage-missing".into())
                })?;
            require(
                found.scope == expected.scope
                    && found.tested_subject == expected.tested_subject
                    && found.omissions == expected.omissions
                    && found.outcome == Outcome::Passed
                    && found.enforcement == Enforcement::Enforced,
                "required-coverage-not-enforced",
            )?;
        }
        for e in &self.exceptions {
            let allowed = required
                .allowed_exceptions
                .iter()
                .find(|a| a.id == e.id)
                .ok_or_else(|| crate::Error::Invalid("exception-not-reviewed".into()))?;
            require(
                serde_json::to_value(e).map_err(|_| crate::Error::Json)?
                    == serde_json::to_value(allowed).map_err(|_| crate::Error::Json)?,
                "exception-policy-mismatch",
            )?;
        }
        Ok(())
    }
}
