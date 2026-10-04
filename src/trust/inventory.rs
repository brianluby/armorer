//! Release asset identities and independently derived, acyclic relationships.
use super::policy::{Predicate, VerificationPolicy};
use super::{ByteIdentity, InputIdentity, asset_name, require};
use crate::{
    Result,
    config::{Config, Profile},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const INVENTORY_NAME: &str = "armorer-release-inventory.json";
pub const INVENTORY_BUNDLE_NAME: &str = "armorer-release-inventory.provenance.sigstore.json";

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum AssetRole {
    Distributable,
    CargoSbom,
    CargoGraph,
    NativeSbom,
    EmbeddedMetadata,
    Diagnostic,
    RebuildEvidence,
    BuildEvidence,
    PackageEvidence,
    TransformationEvidence,
    AttestationBundle,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub name: String,
    pub role: AssetRole,
    pub bytes: ByteIdentity,
    /// Exact asset names; relationships point from evidence to subjects.
    pub subjects: Vec<String>,
    pub predicate: Option<Predicate>,
    /// The SBOM predicate must be compared with these exact published bytes.
    pub predicate_asset: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRequirement {
    pub name: String,
    pub role: AssetRole,
    pub subjects: Vec<String>,
    pub predicate: Option<Predicate>,
    pub predicate_asset: Option<String>,
    pub required: bool,
}

/// Versioned fixed naming/layout. No asset set is learned from release claims.
/// A library's target-associated .crate is a source package, not an executable.
pub fn expected_assets(
    config: &Config,
    policy: &VerificationPolicy,
) -> Result<Vec<AssetRequirement>> {
    let mut specs = Vec::new();
    for d in &config.deliverables {
        for target in &d.targets {
            let key = format!("{}--{}--{}", d.id, target, d.feature_set);
            let final_name = format!(
                "{key}{}",
                if d.profile == Profile::Library {
                    ".crate"
                } else {
                    ".tar.gz"
                }
            );
            specs.push(AssetRequirement {
                name: final_name.clone(),
                role: AssetRole::Distributable,
                subjects: vec![],
                predicate: None,
                predicate_asset: None,
                required: true,
            });
            let mut records = vec![
                (AssetRole::CargoSbom, ".cdx.json", true),
                (AssetRole::CargoGraph, ".cargo-graph.json", true),
                (AssetRole::BuildEvidence, ".build.json", true),
                (AssetRole::PackageEvidence, ".package.json", true),
            ];
            if d.profile != Profile::Library && target == "aarch64-apple-darwin" {
                records.push((AssetRole::TransformationEvidence, ".apple.json", true));
            }
            for (role, required) in &policy.supplemental_assets {
                let suffix = match role {
                    AssetRole::NativeSbom => ".native.cdx.json",
                    AssetRole::EmbeddedMetadata => ".embedded.json",
                    AssetRole::Diagnostic => ".diagnostic.json",
                    AssetRole::RebuildEvidence => ".rebuild.json",
                    _ => return Err(crate::Error::Invalid("invalid-supplemental-role".into())),
                };
                records.push((*role, suffix, *required));
            }
            for (role, suffix, required) in records {
                let name = format!("{key}{suffix}");
                specs.push(AssetRequirement {
                    name: name.clone(),
                    role,
                    subjects: vec![final_name.clone()],
                    predicate: None,
                    predicate_asset: None,
                    required,
                });
                specs.push(AssetRequirement {
                    name: format!("{name}.provenance.sigstore.json"),
                    role: AssetRole::AttestationBundle,
                    subjects: vec![name],
                    predicate: Some(Predicate::SlsaProvenanceV1),
                    predicate_asset: None,
                    required,
                });
            }
            specs.push(AssetRequirement {
                name: format!("{final_name}.provenance.sigstore.json"),
                role: AssetRole::AttestationBundle,
                subjects: vec![final_name.clone()],
                predicate: Some(Predicate::SlsaProvenanceV1),
                predicate_asset: None,
                required: true,
            });
            specs.push(AssetRequirement {
                name: format!("{final_name}.sbom.sigstore.json"),
                role: AssetRole::AttestationBundle,
                subjects: vec![final_name],
                predicate: Some(Predicate::CycloneDxV15),
                predicate_asset: Some(format!("{key}.cdx.json")),
                required: true,
            });
        }
    }
    let mut names = BTreeSet::new();
    require(
        specs.len() <= 8192
            && specs
                .iter()
                .all(|s| asset_name(&s.name) && names.insert(&s.name)),
        "invalid-derived-asset-set",
    )?;
    Ok(specs)
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReleaseInventory {
    #[schemars(range(min = 1, max = 1))]
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub schema_version: u32,
    pub inputs: InputIdentity,
    pub assets: Vec<Asset>,
}

impl ReleaseInventory {
    /// Check release claims against trusted input context and reviewed intent.
    /// This is semantic consistency, not cryptographic/platform verification.
    pub fn validate_against(
        &self,
        config: &Config,
        policy: &VerificationPolicy,
        expected_inputs: &InputIdentity,
        now: u64,
    ) -> Result<()> {
        require(
            self.schema_version == 1,
            "unsupported-release-inventory-version",
        )?;
        self.inputs.validate()?;
        expected_inputs.validate()?;
        policy.validate(now)?;
        policy.accept_source(&expected_inputs.source)?;
        require(
            self.inputs == *expected_inputs
                && config.repository == expected_inputs.source.repository,
            "release-input-mismatch",
        )?;
        let expected = expected_assets(config, policy)?;
        let expected_by_name: BTreeMap<_, _> =
            expected.iter().map(|asset| (&asset.name, asset)).collect();
        let mut actual = BTreeMap::new();
        for a in &self.assets {
            a.bytes.validate()?;
            require(
                asset_name(&a.name) && actual.insert(&a.name, a).is_none(),
                "invalid-or-duplicate-asset",
            )?;
            require(
                a.name != INVENTORY_NAME && a.name != INVENTORY_BUNDLE_NAME,
                "inventory-hash-cycle",
            )?;
            let spec = expected_by_name
                .get(&a.name)
                .ok_or_else(|| crate::Error::Invalid("unexpected-asset".into()))?;
            require(
                a.role == spec.role
                    && a.subjects == spec.subjects
                    && a.predicate == spec.predicate
                    && a.predicate_asset == spec.predicate_asset,
                "asset-relationship-mismatch",
            )?;
        }
        for e in &expected {
            let needed = e.required
                || (e.role == AssetRole::AttestationBundle
                    && e.subjects.iter().any(|s| actual.contains_key(s)));
            require(
                !needed || actual.contains_key(&e.name),
                "required-asset-missing",
            )?;
        }
        for a in &self.assets {
            for name in a.subjects.iter().chain(a.predicate_asset.iter()) {
                require(actual.contains_key(name), "dangling-asset-subject")?;
            }
        }
        // Expected relationships only point bundle -> evidence -> distributable.
        // Exact equality above rules out cycles and unsigned/final subject swaps.
        Ok(())
    }

    /// Compare an already bounded asset-byte map with the inventory, without
    /// executing or extracting any artifact. #8 authenticates the inventory first.
    pub fn compare_asset_bytes(&self, supplied: &BTreeMap<String, Vec<u8>>) -> Result<()> {
        require(supplied.len() == self.assets.len(), "asset-set-mismatch")?;
        let mut seen = BTreeSet::new();
        for a in &self.assets {
            require(seen.insert(&a.name), "duplicate-asset")?;
            let bytes = supplied
                .get(&a.name)
                .ok_or_else(|| crate::Error::Invalid("asset-missing".into()))?;
            a.bytes.matches(bytes)?;
        }
        Ok(())
    }
}

impl ReleaseInventory {
    /// Check the complete published set, including detached inventory authentication.
    /// Both expected identities must come from separately authenticated verification;
    /// this method only compares bytes and the parsed inventory representation.
    pub fn compare_published_bytes(
        &self,
        supplied: &BTreeMap<String, Vec<u8>>,
        expected_inventory: &ByteIdentity,
        expected_inventory_bundle: &ByteIdentity,
    ) -> Result<()> {
        require(
            supplied.len() == self.assets.len() + 2,
            "published-asset-set-mismatch",
        )?;
        let inventory_bytes = supplied
            .get(INVENTORY_NAME)
            .ok_or_else(|| crate::Error::Invalid("published-inventory-missing".into()))?;
        let bundle = supplied
            .get(INVENTORY_BUNDLE_NAME)
            .ok_or_else(|| crate::Error::Invalid("inventory-authentication-missing".into()))?;
        expected_inventory.matches(inventory_bytes)?;
        expected_inventory_bundle.matches(bundle)?;
        let parsed: ReleaseInventory = super::parse_json(inventory_bytes)?;
        require(
            serde_json::to_value(&parsed).map_err(|_| crate::Error::Json)?
                == serde_json::to_value(self).map_err(|_| crate::Error::Json)?,
            "published-inventory-content-mismatch",
        )?;
        let mut seen = BTreeSet::new();
        for a in &self.assets {
            require(seen.insert(&a.name), "duplicate-asset")?;
            let bytes = supplied
                .get(&a.name)
                .ok_or_else(|| crate::Error::Invalid("published-asset-missing".into()))?;
            a.bytes.matches(bytes)?;
        }
        Ok(())
    }
}
