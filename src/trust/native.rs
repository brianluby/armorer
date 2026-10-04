//! Explicit v2 archive/member identities; frozen catalog v1 retains its original meaning.
use crate::{
    Result,
    config::{Lock, identifier},
    trust::{
        ByteIdentity, InputIdentity, Review, Source, WorkflowIdentity, asset_name,
        capability::{AdapterId, CapabilityId},
        evidence::{InputKind, ToolEvidence},
        require, stable_version,
    },
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_NATIVE_MEMBER: u64 = 128 * 1024 * 1024;

/// Only the three qualified native v0.1 hosts are representable.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema,
)]
pub enum NativeTarget {
    #[serde(rename = "x86_64-unknown-linux-gnu")]
    LinuxX86,
    #[serde(rename = "aarch64-unknown-linux-gnu")]
    LinuxArm,
    #[serde(rename = "aarch64-apple-darwin")]
    MacArm,
}
impl NativeTarget {
    /// Return the exact Rust target used in reviewed selection and catalog bindings.
    pub fn name(self) -> &'static str {
        match self {
            Self::LinuxX86 => "x86_64-unknown-linux-gnu",
            Self::LinuxArm => "aarch64-unknown-linux-gnu",
            Self::MacArm => "aarch64-apple-darwin",
        }
    }
    /// Return a fixed, flat runtime member name; caller archive paths are never used.
    pub fn runtime_member(self) -> String {
        format!("armorer--{}", self.name())
    }
    /// Reject unsupported native execution hosts instead of selecting a foreign member.
    pub fn current() -> Result<Self> {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("linux", "x86_64") => Ok(Self::LinuxX86),
            ("linux", "aarch64") => Ok(Self::LinuxArm),
            ("macos", "aarch64") => Ok(Self::MacArm),
            _ => Err(crate::Error::Invalid("unsupported-runtime-platform".into())),
        }
    }
}

/// A runtime archive has exactly one bounded executable member per qualified host.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMember {
    pub name: String,
    pub bytes: ByteIdentity,
}

/// Canonical USTAR bytes are common to every release selection; members remain target-specific.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeDistributionV1 {
    #[schemars(range(min = 1, max = 1))]
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub schema_version: u32,
    pub version: String,
    pub source: Source,
    pub compiler: String,
    pub cargo_lock: ByteIdentity,
    pub build_workflow: WorkflowIdentity,
    pub authentication_record: ByteIdentity,
    pub distribution: ByteIdentity,
    #[schemars(extend("minProperties" = 3, "maxProperties" = 3, "required" = serde_json::json!(["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "aarch64-apple-darwin"])))]
    pub members: BTreeMap<NativeTarget, RuntimeMember>,
}
impl RuntimeDistributionV1 {
    /// Validate independently approved metadata, not producer provenance or archive contents.
    pub fn validate(&self, inputs: &InputIdentity, expected_source: &Source) -> Result<()> {
        self.source.validate()?;
        expected_source.validate()?;
        self.build_workflow.validate()?;
        self.cargo_lock.validate()?;
        self.authentication_record.validate()?;
        self.distribution.validate()?;
        require(
            self.schema_version == 1
                && stable_version(&self.version)
                && self.version == inputs.runtime_version
                && self.compiler == "1.95.0"
                && self.source == *expected_source
                && self.build_workflow.repository == self.source.repository
                && self.build_workflow.commit == self.source.commit
                && self.distribution == inputs.runtime
                && self.members.len() == 3,
            "runtime-distribution-context-mismatch",
        )?;
        for target in [
            NativeTarget::LinuxX86,
            NativeTarget::LinuxArm,
            NativeTarget::MacArm,
        ] {
            let member = self
                .members
                .get(&target)
                .ok_or_else(|| crate::Error::Invalid("runtime-member-missing".into()))?;
            member.bytes.validate()?;
            require(
                member.name == target.runtime_member() && member.bytes.size <= MAX_NATIVE_MEMBER,
                "invalid-runtime-member",
            )?;
        }
        Ok(())
    }
}

/// Reviewed container formats do not imply that the consumer may extract a tool or release artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DistributionFormat {
    Raw,
    TarGz,
    TarXz,
}

/// An exact input uses distribution bytes; a native member separately binds executed bytes and target.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(tag = "material", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PinMaterial {
    Exact,
    NativeMember {
        target: NativeTarget,
        name: String,
        bytes: ByteIdentity,
    },
}

/// Lock digests always retain the distribution identity, including compressed tool archives.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NativePin {
    pub name: String,
    pub version: String,
    pub kind: InputKind,
    pub format: DistributionFormat,
    pub distribution: ByteIdentity,
    pub authentication_record: ByteIdentity,
    pub material: PinMaterial,
}
impl NativePin {
    /// Match only common inputs or the explicitly selected native target.
    pub fn applies_to(&self, target: &str) -> bool {
        match &self.material {
            PinMaterial::Exact => true,
            PinMaterial::NativeMember { target: native, .. } => native.name() == target,
        }
    }
    /// Compare authenticated observation bytes to the executed member without weakening archive locks.
    pub fn matches(&self, tool: &ToolEvidence, target: &str) -> bool {
        let bytes = match &self.material {
            PinMaterial::Exact => &self.distribution,
            PinMaterial::NativeMember { bytes, .. } => bytes,
        };
        self.applies_to(target)
            && self.name == tool.name
            && self.kind == tool.kind
            && self.version == tool.version
            && *bytes == tool.bytes
            && self.authentication_record == tool.authentication_record
    }
    /// Reject ambiguous material kinds, unsafe member names and oversized native executable claims.
    fn validate(&self) -> Result<()> {
        require(
            identifier(&self.name) && !self.version.trim().is_empty() && self.version.len() <= 100,
            "invalid-native-catalog-pin",
        )?;
        self.distribution.validate()?;
        self.authentication_record.validate()?;
        match &self.material {
            PinMaterial::Exact => require(
                self.format == DistributionFormat::Raw,
                "native-catalog-exact-format-mismatch",
            ),
            PinMaterial::NativeMember { name, bytes, .. } => {
                bytes.validate()?;
                require(
                    self.kind == InputKind::Tool
                        && asset_name(name)
                        && bytes.size <= MAX_NATIVE_MEMBER
                        && (self.format != DistributionFormat::Raw || *bytes == self.distribution),
                    "native-catalog-member-mismatch",
                )
            }
        }
    }
}

/// An adapter uses explicit v2 pins while preserving the reviewed compatibility and workflow contract.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NativeAdapter {
    pub id: AdapterId,
    pub capability: CapabilityId,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub config_versions: Vec<u32>,
    pub runtime_versions: Vec<String>,
    pub workflow: WorkflowIdentity,
    pub pins: Vec<NativePin>,
    pub review: Review,
}

/// Explicitly selected v2 authority; never a fallback interpretation of catalog v1.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NativeCatalogV2 {
    #[schemars(range(min = 2, max = 2))]
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub schema_version: u32,
    pub previous_catalog: Option<ByteIdentity>,
    pub runtime: RuntimeDistributionV1,
    pub adapters: Vec<NativeAdapter>,
}
impl NativeCatalogV2 {
    /// Check archive locks, explicit native semantics and exact independent runtime source approval.
    pub fn validate(
        &self,
        lock: &Lock,
        inputs: &InputIdentity,
        runtime_source: &Source,
        now: u64,
    ) -> Result<()> {
        require(
            self.schema_version == 2
                && lock.runtime_version == inputs.runtime_version
                && !self.adapters.is_empty()
                && self.adapters.len() <= 32,
            "invalid-native-catalog",
        )?;
        if let Some(previous) = &self.previous_catalog {
            previous.validate()?;
        }
        self.runtime.validate(inputs, runtime_source)?;
        let mut ids = BTreeSet::new();
        let mut pins = BTreeSet::new();
        for adapter in &self.adapters {
            require(ids.insert(format!("{:?}", adapter.id)), "duplicate-adapter")?;
            adapter.workflow.validate()?;
            adapter.review.validate_at(now)?;
            require(
                adapter.workflow.repository == lock.workflows.repository
                    && adapter.workflow.commit == lock.workflows.commit
                    && adapter.config_versions == [1]
                    && adapter.runtime_versions.contains(&lock.runtime_version)
                    && adapter.runtime_versions.iter().all(|v| stable_version(v))
                    && !adapter.pins.is_empty()
                    && adapter.pins.len() <= 128,
                "adapter-incompatible",
            )?;
            require(
                adapter.capability == super::capability::adapter_capability(adapter.id),
                "adapter-capability-mismatch",
            )?;
            for pin in &adapter.pins {
                pin.validate()?;
                require(pins.insert(&pin.name), "duplicate-native-catalog-pin")?;
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
