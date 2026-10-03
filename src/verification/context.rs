//! Independently approved release expectations; never learned from downloaded release files.
use super::{
    cyclonedx, io,
    sigstore::{self, ExpectedAttestation, OfflineVerifier, Trigger},
};
use crate::{
    Error, Result,
    config::{self, Config, Lock, Profile},
    trust::{
        ByteIdentity, InputIdentity, Review, WorkflowIdentity,
        capability::{AdapterId, CapabilityId, Catalog, Enforcement},
        evidence::{EvidenceRequirements, InputKind, Outcome, Selection, StepKind},
        policy::{EvidenceScope, Predicate, VerificationPolicy},
        require,
    },
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub const CONTEXT_NAME: &str = "armorer-verification-context.json";
const MAX_CONTEXT: u64 = 4 * 1024 * 1024;

/// Only the existing supported secure release/rehearsal invoking events are representable.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema)]
pub enum ContextTrigger {
    #[serde(rename = "push")]
    Push,
    #[serde(rename = "workflow_dispatch")]
    WorkflowDispatch,
}
impl ContextTrigger {
    /// Convert a reviewed fixed event into the native verifier's exact trigger enum.
    fn trigger(self) -> Trigger {
        match self {
            Self::Push => Trigger::Push,
            Self::WorkflowDispatch => Trigger::WorkflowDispatch,
        }
    }
}

/// The target name/version and evidence requirements are independently resolved from reviewed source.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReleaseSelection {
    pub selection: Selection,
    pub root_component_name: String,
    pub evidence_requirements: EvidenceRequirements,
}

/// Serializable claims become trusted only through an independently approved exact context digest.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReleaseExpectations {
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    pub inputs: InputIdentity,
    pub caller_workflow: WorkflowIdentity,
    pub trigger: ContextTrigger,
    pub catalog: ByteIdentity,
    pub verification_policy: ByteIdentity,
    pub native_sbom_validator: ByteIdentity,
    /// One exact signer per fixed scope; multiple policy candidates never select a signer implicitly.
    pub signers: BTreeMap<EvidenceScope, WorkflowIdentity>,
    pub selections: Vec<ReleaseSelection>,
    pub review: Review,
}

/// Private-constructor independently approved context, distinct from producer JSON and policy review syntax.
/// ```compile_fail
/// let context: armorer::verification::context::TrustedReleaseContext = serde_json::from_str("{}").unwrap();
/// ```
pub struct TrustedReleaseContext {
    expectations: ReleaseExpectations,
    identity: ByteIdentity,
    config: Config,
    lock: Lock,
    catalog: Catalog,
    policy: VerificationPolicy,
}
impl TrustedReleaseContext {
    /// Match the independently approved context SHA first, then exact config/locks/catalog/policy bytes.
    /// This reads trusted input files only; it never discovers, builds or executes a consuming repository.
    pub fn open(directory: &Path, independently_approved_sha256: &str) -> Result<Self> {
        require(
            config::hex_digest(independently_approved_sha256, 64),
            "invalid-approved-release-context-digest",
        )?;
        require(
            std::fs::symlink_metadata(directory)?.is_dir(),
            "release-context-directory-type",
        )?;
        let bytes = io::read_bounded(&directory.join(CONTEXT_NAME), MAX_CONTEXT)?;
        require(
            crate::digest(&bytes) == independently_approved_sha256,
            "unapproved-release-context",
        )?;
        let expectations: ReleaseExpectations = io::parse(&bytes)?;
        let now = sigstore::wall_time()?;
        require(
            expectations.schema_version == 1,
            "unsupported-release-context-version",
        )?;
        expectations.inputs.validate()?;
        expectations.review.validate_at(now)?;
        let (config, config_bytes) = config::load_config(directory)?;
        require(
            crate::digest(&config_bytes) == expectations.inputs.config_sha256,
            "approved-config-byte-mismatch",
        )?;
        let (lock, lock_bytes) = config::load_lock(directory, &expectations.inputs.config_sha256)?
            .ok_or_else(|| Error::Invalid("approved-release-lock-missing".into()))?;
        require(
            crate::digest(&lock_bytes) == expectations.inputs.lock_sha256
                && crate::digest(&io::read_bounded(&directory.join("Cargo.lock"), 1_048_576)?)
                    == expectations.inputs.cargo_lock_sha256
                && lock.runtime_version == expectations.inputs.runtime_version
                && lock.workflows.repository == expectations.inputs.run.workflow.repository
                && lock.workflows.commit == expectations.inputs.run.workflow.commit,
            "approved-release-input-mismatch",
        )?;
        expectations.catalog.validate()?;
        let catalog_bytes = io::read_bounded(&directory.join("catalog.json"), 1_048_576)?;
        expectations.catalog.matches(&catalog_bytes)?;
        let catalog: Catalog = io::parse(&catalog_bytes)?;
        catalog.validate(&lock, now)?;
        for id in [
            AdapterId::RustNative,
            AdapterId::CargoCyclonedx,
            AdapterId::GithubSigstore,
        ] {
            require(
                catalog.adapters.iter().any(|adapter| adapter.id == id),
                "release-baseline-adapter-missing",
            )?;
        }
        expectations.verification_policy.validate()?;
        let policy_bytes =
            io::read_bounded(&directory.join("verification-policy.json"), 1_048_576)?;
        expectations.verification_policy.matches(&policy_bytes)?;
        let policy: VerificationPolicy = io::parse(&policy_bytes)?;
        policy.validate(now)?;
        policy.accept_source(&expectations.inputs.source)?;
        require(
            config.repository == expectations.inputs.source.repository
                && policy.repository == config.repository
                && expectations.native_sbom_validator == cyclonedx::qualified_native_validator()?,
            "release-context-authority-mismatch",
        )?;
        let context = Self {
            expectations,
            identity: ByteIdentity::from_bytes(&bytes),
            config,
            lock,
            catalog,
            policy,
        };
        context.validate_at(now)?;
        Ok(context)
    }

    /// Recheck current review/policy/catalog expiry and all independently selected contexts.
    pub(crate) fn validate_at(&self, now: u64) -> Result<()> {
        self.expectations.review.validate_at(now)?;
        self.policy.validate(now)?;
        self.catalog.validate(&self.lock, now)?;
        let source = &self.expectations.inputs.source;
        let caller = &self.expectations.caller_workflow;
        caller.validate()?;
        require(
            caller.repository == source.repository && caller.commit == source.commit,
            "release-context-caller-mismatch",
        )?;
        let scopes = [
            (EvidenceScope::FinalArtifact, Predicate::SlsaProvenanceV1),
            (EvidenceScope::CargoSbomFile, Predicate::SlsaProvenanceV1),
            (EvidenceScope::SbomPredicate, Predicate::CycloneDxV15),
            (EvidenceScope::EvidenceFile, Predicate::SlsaProvenanceV1),
            (EvidenceScope::Inventory, Predicate::SlsaProvenanceV1),
        ];
        require(
            self.expectations.signers.len() == scopes.len(),
            "release-context-signer-set-mismatch",
        )?;
        for (scope, predicate) in scopes {
            let expected = self.attestation("context-check.json", scope, predicate)?;
            expected.validate(&self.policy)?;
            require(
                expected.run.workflow.repository == self.lock.workflows.repository
                    && expected.run.workflow.commit == self.lock.workflows.commit,
                "release-context-workflow-pin-mismatch",
            )?;
        }
        let expected_keys: BTreeSet<_> = self
            .config
            .deliverables
            .iter()
            .flat_map(|deliverable| {
                deliverable.targets.iter().map(move |target| {
                    format!(
                        "{}--{}--{}",
                        deliverable.id, target, deliverable.feature_set
                    )
                })
            })
            .collect();
        require(
            self.expectations.selections.len() == expected_keys.len() && expected_keys.len() <= 384,
            "release-context-selection-set-mismatch",
        )?;
        let mut seen = BTreeSet::new();
        for selected in &self.expectations.selections {
            let selection = &selected.selection;
            selection.validate_against(&self.config)?;
            let key = selection.key();
            require(
                expected_keys.contains(&key) && seen.insert(key),
                "release-context-selection-set-mismatch",
            )?;
            require(
                !selected.root_component_name.is_empty()
                    && selected.root_component_name.len() <= 1024
                    && !selected.root_component_name.chars().any(char::is_control),
                "release-context-root-name-invalid",
            )?;
            let requirements = &selected.evidence_requirements;
            requirements.review.validate_at(now)?;
            require(
                requirements.schema_version == 1
                    && requirements.max_age_seconds > 0
                    && requirements.inputs == self.expectations.inputs
                    && requirements.selection == *selection
                    && requirements.catalog == self.expectations.catalog,
                "release-context-evidence-requirements-mismatch",
            )?;
            let mac =
                selection.profile != Profile::Library && selection.target == "aarch64-apple-darwin";
            if mac {
                require(
                    self.policy.apple_team.is_some()
                        && requirements.apple_team == self.policy.apple_team
                        && self
                            .catalog
                            .adapters
                            .iter()
                            .any(|adapter| adapter.id == AdapterId::AppleNative),
                    "release-context-apple-requirements-missing",
                )?;
            } else {
                require(
                    requirements.apple_team.is_none(),
                    "release-context-unexpected-apple-requirements",
                )?;
            }
            let required_steps: BTreeSet<_> = if mac {
                [
                    StepKind::Build,
                    StepKind::Sign,
                    StepKind::Notarize,
                    StepKind::Package,
                ]
                .into_iter()
                .collect()
            } else {
                [StepKind::Build, StepKind::Package].into_iter().collect()
            };
            require(
                requirements.steps.keys().copied().collect::<BTreeSet<_>>() == required_steps,
                "release-context-step-set-mismatch",
            )?;
            require(
                !requirements.tools.is_empty()
                    && requirements.tools.len() <= 128
                    && requirements.required_coverage.len() <= 128
                    && requirements.allowed_exceptions.len() <= 128,
                "release-context-evidence-bounds",
            )?;
            let mut tools = BTreeSet::new();
            for tool in &requirements.tools {
                tool.bytes.validate()?;
                tool.authentication_record.validate()?;
                require(
                    tools.insert(&tool.name)
                        && tool.observed_at > 0
                        && tool.observed_at <= now
                        && tool
                            .max_age_seconds
                            .is_none_or(|age| age > 0 && now - tool.observed_at <= age)
                        && (tool.kind != InputKind::Database || tool.max_age_seconds.is_some())
                        && self
                            .catalog
                            .adapters
                            .iter()
                            .flat_map(|adapter| &adapter.pins)
                            .any(|pin| {
                                pin.name == tool.name
                                    && pin.version == tool.version
                                    && pin.distribution == tool.bytes
                                    && pin.authentication_record == tool.authentication_record
                            }),
                    "release-context-tool-pin-or-time-mismatch",
                )?;
            }
            for adapter in self.catalog.adapters.iter().filter(|adapter| {
                matches!(
                    adapter.id,
                    AdapterId::RustNative | AdapterId::CargoCyclonedx | AdapterId::GithubSigstore
                ) || (mac && adapter.id == AdapterId::AppleNative)
            }) {
                for pin in &adapter.pins {
                    require(
                        requirements.tools.iter().any(|tool| {
                            tool.name == pin.name
                                && tool.version == pin.version
                                && tool.bytes == pin.distribution
                                && tool.authentication_record == pin.authentication_record
                        }),
                        "release-context-baseline-tool-missing",
                    )?;
                }
            }
            let mut coverage = BTreeSet::new();
            for record in &requirements.required_coverage {
                record.tested_subject.validate()?;
                require(
                    coverage.insert(record.capability)
                        && !record.scope.trim().is_empty()
                        && record.outcome == Outcome::Passed
                        && record.enforcement == Enforcement::Enforced
                        && record.exception_ids.is_empty(),
                    "release-context-required-coverage-invalid",
                )?;
            }
            for capability in [
                CapabilityId::CargoSbom,
                CapabilityId::DependencyPolicy,
                CapabilityId::PlatformAttestations,
            ] {
                require(
                    coverage.contains(&capability),
                    "release-context-baseline-coverage-missing",
                )?;
            }
            if mac {
                require(
                    coverage.contains(&CapabilityId::AppleSigning),
                    "release-context-apple-coverage-missing",
                )?;
            }
            let mut exceptions = BTreeSet::new();
            for exception in &requirements.allowed_exceptions {
                exception.validate_at(now)?;
                require(
                    exceptions.insert(&exception.id),
                    "release-context-duplicate-exception",
                )?;
            }
            for workflow in requirements.steps.values() {
                workflow.validate()?;
                require(
                    workflow.repository == self.lock.workflows.repository
                        && workflow.commit == self.lock.workflows.commit,
                    "release-context-step-workflow-pin-mismatch",
                )?;
            }
        }
        Ok(())
    }

    /// Bind an independently opened cryptographic adapter to this exact approved policy identity.
    pub(crate) fn bind_verifier(&self, verifier: &OfflineVerifier) -> Result<()> {
        require(
            verifier.policy_identity() == &self.expectations.verification_policy,
            "release-context-verifier-policy-mismatch",
        )
    }
    /// Construct a fixed scoped request without learning signer/source/run expectations from a release.
    pub(crate) fn attestation(
        &self,
        name: &str,
        scope: EvidenceScope,
        predicate: Predicate,
    ) -> Result<ExpectedAttestation> {
        let workflow = self
            .expectations
            .signers
            .get(&scope)
            .ok_or_else(|| Error::Invalid("release-context-signer-missing".into()))?;
        let mut run = self.expectations.inputs.run.clone();
        run.workflow = workflow.clone();
        Ok(ExpectedAttestation {
            source: self.expectations.inputs.source.clone(),
            run,
            caller_workflow: self.expectations.caller_workflow.clone(),
            trigger: self.expectations.trigger.trigger(),
            predicate,
            scope,
            subject_name: name.into(),
        })
    }
    /// Return the authenticated context bytes' identity, never a producer assertion.
    pub fn identity(&self) -> &ByteIdentity {
        &self.identity
    }
    /// Borrow the exact independently approved input identities.
    pub fn inputs(&self) -> &InputIdentity {
        &self.expectations.inputs
    }
    /// Borrow independently approved intent without permitting mutation.
    pub fn config(&self) -> &Config {
        &self.config
    }
    /// Borrow the independently approved exact workflow/tool lock.
    pub fn lock(&self) -> &Lock {
        &self.lock
    }
    /// Borrow the independently approved catalog identity.
    pub fn catalog_identity(&self) -> &ByteIdentity {
        &self.expectations.catalog
    }
    /// Borrow the independently approved verification policy.
    pub fn policy(&self) -> &VerificationPolicy {
        &self.policy
    }
    /// Borrow all independently selected roots, versions and evidence requirements.
    pub fn selections(&self) -> &[ReleaseSelection] {
        &self.expectations.selections
    }
    /// Return the independently approved compiled native complete-SBOM validator identity.
    pub fn sbom_validator(&self) -> &ByteIdentity {
        &self.expectations.native_sbom_validator
    }
}
