//! Private protocol doubles exercise transitions only; they do not establish live publication acceptance.
use super::*;
use serde_json::json;

#[derive(Default)]
struct Fake {
    remote: Option<RemoteRelease>,
    bytes: BTreeMap<u64, Vec<u8>>,
    creates: usize,
    uploads: usize,
    publishes: usize,
    checks: usize,
    fail_check_at: Option<usize>,
    actor: Option<Option<u64>>,
    triggering_actor: Option<Option<u64>>,
    triggering_actor_change_at: Option<usize>,
    ambiguous_create: bool,
    ambiguous_upload: bool,
    ambiguous_publish: bool,
    bad_download: bool,
    mutate_download: bool,
    bad_immutable: bool,
    bad_attestation: bool,
}
impl Backend for Fake {
    /// Supply test-only prerequisite observations; no production caller can install this backend.
    fn check(
        &mut self,
        _: &FrozenRelease,
        policy: &TrustedPublicationPolicy,
    ) -> Result<CapabilityReport> {
        self.checks += 1;
        if self.fail_check_at == Some(self.checks) {
            return Err(Error::Invalid("test-mutable-gate-changed".into()));
        }
        if self.triggering_actor_change_at == Some(self.checks) {
            self.triggering_actor = Some(Some(3));
        }
        Ok(CapabilityReport {
            observed_at: now()?,
            repository_id: policy.policy.repository_id,
            actor_id: self.actor.unwrap_or(Some(1)),
            triggering_actor_id: self.triggering_actor.unwrap_or(Some(1)),
            gates: github::REQUIRED_GATES
                .iter()
                .map(|name| ((*name).into(), GateState::Enforced))
                .collect(),
            evidence: BTreeMap::new(),
            limitations: vec!["Synthetic state-machine observations only".into()],
        })
    }
    /// Return a conflicting pre-existing draft or publication before any new test write.
    fn releases_for_tag(&mut self, tag: &str) -> Result<Vec<RemoteRelease>> {
        Ok(self
            .remote
            .iter()
            .filter(|r| r.tag == tag)
            .cloned()
            .collect())
    }
    /// Require the exact owned ID instead of selecting a current release by tag.
    fn release(&mut self, id: u64) -> Result<RemoteRelease> {
        self.remote
            .as_ref()
            .filter(|r| r.id == id)
            .cloned()
            .ok_or(Error::Json)
    }
    /// Simulate server acceptance followed by an ambiguous client failure to test non-retry ownership.
    fn create(&mut self, release: &FrozenRelease, body: &str) -> Result<RemoteRelease> {
        self.creates += 1;
        let value = RemoteRelease {
            id: 42,
            tag: release.plan.tag.clone(),
            target: release.plan.inputs.source.commit.clone(),
            author: 2,
            body: body.into(),
            draft: true,
            immutable: false,
            prerelease: false,
            assets: vec![],
        };
        self.remote = Some(value.clone());
        if self.ambiguous_create {
            Err(Error::Json)
        } else {
            Ok(value)
        }
    }
    /// Retain exact uploaded bytes and IDs while counting every test mutation.
    fn upload(&mut self, id: u64, name: &str, path: &Path) -> Result<RemoteAsset> {
        self.uploads += 1;
        let bytes = fs::read(path)?;
        let asset = RemoteAsset {
            id: 100 + self.uploads as u64,
            name: name.into(),
            bytes: ByteIdentity::from_bytes(&bytes),
            uploader: 2,
        };
        require(
            self.remote.as_ref().is_some_and(|r| r.id == id && r.draft),
            "test-upload-not-owned",
        )?;
        self.remote
            .as_mut()
            .ok_or(Error::Json)?
            .assets
            .push(asset.clone());
        self.bytes.insert(asset.id, bytes);
        if self.ambiguous_upload {
            Err(Error::Json)
        } else {
            Ok(asset)
        }
    }
    /// Inject wrong served bytes or a during-download release mutation without pretending to sign anything.
    fn download(&mut self, id: u64, _: u64) -> Result<Vec<u8>> {
        if self.mutate_download {
            self.remote.as_mut().ok_or(Error::Json)?.body.push('x');
        }
        if self.bad_download {
            return Ok(b"tampered".to_vec());
        }
        self.bytes.get(&id).cloned().ok_or(Error::Json)
    }
    /// Simulate one publication and optionally lose its response after immutable server acceptance.
    fn publish(&mut self, id: u64) -> Result<()> {
        self.publishes += 1;
        let value = self
            .remote
            .as_mut()
            .filter(|r| r.id == id)
            .ok_or(Error::Json)?;
        value.draft = false;
        value.immutable = !self.bad_immutable;
        if self.ambiguous_publish {
            Err(Error::Json)
        } else {
            Ok(())
        }
    }
    /// Exercise the post-publication proof gate; genuine native cryptography is separately qualified.
    fn verify_attestation(&mut self, _: &FrozenRelease, _: &DraftVerification) -> Result<()> {
        require(!self.bad_attestation, "test-attestation-failure")
    }
}

/// Construct inert private test state; production FrozenRelease has only the complete authenticated-proof constructor.
fn fixture() -> (Controller<Fake>, ApprovedPublication, tempfile::TempDir) {
    let workspace = tempfile::tempdir().unwrap();
    let inputs: InputIdentity = serde_json::from_value(json!({"source":{"repository":"brianluby/armorer","commit":"a".repeat(40),"git_ref":"refs/tags/v1.2.3"},
        "config_sha256":"b".repeat(64),"lock_sha256":"c".repeat(64),"cargo_lock_sha256":"d".repeat(64),"runtime":{"sha256":"e".repeat(64),"size":1},"runtime_version":"0.1.0",
        "run":{"id":10,"attempt":1,"workflow":{"repository":"brianluby/armorer-workflows","path":".github/workflows/release.yml","commit":"f".repeat(40)}}})).unwrap();
    let context = ByteIdentity::from_bytes(b"independent context");
    let current = now().unwrap();
    let policy_value = PublicationPolicy {
        schema_version: 1,
        context: context.clone(),
        repository_id: 5,
        owner_id: 6,
        release_attestation_root: ByteIdentity::from_bytes(b"independent release root"),
        default_branch: "main".into(),
        controller_workflow: inputs.run.workflow.clone(),
        allowed_actor_ids: [1, 2, 3].into(),
        allowed_publisher_ids: [2].into(),
        allowed_approver_ids: [1, 3].into(),
        publish_environment: "release-publish".into(),
        publish_environment_id: 9,
        max_observation_age: 30,
        expires_at: current + 3600,
    };
    let policy = TrustedPublicationPolicy {
        identity: ByteIdentity::from_bytes(&serde_json::to_vec(&policy_value).unwrap()),
        policy: policy_value,
    };
    let mut assets = BTreeMap::new();
    for (name, bytes) in [
        ("one.tar.gz", b"inert archive".as_slice()),
        ("one.cdx.json", b"inert sbom".as_slice()),
    ] {
        io::write_readonly(&workspace.path().join(name), bytes).unwrap();
        assets.insert(name.into(), ByteIdentity::from_bytes(bytes));
    }
    let plan = PublicationPlan {
        schema_version: 1,
        context,
        policy: policy.identity.clone(),
        inputs,
        tag: "v1.2.3".into(),
        assets,
    };
    let identity = ByteIdentity::from_bytes(&serde_json::to_vec(&plan).unwrap());
    let release = FrozenRelease {
        workspace,
        plan,
        identity: identity.clone(),
    };
    let approval = ApprovedPublication {
        approval: PublicationApproval {
            schema_version: 1,
            kind: ApprovalKind::StageDraft,
            plan: identity,
            approver_id: 3,
            approved_at: current,
            expires_at: current + 1800,
            release_id: None,
            served_bytes: None,
        },
        identity: ByteIdentity::from_bytes(b"independent stage approval"),
    };
    let directory = tempfile::tempdir().unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let state = StateStore::open(directory.path(), policy.policy.repository_id).unwrap();
    (
        Controller {
            backend: Fake::default(),
            release,
            policy,
            state,
            context: None,
        },
        approval,
        directory,
    )
}
/// Bind a fresh test publication approval to the actual served-byte receipt and exact owned ID.
fn publish_approval(
    controller: &Controller<Fake>,
    receipt: &DraftVerification,
) -> ApprovedPublication {
    ApprovedPublication {
        approval: PublicationApproval {
            schema_version: 1,
            kind: ApprovalKind::PublishDraft,
            plan: controller.release.identity.clone(),
            approver_id: 3,
            approved_at: now().unwrap(),
            expires_at: now().unwrap() + 600,
            release_id: Some(receipt.release_id),
            served_bytes: Some(receipt.receipt.clone()),
        },
        identity: ByteIdentity::from_bytes(b"independent publication approval"),
    }
}

#[test]
/// An identical owned retry re-downloads bytes while creating and uploading nothing twice.
fn owned_retry_and_one_time_publication_preserve_exact_ids() {
    let (mut c, stage, _dir) = fixture();
    let receipt = c.stage(&stage).unwrap();
    assert_eq!(c.stage(&stage).unwrap(), receipt);
    assert_eq!(
        (c.backend.creates, c.backend.uploads, c.backend.publishes),
        (1, 2, 0)
    );
    let publish = publish_approval(&c, &receipt);
    assert_eq!(c.publish(&publish).unwrap(), receipt);
    assert!(c.publish(&publish).is_err());
    assert!(c.stage(&stage).is_err());
    assert_eq!(c.recover_published().unwrap(), receipt);
    assert_eq!(c.backend.publishes, 1);
}
#[test]
/// Every unavailable required gate and every omitted gate prevents a mutation permit.
fn required_capability_states_and_missing_gates_fail_closed() {
    let (mut c, _, _dir) = fixture();
    let original = c.backend.check(&c.release, &c.policy).unwrap();
    for name in github::REQUIRED_GATES {
        for state in [
            GateState::Disabled,
            GateState::Denied,
            GateState::Unknown,
            GateState::Unsupported,
            GateState::Error,
        ] {
            let mut changed = original.clone();
            changed.gates.insert((*name).into(), state);
            assert!(changed.require_ready(now().unwrap(), 30).is_err());
        }
        let mut changed = original.clone();
        changed.gates.remove(*name);
        assert!(changed.require_ready(now().unwrap(), 30).is_err());
    }
    assert_eq!(c.backend.creates, 0);
}
#[test]
/// Expired, wrong-action, substituted-plan and unauthorized human approvals issue zero writes.
fn approval_scope_identity_and_expiry_block_every_write() {
    for case in 0..5 {
        let (mut c, mut approval, _dir) = fixture();
        match case {
            0 => approval.approval.expires_at = now().unwrap(),
            1 => approval.approval.plan = ByteIdentity::from_bytes(b"other plan"),
            2 => approval.approval.kind = ApprovalKind::PublishDraft,
            3 => approval.approval.approver_id = 99,
            _ => approval.approval.approved_at = now().unwrap() + 100,
        }
        assert!(c.stage(&approval).is_err());
        assert_eq!(
            (c.backend.creates, c.backend.uploads, c.backend.publishes),
            (0, 0, 0)
        );
    }
}
#[test]
/// An existing foreign draft or published release is never adopted, replaced or deleted.
fn foreign_existing_draft_and_published_release_block_creation() {
    for draft in [true, false] {
        let (mut c, approval, _dir) = fixture();
        let mut foreign = c.backend.create(&c.release, "foreign").unwrap();
        foreign.draft = draft;
        c.backend.remote = Some(foreign);
        c.backend.creates = 0;
        assert!(c.stage(&approval).is_err());
        assert_eq!(
            (c.backend.creates, c.backend.uploads, c.backend.publishes),
            (0, 0, 0)
        );
    }
}
#[test]
/// A journal bound to another run, attempt, asset set or policy cannot authorize any retry.
fn cross_run_attempt_asset_and_policy_journals_are_conflicts() {
    for case in 0..4 {
        let (mut c, mut approval, _dir) = fixture();
        c.stage(&approval).unwrap();
        match case {
            0 => c.release.plan.inputs.run.id += 1,
            1 => c.release.plan.inputs.run.attempt += 1,
            2 => {
                c.release
                    .plan
                    .assets
                    .insert("extra".into(), ByteIdentity::from_bytes(b"extra"));
            }
            _ => c.release.plan.policy = ByteIdentity::from_bytes(b"other policy"),
        }
        c.release.identity =
            ByteIdentity::from_bytes(&serde_json::to_vec(&c.release.plan).unwrap());
        approval.approval.plan = c.release.identity.clone();
        assert!(c.stage(&approval).is_err());
        assert_eq!(
            (c.backend.creates, c.backend.uploads, c.backend.publishes),
            (1, 2, 0)
        );
    }
}
#[test]
/// Ambiguous create and upload outcomes are retained before the write and never automatically retried.
fn ambiguous_draft_and_upload_failures_require_recovery_review() {
    for create in [true, false] {
        let (mut c, approval, _dir) = fixture();
        c.backend.ambiguous_create = create;
        c.backend.ambiguous_upload = !create;
        assert!(c.stage(&approval).is_err());
        let counts = (c.backend.creates, c.backend.uploads);
        assert!(c.stage(&approval).is_err());
        assert_eq!((c.backend.creates, c.backend.uploads), counts);
        let journal = c.state.load(&c.release.identity).unwrap().unwrap();
        assert_eq!(
            journal.phase,
            if create {
                Phase::Creating
            } else {
                Phase::Uploading
            }
        );
    }
}
#[test]
/// A lost publish response permits only read-only immutable-byte and attestation recovery.
fn ambiguous_publication_never_repeats_the_provider_write() {
    let (mut c, stage, _dir) = fixture();
    let served = c.stage(&stage).unwrap();
    let publish = publish_approval(&c, &served);
    c.backend.ambiguous_publish = true;
    assert!(c.publish(&publish).is_err());
    assert!(c.publish(&publish).is_err());
    assert_eq!(c.recover_published().unwrap(), served);
    assert_eq!(c.backend.publishes, 1);
}
#[test]
/// Missing, extra, replaced-ID, wrong-uploader and wrong-digest assets cannot be repaired by a retry.
fn draft_mutations_never_trigger_replacement_uploads() {
    for case in 0..5 {
        let (mut c, stage, _dir) = fixture();
        c.stage(&stage).unwrap();
        let remote = c.backend.remote.as_mut().unwrap();
        match case {
            0 => {
                remote.assets.pop();
            }
            1 => {
                let mut a = remote.assets[0].clone();
                a.id = 999;
                a.name = "extra".into();
                remote.assets.push(a);
            }
            2 => remote.assets[0].id += 99,
            3 => remote.assets[0].uploader = 99,
            _ => remote.assets[0].bytes = ByteIdentity::from_bytes(b"tamper"),
        }
        assert!(c.stage(&stage).is_err());
        assert_eq!(
            (c.backend.creates, c.backend.uploads, c.backend.publishes),
            (1, 2, 0)
        );
    }
}
#[test]
/// Wrong served bytes and bracketed draft mutations fail before a publication approval is possible.
fn actual_download_tamper_and_mutation_block_draft_verification() {
    for mutate in [true, false] {
        let (mut c, stage, _dir) = fixture();
        c.backend.mutate_download = mutate;
        c.backend.bad_download = !mutate;
        assert!(c.stage(&stage).is_err());
        assert_eq!(c.backend.publishes, 0);
        assert_ne!(
            c.state.load(&c.release.identity).unwrap().unwrap().phase,
            Phase::DraftVerified
        );
    }
}
#[test]
/// A changed prerequisite immediately before the publish boundary performs no public write.
fn final_mutable_preflight_failure_does_not_publish() {
    let (mut c, stage, _dir) = fixture();
    let served = c.stage(&stage).unwrap();
    let approval = publish_approval(&c, &served);
    c.backend.fail_check_at = Some(c.backend.checks + 2);
    assert!(c.publish(&approval).is_err());
    assert_eq!(c.backend.publishes, 0);
}
#[test]
/// Self-review and approvals for another release ID or served receipt cannot publish.
fn publication_approval_requires_independent_reviewer_and_exact_draft() {
    for case in 0..3 {
        let (mut c, stage, _dir) = fixture();
        let served = c.stage(&stage).unwrap();
        let mut approval = publish_approval(&c, &served);
        match case {
            0 => approval.approval.approver_id = 1,
            1 => approval.approval.release_id = Some(43),
            _ => approval.approval.served_bytes = Some(ByteIdentity::from_bytes(b"other receipt")),
        }
        assert!(c.publish(&approval).is_err());
        assert_eq!(c.backend.publishes, 0);
    }
}
#[test]
/// A rerun initiator, missing initiator or zero ID cannot authorize a publication write.
fn rerun_initiator_cannot_approve_publication() {
    for (actor, triggering_actor) in [
        (Some(1), Some(3)),
        (Some(1), None),
        (Some(1), Some(0)),
        (None, Some(2)),
        (Some(0), Some(2)),
    ] {
        let (mut c, stage, _dir) = fixture();
        let served = c.stage(&stage).unwrap();
        let approval = publish_approval(&c, &served);
        c.backend.actor = Some(actor);
        c.backend.triggering_actor = Some(triggering_actor);
        assert!(matches!(
            c.publish(&approval),
            Err(Error::Invalid(reason)) if reason == "publication-self-approval"
        ));
        assert_eq!(
            (c.backend.creates, c.backend.uploads, c.backend.publishes),
            (1, 2, 0)
        );
        assert_eq!(
            c.state.load(&c.release.identity).unwrap().unwrap().phase,
            Phase::DraftVerified
        );
    }
}
#[test]
/// A rerun actor change at the last mutable preflight still prevents the public write.
fn rerun_reviewer_change_at_final_preflight_does_not_publish() {
    let (mut c, stage, _dir) = fixture();
    let served = c.stage(&stage).unwrap();
    let approval = publish_approval(&c, &served);
    c.backend.triggering_actor_change_at = Some(c.backend.checks + 2);
    assert!(matches!(
        c.publish(&approval),
        Err(Error::Invalid(reason)) if reason == "publication-self-approval"
    ));
    assert_eq!(c.backend.publishes, 0);
    assert_eq!(
        c.state.load(&c.release.identity).unwrap().unwrap().phase,
        Phase::DraftVerified
    );
}
#[test]
/// A third independent reviewer can approve when the original and rerun actors differ.
fn distinct_original_and_rerun_actors_allow_independent_reviewer() {
    let (mut c, stage, _dir) = fixture();
    let served = c.stage(&stage).unwrap();
    let approval = publish_approval(&c, &served);
    c.backend.triggering_actor = Some(Some(2));
    c.publish(&approval).unwrap();
    assert_eq!(c.backend.publishes, 1);
    assert_eq!(
        c.state.load(&c.release.identity).unwrap().unwrap().phase,
        Phase::Complete
    );
}
#[test]
/// Post-publication immutability and cryptographic-proof failures preserve an incomplete incident state.
fn post_publication_failures_never_claim_completion_or_repair_bytes() {
    for immutable in [true, false] {
        let (mut c, stage, _dir) = fixture();
        let served = c.stage(&stage).unwrap();
        let approval = publish_approval(&c, &served);
        c.backend.bad_immutable = immutable;
        c.backend.bad_attestation = !immutable;
        assert!(c.publish(&approval).is_err());
        assert_ne!(
            c.state.load(&c.release.identity).unwrap().unwrap().phase,
            Phase::Complete
        );
        assert!(c.stage(&stage).is_err());
        assert_eq!((c.backend.uploads, c.backend.publishes), (2, 1));
    }
}
#[test]
/// One kernel lock covers different tags and releases automatically when its owning process drops it.
fn repository_wide_lock_serializes_concurrent_tags_and_releases_on_drop() {
    let directory = tempfile::tempdir().unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let first = StateStore::open(directory.path(), 5).unwrap();
    assert!(StateStore::open(directory.path(), 5).is_err());
    drop(first);
    assert!(StateStore::open(directory.path(), 5).is_ok());
}
#[test]
/// A duplicate descriptor cannot keep a finished controller locked or release its successor's lock.
fn publication_lock_releases_before_duplicate_descriptors_close() {
    let directory = tempfile::tempdir().unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let first = StateStore::open(directory.path(), 5).unwrap();
    let inherited = first._lock.try_clone().unwrap();
    assert!(StateStore::open(directory.path(), 5).is_err());
    drop(first);
    let next = StateStore::open(directory.path(), 5).unwrap();
    drop(inherited);
    assert!(StateStore::open(directory.path(), 5).is_err());
    drop(next);
    StateStore::open(directory.path(), 5).unwrap();
}
#[test]
/// A symlink or group-writable state directory cannot hold publication ownership.
fn unsafe_state_directory_and_symlink_lock_fail_closed() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let link = root.path().join("link");
    symlink(target.path(), &link).unwrap();
    assert!(StateStore::open(&link, 5).is_err());
    fs::set_permissions(target.path(), fs::Permissions::from_mode(0o770)).unwrap();
    assert!(StateStore::open(target.path(), 5).is_err());
    fs::set_permissions(target.path(), fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(root.path().join("unrelated"), b"preserve").unwrap();
    symlink(root.path().join("unrelated"), target.path().join("5.lock")).unwrap();
    assert!(StateStore::open(target.path(), 5).is_err());
    assert_eq!(
        fs::read(root.path().join("unrelated")).unwrap(),
        b"preserve"
    );
}
