//! Independent consumer expectations. Never populate this policy from downloads.
use super::inventory::AssetRole;
use super::{ByteIdentity, Review, Source, WorkflowIdentity, release_ref, release_tag, require};
use crate::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema,
)]
pub enum Predicate {
    #[serde(rename = "https://slsa.dev/provenance/v1")]
    SlsaProvenanceV1,
    #[serde(rename = "https://cyclonedx.org/bom")]
    CycloneDxV15,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TrustBackend {
    SigstorePublicGood,
    GithubPrivate,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrustRoots {
    pub backend: TrustBackend,
    pub trusted_root: ByteIdentity,
    pub verifier: ByteIdentity,
    pub verifier_version: String,
    pub review: Review,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SignerRule {
    pub predicate: Predicate,
    pub workflow: WorkflowIdentity,
    /// Includes inventory authentication; never an arbitrary custom predicate.
    pub scope: EvidenceScope,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceScope {
    FinalArtifact,
    CargoSbomFile,
    SbomPredicate,
    EvidenceFile,
    Inventory,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HistoricalRelease {
    pub source: Source,
    pub assets: BTreeMap<String, ByteIdentity>,
    pub review: Review,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum VerificationMode {
    Release,
    Rehearsal,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerificationPolicy {
    pub mode: VerificationMode,
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    pub repository: String,
    /// Exact reviewed release refs and commits; no regex or latest-tag policy.
    pub sources: Vec<Source>,
    pub signers: Vec<SignerRule>,
    pub roots: TrustRoots,
    /// v0.1 accepts hosted runners only.
    pub deny_self_hosted_runners: bool,
    /// Supplemental roles, true = required, false = reporting when supplied.
    /// Missing diagnostics can never relax the fixed baseline asset set.
    pub supplemental_assets: BTreeMap<AssetRole, bool>,
    pub apple_team: Option<String>,
    pub historical: Vec<HistoricalRelease>,
    pub review: Review,
}

impl VerificationPolicy {
    pub fn validate(&self, now: u64) -> Result<()> {
        require(
            self.schema_version == 1
                && crate::config::repository_name(&self.repository)
                && self.deny_self_hosted_runners,
            "invalid-verification-policy",
        )?;
        self.review.validate_at(now)?;
        self.roots.review.validate_at(now)?;
        self.roots.trusted_root.validate()?;
        self.roots.verifier.validate()?;
        require(
            super::stable_version(&self.roots.verifier_version),
            "invalid-verifier-version",
        )?;
        require(
            !self.sources.is_empty()
                && self.sources.len() <= 1024
                && self.historical.len() <= 1024
                && self.signers.len() <= 128,
            "invalid-policy-scope",
        )?;
        let mut sources = BTreeSet::new();
        for source in &self.sources {
            source.validate()?;
            let ref_allowed = match self.mode {
                VerificationMode::Release => release_ref(&source.git_ref),
                VerificationMode::Rehearsal => source.git_ref.starts_with("refs/heads/"),
            };
            require(ref_allowed, "source-mode-mismatch")?;
            require(
                source.repository == self.repository && sources.insert(&source.git_ref),
                "duplicate-or-wrong-policy-source",
            )?;
        }
        let baseline = [
            (EvidenceScope::FinalArtifact, Predicate::SlsaProvenanceV1),
            (EvidenceScope::CargoSbomFile, Predicate::SlsaProvenanceV1),
            (EvidenceScope::SbomPredicate, Predicate::CycloneDxV15),
            (EvidenceScope::EvidenceFile, Predicate::SlsaProvenanceV1),
            (EvidenceScope::Inventory, Predicate::SlsaProvenanceV1),
        ];
        let mut rules = BTreeSet::new();
        for rule in &self.signers {
            rule.workflow.validate()?;
            require(
                baseline.contains(&(rule.scope, rule.predicate))
                    && rules.insert((
                        rule.scope,
                        rule.predicate,
                        &rule.workflow.repository,
                        &rule.workflow.path,
                        &rule.workflow.commit,
                    )),
                "invalid-or-duplicate-signer-rule",
            )?;
        }
        for (scope, predicate) in baseline {
            require(
                self.signers
                    .iter()
                    .any(|r| r.scope == scope && r.predicate == predicate),
                "baseline-signer-rule-missing",
            )?;
        }
        for role in self.supplemental_assets.keys() {
            require(
                matches!(
                    role,
                    AssetRole::NativeSbom
                        | AssetRole::EmbeddedMetadata
                        | AssetRole::Diagnostic
                        | AssetRole::RebuildEvidence
                ),
                "invalid-supplemental-role",
            )?;
        }
        if let Some(team) = &self.apple_team {
            require(
                team.len() == 10
                    && team
                        .bytes()
                        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()),
                "invalid-apple-team",
            )?;
        }
        let mut historical = BTreeSet::new();
        for h in &self.historical {
            h.source.validate()?;
            require(
                self.mode == VerificationMode::Release && release_ref(&h.source.git_ref),
                "historical-mode-mismatch",
            )?;
            h.review.validate_at(now)?;
            require(
                h.source.repository == self.repository
                    && historical.insert(&h.source.git_ref)
                    && !sources.contains(&h.source.git_ref)
                    && !h.assets.is_empty()
                    && !h.limitations.is_empty(),
                "invalid-historical-policy",
            )?;
            for (name, bytes) in &h.assets {
                require(super::asset_name(name), "invalid-historical-asset-name")?;
                bytes.validate()?;
            }
        }
        Ok(())
    }

    pub fn accept_source(&self, source: &Source) -> Result<()> {
        require(self.sources.contains(source), "source-not-allowed")
    }

    /// Compare claims extracted by #8 from a successfully verified certificate.
    /// Supplying ordinary producer JSON to this function does not authenticate it.
    pub fn accept_signer(
        &self,
        source: &Source,
        workflow: &WorkflowIdentity,
        predicate: Predicate,
        scope: EvidenceScope,
        hosted: bool,
    ) -> Result<()> {
        self.accept_source(source)?;
        require(
            hosted
                && self.signers.iter().any(|s| {
                    s.workflow == *workflow && s.predicate == predicate && s.scope == scope
                }),
            "signer-or-predicate-not-allowed",
        )
    }

    /// Explicit historical selection, never an error fallback. Return weaker
    /// status only after exact allowlist bytes are matched. #8 must refuse this
    /// path if required/invalid attestation evidence exists for the requested mode.
    pub fn compare_historical_bytes(
        &self,
        tag: &str,
        source: &Source,
        supplied: &BTreeMap<String, Vec<u8>>,
        now: u64,
    ) -> Result<()> {
        self.validate(now)?;
        require(
            self.mode == VerificationMode::Release,
            "historical-rehearsal-forbidden",
        )?;
        require(
            release_tag(tag) && source.git_ref == format!("refs/tags/{tag}"),
            "historical-ref-mismatch",
        )?;
        let h = self
            .historical
            .iter()
            .find(|h| h.source == *source)
            .ok_or_else(|| crate::Error::Invalid("historical-release-not-allowed".into()))?;
        require(
            supplied.len() == h.assets.len(),
            "historical-asset-set-mismatch",
        )?;
        for (name, expected) in &h.assets {
            let bytes = supplied
                .get(name)
                .ok_or_else(|| crate::Error::Invalid("historical-asset-missing".into()))?;
            expected.matches(bytes)?;
        }
        Ok(())
    }
}
