//! Fixed-route, hash-qualified native GitHub adapter. No raw commands, URLs or replace/delete/tag APIs.
use super::*;
use crate::verification::sigstore::{self, Limits};
use base64::Engine;
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    process::{Command, Stdio},
    time::Duration,
};

pub(super) const REQUIRED_GATES: &[&str] = &[
    "immutable-releases",
    "publish-environment-settings",
    "current-attempt-environment-enforcement",
    "runner-wide-serialization",
    "effective-stable-tag-protection",
    "stable-source-tag",
    "protected-default-branch",
    "source-ancestry",
    "exact-current-attempt",
    "actor-permission",
    "controller-workflow-pin",
];

/// Separate availability and enforcement states; a positive setting is not a credential or approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GateState {
    Enforced,
    Disabled,
    Denied,
    Unknown,
    Unsupported,
    Error,
}

/// Native observations are informational, with no deserialization or callable mutation permit.
#[derive(Debug, Clone, Serialize)]
pub struct CapabilityReport {
    pub observed_at: u64,
    pub repository_id: u64,
    pub actor_id: Option<u64>,
    pub gates: BTreeMap<String, GateState>,
    pub evidence: BTreeMap<String, ByteIdentity>,
    pub limitations: Vec<String>,
}
impl CapabilityReport {
    /// Require every independently selected gate, exact freshness and no silent reporting downgrade.
    pub(super) fn require_ready(&self, current: u64, max_age: u64) -> Result<()> {
        require(
            self.observed_at > 0
                && self.observed_at <= current
                && current - self.observed_at <= max_age
                && REQUIRED_GATES
                    .iter()
                    .all(|name| self.gates.get(*name) == Some(&GateState::Enforced))
                && self.gates.values().all(|v| *v == GateState::Enforced),
            "publication-prerequisites-unproven",
        )
    }
}

/// Qualified native client with fixed GitHub authority and opaque existing authentication.
pub struct NativeGithub {
    workspace: tempfile::TempDir,
    executable: PathBuf,
    root: PathBuf,
    root_identity: ByteIdentity,
    repository: String,
    token: Option<OsString>,
    owner_home: Option<PathBuf>,
    repository_id: u64,
    owner_id: u64,
}
impl NativeGithub {
    /// Open the protected job's existing token only after approved policy and native bytes are authenticated.
    /// Tokens are never returned, serialized, printed, accepted as method arguments or used for subprocess discovery.
    pub fn open(
        executable: &Path,
        trusted_release_root: &Path,
        policy: &TrustedPublicationPolicy,
    ) -> Result<Self> {
        let mut client = Self::qualified(executable, trusted_release_root, policy)?;
        client.token = Some(
            std::env::var_os("GH_TOKEN")
                .or_else(|| std::env::var_os("GITHUB_TOKEN"))
                .filter(|v| !v.is_empty() && v.len() <= 65536)
                .ok_or_else(|| Error::Invalid("publication-authentication-unavailable".into()))?,
        );
        Ok(client)
    }
    /// Use existing owner CLI configuration for GET-only qualification; this client rejects every provider write.
    pub fn open_owner_read_only(
        executable: &Path,
        trusted_release_root: &Path,
        policy: &TrustedPublicationPolicy,
        owner_home: &Path,
    ) -> Result<Self> {
        let mut client = Self::qualified(executable, trusted_release_root, policy)?;
        require(
            fs::symlink_metadata(owner_home)?.is_dir() && owner_home.is_absolute(),
            "publication-owner-home-type",
        )?;
        client.owner_home = Some(owner_home.to_owned());
        Ok(client)
    }
    /// Snapshot and authenticate the fixed official executable and independently approved release-specific root.
    fn qualified(
        executable: &Path,
        root: &Path,
        policy: &TrustedPublicationPolicy,
    ) -> Result<Self> {
        policy.validate(now()?)?;
        let workspace = private_workspace("armorer-native-publication-")?;
        let native = workspace.path().join("gh");
        require(
            io::snapshot(executable, &native, io::MAX_VERIFIER)?
                == sigstore::qualified_native_verifier()?,
            "publication-native-byte-substitution",
        )?;
        sigstore::native_executable(&native)?;
        io::readonly(&native, true)?;
        let root_copy = workspace.path().join("trusted-root.jsonl");
        require(
            io::snapshot(root, &root_copy, io::MAX_ROOT)? == policy.policy.release_attestation_root,
            "publication-root-substitution",
        )?;
        io::readonly(&root_copy, false)?;
        fs::create_dir(workspace.path().join("config"))?;
        Ok(Self {
            workspace,
            executable: native,
            root: root_copy,
            root_identity: policy.policy.release_attestation_root.clone(),
            repository: policy.policy.controller_workflow.repository.clone(),
            token: None,
            owner_home: None,
            repository_id: policy.policy.repository_id,
            owner_id: policy.policy.owner_id,
        })
    }
    /// Bind the source repository separately from the reusable controller repository, before any request.
    pub fn bind_repository(
        mut self,
        context: &TrustedReleaseContext,
        policy: &TrustedPublicationPolicy,
    ) -> Result<Self> {
        require(
            context.identity() == &policy.policy.context,
            "publication-native-context-binding",
        )?;
        self.repository = context.inputs().source.repository.clone();
        Ok(self)
    }
    /// Borrow the independently selected fixed repository route.
    pub(super) fn repository(&self) -> &str {
        &self.repository
    }
    /// Execute a fixed native operation with a cleared environment, closed stdin and bounded redacted streams.
    fn command(&self, args: &[String], stdout: usize) -> Result<(bool, Vec<u8>)> {
        require(
            io::identity(&self.executable, io::MAX_VERIFIER)?
                == sigstore::qualified_native_verifier()?
                && io::identity(&self.root, io::MAX_ROOT)? == self.root_identity,
            "publication-native-snapshot-changed",
        )?;
        let mut command = Command::new(&self.executable);
        command
            .args(args)
            .env_clear()
            .current_dir(self.workspace.path())
            .stdin(Stdio::null())
            .env(
                "HOME",
                self.owner_home.as_deref().unwrap_or(self.workspace.path()),
            )
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
            .env("GH_HOST", "github.com")
            .env("GH_PROMPT_DISABLED", "1")
            .env("GH_NO_UPDATE_NOTIFIER", "1")
            .env("GH_NO_EXTENSION_UPDATE_NOTIFIER", "1")
            .env("NO_COLOR", "1")
            .env("LANG", "C.UTF-8");
        if let Some(token) = &self.token {
            command
                .env("GH_TOKEN", token)
                .env("GH_CONFIG_DIR", self.workspace.path().join("config"));
        }
        sigstore::run_process_status(
            command,
            Limits {
                stdout,
                stderr: 65536,
                timeout: Duration::from_secs(120),
            },
        )
    }
    /// Request only a compiled repository route, never a caller URL or a provider-returned upload URL.
    fn response(&self, suffix: &str) -> Result<Response> {
        let endpoint = if suffix.is_empty() {
            format!("repos/{}", self.repository)
        } else {
            format!("repos/{}/{}", self.repository, suffix)
        };
        let args = vec![
            "api".into(),
            endpoint,
            "--method".into(),
            "GET".into(),
            "--include".into(),
            "--hostname".into(),
            "github.com".into(),
            "-H".into(),
            "Accept: application/vnd.github+json".into(),
            "-H".into(),
            "X-GitHub-Api-Version: 2026-03-10".into(),
            "-H".into(),
            "Cache-Control: no-cache".into(),
        ];
        let (success, bytes) = self.command(&args, 4 * 1024 * 1024)?;
        Response::parse(success, &bytes)
    }
    /// Read a successful JSON response under duplicate-key and structural limits.
    fn get(&self, suffix: &str) -> Result<Value> {
        let response = self.response(suffix)?;
        require(
            response.status == 200,
            "publication-provider-read-unavailable",
        )?;
        io::parse(&response.bytes)
    }
    /// Paginate a fixed list endpoint completely within an explicit twenty-page cap.
    fn list(&self, suffix: &str) -> Result<Vec<Value>> {
        let mut all = Vec::new();
        for page in 1..=20 {
            let value = self.get(&format!(
                "{suffix}{}per_page=100&page={page}",
                if suffix.contains('?') { "&" } else { "?" }
            ))?;
            let values = value.as_array().ok_or(Error::Json)?;
            require(values.len() <= 100, "publication-page-size")?;
            all.extend(values.iter().cloned());
            if values.len() < 100 {
                return Ok(all);
            }
        }
        Err(Error::Invalid("publication-pagination-limit".into()))
    }
    /// Observe immutable and protected-environment settings twice while preserving inaccessible states.
    pub fn observe_capabilities(
        &self,
        policy: &TrustedPublicationPolicy,
    ) -> Result<CapabilityReport> {
        policy.validate(now()?)?;
        let before = self.capability_snapshot(policy)?;
        let after = self.capability_snapshot(policy)?;
        require(
            before.gates == after.gates && before.evidence == after.evidence,
            "publication-capabilities-changed",
        )?;
        Ok(after)
    }
    /// Snapshot authenticated fixed authority, settings and stable identity; metadata never proves job enforcement.
    fn capability_snapshot(&self, policy: &TrustedPublicationPolicy) -> Result<CapabilityReport> {
        let repo = self.get("")?;
        require(
            repo["id"].as_u64() == Some(policy.policy.repository_id)
                && repo["owner"]["id"].as_u64() == Some(policy.policy.owner_id)
                && repo["full_name"].as_str() == Some(self.repository.as_str())
                && repo["default_branch"].as_str() == Some(policy.policy.default_branch.as_str())
                && repo["fork"].as_bool() == Some(false),
            "publication-repository-substitution",
        )?;
        let mut report = CapabilityReport { observed_at: now()?, repository_id: policy.policy.repository_id, actor_id: None,
            gates: BTreeMap::new(), evidence: BTreeMap::new(), limitations: vec![
                "Repository settings do not authenticate current-attempt environment approval or exclusive controller execution.".into(),
                "Local locks do not serialize independent Actions runners; a separately authenticated runner-wide gate is required.".into()] };
        report.evidence.insert(
            "repository".into(),
            ByteIdentity::from_bytes(&serde_json::to_vec(&repo).map_err(|_| Error::Json)?),
        );
        let immutable = self.response("immutable-releases")?;
        let immutable_state = if immutable.status == 200 {
            match io::parse::<Value>(&immutable.bytes)?["enabled"].as_bool() {
                Some(true) => GateState::Enforced,
                Some(false) => GateState::Disabled,
                None => GateState::Error,
            }
        } else {
            unavailable(immutable.status)
        };
        report
            .gates
            .insert("immutable-releases".into(), immutable_state);
        report.evidence.insert(
            "immutable-releases".into(),
            ByteIdentity::from_bytes(&immutable.bytes),
        );
        let environment = self.response(&format!(
            "environments/{}",
            policy.policy.publish_environment
        ))?;
        let environment_state = if environment.status == 200 {
            let value: Value = io::parse(&environment.bytes)?;
            require(
                value["id"].as_u64() == Some(policy.policy.publish_environment_id)
                    && value["name"].as_str() == Some(policy.policy.publish_environment.as_str()),
                "publication-environment-substitution",
            )?;
            let reviewers = value["protection_rules"]
                .as_array()
                .ok_or(Error::Json)?
                .iter()
                .any(|rule| {
                    rule["type"] == "required_reviewers"
                        && rule["prevent_self_review"] == true
                        && rule["reviewers"].as_array().is_some_and(|entries| {
                            !entries.is_empty()
                                && entries.iter().all(|entry| {
                                    entry["type"] == "User"
                                        && entry["reviewer"]["id"].as_u64().is_some_and(|id| {
                                            policy.policy.allowed_approver_ids.contains(&id)
                                        })
                                })
                        })
                });
            if reviewers && value["can_admins_bypass"] == false {
                GateState::Enforced
            } else {
                GateState::Unknown
            }
        } else {
            unavailable(environment.status)
        };
        report
            .gates
            .insert("publish-environment-settings".into(), environment_state);
        report.evidence.insert(
            "publish-environment".into(),
            ByteIdentity::from_bytes(&environment.bytes),
        );
        report.gates.insert(
            "current-attempt-environment-enforcement".into(),
            GateState::Unsupported,
        );
        report
            .gates
            .insert("runner-wide-serialization".into(), GateState::Unsupported);
        Ok(report)
    }
    /// Resolve an existing stable tag to its independently expected source commit; no tag creation or movement exists.
    fn tag_commit(&self, tag: &str) -> Result<(String, String)> {
        let value = self.get(&format!("git/ref/tags/{tag}"))?;
        require(
            value["ref"] == format!("refs/tags/{tag}"),
            "publication-tag-ref-mismatch",
        )?;
        let raw = text(&value["object"], "sha")?.to_owned();
        let mut object = value["object"].clone();
        for _ in 0..4 {
            let sha = text(&object, "sha")?;
            require(
                crate::config::hex_digest(sha, 40),
                "publication-tag-object-digest",
            )?;
            match text(&object, "type")? {
                "commit" => return Ok((raw, sha.to_owned())),
                "tag" => {
                    object = self.get(&format!("git/tags/{sha}"))?["object"].clone();
                }
                _ => return Err(Error::Invalid("publication-tag-object-type".into())),
            }
        }
        Err(Error::Invalid("publication-tag-peel-limit".into()))
    }
    /// Bind actor permissions, exact current run/attempt, protected default branch and source ancestry.
    fn source_gates(
        &self,
        release: &FrozenRelease,
        policy: &TrustedPublicationPolicy,
        report: &mut CapabilityReport,
    ) -> Result<()> {
        let source = &release.plan.inputs.source;
        let (_, commit) = self.tag_commit(&release.plan.tag)?;
        require(commit == source.commit, "publication-tag-source-mismatch")?;
        let branch = self.get(&format!("branches/{}", policy.policy.default_branch))?;
        require(
            branch["protected"] == true && branch["name"] == policy.policy.default_branch,
            "publication-default-branch-unprotected",
        )?;
        let head = text(&branch["commit"], "sha")?;
        require(
            crate::config::hex_digest(head, 40),
            "publication-default-branch-digest",
        )?;
        let comparison = self.get(&format!("compare/{}...{head}", source.commit))?;
        require(
            matches!(comparison["status"].as_str(), Some("ahead" | "identical"))
                && comparison["merge_base_commit"]["sha"] == source.commit,
            "publication-source-not-default-branch-ancestor",
        )?;
        let run = &release.plan.inputs.run;
        let value = self.get(&format!("actions/runs/{}/attempts/{}", run.id, run.attempt))?;
        require(
            value["id"].as_u64() == Some(run.id)
                && value["run_attempt"].as_u64() == Some(run.attempt)
                && value["head_sha"] == source.commit
                && value["head_branch"] == release.plan.tag
                && value["event"] == "push"
                && value["repository"]["id"].as_u64() == Some(policy.policy.repository_id)
                && value["head_repository"]["id"].as_u64() == Some(policy.policy.repository_id)
                && value["head_repository"]["fork"] == false
                && value["status"] == "in_progress",
            "publication-run-substitution-or-unsafe-trigger",
        )?;
        let latest = self.get(&format!("actions/runs/{}", run.id))?;
        require(
            latest["run_attempt"].as_u64() == Some(run.attempt)
                && latest["head_sha"] == source.commit
                && latest["status"] == "in_progress",
            "publication-attempt-no-longer-current",
        )?;
        for key in ["actor", "triggering_actor"] {
            require(
                value[key]["id"]
                    .as_u64()
                    .is_some_and(|id| policy.policy.allowed_actor_ids.contains(&id)),
                "publication-actor-not-approved",
            )?;
            let login = text(&value[key], "login")?;
            require(asset_name(login), "publication-actor-route")?;
            let permission = self.get(&format!("collaborators/{login}/permission"))?;
            require(
                permission["user"]["id"] == value[key]["id"]
                    && matches!(
                        permission["permission"].as_str(),
                        Some("admin" | "write" | "maintain")
                    ),
                "publication-actor-permission-denied",
            )?;
        }
        let workflow = &policy.policy.controller_workflow;
        report.actor_id = value["actor"]["id"].as_u64();
        require(
            value["referenced_workflows"]
                .as_array()
                .is_some_and(|workflows| {
                    workflows.iter().any(|entry| {
                        entry["sha"] == workflow.commit
                            && entry["path"].as_str().is_some_and(|path| {
                                path == format!(
                                    "{}/{}@{}",
                                    workflow.repository, workflow.path, workflow.commit
                                )
                            })
                    })
                }),
            "publication-controller-pin-not-observed",
        )?;
        let mut protected = false;
        for summary in self.list("rulesets?includes_parents=true")? {
            let id = summary["id"]
                .as_u64()
                .filter(|id| *id > 0)
                .ok_or(Error::Json)?;
            let rule = self.get(&format!("rulesets/{id}?includes_parents=true"))?;
            let names = &rule["conditions"]["ref_name"];
            let matches = names["include"]
                .as_array()
                .is_some_and(|values| values.iter().any(|v| v == "~ALL" || v == &source.git_ref));
            let empty_excludes = names["exclude"].as_array().is_some_and(Vec::is_empty);
            let no_bypass = rule["bypass_actors"].as_array().is_some_and(Vec::is_empty);
            let rules = rule["rules"].as_array().ok_or(Error::Json)?;
            protected |= rule["target"] == "tag"
                && rule["enforcement"] == "active"
                && matches
                && empty_excludes
                && no_bypass
                && ["deletion", "update"]
                    .iter()
                    .all(|name| rules.iter().any(|entry| entry["type"] == *name));
        }
        report.gates.insert(
            "effective-stable-tag-protection".into(),
            if protected {
                GateState::Enforced
            } else {
                GateState::Unknown
            },
        );
        for name in [
            "stable-source-tag",
            "protected-default-branch",
            "source-ancestry",
            "exact-current-attempt",
            "actor-permission",
            "controller-workflow-pin",
        ] {
            report.gates.insert(name.into(), GateState::Enforced);
        }
        report.evidence.insert(
            "current-run".into(),
            ByteIdentity::from_bytes(&serde_json::to_vec(&value).map_err(|_| Error::Json)?),
        );
        Ok(())
    }
    /// Issue only compiled create/upload/publish operations, after the controller's private approval/gate boundary.
    fn write(&self, method: &str, endpoint: String, body: &[u8], binary: bool) -> Result<Vec<u8>> {
        require(
            self.token.is_some() && self.owner_home.is_none(),
            "read-only-publication-client",
        )?;
        let mut input = tempfile::NamedTempFile::new_in(self.workspace.path())?;
        input.write_all(body)?;
        input.as_file().sync_all()?;
        let args = vec![
            "api".into(),
            endpoint,
            "--method".into(),
            method.into(),
            "--input".into(),
            input.path().to_string_lossy().into_owned(),
            "-H".into(),
            if binary {
                "Content-Type: application/octet-stream".into()
            } else {
                "Content-Type: application/json".into()
            },
            "-H".into(),
            "X-GitHub-Api-Version: 2026-03-10".into(),
        ];
        let (success, bytes) = self.command(&args, 4 * 1024 * 1024)?;
        require(success, "publication-write-outcome-ambiguous")?;
        Ok(bytes)
    }
    /// Parse a release and independently list its complete exact asset set by authenticated release ID.
    fn parse_release(&self, value: &Value) -> Result<RemoteRelease> {
        let id = value["id"].as_u64().filter(|v| *v > 0).ok_or(Error::Json)?;
        let assets = self
            .list(&format!("releases/{id}/assets"))?
            .iter()
            .map(parse_asset)
            .collect::<Result<Vec<_>>>()?;
        Ok(RemoteRelease {
            id,
            tag: text(value, "tag_name")?.into(),
            target: text(value, "target_commitish")?.into(),
            author: value["author"]["id"].as_u64().ok_or(Error::Json)?,
            body: text(value, "body")?.into(),
            draft: value["draft"].as_bool().ok_or(Error::Json)?,
            immutable: value["immutable"].as_bool().ok_or(Error::Json)?,
            prerelease: value["prerelease"].as_bool().ok_or(Error::Json)?,
            assets,
        })
    }
}

impl Backend for NativeGithub {
    /// Authenticate mutable prerequisites twice; unsupported current-attempt enforcement always blocks writes.
    fn check(
        &mut self,
        release: &FrozenRelease,
        policy: &TrustedPublicationPolicy,
    ) -> Result<CapabilityReport> {
        let mut report = self.observe_capabilities(policy)?;
        self.source_gates(release, policy, &mut report)?;
        let mut after = self.observe_capabilities(policy)?;
        self.source_gates(release, policy, &mut after)?;
        require(
            report.gates == after.gates && report.evidence == after.evidence,
            "publication-prerequisites-changed",
        )?;
        Ok(after)
    }
    /// Refuse every existing release for a tag unless a matching protected journal already owns its exact ID.
    fn releases_for_tag(&mut self, tag: &str) -> Result<Vec<RemoteRelease>> {
        self.list("releases")?
            .iter()
            .filter(|value| value["tag_name"] == tag)
            .map(|value| self.parse_release(value))
            .collect()
    }
    /// Read an exact journal-owned release ID without falling back to a moving tag or latest release.
    fn release(&mut self, id: u64) -> Result<RemoteRelease> {
        self.parse_release(&self.get(&format!("releases/{id}"))?)
    }
    /// Create a draft for an already existing verified tag with a fixed ownership marker and source commit.
    fn create(&mut self, release: &FrozenRelease, body: &str) -> Result<RemoteRelease> {
        let bytes = self.write("POST", format!("repos/{}/releases", self.repository), &serde_json::to_vec(&json!({"tag_name":release.plan.tag,
            "target_commitish":release.plan.inputs.source.commit, "name":release.plan.tag, "body":body, "draft":true,
            "prerelease":false, "generate_release_notes":false, "make_latest":"false"})).map_err(|_| Error::Json)?, false)?;
        self.parse_release(&io::parse::<Value>(&bytes)?)
    }
    /// Upload a frozen file through the fixed upload authority; never use clobber, provider URLs or deletion.
    fn upload(&mut self, id: u64, name: &str, path: &Path) -> Result<RemoteAsset> {
        require(asset_name(name) && id > 0, "publication-upload-route")?;
        let bytes = io::read_bounded(path, 1024 * 1024 * 1024)?;
        let result = self.write(
            "POST",
            format!(
                "https://uploads.github.com/repos/{}/releases/{id}/assets?name={name}",
                self.repository
            ),
            &bytes,
            true,
        )?;
        let asset = parse_asset(&io::parse::<Value>(&result)?)?;
        require(
            asset.name == name && asset.bytes == ByteIdentity::from_bytes(&bytes),
            "publication-upload-result-substitution",
        )?;
        Ok(asset)
    }
    /// Download bounded inert bytes by exact asset ID with native authenticated content negotiation.
    fn download(&mut self, id: u64, maximum: u64) -> Result<Vec<u8>> {
        let args = vec![
            "api".into(),
            format!("repos/{}/releases/assets/{id}", self.repository),
            "--method".into(),
            "GET".into(),
            "-H".into(),
            "Accept: application/octet-stream".into(),
            "-H".into(),
            "X-GitHub-Api-Version: 2026-03-10".into(),
        ];
        let (success, bytes) =
            self.command(&args, usize::try_from(maximum).map_err(|_| Error::Json)?)?;
        require(success, "publication-served-byte-download-failed")?;
        Ok(bytes)
    }
    /// Publish only an existing journal-owned ID; never create a tag or replace release assets.
    fn publish(&mut self, id: u64) -> Result<()> {
        self.write(
            "PATCH",
            format!("repos/{}/releases/{id}", self.repository),
            b"{\"draft\":false,\"make_latest\":\"false\"}",
            false,
        )?;
        Ok(())
    }
    /// Run pinned genuine cryptographic verification against the separate approved root and exact signed statement.
    fn verify_attestation(
        &mut self,
        release: &FrozenRelease,
        served: &DraftVerification,
    ) -> Result<()> {
        let (raw_ref, commit) = self.tag_commit(&release.plan.tag)?;
        require(
            commit == release.plan.inputs.source.commit,
            "published-tag-source-changed",
        )?;
        let args = vec![
            "release".into(),
            "verify".into(),
            release.plan.tag.clone(),
            "--repo".into(),
            self.repository.clone(),
            "--format".into(),
            "json".into(),
            "--custom-trusted-root".into(),
            self.root.to_string_lossy().into_owned(),
        ];
        let (success, bytes) = self.command(&args, 4 * 1024 * 1024)?;
        require(
            success,
            "published-release-attestation-authentication-failed",
        )?;
        let value: Value = io::parse(&bytes)?;
        validate_attestation_result(
            &value,
            release,
            served,
            &raw_ref,
            self.repository_id,
            self.owner_id,
        )?;
        for name in release.plan.assets.keys() {
            let path = release.asset_path(name)?;
            let args = vec![
                "release".into(),
                "verify-asset".into(),
                release.plan.tag.clone(),
                path.to_string_lossy().into_owned(),
                "--repo".into(),
                self.repository.clone(),
                "--format".into(),
                "json".into(),
                "--custom-trusted-root".into(),
                self.root.to_string_lossy().into_owned(),
            ];
            let (success, bytes) = self.command(&args, 4 * 1024 * 1024)?;
            require(success, "published-release-asset-attestation-failed")?;
            validate_attestation_result(
                &io::parse(&bytes)?,
                release,
                served,
                &raw_ref,
                self.repository_id,
                self.owner_id,
            )?;
        }
        Ok(())
    }
}

struct Response {
    status: u16,
    bytes: Vec<u8>,
}
impl Response {
    /// Parse native CLI included headers without trusting reflected provider error text or redirects.
    fn parse(success: bool, bytes: &[u8]) -> Result<Self> {
        let boundary = bytes
            .windows(4)
            .position(|v| v == b"\r\n\r\n")
            .map(|i| (i, 4))
            .or_else(|| bytes.windows(2).position(|v| v == b"\n\n").map(|i| (i, 2)))
            .ok_or(Error::Json)?;
        require(boundary.0 <= 65536, "publication-http-header-limit")?;
        let headers = std::str::from_utf8(&bytes[..boundary.0]).map_err(|_| Error::Json)?;
        let first = headers.lines().next().ok_or(Error::Json)?;
        require(
            first.starts_with("HTTP/"),
            "publication-http-status-missing",
        )?;
        let status: u16 = first
            .split_whitespace()
            .nth(1)
            .ok_or(Error::Json)?
            .parse()
            .map_err(|_| Error::Json)?;
        require(
            (100..=599).contains(&status) && success == (200..=299).contains(&status),
            "publication-http-exit-status-mismatch",
        )?;
        Ok(Self {
            status,
            bytes: bytes[boundary.0 + boundary.1..].to_vec(),
        })
    }
}
/// Preserve ambiguous 403 authorization/throttle semantics instead of reflecting provider messages.
fn unavailable(status: u16) -> GateState {
    match status {
        401 => GateState::Denied,
        403 | 404 => GateState::Unknown,
        410 => GateState::Unsupported,
        _ => GateState::Error,
    }
}
/// Require a bounded string from an authenticated native response without returning untrusted diagnostics.
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .filter(|v| v.len() <= 8192)
        .ok_or(Error::Json)
}
/// Bind uploaded native metadata to a nonempty bounded exact identity; null digests never receive partial credit.
fn parse_asset(value: &Value) -> Result<RemoteAsset> {
    let name = text(value, "name")?;
    require(
        asset_name(name) && value["state"] == "uploaded",
        "publication-asset-metadata",
    )?;
    let bytes = ByteIdentity {
        sha256: text(value, "digest")?
            .strip_prefix("sha256:")
            .ok_or(Error::Json)?
            .into(),
        size: value["size"].as_u64().ok_or(Error::Json)?,
    };
    bytes.validate()?;
    Ok(RemoteAsset {
        id: value["id"]
            .as_u64()
            .filter(|id| *id > 0)
            .ok_or(Error::Json)?,
        name: name.into(),
        bytes,
        uploader: value["uploader"]["id"]
            .as_u64()
            .filter(|id| *id > 0)
            .ok_or(Error::Json)?,
    })
}
/// Require actual verified v0.2 identity, the exact signed payload and a complete one-to-one subject set.
fn validate_attestation_result(
    value: &Value,
    release: &FrozenRelease,
    served: &DraftVerification,
    tag_object: &str,
    repository_id: u64,
    owner_id: u64,
) -> Result<()> {
    let result = &value["verificationResult"];
    require(
        result["mediaType"] == "application/vnd.dev.sigstore.verificationresult+json;version=0.1"
            && result["signature"]["certificate"]["subjectAlternativeName"]
                == "https://dotcom.releases.github.com"
            && result["verifiedTimestamps"]
                .as_array()
                .is_some_and(|items| !items.is_empty()),
        "release-attestation-verification-result-missing",
    )?;
    let payload = value["attestation"]["bundle"]["dsseEnvelope"]["payload"]
        .as_str()
        .ok_or(Error::Json)?;
    require(
        payload.len() <= 4 * 1024 * 1024,
        "release-attestation-payload-limit",
    )?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|_| Error::Json)?;
    let statement: Value = io::parse(&decoded)?;
    require(
        statement == result["statement"]
            && statement["_type"] == "https://in-toto.io/Statement/v1"
            && statement["predicateType"] == "https://in-toto.io/attestation/release/v0.2",
        "release-attestation-signed-statement-mismatch",
    )?;
    let purl = format!(
        "pkg:github/{}@{}",
        release.plan.inputs.source.repository, release.plan.tag
    );
    let predicate = &statement["predicate"];
    let release_id_text = served.release_id.to_string();
    let repository_id_text = repository_id.to_string();
    let owner_id_text = owner_id.to_string();
    require(
        predicate["tag"] == release.plan.tag
            && predicate["repository"] == release.plan.inputs.source.repository
            && predicate["purl"] == purl
            && predicate["databaseId"].as_str() == Some(release_id_text.as_str())
            && predicate["repositoryId"].as_str() == Some(repository_id_text.as_str())
            && predicate["packageId"].as_str() == Some(repository_id_text.as_str())
            && predicate["ownerId"].as_str() == Some(owner_id_text.as_str()),
        "release-attestation-identity-mismatch",
    )?;
    let subjects = statement["subject"].as_array().ok_or(Error::Json)?;
    require(
        subjects.len() == release.plan.assets.len() + 1,
        "release-attestation-subject-set-mismatch",
    )?;
    let mut names = BTreeSet::new();
    let mut source_count = 0;
    for subject in subjects {
        if subject["uri"] == purl {
            require(
                subject["digest"] == json!({"sha1":tag_object}),
                "release-attestation-tag-object-mismatch",
            )?;
            source_count += 1;
        } else {
            let name = text(subject, "name")?;
            let asset = release.plan.assets.get(name).ok_or(Error::Json)?;
            require(
                names.insert(name) && subject["digest"] == json!({"sha256":asset.sha256}),
                "release-attestation-asset-mismatch",
            )?;
        }
    }
    require(
        source_count == 1 && names.len() == release.plan.assets.len(),
        "release-attestation-incomplete",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Read retained genuine output solely for post-cryptographic consistency tests.
    fn fixture() -> (Value, FrozenRelease, DraftVerification) {
        let value: Value = io::parse(include_bytes!(
            "../../../tests/fixtures/release-attestation-v02/verified-cli-release.json"
        ))
        .unwrap();
        let statement = &value["verificationResult"]["statement"];
        let assets = statement["subject"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|subject| {
                subject["name"].as_str().map(|name| {
                    (
                        name.to_owned(),
                        ByteIdentity {
                            sha256: subject["digest"]["sha256"].as_str().unwrap().into(),
                            size: 1,
                        },
                    )
                })
            })
            .collect::<BTreeMap<_, _>>();
        // The static baseline is independent of a newly verified candidate. Sizes here are synthetic:
        // this parser test does not authenticate asset bytes or construct a production FrozenRelease.
        let inputs: InputIdentity = serde_json::from_value(json!({"source":{"repository":"cli/cli","commit":"fc4b137cdef0a6bd28fd461b7cf9c84a5812a8cd","git_ref":"refs/tags/v2.102.0"},
            "config_sha256":"a".repeat(64),"lock_sha256":"b".repeat(64),"cargo_lock_sha256":"c".repeat(64),"runtime":{"sha256":"d".repeat(64),"size":1},"runtime_version":"0.1.0",
            "run":{"id":1,"attempt":1,"workflow":{"repository":"cli/cli","path":".github/workflows/release.yml","commit":"fc4b137cdef0a6bd28fd461b7cf9c84a5812a8cd"}}})).unwrap();
        let plan = PublicationPlan {
            schema_version: 1,
            context: ByteIdentity::from_bytes(b"test context"),
            policy: ByteIdentity::from_bytes(b"test policy"),
            inputs,
            tag: "v2.102.0".into(),
            assets,
        };
        let frozen = FrozenRelease {
            workspace: private_workspace("armorer-release-statement-test-").unwrap(),
            identity: ByteIdentity::from_bytes(&serde_json::to_vec(&plan).unwrap()),
            plan,
        };
        let served = DraftVerification {
            release_id: 399674740,
            receipt: ByteIdentity::from_bytes(b"test served receipt"),
            assets: BTreeMap::new(),
        };
        (value, frozen, served)
    }
    /// Bind the official static tag, repository, owner and release IDs after actual native cryptography.
    fn check(value: &Value, frozen: &FrozenRelease, served: &DraftVerification) -> Result<()> {
        validate_attestation_result(
            value,
            frozen,
            served,
            "fc4b137cdef0a6bd28fd461b7cf9c84a5812a8cd",
            212613049,
            59704711,
        )
    }
    /// Keep a modified payload and result consistent to exercise the independent identity layer only.
    fn update_payload(value: &mut Value) {
        value["attestation"]["bundle"]["dsseEnvelope"]["payload"] =
            base64::engine::general_purpose::STANDARD
                .encode(serde_json::to_vec(&value["verificationResult"]["statement"]).unwrap())
                .into();
    }
    #[test]
    /// A genuine retained result must bind every exact release and subject identity, with no extra assets.
    fn release_attestation_requires_exact_v02_signed_subject_and_provider_ids() {
        let (original, frozen, served) = fixture();
        assert!(check(&original, &frozen, &served).is_ok());
        for key in [
            "databaseId",
            "ownerId",
            "repositoryId",
            "packageId",
            "repository",
            "purl",
            "tag",
        ] {
            let mut value = original.clone();
            value["verificationResult"]["statement"]["predicate"][key] = "other".into();
            update_payload(&mut value);
            assert!(check(&value, &frozen, &served).is_err(), "{key}");
        }
        for case in 0..6 {
            let mut value = original.clone();
            let s = &mut value["verificationResult"]["statement"];
            match case {
                0 => s["predicateType"] = "https://in-toto.io/attestation/release/v0.1".into(),
                1 => {
                    s["subject"].as_array_mut().unwrap().pop();
                }
                2 => {
                    let extra = s["subject"][1].clone();
                    s["subject"].as_array_mut().unwrap().push(extra);
                }
                3 => s["subject"][1]["digest"]["sha256"] = "f".repeat(64).into(),
                4 => s["subject"][0]["digest"]["sha1"] = "f".repeat(40).into(),
                _ => s["subject"][1]["name"] = "other.tar.gz".into(),
            };
            update_payload(&mut value);
            assert!(check(&value, &frozen, &served).is_err());
        }
    }
    #[test]
    /// Exit status or an unsigned lookalike statement cannot substitute for verified result, SAN and timestamps.
    fn missing_crypto_result_wrong_san_and_unsigned_result_rejected() {
        let (original, frozen, served) = fixture();
        for case in 0..4 {
            let mut v = original.clone();
            match case {
                0 => v["verificationResult"] = Value::Null,
                1 => {
                    v["verificationResult"]["signature"]["certificate"]["subjectAlternativeName"] =
                        "https://other.example".into()
                }
                2 => v["verificationResult"]["verifiedTimestamps"] = json!([]),
                _ => v["verificationResult"]["statement"]["predicate"]["tag"] = "v9.9.9".into(),
            };
            assert!(check(&v, &frozen, &served).is_err());
        }
    }
    #[test]
    /// Header parsing distinguishes actual denied, ambiguous throttled/denied and provider error states.
    fn native_http_status_and_exit_binding_preserves_unavailable_states() {
        for (status, state) in [
            (401, GateState::Denied),
            (403, GateState::Unknown),
            (404, GateState::Unknown),
            (410, GateState::Unsupported),
            (429, GateState::Error),
        ] {
            let r=Response::parse(false,format!("HTTP/2.0 {status} Error\r\nContent-Type: application/json\r\n\r\n{{\"message\":\"untrusted reflected text\"}}").as_bytes()).unwrap();
            assert_eq!(unavailable(r.status), state);
        }
        assert!(Response::parse(true, b"HTTP/2.0 403 Forbidden\n\n{}").is_err());
        assert!(Response::parse(false, b"HTTP/2.0 200 OK\n\n{}").is_err());
        assert!(Response::parse(true, b"{}\n\nHTTP/2.0 200 OK").is_err());
    }
    #[test]
    #[ignore = "requires independently qualified native gh and existing read-only authentication"]
    /// Verify the real public immutable CLI release against the frozen baseline; this grants no Armorer permit.
    fn real_pinned_gh_release_attestation_matches_frozen_baseline() {
        let executable = PathBuf::from(
            std::env::var_os("ARMORER_TEST_GH").expect("qualified native gh required"),
        );
        assert_eq!(
            io::identity(&executable, io::MAX_VERIFIER).unwrap(),
            sigstore::qualified_native_verifier().unwrap()
        );
        sigstore::native_executable(&executable).unwrap();
        let work = private_workspace("armorer-real-release-fixture-").unwrap();
        let mut command = Command::new(executable);
        command
            .args([
                "release", "verify", "v2.102.0", "--repo", "cli/cli", "--format", "json",
            ])
            .env_clear()
            .stdin(Stdio::null())
            .current_dir(work.path())
            .env("HOME", work.path())
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
            .env("GH_HOST", "github.com")
            .env("GH_PROMPT_DISABLED", "1")
            .env("GH_NO_UPDATE_NOTIFIER", "1");
        if let Some(token) =
            std::env::var_os("GH_TOKEN").or_else(|| std::env::var_os("GITHUB_TOKEN"))
        {
            command.env("GH_TOKEN", token);
        } else {
            command.env(
                "HOME",
                std::env::var_os("HOME").expect("existing owner configuration required"),
            );
        }
        let bytes = sigstore::run_process(
            command,
            Limits {
                stdout: 4 * 1024 * 1024,
                stderr: 65536,
                timeout: Duration::from_secs(120),
            },
        )
        .unwrap();
        let (_, frozen, served) = fixture();
        check(&io::parse::<Value>(&bytes).unwrap(), &frozen, &served).unwrap();
    }

    #[test]
    #[ignore = "requires independently qualified native gh and existing GET-only authentication"]
    /// Exercise real fixed-route settings reads and reject all write methods, native substitution and root substitution.
    fn real_native_publication_observations_never_grant_write_authority() {
        let executable = PathBuf::from(
            std::env::var_os("ARMORER_TEST_GH").expect("qualified native gh required"),
        );
        let work = private_workspace("armorer-real-publication-observation-").unwrap();
        let root = work.path().join("test-only-root.json");
        let root_bytes = include_bytes!("../../../tests/fixtures/sigstore/trusted_root.json");
        io::write_readonly(&root, root_bytes).unwrap();
        // This private test policy selects GET routes only. It is neither an independently
        // approved production policy nor qualification of this fixture as a release-signing root.
        let value = PublicationPolicy {
            schema_version: 1,
            context: ByteIdentity::from_bytes(b"test-only observation intent"),
            repository_id: 1398918200,
            owner_id: 3779002,
            release_attestation_root: ByteIdentity::from_bytes(root_bytes),
            default_branch: "main".into(),
            controller_workflow: WorkflowIdentity {
                repository: "brianluby/armorer".into(),
                path: ".github/workflows/development.yml".into(),
                commit: "5c5abeebb73b9d76ecc092a1c2d84956739fa587".into(),
            },
            allowed_actor_ids: [3779002].into(),
            allowed_publisher_ids: [3779002].into(),
            allowed_approver_ids: [3779002].into(),
            publish_environment: "armorer-readonly-qualification-v1".into(),
            publish_environment_id: 1,
            max_observation_age: 30,
            expires_at: now().unwrap() + 600,
        };
        let policy = TrustedPublicationPolicy {
            identity: ByteIdentity::from_bytes(&serde_json::to_vec(&value).unwrap()),
            policy: value,
        };
        let owner = PathBuf::from(std::env::var_os("HOME").expect("existing owner HOME required"));
        let readonly =
            NativeGithub::open_owner_read_only(&executable, &root, &policy, &owner).unwrap();
        // This guard precedes process startup, even when a workflow token exists in
        // the parent. No POST/PATCH reaches GitHub through this read-only client.
        for (method, route) in [
            ("POST", "repos/brianluby/armorer/releases"),
            (
                "POST",
                "https://uploads.github.com/repos/brianluby/armorer/releases/1/assets?name=inert",
            ),
            ("PATCH", "repos/brianluby/armorer/releases/1"),
        ] {
            assert!(
                readonly
                    .write(method, route.into(), b"{}", false)
                    .unwrap_err()
                    .to_string()
                    .contains("read-only-publication-client")
            );
        }
        let offered = work.path().join("offered-native");
        fs::write(&offered, b"unqualified executable bytes").unwrap();
        assert!(NativeGithub::open_owner_read_only(&offered, &root, &policy, &owner).is_err());
        let substituted_root = work.path().join("substituted-root.json");
        fs::write(&substituted_root, b"unapproved root bytes").unwrap();
        assert!(
            NativeGithub::open_owner_read_only(&executable, &substituted_root, &policy, &owner)
                .is_err()
        );
        let observer = if std::env::var_os("GH_TOKEN")
            .or_else(|| std::env::var_os("GITHUB_TOKEN"))
            .is_some()
        {
            NativeGithub::open(&executable, &root, &policy).unwrap()
        } else {
            readonly
        };
        // Fixed GETs alone are executed. Their unavailable states are retained,
        // and required attempt/serialization gates remain unproven regardless
        // of the current repository settings or the test policy's claims.
        let report = observer.observe_capabilities(&policy).unwrap();
        assert_eq!(report.repository_id, 1398918200);
        for name in [
            "current-attempt-environment-enforcement",
            "runner-wide-serialization",
        ] {
            assert_eq!(report.gates.get(name), Some(&GateState::Unsupported));
        }
        assert!(report.require_ready(now().unwrap(), 30).is_err());
        println!("{}", serde_json::to_string(&report).unwrap());
    }
}
