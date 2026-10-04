//! Bounded capability policy and reviewed catalog sidecars; no adapter execution.
use super::{ByteIdentity, Review, WorkflowIdentity, require, stable_version};
use crate::{
    Result,
    config::{Config, Lock, hex_digest},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityId {
    PlatformAttestations,
    ImmutableReleases,
    ProtectedTags,
    ProtectedPublishEnvironment,
    ProtectedSigningEnvironment,
    AppleSigning,
    CargoSbom,
    DependencyPolicy,
    Codeql,
    CargoVet,
    EmbeddedMetadata,
    NativeSbom,
    LinuxEgress,
    IndependentRebuild,
    Scorecard,
    DistBuild,
    RegistryPublish,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityPolicy {
    Disabled,
    Reporting,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Availability {
    Unknown,
    Unsupported,
    Error,
    Available,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Enforcement {
    Disabled,
    NotTested,
    Skipped,
    Reporting,
    Enforced,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityObservation {
    #[schemars(range(min = 1, max = 1))]
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub schema_version: u32,
    pub capability: CapabilityId,
    pub availability: Availability,
    pub enforcement: Enforcement,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub observed_at: u64,
    pub evidence: Option<ByteIdentity>,
    pub limitations: Vec<String>,
}

impl CapabilityObservation {
    /// A positive observation needs retained evidence. Authenticity/freshness
    /// must be checked against platform settings by #10 before publication.
    pub fn satisfies(
        &self,
        expected: CapabilityId,
        policy: CapabilityPolicy,
        now: u64,
        max_age: u64,
    ) -> Result<()> {
        require(
            self.schema_version == 1,
            "unsupported-capability-observation-version",
        )?;
        require(
            self.capability == expected,
            "capability-observation-mismatch",
        )?;
        require(
            self.observed_at > 0 && self.observed_at <= now && now - self.observed_at <= max_age,
            "stale-capability-observation",
        )?;
        if let Some(e) = &self.evidence {
            e.validate()?;
        }
        if policy == CapabilityPolicy::Required {
            require(
                self.availability == Availability::Available
                    && self.enforcement == Enforcement::Enforced
                    && self.evidence.is_some(),
                "required-capability-unproven",
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapabilityConfig {
    #[schemars(range(min = 1, max = 1))]
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub schema_version: u32,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub config_sha256: String,
    pub catalog: ByteIdentity,
    pub decisions: BTreeMap<CapabilityId, CapabilityPolicy>,
    pub review: Review,
}

impl CapabilityConfig {
    /// Bind exact config/catalog bytes and current review, requiring baseline and applicable Apple policies.
    pub fn validate(
        &self,
        config: &Config,
        exact_config_sha256: &str,
        expected_catalog: &ByteIdentity,
        now: u64,
    ) -> Result<()> {
        require(
            self.schema_version == 1
                && hex_digest(&self.config_sha256, 64)
                && self.config_sha256 == exact_config_sha256,
            "capability-config-binding",
        )?;
        self.catalog.validate()?;
        expected_catalog.validate()?;
        require(
            self.catalog == *expected_catalog,
            "capability-catalog-binding",
        )?;
        self.review.validate_at(now)?;
        for id in [
            CapabilityId::PlatformAttestations,
            CapabilityId::ImmutableReleases,
            CapabilityId::ProtectedTags,
            CapabilityId::ProtectedPublishEnvironment,
            CapabilityId::CargoSbom,
            CapabilityId::DependencyPolicy,
        ] {
            require(
                self.decisions.get(&id) == Some(&CapabilityPolicy::Required),
                "baseline-capability-required",
            )?;
        }
        if config.deliverables.iter().any(|d| {
            d.profile != crate::config::Profile::Library
                && d.targets.iter().any(|t| t == "aarch64-apple-darwin")
        }) {
            for id in [
                CapabilityId::AppleSigning,
                CapabilityId::ProtectedSigningEnvironment,
            ] {
                require(
                    self.decisions.get(&id) == Some(&CapabilityPolicy::Required),
                    "apple-capability-required",
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AdapterId {
    RustNative,
    AppleNative,
    CargoCyclonedx,
    GithubSigstore,
    Codeql,
    CargoVet,
    CargoAuditable,
    Syft,
    HardenRunner,
    IndependentRebuild,
    Scorecard,
    DistBuildOnly,
    CratesIo,
}

/// Catalog pins include tools/actions/helpers/databases/query packs/schemas/policies.
/// Locations are fixed by the reviewed adapter implementation, never caller URLs.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CatalogPin {
    pub name: String,
    pub version: String,
    pub distribution: ByteIdentity,
    pub authentication_record: ByteIdentity,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Adapter {
    pub id: AdapterId,
    pub capability: CapabilityId,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub config_versions: Vec<u32>,
    pub runtime_versions: Vec<String>,
    pub workflow: WorkflowIdentity,
    pub pins: Vec<CatalogPin>,
    pub review: Review,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    #[schemars(range(min = 1, max = 1))]
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub schema_version: u32,
    pub previous_catalog: Option<ByteIdentity>,
    pub adapters: Vec<Adapter>,
}

impl Catalog {
    /// Checks reviewed compatibility and pin-record syntax. The caller must
    /// independently authenticate catalog bytes and each upstream distribution.
    pub fn validate(&self, lock: &Lock, now: u64) -> Result<()> {
        require(
            self.schema_version == 1 && !self.adapters.is_empty() && self.adapters.len() <= 32,
            "invalid-catalog",
        )?;
        if let Some(p) = &self.previous_catalog {
            p.validate()?;
        }
        let mut ids = std::collections::BTreeSet::new();
        for a in &self.adapters {
            require(ids.insert(format!("{:?}", a.id)), "duplicate-adapter")?;
            a.workflow.validate()?;
            a.review.validate_at(now)?;
            require(
                a.workflow.repository == lock.workflows.repository
                    && a.workflow.commit == lock.workflows.commit
                    && a.config_versions == [1]
                    && a.runtime_versions.contains(&lock.runtime_version)
                    && a.runtime_versions.iter().all(|v| stable_version(v))
                    && !a.pins.is_empty(),
                "adapter-incompatible",
            )?;
            let expected = adapter_capability(a.id);
            require(a.capability == expected, "adapter-capability-mismatch")?;
            let mut pins = std::collections::BTreeSet::new();
            for pin in &a.pins {
                require(
                    crate::config::identifier(&pin.name)
                        && pins.insert(&pin.name)
                        && !pin.version.trim().is_empty()
                        && pin.version.len() <= 100,
                    "invalid-catalog-pin",
                )?;
                pin.distribution.validate()?;
                pin.authentication_record.validate()?;
            }
        }
        require(!lock.tools.is_empty(), "catalog-lock-tools-missing")?;
        for (name, pin) in &lock.tools {
            require(
                self.adapters.iter().flat_map(|a| &a.pins).any(|p| {
                    p.name == *name
                        && p.version == pin.version
                        && p.distribution.sha256 == pin.sha256
                }),
                "catalog-lock-pin-mismatch",
            )?;
        }
        Ok(())
    }
}

/// Share the exact reviewed adapter/capability relation across explicit catalog versions.
pub(crate) fn adapter_capability(id: AdapterId) -> CapabilityId {
    match id {
        AdapterId::RustNative => CapabilityId::DependencyPolicy,
        AdapterId::AppleNative => CapabilityId::AppleSigning,
        AdapterId::CargoCyclonedx => CapabilityId::CargoSbom,
        AdapterId::GithubSigstore => CapabilityId::PlatformAttestations,
        AdapterId::Codeql => CapabilityId::Codeql,
        AdapterId::CargoVet => CapabilityId::CargoVet,
        AdapterId::CargoAuditable => CapabilityId::EmbeddedMetadata,
        AdapterId::Syft => CapabilityId::NativeSbom,
        AdapterId::HardenRunner => CapabilityId::LinuxEgress,
        AdapterId::IndependentRebuild => CapabilityId::IndependentRebuild,
        AdapterId::Scorecard => CapabilityId::Scorecard,
        AdapterId::DistBuildOnly => CapabilityId::DistBuild,
        AdapterId::CratesIo => CapabilityId::RegistryPublish,
    }
}
