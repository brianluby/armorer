//! Owned release transitions over authenticated files and independently approved intent.
//! Serialized plans and journals do not authenticate release bytes or grant a provider permit.
mod github;
#[cfg(test)]
mod tests;

use super::{
    apple::VerifiedAppleRelease, context::TrustedReleaseContext, io,
    release::AuthenticatedReleaseFiles,
};
use crate::{
    Error, Result,
    trust::{
        ByteIdentity, InputIdentity, WorkflowIdentity, asset_name,
        inventory::{INVENTORY_BUNDLE_NAME, INVENTORY_NAME},
        policy::VerificationMode,
        release_tag, require,
    },
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub use github::{CapabilityReport, GateState, NativeGithub};

/// Independently approved publication policy, separately versioned from frozen trust v1.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublicationPolicy {
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub schema_version: u32,
    pub context: ByteIdentity,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub repository_id: u64,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub owner_id: u64,
    pub release_attestation_root: ByteIdentity,
    pub default_branch: String,
    pub controller_workflow: WorkflowIdentity,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub allowed_actor_ids: BTreeSet<u64>,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub allowed_publisher_ids: BTreeSet<u64>,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub allowed_approver_ids: BTreeSet<u64>,
    pub publish_environment: String,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub publish_environment_id: u64,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub max_observation_age: u64,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub expires_at: u64,
}

/// Exact policy bytes become intent only through an independently supplied approved digest.
/// ```compile_fail
/// let p: armorer::verification::publication::TrustedPublicationPolicy = serde_json::from_str("{}").unwrap();
/// ```
pub struct TrustedPublicationPolicy {
    policy: PublicationPolicy,
    identity: ByteIdentity,
}
impl TrustedPublicationPolicy {
    /// Authenticate policy bytes before decoding settings or opening a credential-bearing adapter.
    pub fn open(
        path: &Path,
        approved_sha256: &str,
        context: &TrustedReleaseContext,
    ) -> Result<Self> {
        let bytes = io::read_bounded(path, 1024 * 1024)?;
        require(
            crate::config::hex_digest(approved_sha256, 64)
                && crate::digest(&bytes) == approved_sha256,
            "unapproved-publication-policy",
        )?;
        let policy: PublicationPolicy = io::parse(&bytes)?;
        require(
            policy.context == *context.identity(),
            "publication-context-mismatch",
        )?;
        let trusted = Self {
            policy,
            identity: ByteIdentity::from_bytes(&bytes),
        };
        trusted.validate(now()?)?;
        Ok(trusted)
    }
    /// Reject expired intent, unknown identities, unsafe routes and an unbounded freshness window.
    fn validate(&self, current: u64) -> Result<()> {
        let p = &self.policy;
        p.context.validate()?;
        p.release_attestation_root.validate()?;
        p.controller_workflow.validate()?;
        require(
            p.schema_version == 1
                && p.repository_id > 0
                && p.owner_id > 0
                && asset_name(&p.default_branch)
                && asset_name(&p.publish_environment)
                && p.publish_environment_id > 0
                && (1..=300).contains(&p.max_observation_age)
                && current < p.expires_at
                && [
                    &p.allowed_actor_ids,
                    &p.allowed_publisher_ids,
                    &p.allowed_approver_ids,
                ]
                .iter()
                .all(|ids| !ids.is_empty() && ids.len() <= 100 && !ids.contains(&0)),
            "invalid-publication-policy",
        )
    }
    /// Return the exact approved policy identity, never a live capability or permission.
    pub fn identity(&self) -> &ByteIdentity {
        &self.identity
    }
}

/// Reviewable exact publication intent. It cannot construct authenticated file or platform proofs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublicationPlan {
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub schema_version: u32,
    pub context: ByteIdentity,
    pub policy: ByteIdentity,
    pub inputs: InputIdentity,
    pub tag: String,
    pub assets: BTreeMap<String, ByteIdentity>,
}

/// An authenticated private file set with an independently derived publication plan.
pub struct FrozenRelease {
    workspace: tempfile::TempDir,
    plan: PublicationPlan,
    identity: ByteIdentity,
}
impl FrozenRelease {
    /// Freeze every cryptographically verified file, including the detached inventory, without executing assets.
    pub fn freeze(
        files: &AuthenticatedReleaseFiles,
        apple: &VerifiedAppleRelease,
        context: &TrustedReleaseContext,
        policy: &TrustedPublicationPolicy,
    ) -> Result<Self> {
        context.validate_at(now()?)?;
        policy.validate(now()?)?;
        require(
            files.context_identity() == context.identity()
                && apple.context_identity() == context.identity()
                && policy.policy.context == *context.identity()
                && context.policy().mode == VerificationMode::Release,
            "publication-requires-exact-release-proofs",
        )?;
        let mut names = files
            .inventory()
            .assets
            .iter()
            .map(|a| a.name.clone())
            .collect::<BTreeSet<_>>();
        names.insert(INVENTORY_NAME.into());
        names.insert(INVENTORY_BUNDLE_NAME.into());
        let workspace = private_workspace("armorer-frozen-release-")?;
        let mut assets = BTreeMap::new();
        let mut total = 0_u64;
        for name in names {
            require(asset_name(&name), "invalid-publication-asset-name")?;
            let bytes = files.asset_bytes(&name)?;
            total = total.checked_add(bytes.len() as u64).ok_or(Error::Json)?;
            require(
                total <= 4 * 1024 * 1024 * 1024,
                "publication-total-byte-limit",
            )?;
            let identity = ByteIdentity::from_bytes(&bytes);
            identity.validate()?;
            io::write_readonly(&workspace.path().join(&name), &bytes)?;
            assets.insert(name, identity);
        }
        let tag = context
            .inputs()
            .source
            .git_ref
            .strip_prefix("refs/tags/")
            .ok_or_else(|| Error::Invalid("publication-stable-tag-required".into()))?
            .to_owned();
        require(
            release_tag(&tag) && files.inventory().inputs == *context.inputs(),
            "publication-source-binding",
        )?;
        let plan = PublicationPlan {
            schema_version: 1,
            context: context.identity().clone(),
            policy: policy.identity.clone(),
            inputs: context.inputs().clone(),
            tag,
            assets,
        };
        let identity =
            ByteIdentity::from_bytes(&serde_json::to_vec(&plan).map_err(|_| Error::Json)?);
        Ok(Self {
            workspace,
            plan,
            identity,
        })
    }
    /// Borrow exact reviewable intent; this projection carries no mutation authority.
    pub fn plan(&self) -> &PublicationPlan {
        &self.plan
    }
    /// Identify the canonical plan bytes that a separate approval must bind.
    pub fn identity(&self) -> &ByteIdentity {
        &self.identity
    }
    /// Rehash one inert frozen snapshot immediately before an upload or native asset verification.
    fn asset_path(&self, name: &str) -> Result<PathBuf> {
        let identity = self.plan.assets.get(name).ok_or(Error::Json)?;
        let path = self.workspace.path().join(name);
        require(
            io::identity(&path, identity.size)? == *identity,
            "frozen-publication-bytes-changed",
        )?;
        Ok(path)
    }
}

/// Draft staging and public publication are distinct, independently approved actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalKind {
    StageDraft,
    PublishDraft,
}

/// Human approval claims must be authenticated by an independent digest before they become a permit.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublicationApproval {
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub schema_version: u32,
    pub kind: ApprovalKind,
    pub plan: ByteIdentity,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub approver_id: u64,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub approved_at: u64,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub expires_at: u64,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub release_id: Option<u64>,
    pub served_bytes: Option<ByteIdentity>,
}
/// Private approval proof. A producer-supplied JSON record is never an independent approval.
pub struct ApprovedPublication {
    approval: PublicationApproval,
    identity: ByteIdentity,
}
impl ApprovedPublication {
    /// Match separately approved bytes first; bind action, plan, approver and a bounded current window.
    pub fn open(
        path: &Path,
        independently_approved_sha256: &str,
        release: &FrozenRelease,
        policy: &TrustedPublicationPolicy,
    ) -> Result<Self> {
        let bytes = io::read_bounded(path, 1024 * 1024)?;
        require(
            crate::config::hex_digest(independently_approved_sha256, 64)
                && crate::digest(&bytes) == independently_approved_sha256,
            "unapproved-publication-action",
        )?;
        let proof = Self {
            approval: io::parse(&bytes)?,
            identity: ByteIdentity::from_bytes(&bytes),
        };
        proof.validate(release, policy, now()?)?;
        Ok(proof)
    }
    /// Recheck exact intent and approval validity at each mutation boundary.
    fn validate(
        &self,
        release: &FrozenRelease,
        policy: &TrustedPublicationPolicy,
        current: u64,
    ) -> Result<()> {
        let a = &self.approval;
        a.plan.validate()?;
        require(
            a.schema_version == 1
                && a.plan == release.identity
                && policy.identity == release.plan.policy
                && policy.policy.allowed_approver_ids.contains(&a.approver_id)
                && a.approved_at > 0
                && a.approved_at <= current
                && current < a.expires_at
                && a.expires_at - a.approved_at <= 86400,
            "invalid-or-expired-publication-approval",
        )?;
        match a.kind {
            ApprovalKind::StageDraft => require(
                a.release_id.is_none() && a.served_bytes.is_none(),
                "stage-approval-scope",
            ),
            ApprovalKind::PublishDraft => {
                require(
                    a.release_id.is_some_and(|id| id > 0),
                    "publish-approval-release-required",
                )?;
                a.served_bytes.as_ref().ok_or(Error::Json)?.validate()
            }
        }
    }
}

/// Serialized progress is retained in a protected controller-owned directory, outside build workspaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Phase {
    Creating,
    OwnedDraft,
    Uploading,
    Uploaded,
    DraftVerified,
    Publishing,
    Published,
    Complete,
    Conflict,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema_version: u32,
    plan: ByteIdentity,
    phase: Phase,
    release_id: Option<u64>,
    served_bytes: Option<ByteIdentity>,
    approval: Option<ByteIdentity>,
    uploaded: BTreeMap<String, ServedAsset>,
}

/// A served-byte receipt identifies actual asset IDs and rehashed downloads, not provider digest claims alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DraftVerification {
    pub release_id: u64,
    pub receipt: ByteIdentity,
    pub assets: BTreeMap<String, ServedAsset>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServedAsset {
    pub id: u64,
    pub bytes: ByteIdentity,
}

/// One local repository-wide lock serializes tags; Actions must additionally serialize independent runners.
struct StateStore {
    root: PathBuf,
    _lock: File,
    journal: PathBuf,
}
impl Drop for StateStore {
    /// Release the completed controller lock even while a concurrent spawn retains a duplicate.
    fn drop(&mut self) {
        // File closure remains a fallback if explicit unlocking fails.
        let _ = self._lock.unlock();
    }
}
impl StateStore {
    /// Acquire a kernel-released exclusive lock; refuse links, unsafe permissions and conflicting journals.
    fn open(root: &Path, repository_id: u64) -> Result<Self> {
        let metadata = fs::symlink_metadata(root)?;
        require(metadata.is_dir(), "publication-state-directory-type")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            require(
                metadata.permissions().mode() & 0o077 == 0,
                "publication-state-directory-permissions",
            )?;
        }
        let root = root.canonicalize()?;
        let path = root.join(format!("{repository_id}.lock"));
        if path.try_exists()? {
            require(
                fs::symlink_metadata(&path)?.is_file(),
                "publication-lock-type",
            )?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)?;
        require(file.try_lock().is_ok(), "publication-controller-busy")?;
        let journal = root.join(format!("{repository_id}.json"));
        Ok(Self {
            root,
            _lock: file,
            journal,
        })
    }
    /// Load a bounded exact-plan journal; it is state, never a substitute for remote authentication.
    fn load(&self, plan: &ByteIdentity) -> Result<Option<Journal>> {
        if !self.journal.try_exists()? {
            return Ok(None);
        }
        let value: Journal = io::parse(&io::read_bounded(&self.journal, 1024 * 1024)?)?;
        require(
            value.schema_version == 1 && value.plan == *plan,
            "publication-journal-conflict",
        )?;
        Ok(Some(value))
    }
    /// Persist mutation intent before a network write, with atomic replacement and directory fsync.
    fn save(&self, journal: &Journal) -> Result<()> {
        let mut file = tempfile::NamedTempFile::new_in(&self.root)?;
        file.write_all(&serde_json::to_vec(journal).map_err(|_| Error::Json)?)?;
        file.as_file().sync_all()?;
        file.persist(&self.journal)
            .map_err(|_| Error::Invalid("publication-journal-persist-failed".into()))?;
        File::open(&self.root)?.sync_all()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RemoteAsset {
    id: u64,
    name: String,
    bytes: ByteIdentity,
    uploader: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct RemoteRelease {
    id: u64,
    tag: String,
    target: String,
    author: u64,
    body: String,
    draft: bool,
    immutable: bool,
    prerelease: bool,
    assets: Vec<RemoteAsset>,
}

/// Only the compiled native backend can supply production observations; test doubles remain private.
trait Backend {
    /// Obtain authenticated current prerequisites through the sealed native implementation.
    fn check(
        &mut self,
        release: &FrozenRelease,
        policy: &TrustedPublicationPolicy,
    ) -> Result<CapabilityReport>;
    /// List existing tag conflicts without adopting any unowned release.
    fn releases_for_tag(&mut self, tag: &str) -> Result<Vec<RemoteRelease>>;
    /// Read only one previously journal-owned immutable provider ID.
    fn release(&mut self, id: u64) -> Result<RemoteRelease>;
    /// Create a fixed empty owned draft after authenticating the exact source tag.
    fn create(&mut self, release: &FrozenRelease, body: &str) -> Result<RemoteRelease>;
    /// Upload a previously frozen exact asset without overwrite or deletion.
    fn upload(&mut self, id: u64, name: &str, path: &Path) -> Result<RemoteAsset>;
    /// Return bounded inert served bytes for an exact authenticated asset ID.
    fn download(&mut self, id: u64, maximum: u64) -> Result<Vec<u8>>;
    /// Publish only the separately approved journal-owned provider ID once.
    fn publish(&mut self, id: u64) -> Result<()>;
    /// Authenticate the exact complete published subject set through the pinned native verifier.
    fn verify_attestation(
        &mut self,
        release: &FrozenRelease,
        served: &DraftVerification,
    ) -> Result<()>;
}

struct Controller<B: Backend> {
    backend: B,
    release: FrozenRelease,
    policy: TrustedPublicationPolicy,
    state: StateStore,
    context: Option<TrustedReleaseContext>,
}
impl<B: Backend> Controller<B> {
    /// Bind ownership and current native prerequisites before every write, including identical retries.
    fn preflight(&mut self, approval: &ApprovedPublication, kind: ApprovalKind) -> Result<()> {
        if let Some(context) = &self.context {
            context.validate_at(now()?)?;
            require(
                context.identity() == &self.release.plan.context
                    && context.inputs() == &self.release.plan.inputs,
                "publication-current-context-binding",
            )?;
        }
        self.policy.validate(now()?)?;
        approval.validate(&self.release, &self.policy, now()?)?;
        require(
            approval.approval.kind == kind,
            "publication-approval-action-mismatch",
        )?;
        let report = self.backend.check(&self.release, &self.policy)?;
        report.require_ready(now()?, self.policy.policy.max_observation_age)?;
        if kind == ApprovalKind::PublishDraft {
            require(
                [report.actor_id, report.triggering_actor_id]
                    .into_iter()
                    .all(|actor| {
                        actor.is_some_and(|id| id > 0 && id != approval.approval.approver_id)
                    }),
                "publication-self-approval",
            )?;
        }
        Ok(())
    }
    /// Derive the exact nonsecret ownership marker from independently authenticated plan/run/attempt.
    fn marker(&self) -> String {
        format!(
            "Armorer owned draft v1\nplan-sha256: {}\nrun: {}\nattempt: {}\n",
            self.release.identity.sha256,
            self.release.plan.inputs.run.id,
            self.release.plan.inputs.run.attempt
        )
    }
    /// Require the owned release and a subset or exact set of unchanged provider asset identities.
    fn validate_remote(&self, value: &RemoteRelease, complete: bool, draft: bool) -> Result<()> {
        require(
            value.id > 0
                && value.tag == self.release.plan.tag
                && value.target == self.release.plan.inputs.source.commit
                && self
                    .policy
                    .policy
                    .allowed_publisher_ids
                    .contains(&value.author)
                && value.draft == draft
                && !value.prerelease
                && (!draft || value.body == self.marker())
                && (draft || value.immutable),
            "owned-release-conflict",
        )?;
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        for asset in &value.assets {
            require(
                asset.id > 0
                    && ids.insert(asset.id)
                    && names.insert(&asset.name)
                    && self.release.plan.assets.get(&asset.name) == Some(&asset.bytes)
                    && self
                        .policy
                        .policy
                        .allowed_publisher_ids
                        .contains(&asset.uploader),
                "owned-asset-conflict",
            )?;
        }
        require(
            !complete || names.len() == self.release.plan.assets.len(),
            "owned-release-incomplete",
        )
    }
    /// Create only a new owned draft, then upload missing exact assets without overwrite or tag mutation APIs.
    fn stage(&mut self, approval: &ApprovedPublication) -> Result<DraftVerification> {
        self.preflight(approval, ApprovalKind::StageDraft)?;
        let mut journal = if let Some(journal) = self.state.load(&self.release.identity)? {
            require(
                matches!(
                    journal.phase,
                    Phase::OwnedDraft | Phase::Uploaded | Phase::DraftVerified
                ),
                "draft-retry-requires-owned-state",
            )?;
            journal
        } else {
            require(
                self.backend
                    .releases_for_tag(&self.release.plan.tag)?
                    .is_empty(),
                "release-tag-already-owned-or-published",
            )?;
            let mut journal = Journal {
                schema_version: 1,
                plan: self.release.identity.clone(),
                phase: Phase::Creating,
                release_id: None,
                served_bytes: None,
                approval: Some(approval.identity.clone()),
                uploaded: BTreeMap::new(),
            };
            self.state.save(&journal)?;
            self.preflight(approval, ApprovalKind::StageDraft)?;
            let remote = self.backend.create(&self.release, &self.marker())?;
            self.validate_remote(&remote, false, true)?;
            require(remote.assets.is_empty(), "new-draft-not-empty")?;
            journal.release_id = Some(remote.id);
            journal.phase = Phase::OwnedDraft;
            self.state.save(&journal)?;
            journal
        };
        let id = journal.release_id.ok_or(Error::Json)?;
        let remote = self.backend.release(id)?;
        self.validate_remote(&remote, false, true)?;
        self.validate_recorded_assets(&remote, &journal)?;
        if let Some(previous) = &journal.served_bytes {
            let verified = self.verify_bytes(id, true)?;
            require(
                previous == &verified.receipt,
                "draft-retry-served-identity-conflict",
            )?;
            return Ok(verified);
        }
        let names = self.release.plan.assets.keys().cloned().collect::<Vec<_>>();
        for name in names {
            let current = self.backend.release(id)?;
            self.validate_remote(&current, false, true)?;
            self.validate_recorded_assets(&current, &journal)?;
            if current.assets.iter().any(|asset| asset.name == name) {
                continue;
            }
            let path = self.release.asset_path(&name)?;
            self.preflight(approval, ApprovalKind::StageDraft)?;
            journal.phase = Phase::Uploading;
            self.state.save(&journal)?;
            let uploaded = self.backend.upload(id, &name, &path)?;
            require(
                uploaded.name == name
                    && self.release.plan.assets.get(&name) == Some(&uploaded.bytes)
                    && uploaded.id > 0
                    && self
                        .policy
                        .policy
                        .allowed_publisher_ids
                        .contains(&uploaded.uploader),
                "publication-upload-result-conflict",
            )?;
            journal.uploaded.insert(
                name,
                ServedAsset {
                    id: uploaded.id,
                    bytes: uploaded.bytes,
                },
            );
            journal.phase = Phase::OwnedDraft;
            self.state.save(&journal)?;
        }
        journal.phase = Phase::Uploaded;
        self.state.save(&journal)?;
        let verified = self.verify_bytes(id, true)?;
        journal.phase = Phase::DraftVerified;
        journal.served_bytes = Some(verified.receipt.clone());
        self.state.save(&journal)?;
        Ok(verified)
    }
    /// Reject unknown, replaced or missing asset IDs, even when their claimed bytes match the plan.
    fn validate_recorded_assets(&self, remote: &RemoteRelease, journal: &Journal) -> Result<()> {
        require(
            remote.assets.len() == journal.uploaded.len()
                && remote.assets.iter().all(|asset| {
                    journal.uploaded.get(&asset.name)
                        == Some(&ServedAsset {
                            id: asset.id,
                            bytes: asset.bytes.clone(),
                        })
                }),
            "owned-draft-asset-id-conflict",
        )
    }
    /// Re-download all assets by exact provider ID and bracket them with unchanged release snapshots.
    fn verify_bytes(&mut self, id: u64, draft: bool) -> Result<DraftVerification> {
        let before = self.backend.release(id)?;
        self.validate_remote(&before, true, draft)?;
        let mut assets = BTreeMap::new();
        for asset in &before.assets {
            let bytes = self.backend.download(asset.id, asset.bytes.size)?;
            asset.bytes.matches(&bytes)?;
            assets.insert(
                asset.name.clone(),
                ServedAsset {
                    id: asset.id,
                    bytes: asset.bytes.clone(),
                },
            );
        }
        let after = self.backend.release(id)?;
        require(before == after, "release-mutated-during-download")?;
        let receipt = ByteIdentity::from_bytes(
            &serde_json::to_vec(&(self.release.identity(), id, &assets))
                .map_err(|_| Error::Json)?,
        );
        Ok(DraftVerification {
            release_id: id,
            receipt,
            assets,
        })
    }
    /// Publish only the exact freshly downloaded owned draft approved independently for this action.
    fn publish(&mut self, approval: &ApprovedPublication) -> Result<DraftVerification> {
        self.preflight(approval, ApprovalKind::PublishDraft)?;
        let mut journal = self
            .state
            .load(&self.release.identity)?
            .ok_or(Error::Json)?;
        require(
            journal.phase == Phase::DraftVerified
                && journal.release_id == approval.approval.release_id
                && journal.served_bytes == approval.approval.served_bytes,
            "publish-approval-owned-draft-mismatch",
        )?;
        let id = journal.release_id.ok_or(Error::Json)?;
        let served = self.verify_bytes(id, true)?;
        require(
            journal.served_bytes.as_ref() == Some(&served.receipt),
            "approved-draft-bytes-changed",
        )?;
        self.preflight(approval, ApprovalKind::PublishDraft)?;
        let current = self.backend.release(id)?;
        self.validate_remote(&current, true, true)?;
        self.validate_recorded_assets(&current, &journal)?;
        journal.phase = Phase::Publishing;
        journal.approval = Some(approval.identity.clone());
        self.state.save(&journal)?;
        self.backend.publish(id)?;
        self.finish_published(journal)
    }
    /// Authenticate published immutable served bytes and the native cryptographically verified release attestation.
    fn finish_published(&mut self, mut journal: Journal) -> Result<DraftVerification> {
        let id = journal.release_id.ok_or(Error::Json)?;
        let served = self.verify_bytes(id, false)?;
        require(
            journal.served_bytes.as_ref() == Some(&served.receipt),
            "published-byte-set-conflict",
        )?;
        journal.phase = Phase::Published;
        self.state.save(&journal)?;
        self.backend.verify_attestation(&self.release, &served)?;
        journal.phase = Phase::Complete;
        self.state.save(&journal)?;
        Ok(served)
    }
    /// Recover an ambiguous publish through read-only verification; never repeat the publication write.
    fn recover_published(&mut self) -> Result<DraftVerification> {
        let journal = self
            .state
            .load(&self.release.identity)?
            .ok_or(Error::Json)?;
        require(
            matches!(
                journal.phase,
                Phase::Publishing | Phase::Published | Phase::Complete
            ),
            "published-recovery-state-required",
        )?;
        self.finish_published(journal)
    }
}

/// Native controller; callers cannot install a fake backend or deserialize authenticated authority.
pub struct OwnedDraftController {
    inner: Controller<NativeGithub>,
}
impl OwnedDraftController {
    /// Acquire protected local ownership state after authenticating the independently reviewed release plan.
    pub fn open(
        backend: NativeGithub,
        release: FrozenRelease,
        policy: TrustedPublicationPolicy,
        state_directory: &Path,
        context: TrustedReleaseContext,
    ) -> Result<Self> {
        policy.validate(now()?)?;
        require(
            release.plan.policy == policy.identity
                && backend.repository() == release.plan.inputs.source.repository
                && context.identity() == &release.plan.context
                && context.inputs() == &release.plan.inputs,
            "publication-controller-binding",
        )?;
        let state = StateStore::open(state_directory, policy.policy.repository_id)?;
        Ok(Self {
            inner: Controller {
                backend,
                release,
                policy,
                state,
                context: Some(context),
            },
        })
    }
    /// Read current platform observations without creating drafts, uploading assets or publishing.
    pub fn inspect(&mut self) -> Result<CapabilityReport> {
        self.inner
            .backend
            .check(&self.inner.release, &self.inner.policy)
    }
    /// Stage and verify only a separately approved new or identical owned draft; never replace assets.
    pub fn stage(&mut self, approval: &ApprovedPublication) -> Result<DraftVerification> {
        self.inner.stage(approval)
    }
    /// Recheck every mutable gate and served byte before the separately approved one-time publication.
    pub fn publish(&mut self, approval: &ApprovedPublication) -> Result<DraftVerification> {
        self.inner.publish(approval)
    }
    /// Verify a possibly completed write without another publication attempt or any tag mutation.
    pub fn recover_published(&mut self) -> Result<DraftVerification> {
        self.inner.recover_published()
    }
}

/// Read wall time for security freshness without accepting a caller-supplied clock.
fn now() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| Error::Invalid("publication-clock-unavailable".into()))
}

/// Create a private directory before writing snapshots or opening any credential-bearing child.
fn private_workspace(prefix: &str) -> Result<tempfile::TempDir> {
    Ok(crate::filesystem::private_tempdir(prefix, None)?)
}
