//! Publication identity and receipt consistency. No remote writes or state machine.
use super::capability::{CapabilityId, CapabilityObservation, CapabilityPolicy};
use super::{ByteIdentity, InputIdentity, Review, Source, release_ref, release_tag, require};
use crate::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Trigger {
    StableTagPush,
    ProtectedDefaultBranchRehearsal,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TriggerContext {
    pub event: String,
    pub repository: String,
    pub source_commit: String,
    pub git_ref: String,
    pub fork: bool,
}

/// The caller supplies trusted platform context and a reviewed default branch.
/// Protected-ref, ancestry and actor authorization require live #10 checks.
pub fn validate_trigger(
    context: &TriggerContext,
    expected: &Source,
    mode: Trigger,
    default_branch: &str,
) -> Result<()> {
    expected.validate()?;
    require(
        !context.fork
            && context.repository == expected.repository
            && context.source_commit == expected.commit,
        "unauthorized-trigger-identity",
    )?;
    let allowed = match mode {
        Trigger::StableTagPush => {
            release_ref(&expected.git_ref)
                && context.event == "push"
                && context.git_ref == expected.git_ref
        }
        Trigger::ProtectedDefaultBranchRehearsal => {
            context.event == "workflow_dispatch"
                && super::valid_source_ref(&format!("refs/heads/{default_branch}"))
                && context.git_ref == format!("refs/heads/{default_branch}")
                && expected.git_ref == context.git_ref
        }
    };
    require(allowed, "unauthorized-release-trigger")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum LifecycleStage {
    Configured,
    CiVerified,
    ReleaseRehearsed,
    Published,
    ProvenanceVerified,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LifecycleRecord {
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    pub inputs: InputIdentity,
    pub stage: LifecycleStage,
    pub policy: ByteIdentity,
    pub recorded_at: u64,
    pub evidence: Vec<ByteIdentity>,
    pub limitations: Vec<String>,
}

impl LifecycleRecord {
    /// Each stage has its own evidence; a scanner pass cannot advance a stage.
    pub fn validate(&self, now: u64) -> Result<()> {
        require(
            self.schema_version == 1
                && self.recorded_at > 0
                && self.recorded_at <= now
                && !self.evidence.is_empty(),
            "invalid-lifecycle-record",
        )?;
        self.inputs.validate()?;
        require(
            !matches!(
                self.stage,
                LifecycleStage::Published | LifecycleStage::ProvenanceVerified
            ) || release_ref(&self.inputs.source.git_ref),
            "release-lifecycle-needs-tag",
        )?;
        self.policy.validate()?;
        for e in &self.evidence {
            e.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum GithubState {
    OwnedDraft,
    Uploaded,
    DraftBytesVerified,
    Approved,
    Published,
    ReleaseAttestationVerified,
    ProvenanceVerified,
    Conflict,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GithubReceipt {
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    pub inputs: InputIdentity,
    pub tag: String,
    pub release_id: u64,
    pub inventory: ByteIdentity,
    pub policy: ByteIdentity,
    pub state: GithubState,
    pub draft_download_receipt: Option<ByteIdentity>,
    pub approval: Option<Review>,
    pub immutable_setting: Option<CapabilityObservation>,
    pub published_at: Option<u64>,
    pub release_attestation: Option<ByteIdentity>,
    pub provenance_verification: Option<ByteIdentity>,
    pub conflict: Option<ConflictRecovery>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ConflictRecovery {
    InvestigateOwnedDraft,
    ReviewNewVersion,
    IncidentForPublishedRelease,
}

impl GithubReceipt {
    pub fn validate(&self, now: u64, max_setting_age: u64) -> Result<()> {
        self.inputs.validate()?;
        self.inventory.validate()?;
        self.policy.validate()?;
        require(
            self.schema_version == 1
                && self.release_id > 0
                && release_tag(&self.tag)
                && release_ref(&self.inputs.source.git_ref)
                && self.inputs.source.git_ref == format!("refs/tags/{}", self.tag),
            "invalid-github-receipt",
        )?;
        let draft_verified = matches!(
            self.state,
            GithubState::DraftBytesVerified
                | GithubState::Approved
                | GithubState::Published
                | GithubState::ReleaseAttestationVerified
                | GithubState::ProvenanceVerified
        );
        let approved = matches!(
            self.state,
            GithubState::Approved
                | GithubState::Published
                | GithubState::ReleaseAttestationVerified
                | GithubState::ProvenanceVerified
        );
        let published = matches!(
            self.state,
            GithubState::Published
                | GithubState::ReleaseAttestationVerified
                | GithubState::ProvenanceVerified
        );
        require(
            !draft_verified || self.draft_download_receipt.is_some(),
            "draft-verification-receipt-missing",
        )?;
        if let Some(r) = &self.draft_download_receipt {
            r.validate()?;
        }
        if approved {
            self.approval
                .as_ref()
                .ok_or_else(|| crate::Error::Invalid("publication-approval-missing".into()))?
                .validate_at(now)?;
            let setting = self
                .immutable_setting
                .as_ref()
                .ok_or_else(|| crate::Error::Invalid("immutable-setting-missing".into()))?;
            require(
                setting.capability == CapabilityId::ImmutableReleases,
                "wrong-setting-capability",
            )?;
            setting.satisfies(
                CapabilityId::ImmutableReleases,
                CapabilityPolicy::Required,
                now,
                max_setting_age,
            )?;
        }
        require(
            !published || self.published_at.is_some_and(|t| t > 0 && t <= now),
            "published-observation-missing",
        )?;
        if matches!(
            self.state,
            GithubState::ReleaseAttestationVerified | GithubState::ProvenanceVerified
        ) {
            self.release_attestation
                .as_ref()
                .ok_or_else(|| crate::Error::Invalid("release-attestation-receipt-missing".into()))?
                .validate()?;
        }
        if self.state == GithubState::ProvenanceVerified {
            self.provenance_verification
                .as_ref()
                .ok_or_else(|| crate::Error::Invalid("provenance-receipt-missing".into()))?
                .validate()?;
        }
        require(
            (self.state == GithubState::Conflict) == self.conflict.is_some(),
            "conflict-recovery-mismatch",
        )
    }

    /// Owned retries must match the complete source/config/lock/runtime/run/attempt,
    /// inventory/policy and remote draft identity. Never authorizes replacement.
    pub fn same_owned_retry(&self, next: &Self) -> Result<()> {
        let resumable = |state| {
            matches!(
                state,
                GithubState::OwnedDraft
                    | GithubState::Uploaded
                    | GithubState::DraftBytesVerified
                    | GithubState::Approved
            )
        };
        require(
            resumable(self.state) && resumable(next.state),
            "draft-not-resumable",
        )?;
        require(
            self.inputs == next.inputs
                && self.tag == next.tag
                && self.release_id == next.release_id
                && self.inventory == next.inventory
                && self.policy == next.policy,
            "conflicting-retry-identity",
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Registry {
    CratesIo,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CrateIntent {
    pub name: String,
    pub version: String,
    pub archive: ByteIdentity,
    /// Names within this approved set. External dependencies must be separately
    /// resolved/byte-verified before preparation by the future adapter.
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublishSet {
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    pub inputs: InputIdentity,
    pub registry: Registry,
    pub crates: Vec<CrateIntent>,
}

impl PublishSet {
    pub fn validate(&self) -> Result<()> {
        self.inputs.validate()?;
        require(
            self.schema_version == 1
                && release_ref(&self.inputs.source.git_ref)
                && !self.crates.is_empty()
                && self.crates.len() <= 128,
            "invalid-publish-set",
        )?;
        let mut visited = BTreeSet::new();
        let mut names = BTreeSet::new();
        for c in &self.crates {
            c.archive.validate()?;
            require(
                crate::config::identifier(&c.name)
                    && names.insert(&c.name)
                    && super::stable_version(&c.version),
                "invalid-or-duplicate-publish-crate",
            )?;
        }
        // The frozen set is topologically ordered. An edge to an unselected or
        // later crate fails closed, including every dependency cycle.
        for c in &self.crates {
            let unique: BTreeSet<_> = c.dependencies.iter().collect();
            require(
                unique.len() == c.dependencies.len()
                    && c.dependencies.iter().all(|d| visited.contains(d)),
                "invalid-publish-dag",
            )?;
            visited.insert(c.name.clone());
        }
        Ok(())
    }
    /// Hash compact typed intent bytes with no receipt/status/self-hash fields.
    pub fn identity(&self) -> Result<ByteIdentity> {
        self.validate()?;
        Ok(ByteIdentity::from_bytes(
            &serde_json::to_vec(self).map_err(|_| crate::Error::Json)?,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RegistryState {
    Prepared,
    Uploaded,
    UploadResultUnknown,
    RegistryObserved,
    RegistryBytesVerified,
    Conflict,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CrateReceipt {
    pub name: String,
    pub version: String,
    pub state: RegistryState,
    pub registry_bytes: Option<ByteIdentity>,
    pub index_sha256: Option<String>,
    pub observation: Option<ByteIdentity>,
    pub conflict: Option<ConflictRecovery>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RegistryReceipt {
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    pub publish_set: ByteIdentity,
    pub registry: Registry,
    pub crates: Vec<CrateReceipt>,
    pub recorded_at: u64,
}

impl RegistryReceipt {
    pub fn validate_against(&self, set: &PublishSet, now: u64) -> Result<()> {
        require(
            self.schema_version == 1
                && self.registry == set.registry
                && self.publish_set == set.identity()?
                && self.recorded_at > 0
                && self.recorded_at <= now
                && self.crates.len() == set.crates.len(),
            "registry-publish-set-mismatch",
        )?;
        let mut states = BTreeMap::new();
        for (r, c) in self.crates.iter().zip(&set.crates) {
            require(
                r.name == c.name && r.version == c.version,
                "registry-crate-mismatch",
            )?;
            if r.state != RegistryState::Prepared && r.state != RegistryState::Conflict {
                require(
                    c.dependencies
                        .iter()
                        .all(|d| states.get(d) == Some(&RegistryState::RegistryBytesVerified)),
                    "registry-dependency-unverified",
                )?;
            }
            if matches!(
                r.state,
                RegistryState::RegistryObserved | RegistryState::RegistryBytesVerified
            ) {
                r.observation
                    .as_ref()
                    .ok_or_else(|| crate::Error::Invalid("registry-observation-missing".into()))?
                    .validate()?;
            }
            if r.state == RegistryState::RegistryBytesVerified {
                require(
                    r.registry_bytes.as_ref() == Some(&c.archive)
                        && r.index_sha256.as_ref() == Some(&c.archive.sha256),
                    "registry-byte-mismatch",
                )?;
            }
            require(
                matches!(
                    r.state,
                    RegistryState::RegistryBytesVerified | RegistryState::Conflict
                ) || (r.registry_bytes.is_none() && r.index_sha256.is_none()),
                "registry-byte-evidence-state-mismatch",
            )?;
            if let Some(b) = &r.registry_bytes {
                b.validate()?;
            }
            if let Some(d) = &r.index_sha256 {
                require(
                    crate::config::hex_digest(d, 64),
                    "invalid-registry-index-digest",
                )?;
            }
            require(
                (r.state == RegistryState::Conflict) == r.conflict.is_some(),
                "registry-conflict-recovery-mismatch",
            )?;
            states.insert(c.name.clone(), r.state);
        }
        Ok(())
    }
    pub fn same_retry_set(&self, next: &Self) -> Result<()> {
        require(
            self.publish_set == next.publish_set && self.registry == next.registry,
            "conflicting-registry-retry-set",
        )
    }
}
