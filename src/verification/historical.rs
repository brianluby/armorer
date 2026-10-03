//! Explicit legacy byte comparison. Never called by authenticated verification or on its failure.
use super::{io, sigstore};
use crate::{
    Error, Result,
    config::hex_digest,
    trust::{
        ByteIdentity, Source, asset_name,
        inventory::{INVENTORY_BUNDLE_NAME, INVENTORY_NAME},
        policy::{VerificationMode, VerificationPolicy},
        release_ref, require,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::{Duration, Instant},
};

const MAX_POLICY: u64 = 1024 * 1024;
const MAX_ASSET: u64 = 1024 * 1024 * 1024;
const MAX_TOTAL: u64 = 4 * 1024 * 1024 * 1024;
const MAX_FILES: usize = 8192;
const DEADLINE: Duration = Duration::from_secs(20 * 60);

/// Exact bytes matched an independently selected historical allowlist, without signature authentication.
/// This result cannot become an authenticated attestation or release proof.
/// ```compile_fail
/// let result: armorer::verification::historical::HistoricalByteMatch = serde_json::from_str("{}").unwrap();
/// ```
pub struct HistoricalByteMatch {
    // Retain private inert snapshots for the lifetime of this comparison result.
    _workspace: tempfile::TempDir,
    source: Source,
    policy_identity: ByteIdentity,
    identities: BTreeMap<String, ByteIdentity>,
    limitations: Vec<String>,
}

impl HistoricalByteMatch {
    /// Explicitly compare an exact independently approved historical policy/source against regular files.
    /// No native tool, repository build, network access, extraction or signature fallback is performed.
    /// Modern evidence filenames are forbidden even if mistakenly included in the historical allowlist.
    pub fn verify(
        directory: &Path,
        policy_path: &Path,
        approved_policy_sha256: &str,
        source: &Source,
    ) -> Result<Self> {
        let started = Instant::now();
        require(
            hex_digest(approved_policy_sha256, 64),
            "invalid-approved-historical-policy-digest",
        )?;
        let policy_bytes = io::read_bounded(policy_path, MAX_POLICY)?;
        let policy_identity = ByteIdentity::from_bytes(&policy_bytes);
        require(
            policy_identity.sha256 == approved_policy_sha256,
            "unapproved-historical-policy",
        )?;
        // A downloaded policy cannot assign itself meaning before its independent digest is matched.
        let policy: VerificationPolicy = io::parse(&policy_bytes)?;
        policy.validate(sigstore::wall_time()?)?;
        source.validate()?;
        require(
            policy.mode == VerificationMode::Release && release_ref(&source.git_ref),
            "historical-release-mode-required",
        )?;
        let historical = policy
            .historical
            .iter()
            .find(|item| item.source == *source)
            .ok_or_else(|| Error::Invalid("historical-release-not-allowed".into()))?;
        require(
            historical.assets.len() <= MAX_FILES
                && historical.limitations.len() <= 128
                && historical.limitations.iter().all(|item| {
                    !item.trim().is_empty()
                        && item.len() <= 4096
                        && !item.chars().any(char::is_control)
                }),
            "historical-policy-limits",
        )?;
        let mut total = 0_u64;
        for (name, bytes) in &historical.assets {
            historical_name(name)?;
            total = total
                .checked_add(bytes.size)
                .ok_or_else(|| Error::Invalid("historical-total-byte-limit".into()))?;
        }
        require(total <= MAX_TOTAL, "historical-total-byte-limit")?;
        let names = historical.assets.keys().cloned().collect();
        compare_directory(directory, &names)?;
        let workspace = tempfile::Builder::new()
            .prefix("armorer-historical-bytes-")
            .tempdir()?;
        let mut identities = BTreeMap::new();
        for (name, expected) in &historical.assets {
            deadline(started)?;
            let path = workspace.path().join(name);
            let identity = io::snapshot(&directory.join(name), &path, MAX_ASSET)?;
            require(identity == *expected, "historical-asset-byte-mismatch")?;
            io::readonly(&path, false)?;
            identities.insert(name.clone(), identity);
        }
        // Require a stable exact input set and independently rehash every retained snapshot before success.
        compare_directory(directory, &names)?;
        for (name, expected) in &identities {
            deadline(started)?;
            require(
                io::identity(&workspace.path().join(name), MAX_ASSET)? == *expected,
                "historical-snapshot-byte-mismatch",
            )?;
        }
        deadline(started)?;
        policy.validate(sigstore::wall_time()?)?;
        let limitations = historical.limitations.clone();
        Ok(Self {
            _workspace: workspace,
            source: source.clone(),
            policy_identity,
            identities,
            limitations,
        })
    }

    /// Return the independently requested source identity; no Git or provenance claim was authenticated.
    pub fn source(&self) -> &Source {
        &self.source
    }

    /// Return the exact approved policy bytes used for this comparison.
    pub fn policy_identity(&self) -> &ByteIdentity {
        &self.policy_identity
    }

    /// Return the complete exact byte set matched at verification time, without producer authentication.
    pub fn assets(&self) -> &BTreeMap<String, ByteIdentity> {
        &self.identities
    }

    /// Return reviewed policy limitations, additional to the fixed absence of provenance authentication.
    pub fn limitations(&self) -> &[String] {
        &self.limitations
    }
}

/// Reject known authenticated inventory/attestation formats regardless of case or approved legacy hashes.
fn historical_name(name: &str) -> Result<()> {
    require(asset_name(name), "historical-download-name-invalid")?;
    let lower = name.to_ascii_lowercase();
    require(
        lower != INVENTORY_NAME
            && lower != INVENTORY_BUNDLE_NAME
            && !lower.ends_with(".sigstore.json")
            && !lower.ends_with(".sigstore.jsonl")
            && !lower.ends_with(".intoto.jsonl")
            && !lower.ends_with(".attestation.json")
            && !lower.ends_with(".attestations.json"),
        "historical-attestation-evidence-forbidden",
    )
}

/// Bound enumeration and require exactly the safe regular leaves approved by historical policy.
fn compare_directory(directory: &Path, expected: &BTreeSet<String>) -> Result<()> {
    require(
        fs::symlink_metadata(directory)?.is_dir(),
        "historical-download-directory-type",
    )?;
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| Error::Invalid("historical-download-name-invalid".into()))?;
        historical_name(&name)?;
        require(
            actual.len() < MAX_FILES && entry.file_type()?.is_file(),
            "historical-download-entry-type-or-count",
        )?;
        actual.insert(name);
    }
    require(actual == *expected, "historical-download-file-set-mismatch")
}

/// Check elapsed time between bounded operations; a slow individual filesystem read is not interrupted.
fn deadline(started: Instant) -> Result<()> {
    require(
        started.elapsed() < DEADLINE,
        "historical-verification-deadline",
    )
}
