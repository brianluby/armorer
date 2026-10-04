//! Inventory-first authentication of every declared release asset; no fallback or artifact execution.
use super::{
    context::TrustedReleaseContext,
    cyclonedx::{OfflineSbomValidator, ValidatedSbom},
    graph::CargoGraphV2,
    io,
    sigstore::{self, ExpectedAttestation, OfflineVerifier, VerifiedAttestation},
};
use crate::{
    Error, Result,
    config::Profile,
    trust::{
        ByteIdentity, asset_name,
        evidence::ArtifactEvidence,
        inventory::{Asset, AssetRole, INVENTORY_BUNDLE_NAME, INVENTORY_NAME, ReleaseInventory},
        policy::{EvidenceScope, Predicate},
        require,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::{Duration, Instant},
};

const MAX_METADATA: u64 = 17 * 1024 * 1024;
const MAX_ASSET: u64 = 1024 * 1024 * 1024;
const MAX_TOTAL: u64 = 4 * 1024 * 1024 * 1024;
const MAX_FILES: usize = 8194;
const DEADLINE: Duration = Duration::from_secs(20 * 60);

/// Check the complete authenticated claim before copying any asset bytes.
fn preflight_assets(assets: &[Asset], retained_bytes: u64) -> Result<()> {
    require(assets.len() <= MAX_FILES - 2, "release-file-count-limit")?;
    let mut total = retained_bytes;
    require(total <= MAX_TOTAL, "release-total-byte-limit")?;
    for asset in assets {
        require(
            asset.bytes.size <= asset_cap(asset.role),
            "release-asset-byte-limit",
        )?;
        total = total
            .checked_add(asset.bytes.size)
            .ok_or_else(|| Error::Invalid("release-total-byte-limit".into()))?;
        require(total <= MAX_TOTAL, "release-total-byte-limit")?;
    }
    Ok(())
}

/// Complete authenticated file/graph/schema evidence, distinct from publication or operational acceptance.
/// A failed required slot produces no result, and ordinary JSON cannot manufacture this result.
/// ```compile_fail
/// let proof: armorer::verification::release::AuthenticatedReleaseFiles = serde_json::from_str("{}").unwrap();
/// ```
pub struct AuthenticatedReleaseFiles {
    workspace: tempfile::TempDir,
    inventory: ReleaseInventory,
    inventory_proof: VerifiedAttestation,
    attestations: BTreeMap<String, VerifiedAttestation>,
    sboms: BTreeMap<String, ValidatedSbom>,
    identities: BTreeMap<String, ByteIdentity>,
    context_identity: ByteIdentity,
}
impl AuthenticatedReleaseFiles {
    /// Authenticate inventory before parsing its claims, then rehash every offered file and verify every bundle.
    /// Independently approved context and the exact approved native adapters are mandatory.
    pub fn verify(
        directory: &Path,
        context: &TrustedReleaseContext,
        verifier: &OfflineVerifier,
        sbom_validator: &OfflineSbomValidator,
    ) -> Result<Self> {
        let started = Instant::now();
        context.validate_at(sigstore::wall_time()?)?;
        context.bind_verifier(verifier)?;
        require(
            fs::symlink_metadata(directory)?.is_dir(),
            "release-download-directory-type",
        )?;
        let workspace = crate::filesystem::private_tempdir("armorer-release-files-", None)?;
        let inventory_path = workspace.path().join(INVENTORY_NAME);
        let bundle_path = workspace.path().join(INVENTORY_BUNDLE_NAME);
        let inventory_identity = io::snapshot(
            &directory.join(INVENTORY_NAME),
            &inventory_path,
            MAX_METADATA,
        )?;
        let inventory_bundle = io::snapshot(
            &directory.join(INVENTORY_BUNDLE_NAME),
            &bundle_path,
            io::MAX_BUNDLE,
        )?;
        io::readonly(&inventory_path, false)?;
        io::readonly(&bundle_path, false)?;
        // The observed digest is a subject candidate. Source, signer, run, trigger and roots are independent.
        let inventory_expected = context.attestation(
            INVENTORY_NAME,
            EvidenceScope::Inventory,
            Predicate::SlsaProvenanceV1,
        )?;
        let inventory_proof = verifier.verify(
            &inventory_path,
            &inventory_identity,
            &bundle_path,
            &inventory_expected,
        )?;
        require(
            inventory_proof.bundle() == &inventory_bundle,
            "inventory-authentication-bundle-mismatch",
        )?;
        // No asset filenames, graph claims or other producer JSON are read before that authentication succeeds.
        let inventory: ReleaseInventory =
            io::parse(&io::read_bounded(&inventory_path, MAX_METADATA)?)?;
        preflight_assets(
            &inventory.assets,
            inventory_identity.size + inventory_bundle.size,
        )?;
        inventory.validate_against(
            context.config(),
            context.policy(),
            context.inputs(),
            sigstore::wall_time()?,
        )?;
        let names: BTreeSet<_> = inventory
            .assets
            .iter()
            .map(|asset| asset.name.clone())
            .chain([INVENTORY_NAME.into(), INVENTORY_BUNDLE_NAME.into()])
            .collect();
        compare_directory(directory, &names)?;
        let mut identities = BTreeMap::from([
            (INVENTORY_NAME.into(), inventory_identity),
            (INVENTORY_BUNDLE_NAME.into(), inventory_bundle),
        ]);
        for asset in &inventory.assets {
            deadline(started)?;
            let cap = asset_cap(asset.role);
            require(asset.bytes.size <= cap, "release-asset-byte-limit")?;
            let path = workspace.path().join(&asset.name);
            let identity = io::snapshot(&directory.join(&asset.name), &path, asset.bytes.size)?;
            require(
                identity == asset.bytes,
                "authenticated-release-asset-byte-mismatch",
            )?;
            io::readonly(&path, false)?;
            identities.insert(asset.name.clone(), identity);
        }
        let mut attestations = BTreeMap::new();
        for asset in inventory
            .assets
            .iter()
            .filter(|asset| asset.role == AssetRole::AttestationBundle)
        {
            deadline(started)?;
            context.validate_at(sigstore::wall_time()?)?;
            require(
                asset.subjects.len() == 1,
                "release-attestation-subject-count",
            )?;
            let subject = inventory
                .assets
                .iter()
                .find(|candidate| candidate.name == asset.subjects[0])
                .ok_or_else(|| Error::Invalid("release-attestation-subject-missing".into()))?;
            let predicate = asset
                .predicate
                .ok_or_else(|| Error::Invalid("release-attestation-predicate-missing".into()))?;
            let scope = slot_scope(subject.role, predicate)?;
            let expected = context.attestation(&subject.name, scope, predicate)?;
            let proof = verifier.verify(
                &workspace.path().join(&subject.name),
                &subject.bytes,
                &workspace.path().join(&asset.name),
                &expected,
            )?;
            require(
                proof.bundle() == &asset.bytes,
                "release-attestation-bundle-byte-mismatch",
            )?;
            attestations.insert(asset.name.clone(), proof);
        }
        let mut sboms = BTreeMap::new();
        for selected in context.selections() {
            deadline(started)?;
            let key = selected.selection.key();
            let sbom_name = format!("{key}.cdx.json");
            let graph_name = format!("{key}.cargo-graph.json");
            let sbom_identity = identities
                .get(&sbom_name)
                .ok_or_else(|| Error::Invalid("authenticated-sbom-missing".into()))?;
            let sbom =
                sbom_validator.validate(&workspace.path().join(&sbom_name), sbom_identity)?;
            require(
                sbom.validator() == context.sbom_validator(),
                "release-sbom-validator-context-mismatch",
            )?;
            let final_name = format!(
                "{key}{}",
                if selected.selection.profile == Profile::Library {
                    ".crate"
                } else {
                    ".tar.gz"
                }
            );
            let predicate_bundle = format!("{final_name}.sbom.sigstore.json");
            let proof = attestations
                .get(&predicate_bundle)
                .ok_or_else(|| Error::Invalid("authenticated-sbom-predicate-missing".into()))?;
            validate_sbom_slot(proof.expected(), &final_name)?;
            require(
                identities.get(&final_name) == Some(proof.subject())
                    && proof.predicate() == sbom.document(),
                "authenticated-sbom-predicate-document-mismatch",
            )?;
            let graph: CargoGraphV2 = io::parse(&io::read_bounded(
                &workspace.path().join(&graph_name),
                MAX_METADATA,
            )?)?;
            if let Some(features) = context.resolved_root_features(&selected.selection)? {
                graph.validate_against_with_resolved_features(
                    context.config(),
                    &selected.selection,
                    &selected.root_component_name,
                    context.inputs(),
                    &context.lock().workflows.commit,
                    features,
                )?;
            } else {
                graph.validate_against(
                    context.config(),
                    &selected.selection,
                    &selected.root_component_name,
                    context.inputs(),
                    &context.lock().workflows.commit,
                )?;
            }
            graph.compare_sbom(sbom.document())?;
            let mut records = vec![format!("{key}.build.json"), format!("{key}.package.json")];
            if selected.selection.profile != Profile::Library
                && selected.selection.target == "aarch64-apple-darwin"
            {
                records.push(format!("{key}.apple.json"));
            }
            for name in records {
                let evidence: ArtifactEvidence = io::parse(&io::read_bounded(
                    &workspace.path().join(&name),
                    MAX_METADATA,
                )?)?;
                evidence.validate_against_requirements(
                    context.config(),
                    &inventory,
                    &selected.evidence_requirements,
                    sigstore::wall_time()?,
                )?;
                // References require actual retained authenticated report bytes, never a producer's bare hash.
                for reference in evidence
                    .steps
                    .iter()
                    .flat_map(|step| &step.platform_evidence)
                    .chain(
                        evidence
                            .apple_assertions
                            .iter()
                            .map(|apple| &apple.notarization_log),
                    )
                {
                    validate_retained_reference(&inventory.assets, reference, &final_name)?;
                }
            }
            sboms.insert(sbom_name, sbom);
        }
        // Optional reporting assets are still byte-authenticated. Unsupported required semantic adapters fail closed.
        require(
            !context
                .policy()
                .supplemental_assets
                .values()
                .any(|required| *required),
            "required-supplemental-release-semantics-unsupported",
        )?;
        deadline(started)?;
        context.validate_at(sigstore::wall_time()?)?;
        compare_directory(directory, &names)?;
        for (name, identity) in &identities {
            require(
                io::identity(
                    &workspace.path().join(name),
                    if name == INVENTORY_BUNDLE_NAME {
                        io::MAX_BUNDLE
                    } else if name == INVENTORY_NAME {
                        MAX_METADATA
                    } else {
                        asset_cap(
                            inventory
                                .assets
                                .iter()
                                .find(|a| a.name == *name)
                                .ok_or_else(|| {
                                    Error::Invalid("release-snapshot-asset-missing".into())
                                })?
                                .role,
                        )
                    },
                )? == *identity,
                "release-snapshot-changed",
            )?;
        }
        Ok(Self {
            workspace,
            inventory,
            inventory_proof,
            attestations,
            sboms,
            identities,
            context_identity: context.identity().clone(),
        })
    }

    /// Borrow the authenticated exact inventory, without allowing producer claims to mutate the proof.
    pub fn inventory(&self) -> &ReleaseInventory {
        &self.inventory
    }
    /// Borrow the actual cryptographic proof that authenticated the inventory before it was parsed.
    pub fn inventory_proof(&self) -> &VerifiedAttestation {
        &self.inventory_proof
    }
    /// Borrow every verified bundle proof; no verified subset can construct this result.
    pub fn attestations(&self) -> &BTreeMap<String, VerifiedAttestation> {
        &self.attestations
    }
    /// Borrow complete validated SBOM documents already compared with verified predicates and selected graphs.
    pub fn sboms(&self) -> &BTreeMap<String, ValidatedSbom> {
        &self.sboms
    }
    /// Return the independent context identity used throughout this complete file verification.
    pub fn context_identity(&self) -> &ByteIdentity {
        &self.context_identity
    }
    /// Read one inert verified snapshot only after rechecking its exact identity; never execute or extract it.
    pub fn asset_bytes(&self, name: &str) -> Result<Vec<u8>> {
        let expected = self
            .identities
            .get(name)
            .ok_or_else(|| Error::Invalid("authenticated-release-asset-missing".into()))?;
        let cap = if name == INVENTORY_NAME {
            MAX_METADATA
        } else if name == INVENTORY_BUNDLE_NAME {
            io::MAX_BUNDLE
        } else {
            asset_cap(
                self.inventory
                    .assets
                    .iter()
                    .find(|asset| asset.name == name)
                    .ok_or_else(|| Error::Invalid("authenticated-release-asset-missing".into()))?
                    .role,
            )
        };
        let bytes = io::read_bounded(&self.workspace.path().join(name), cap)?;
        expected.matches(&bytes)?;
        Ok(bytes)
    }
}

/// Recheck the cryptographically matched subject/scope/predicate at the local selected-SBOM boundary.
/// Inventory validation already enforces the pairing; this check never relies on a filename alone.
fn validate_sbom_slot(expected: &ExpectedAttestation, final_name: &str) -> Result<()> {
    require(
        expected.subject_name == final_name
            && expected.scope == EvidenceScope::SbomPredicate
            && expected.predicate == Predicate::CycloneDxV15,
        "authenticated-sbom-predicate-slot-mismatch",
    )
}

/// Require retained evidence bytes, an admitted evidence role and scope to this exact distributable.
fn validate_retained_reference(
    assets: &[Asset],
    reference: &ByteIdentity,
    final_name: &str,
) -> Result<()> {
    require(
        assets.iter().any(|asset| {
            asset.bytes == *reference
                && asset.subjects.iter().any(|subject| subject == final_name)
                && matches!(
                    asset.role,
                    AssetRole::Diagnostic
                        | AssetRole::BuildEvidence
                        | AssetRole::PackageEvidence
                        | AssetRole::TransformationEvidence
                )
        }),
        "release-platform-evidence-bytes-not-retained",
    )
}

/// Map exact role/predicate relationships to independently approved signer scopes.
fn slot_scope(role: AssetRole, predicate: Predicate) -> Result<EvidenceScope> {
    match (role, predicate) {
        (AssetRole::Distributable, Predicate::CycloneDxV15) => Ok(EvidenceScope::SbomPredicate),
        (AssetRole::Distributable, Predicate::SlsaProvenanceV1) => Ok(EvidenceScope::FinalArtifact),
        (AssetRole::CargoSbom, Predicate::SlsaProvenanceV1) => Ok(EvidenceScope::CargoSbomFile),
        (
            AssetRole::CargoGraph
            | AssetRole::BuildEvidence
            | AssetRole::PackageEvidence
            | AssetRole::TransformationEvidence
            | AssetRole::Diagnostic
            | AssetRole::EmbeddedMetadata
            | AssetRole::RebuildEvidence
            | AssetRole::NativeSbom,
            Predicate::SlsaProvenanceV1,
        ) => Ok(EvidenceScope::EvidenceFile),
        _ => Err(Error::Invalid("release-slot-scope-unsupported".into())),
    }
}
/// Enforce fixed per-role caps in addition to the aggregate release budget.
fn asset_cap(role: AssetRole) -> u64 {
    match role {
        AssetRole::Distributable => MAX_ASSET,
        AssetRole::AttestationBundle => io::MAX_BUNDLE,
        _ => MAX_METADATA,
    }
}
/// Refuse partial work after the fixed complete-consumer deadline has elapsed.
fn deadline(started: Instant) -> Result<()> {
    require(
        started.elapsed() < DEADLINE,
        "release-verification-deadline",
    )
}
/// Require a bounded exact set of safe regular leaves; no source directory entry becomes an authority.
fn compare_directory(directory: &Path, expected: &BTreeSet<String>) -> Result<()> {
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| Error::Invalid("release-download-name-invalid".into()))?;
        require(
            actual.len() < MAX_FILES && asset_name(&name) && entry.file_type()?.is_file(),
            "release-download-entry-type-or-count",
        )?;
        actual.insert(name);
    }
    require(actual == *expected, "release-download-file-set-mismatch")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    /// A late oversized claim fails before any asset pathname or copy is attempted.
    fn all_asset_budgets_are_preflighted() {
        let asset = |size| Asset {
            name: "not-opened.tar.gz".into(),
            role: AssetRole::Distributable,
            bytes: ByteIdentity {
                sha256: "a".repeat(64),
                size,
            },
            subjects: vec![],
            predicate: None,
            predicate_asset: None,
        };
        preflight_assets(&[asset(MAX_ASSET)], MAX_TOTAL - MAX_ASSET).unwrap();
        assert!(
            matches!(preflight_assets(&[asset(MAX_ASSET), asset(1)], MAX_TOTAL - MAX_ASSET).unwrap_err(), Error::Invalid(code) if code == "release-total-byte-limit")
        );
        assert!(
            matches!(preflight_assets(&[asset(MAX_ASSET + 1)], 0).unwrap_err(), Error::Invalid(code) if code == "release-asset-byte-limit")
        );
        assert!(
            matches!(preflight_assets(&[], u64::MAX).unwrap_err(), Error::Invalid(code) if code == "release-total-byte-limit")
        );
        let assets = vec![asset(1); MAX_FILES - 1];
        assert!(
            matches!(preflight_assets(&assets, 0).unwrap_err(), Error::Invalid(code) if code == "release-file-count-limit")
        );
    }

    /// Authenticated-looking bytes scoped to selection A cannot satisfy selection B's retained reference.
    #[test]
    fn retained_platform_evidence_requires_the_current_distributable_subject() {
        let first = "app--x86_64-unknown-linux-gnu--minimal.tar.gz";
        let second = "app--aarch64-unknown-linux-gnu--minimal.tar.gz";
        let bytes = ByteIdentity::from_bytes(b"retained first-selection evidence");
        for role in [
            AssetRole::Diagnostic,
            AssetRole::BuildEvidence,
            AssetRole::PackageEvidence,
            AssetRole::TransformationEvidence,
        ] {
            let mut asset = Asset {
                name: "first-selection.build.json".into(),
                role,
                bytes: bytes.clone(),
                subjects: vec![first.into()],
                predicate: None,
                predicate_asset: None,
            };
            validate_retained_reference(&[asset.clone()], &bytes, first).unwrap();
            assert!(validate_retained_reference(&[asset.clone()], &bytes, second).is_err());
            asset.subjects.clear();
            assert!(validate_retained_reference(&[asset.clone()], &bytes, first).is_err());
            asset.subjects.push(second.into());
            validate_retained_reference(&[asset.clone()], &bytes, second).unwrap();
            asset.bytes = ByteIdentity::from_bytes(b"different retained bytes");
            assert!(validate_retained_reference(&[asset.clone()], &bytes, second).is_err());
            asset.bytes = bytes.clone();
            asset.role = AssetRole::Distributable;
            assert!(validate_retained_reference(&[asset], &bytes, second).is_err());
        }
    }

    #[test]
    /// Check post-crypto identity semantics only; no synthetic claim constructs a signature proof.
    fn selected_sbom_slot_rejects_another_target_scope_or_predicate() {
        use crate::trust::{RunIdentity, Source, WorkflowIdentity};
        let final_name = "app--x86_64-unknown-linux-gnu--minimal.tar.gz";
        let workflow = WorkflowIdentity {
            repository: "fixture/workflows".into(),
            path: ".github/workflows/release.yml".into(),
            commit: "b".repeat(40),
        };
        let mut expected = ExpectedAttestation {
            source: Source {
                repository: "fixture/project".into(),
                commit: "a".repeat(40),
                git_ref: "refs/tags/v1.0.0".into(),
            },
            run: RunIdentity {
                id: 12,
                attempt: 1,
                workflow: workflow.clone(),
            },
            caller_workflow: WorkflowIdentity {
                repository: "fixture/project".into(),
                path: workflow.path.clone(),
                commit: "a".repeat(40),
            },
            trigger: sigstore::Trigger::Push,
            scope: EvidenceScope::SbomPredicate,
            predicate: Predicate::CycloneDxV15,
            subject_name: final_name.into(),
        };
        validate_sbom_slot(&expected, final_name).unwrap();
        expected.subject_name = "app--aarch64-unknown-linux-gnu--minimal.tar.gz".into();
        assert!(validate_sbom_slot(&expected, final_name).is_err());
        expected.subject_name = final_name.into();
        expected.scope = EvidenceScope::FinalArtifact;
        assert!(validate_sbom_slot(&expected, final_name).is_err());
        expected.scope = EvidenceScope::SbomPredicate;
        expected.predicate = Predicate::SlsaProvenanceV1;
        assert!(validate_sbom_slot(&expected, final_name).is_err());
    }
    #[test]
    /// Exact regular-leaf downloads reject extras, missing leaves and nested paths.
    fn downloaded_file_set_is_exact_and_never_traverses_subdirectories() {
        let directory = tempfile::tempdir().unwrap();
        let names = BTreeSet::from(["inventory.json".into(), "artifact.tar.gz".into()]);
        for name in &names {
            fs::write(directory.path().join(name), b"inert").unwrap();
        }
        compare_directory(directory.path(), &names).unwrap();
        fs::write(directory.path().join("extra.json"), b"extra").unwrap();
        assert!(compare_directory(directory.path(), &names).is_err());
        fs::remove_file(directory.path().join("extra.json")).unwrap();
        fs::remove_file(directory.path().join("artifact.tar.gz")).unwrap();
        assert!(compare_directory(directory.path(), &names).is_err());
        fs::create_dir(directory.path().join("artifact.tar.gz")).unwrap();
        fs::write(directory.path().join("artifact.tar.gz/hidden"), b"inert").unwrap();
        assert!(compare_directory(directory.path(), &names).is_err());
    }
    #[cfg(unix)]
    #[test]
    /// A leaf symlink cannot substitute an authentic-looking safe filename.
    fn downloaded_safe_name_cannot_hide_symlinked_bytes() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        fs::write(outside.path(), b"inert").unwrap();
        symlink(outside.path(), directory.path().join("artifact.tar.gz")).unwrap();
        assert!(
            compare_directory(
                directory.path(),
                &BTreeSet::from(["artifact.tar.gz".into()])
            )
            .is_err()
        );
    }
    #[test]
    /// Swapping an SBOM predicate onto another evidence role cannot change its independent signer scope.
    fn scoped_predicates_reject_every_non_distributable_cyclonedx_slot() {
        assert_eq!(
            slot_scope(AssetRole::Distributable, Predicate::CycloneDxV15).unwrap(),
            EvidenceScope::SbomPredicate
        );
        for role in [
            AssetRole::CargoSbom,
            AssetRole::CargoGraph,
            AssetRole::BuildEvidence,
            AssetRole::PackageEvidence,
            AssetRole::TransformationEvidence,
            AssetRole::Diagnostic,
            AssetRole::EmbeddedMetadata,
            AssetRole::RebuildEvidence,
            AssetRole::NativeSbom,
            AssetRole::AttestationBundle,
        ] {
            assert!(slot_scope(role, Predicate::CycloneDxV15).is_err());
        }
        assert!(slot_scope(AssetRole::AttestationBundle, Predicate::SlsaProvenanceV1).is_err());
    }
}
