//! Hash-qualified offline gh adapter. A successful exit is only the first gate.
//! Verified proofs have private constructors and cannot be deserialized from claims.
use super::{
    bundle::{self, UnverifiedBundle},
    io,
};
use crate::{
    Error, Result,
    trust::{
        ByteIdentity, RunIdentity, Source, WorkflowIdentity, asset_name,
        policy::{EvidenceScope, Predicate, TrustBackend, VerificationMode, VerificationPolicy},
        require,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const VERIFIER_VERSION: &str = "2.102.0";
/// Exact independently qualified official native gh distribution identity.
/// Updating it requires a reviewed source change and new distribution receipts.
pub fn qualified_native_verifier() -> Result<ByteIdentity> {
    let (sha256, size) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => (
            "7469124f706944133d6a169691dd1c6c3511b12e85878d255e044e2948df4c9b",
            42086560,
        ),
        ("linux", "aarch64") => (
            "93308395c2d296a63a662742c6366e4db413d2a4870d07bd9b84e491c065d65d",
            39059616,
        ),
        ("macos", "aarch64") => (
            "8a4258433c81106343144857750316241759d06dcf16265cf3c4864a8f2f2ad6",
            39834784,
        ),
        _ => return Err(Error::Invalid("unsupported-verifier-platform".into())),
    };
    Ok(ByteIdentity {
        sha256: sha256.into(),
        size,
    })
}
const ISSUER: &str = "https://token.actions.githubusercontent.com";
const RESULT_TYPE: &str = "application/vnd.dev.sigstore.verificationresult+json;version=0.1";
const BUILD_TYPE: &str = "https://slsa-framework.github.io/github-actions-buildtypes/workflow/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Push,
    WorkflowDispatch,
}
impl Trigger {
    /// Map the fixed trigger enum to the exact certificate and CLI event spelling.
    fn name(self) -> &'static str {
        match self {
            Self::Push => "push",
            Self::WorkflowDispatch => "workflow_dispatch",
        }
    }
}

/// Independently supplied expectations, never inferred from a downloaded bundle.
#[derive(Debug, Clone)]
pub struct ExpectedAttestation {
    pub source: Source,
    /// The reusable signer and exact invoking run/attempt.
    pub run: RunIdentity,
    /// Source repository's caller workflow, also bound by the certificate.
    pub caller_workflow: WorkflowIdentity,
    pub trigger: Trigger,
    pub predicate: Predicate,
    pub scope: EvidenceScope,
    pub subject_name: String,
}
impl ExpectedAttestation {
    /// Check independent context against the approved source, mode, signer and scope policy.
    pub(super) fn validate(&self, policy: &VerificationPolicy) -> Result<()> {
        self.source.validate()?;
        self.run.validate()?;
        self.caller_workflow.validate()?;
        require(
            self.caller_workflow.repository == self.source.repository
                && self.caller_workflow.commit == self.source.commit
                && asset_name(&self.subject_name),
            "invalid-attestation-expectations",
        )?;
        require(
            matches!(
                (policy.mode, self.trigger),
                (VerificationMode::Release, Trigger::Push)
                    | (VerificationMode::Rehearsal, Trigger::WorkflowDispatch)
            ),
            "attestation-trigger-mode-mismatch",
        )?;
        policy.accept_signer(
            &self.source,
            &self.run.workflow,
            self.predicate,
            self.scope,
            true,
        )
    }
}

/// Authentication evidence for this exact subject, source, signer, run and scope.
/// This is not a statement that the whole release graph or Build L2 gate passed.
/// ```compile_fail
/// let proof: armorer::verification::sigstore::VerifiedAttestation =
///     serde_json::from_str("{}").unwrap();
/// ```
pub struct VerifiedAttestation {
    expected: ExpectedAttestation,
    subject: ByteIdentity,
    bundle: ByteIdentity,
    predicate: Value,
    verifier: ByteIdentity,
    root: ByteIdentity,
    policy: ByteIdentity,
}
impl std::fmt::Debug for VerifiedAttestation {
    /// Summarize proof identities without logging signed predicates or downloaded metadata.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifiedAttestation")
            .field("subject", &self.subject)
            .field("source", &self.expected.source)
            .field("run", &self.expected.run)
            .field("scope", &self.expected.scope)
            .field("predicate_type", &self.expected.predicate)
            .finish_non_exhaustive()
    }
}
impl VerifiedAttestation {
    /// Return the independent expectations matched by this authenticated statement.
    pub fn expected(&self) -> &ExpectedAttestation {
        &self.expected
    }
    /// Return the rehashed subject digest and byte count matched by this proof.
    pub fn subject(&self) -> &ByteIdentity {
        &self.subject
    }
    /// Return the identity of the exact bare bundle supplied to the verifier.
    pub fn bundle(&self) -> &ByteIdentity {
        &self.bundle
    }
    /// Return the signed predicate; its semantic accuracy still needs consumer reconciliation.
    pub fn predicate(&self) -> &Value {
        &self.predicate
    }
    /// Return the independently qualified native verifier identity used for this proof.
    pub fn verifier(&self) -> &ByteIdentity {
        &self.verifier
    }
    /// Return the approved root snapshot identity used for this proof.
    pub fn trusted_root(&self) -> &ByteIdentity {
        &self.root
    }
    /// Return the exact approved policy byte identity bound to this proof.
    pub fn policy(&self) -> &ByteIdentity {
        &self.policy
    }
}

/// Policy approval SHA must come from outside the artifact/download channel.
/// Root and executable hashes are then taken solely from that approved policy.
/// No credential, user gh configuration, network lookup, shell or plugin is used.
pub struct OfflineVerifier {
    policy: VerificationPolicy,
    policy_identity: ByteIdentity,
    workspace: tempfile::TempDir,
    executable: PathBuf,
    root: PathBuf,
}
impl OfflineVerifier {
    /// Authenticate approved policy bytes and compiled native pins before creating isolated trust snapshots.
    pub fn open(
        policy_path: &Path,
        approved_policy_sha256: &str,
        executable: &Path,
        root: &Path,
    ) -> Result<Self> {
        require(
            crate::config::hex_digest(approved_policy_sha256, 64),
            "invalid-approved-policy-digest",
        )?;
        let bytes = io::read_bounded(policy_path, 1_048_576)?;
        require(
            crate::digest(&bytes) == approved_policy_sha256,
            "unapproved-verification-policy",
        )?;
        let policy: VerificationPolicy = io::parse(&bytes)?;
        policy.validate(wall_time()?)?;
        require(
            policy.roots.verifier_version == VERIFIER_VERSION,
            "unsupported-verifier-version",
        )?;
        require(
            policy.roots.verifier == qualified_native_verifier()?,
            "unqualified-native-verifier-policy",
        )?;
        let workspace = tempfile::Builder::new()
            .prefix("armorer-verify-")
            .tempdir()?;
        let executable_copy = workspace.path().join("gh");
        let root_copy = workspace.path().join("trusted-root.jsonl");
        let executable_identity = io::snapshot(executable, &executable_copy, io::MAX_VERIFIER)?;
        require(
            executable_identity == policy.roots.verifier,
            "unapproved-verifier-bytes",
        )?;
        native_executable(&executable_copy)?;
        let root_identity = io::snapshot(root, &root_copy, io::MAX_ROOT)?;
        require(
            root_identity == policy.roots.trusted_root,
            "unapproved-trusted-root-bytes",
        )?;
        validate_root_transport(&io::read_bounded(&root_copy, io::MAX_ROOT)?)?;
        io::readonly(&executable_copy, true)?;
        io::readonly(&root_copy, false)?;
        Ok(Self {
            policy,
            policy_identity: ByteIdentity::from_bytes(&bytes),
            workspace,
            executable: executable_copy,
            root: root_copy,
        })
    }
    /// Borrow the independently approved policy without permitting mutation.
    pub fn policy(&self) -> &VerificationPolicy {
        &self.policy
    }
    /// Return the exact policy bytes matched to the independent approval SHA.
    pub fn policy_identity(&self) -> &ByteIdentity {
        &self.policy_identity
    }

    /// Authenticate one bare published evidence slot and all of its exact expected identities.
    pub fn verify(
        &self,
        artifact: &Path,
        expected_bytes: &ByteIdentity,
        bundle_path: &Path,
        expected: &ExpectedAttestation,
    ) -> Result<VerifiedAttestation> {
        let bundle = bundle::load_one(bundle_path)?;
        self.verify_one(artifact, expected_bytes, &bundle, expected)
    }

    /// Authenticate every offered gh-download JSONL record independently. Any
    /// invalid, wrong-scope or wrong-identity record fails the entire request.
    /// No proof list is returned until all offered records pass.
    pub fn verify_download(
        &self,
        artifact: &Path,
        expected_bytes: &ByteIdentity,
        download: &Path,
        expected: &ExpectedAttestation,
    ) -> Result<Vec<VerifiedAttestation>> {
        let bundles = bundle::load_download(download)?;
        let mut proofs = Vec::new();
        for bundle in bundles {
            proofs.push(self.verify_one(artifact, expected_bytes, &bundle, expected)?);
        }
        self.policy.validate(wall_time()?)?;
        Ok(proofs)
    }

    /// Snapshot one subject/bundle, verify it with fixed arguments, then validate and seal the proof.
    fn verify_one(
        &self,
        artifact: &Path,
        expected_bytes: &ByteIdentity,
        bundle: &UnverifiedBundle,
        expected: &ExpectedAttestation,
    ) -> Result<VerifiedAttestation> {
        self.policy.validate(wall_time()?)?;
        expected.validate(&self.policy)?;
        expected_bytes.validate()?;
        require(
            io::identity(&self.executable, io::MAX_VERIFIER)? == self.policy.roots.verifier
                && io::identity(&self.root, io::MAX_ROOT)? == self.policy.roots.trusted_root,
            "verifier-trust-snapshot-changed",
        )?;
        let slot = tempfile::Builder::new()
            .prefix("slot-")
            .tempdir_in(self.workspace.path())?;
        let artifact_path = slot.path().join(&expected.subject_name);
        let bundle_path = slot.path().join("bundle.sigstore.json");
        let subject = io::snapshot(artifact, &artifact_path, 1_073_741_824)?;
        require(
            subject == *expected_bytes,
            "attestation-subject-byte-mismatch",
        )?;
        io::readonly(&artifact_path, false)?;
        io::write_readonly(&bundle_path, bundle.bytes())?;
        let home = slot.path().join("home");
        fs::create_dir(&home)?;
        let signer = format!(
            "{}/{}@{}",
            expected.run.workflow.repository,
            expected.run.workflow.path,
            expected.run.workflow.commit
        );
        let predicate = predicate_name(expected.predicate);
        let mut command = Command::new(&self.executable);
        command
            .arg("attestation")
            .arg("verify")
            .arg(&artifact_path)
            .args([
                "--repo",
                &expected.source.repository,
                "--hostname",
                "github.com",
                "--signer-workflow",
                &signer,
                "--signer-digest",
                &expected.run.workflow.commit,
                "--source-digest",
                &expected.source.commit,
                "--source-ref",
                &expected.source.git_ref,
                "--cert-oidc-issuer",
                ISSUER,
                "--deny-self-hosted-runners",
                "--digest-alg",
                "sha256",
                "--predicate-type",
                predicate,
                "--format",
                "json",
            ])
            .arg("--bundle")
            .arg(&bundle_path)
            .arg("--custom-trusted-root")
            .arg(&self.root)
            .current_dir(slot.path())
            .env_clear()
            .env("HOME", &home)
            .env("GH_CONFIG_DIR", &home)
            .env("GH_PROMPT_DISABLED", "1")
            .env("GH_NO_UPDATE_NOTIFIER", "1")
            .env("GH_NO_EXTENSION_UPDATE_NOTIFIER", "1")
            .env("NO_COLOR", "1")
            .stdin(Stdio::null());
        if self.policy.roots.backend == TrustBackend::GithubPrivate {
            command.arg("--no-public-good");
        }
        let output = run_process(command, Limits::default())?;
        // Recheck all snapshots; the result applies to the exact bytes passed to gh.
        require(
            io::identity(&artifact_path, 1_073_741_824)? == subject
                && io::identity(&bundle_path, io::MAX_BUNDLE)?
                    == ByteIdentity::from_bytes(bundle.bytes())
                && io::identity(&self.executable, io::MAX_VERIFIER)? == self.policy.roots.verifier
                && io::identity(&self.root, io::MAX_ROOT)? == self.policy.roots.trusted_root,
            "verification-snapshot-changed",
        )?;
        let predicate = validate_output(
            &output,
            bundle.statement(),
            &subject,
            expected,
            &self.policy.roots.backend,
        )?;
        self.policy.validate(wall_time()?)?;
        Ok(VerifiedAttestation {
            expected: expected.clone(),
            subject,
            bundle: ByteIdentity::from_bytes(bundle.bytes()),
            predicate,
            verifier: self.policy.roots.verifier.clone(),
            root: self.policy.roots.trusted_root.clone(),
            policy: self.policy_identity.clone(),
        })
    }
}

/// Observe current epoch seconds; an unavailable clock cannot bypass review-expiry gates.
pub(super) fn wall_time() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| Error::Invalid("verification-clock-unavailable".into()))
}

/// Map only the two frozen predicate types to exact native verifier flags.
fn predicate_name(predicate: Predicate) -> &'static str {
    match predicate {
        Predicate::SlsaProvenanceV1 => "https://slsa.dev/provenance/v1",
        Predicate::CycloneDxV15 => "https://cyclonedx.org/bom",
    }
}

/// Accept complete bounded root objects or JSONL records, rejecting ambiguous partial input.
fn validate_root_transport(bytes: &[u8]) -> Result<()> {
    if let Ok(value) = io::parse::<Value>(bytes) {
        return require(value.is_object(), "invalid-trusted-root-transport");
    }
    let mut count = 0;
    for line in bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
    {
        count += 1;
        require(count <= 32, "too-many-trusted-root-records")?;
        let root: Value = io::parse(line)?;
        require(root.is_object(), "invalid-trusted-root-transport")?;
    }
    require(count > 0, "trusted-root-record-missing")
}

/// Check supported native headers after authenticating the complete official executable bytes.
pub(super) fn native_executable(path: &Path) -> Result<()> {
    let mut header = [0_u8; 32];
    io::regular(path, io::MAX_VERIFIER)?.read_exact(&mut header)?;
    let supported = if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        &header[..6] == b"\x7fELF\x02\x01" && u16::from_le_bytes([header[18], header[19]]) == 62
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        &header[..6] == b"\x7fELF\x02\x01" && u16::from_le_bytes([header[18], header[19]]) == 183
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        header[..4] == [0xcf, 0xfa, 0xed, 0xfe]
            && u32::from_le_bytes([header[4], header[5], header[6], header[7]]) == 0x0100000c
    } else {
        false
    };
    require(supported, "unsupported-or-non-native-verifier")
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Subject {
    name: String,
    digest: BTreeMap<String, String>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Statement {
    #[serde(rename = "_type")]
    statement_type: String,
    subject: Vec<Subject>,
    #[serde(rename = "predicateType")]
    predicate_type: Predicate,
    predicate: Value,
}

/// Read a required string from verified provider output, failing on absent or wrong-type fields.
fn field<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Invalid("verified-certificate-field-missing".into()))
}
/// Compare an identity exactly; case, regex and prefix relaxations are not accepted.
fn equal_field(value: &Value, key: &str, expected: &str) -> Result<()> {
    require(field(value, key)? == expected, "verified-identity-mismatch")
}

/// Match one cryptographically verified result to the original signed payload and independent context.
fn validate_output(
    output: &[u8],
    signed_statement: &Value,
    subject: &ByteIdentity,
    expected: &ExpectedAttestation,
    backend: &TrustBackend,
) -> Result<Value> {
    let value: Value = io::parse(output)?;
    let results = value.as_array().ok_or(Error::Json)?;
    require(results.len() == 1, "verified-result-count-mismatch")?;
    let result = results[0].get("verificationResult").ok_or(Error::Json)?;
    equal_field(result, "mediaType", RESULT_TYPE)?;
    let timestamps = result
        .get("verifiedTimestamps")
        .and_then(Value::as_array)
        .ok_or(Error::Json)?;
    require(
        !timestamps.is_empty()
            && timestamps
                .iter()
                .all(|t| t.get("timestamp").is_some_and(Value::is_string)),
        "verified-timestamp-missing",
    )?;
    let certificate = result
        .pointer("/signature/certificate")
        .ok_or(Error::Json)?;
    let source_uri = format!("https://github.com/{}", expected.source.repository);
    let signer_uri = format!(
        "https://github.com/{}/{}@{}",
        expected.run.workflow.repository, expected.run.workflow.path, expected.run.workflow.commit
    );
    let caller_uri = format!(
        "{}/{}@{}",
        source_uri, expected.caller_workflow.path, expected.source.git_ref
    );
    let invocation = format!(
        "{source_uri}/actions/runs/{}/attempts/{}",
        expected.run.id, expected.run.attempt
    );
    for (key, content) in [
        ("issuer", ISSUER),
        ("subjectAlternativeName", signer_uri.as_str()),
        ("buildSignerURI", signer_uri.as_str()),
        ("buildSignerDigest", expected.run.workflow.commit.as_str()),
        ("runnerEnvironment", "github-hosted"),
        ("sourceRepositoryURI", source_uri.as_str()),
        ("sourceRepositoryDigest", expected.source.commit.as_str()),
        ("sourceRepositoryRef", expected.source.git_ref.as_str()),
        ("buildConfigURI", caller_uri.as_str()),
        (
            "buildConfigDigest",
            expected.caller_workflow.commit.as_str(),
        ),
        ("buildTrigger", expected.trigger.name()),
        ("runInvocationURI", invocation.as_str()),
        (
            "sourceRepositoryVisibilityAtSigning",
            match backend {
                TrustBackend::SigstorePublicGood => "public",
                TrustBackend::GithubPrivate => "private",
            },
        ),
    ] {
        equal_field(certificate, key, content)?;
    }
    // Legacy extensions, when present, must agree with the required modern ones.
    for (key, content) in [
        ("githubWorkflowTrigger", expected.trigger.name()),
        ("githubWorkflowSHA", expected.source.commit.as_str()),
        ("githubWorkflowRef", expected.source.git_ref.as_str()),
        (
            "githubWorkflowRepository",
            expected.source.repository.as_str(),
        ),
    ] {
        if certificate.get(key).is_some() {
            equal_field(certificate, key, content)?;
        }
    }
    let statement_value = result.get("statement").ok_or(Error::Json)?;
    require(
        statement_value == signed_statement,
        "verified-statement-payload-mismatch",
    )?;
    let statement: Statement =
        serde_json::from_value(statement_value.clone()).map_err(|_| Error::Json)?;
    require(
        statement.statement_type == "https://in-toto.io/Statement/v1"
            && statement.predicate_type == expected.predicate
            && statement.subject.len() == 1,
        "verified-statement-scope-mismatch",
    )?;
    let claimed = &statement.subject[0];
    require(
        claimed.name == expected.subject_name
            && claimed.digest.len() == 1
            && claimed.digest.get("sha256") == Some(&subject.sha256),
        "verified-subject-mismatch",
    )?;
    if expected.predicate == Predicate::SlsaProvenanceV1 {
        let definition = statement
            .predicate
            .get("buildDefinition")
            .ok_or(Error::Json)?;
        equal_field(definition, "buildType", BUILD_TYPE)?;
        let workflow = definition
            .pointer("/externalParameters/workflow")
            .ok_or(Error::Json)?;
        equal_field(workflow, "repository", &source_uri)?;
        equal_field(workflow, "path", &expected.caller_workflow.path)?;
        equal_field(workflow, "ref", &expected.source.git_ref)?;
        let github = definition
            .pointer("/internalParameters/github")
            .ok_or(Error::Json)?;
        equal_field(github, "event_name", expected.trigger.name())?;
        for (parameter, extension) in [
            ("repository_id", "sourceRepositoryIdentifier"),
            ("repository_owner_id", "sourceRepositoryOwnerIdentifier"),
        ] {
            equal_field(github, parameter, field(certificate, extension)?)?;
        }
        let dependencies = definition
            .get("resolvedDependencies")
            .and_then(Value::as_array)
            .ok_or(Error::Json)?;
        let source_dependency_uri = format!("git+{source_uri}@{}", expected.source.git_ref);
        let matches: Vec<_> = dependencies
            .iter()
            .filter(|d| d.get("uri").and_then(Value::as_str) == Some(&source_dependency_uri))
            .collect();
        require(
            matches.len() == 1
                && matches[0]
                    .pointer("/digest/gitCommit")
                    .and_then(Value::as_str)
                    == Some(&expected.source.commit),
            "verified-provenance-source-mismatch",
        )?;
        let details = statement.predicate.get("runDetails").ok_or(Error::Json)?;
        equal_field(
            details.get("builder").ok_or(Error::Json)?,
            "id",
            "https://github.com/actions/runner/github-hosted",
        )?;
        equal_field(
            details.get("metadata").ok_or(Error::Json)?,
            "invocationId",
            &invocation,
        )?;
    }
    Ok(statement.predicate)
}

#[derive(Clone, Copy)]
pub(super) struct Limits {
    pub(super) timeout: Duration,
    pub(super) stdout: usize,
    pub(super) stderr: usize,
}
impl Default for Limits {
    /// Use fixed production time and output limits for every native invocation.
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            stdout: 128 * 1024 * 1024,
            stderr: 1024 * 1024,
        }
    }
}
struct Running(Child);
impl Running {
    /// Kill and reap the verifier process after a timeout, failure or cleanup.
    fn stop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
impl Drop for Running {
    /// Reap a spawned verifier even when an earlier error exits the request.
    fn drop(&mut self) {
        self.stop();
    }
}

/// Drain both bounded streams while enforcing the pinned child exit status and deadline.
pub(super) fn run_process(mut command: Command, limits: Limits) -> Result<Vec<u8>> {
    let child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| Error::Invalid("attestation-verifier-spawn-failed".into()))?;
    let mut running = Running(child);
    let stdout = running.0.stdout.take().ok_or(Error::Json)?;
    let stderr = running.0.stderr.take().ok_or(Error::Json)?;
    let (sender, receiver) = mpsc::channel();
    let stdout_sender = sender.clone();
    std::thread::scope(|scope| {
        scope.spawn(move || {
            let _ = stdout_sender.send((true, capture(stdout, limits.stdout)));
        });
        scope.spawn(move || {
            let _ = sender.send((false, capture(stderr, limits.stderr)));
        });
        let started = Instant::now();
        let mut out = None;
        let mut err = None;
        let mut status = None;
        let outcome = loop {
            while let Ok((is_out, result)) = receiver.try_recv() {
                match result {
                    Ok(bytes) => {
                        if is_out {
                            out = Some(bytes)
                        } else {
                            err = Some(bytes)
                        }
                    }
                    Err(error) => {
                        running.stop();
                        return Err(error);
                    }
                }
            }
            if status.is_none() {
                match running.0.try_wait() {
                    Ok(value) => status = value,
                    Err(_) => break Err(Error::Invalid("attestation-verifier-wait-failed".into())),
                }
            }
            if let Some(status) = status {
                if !status.success() {
                    break Err(Error::Invalid("attestation-authentication-failed".into()));
                }
                if out.is_some() && err.is_some() {
                    break Ok(out.take().ok_or(Error::Json)?);
                }
            }
            if started.elapsed() >= limits.timeout {
                break Err(Error::Invalid("attestation-verifier-timeout".into()));
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        if outcome.is_err() {
            running.stop();
        }
        outcome
    })
}
/// Read only the limit plus one byte and fail when a child stream exceeds its cap.
fn capture(mut stream: impl Read, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    stream
        .by_ref()
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    require(bytes.len() <= limit, "attestation-verifier-output-limit")?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Load retained real-verifier output solely for post-cryptographic consistency tests.
    fn fixture() -> (Value, Value, ByteIdentity, ExpectedAttestation) {
        let output: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/sigstore/verified-result.json"
        ))
        .unwrap();
        let signed = output[0]["verificationResult"]["statement"].clone();
        let source = Source {
            repository: "malancas/attest-demo".into(),
            commit: "95baf27389e83e6a5c48f42e190d48d7abcea19e".into(),
            git_ref: "refs/heads/main".into(),
        };
        let caller_workflow = WorkflowIdentity {
            repository: source.repository.clone(),
            path: ".github/workflows/shared.yml".into(),
            commit: source.commit.clone(),
        };
        let signer = WorkflowIdentity {
            repository: "github/artifact-attestations-workflows".into(),
            path: ".github/workflows/attest.yml".into(),
            commit: "09b495c3f12c7881b3cc17209a327792065c1a1d".into(),
        };
        let expected = ExpectedAttestation {
            source,
            run: RunIdentity {
                id: 9228858953,
                attempt: 1,
                workflow: signer,
            },
            caller_workflow,
            trigger: Trigger::WorkflowDispatch,
            predicate: Predicate::SlsaProvenanceV1,
            scope: EvidenceScope::FinalArtifact,
            subject_name: "github_provenance_demo-0.0.0-py3-none-any.whl".into(),
        };
        let bytes = ByteIdentity {
            sha256: "49a3aa6075e0f49f82843e74b5baa614ad2a588e6675612bf108a0a008c5ac25".into(),
            size: 2962,
        };
        (output, signed, bytes, expected)
    }
    /// Exercise identity checks without constructing a public authenticated proof.
    fn check(
        value: &Value,
        signed: &Value,
        bytes: &ByteIdentity,
        expected: &ExpectedAttestation,
    ) -> Result<Value> {
        validate_output(
            &serde_json::to_vec(value).unwrap(),
            signed,
            bytes,
            expected,
            &TrustBackend::SigstorePublicGood,
        )
    }
    #[test]
    /// Reject every changed certificate, run, provenance or subject identity after the crypto boundary.
    fn verified_provider_output_requires_all_identities_and_exact_single_subject() {
        // This test exercises post-crypto consistency only. Real cryptographic
        // verification is required separately by tests/real_sigstore.rs.
        let (original, signed, bytes, expected) = fixture();
        assert!(check(&original, &signed, &bytes, &expected).is_ok());
        for (key, bad) in [
            ("issuer", "https://untrusted.invalid"),
            (
                "subjectAlternativeName",
                "https://github.com/wrong/workflow",
            ),
            ("buildSignerURI", "https://github.com/wrong/workflow"),
            (
                "buildSignerDigest",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
            ("runnerEnvironment", "self-hosted"),
            ("sourceRepositoryURI", "https://github.com/attacker/repo"),
            (
                "sourceRepositoryDigest",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
            ("sourceRepositoryRef", "refs/heads/attacker"),
            (
                "buildConfigURI",
                "https://github.com/attacker/repo/.github/workflows/shared.yml@refs/heads/main",
            ),
            (
                "buildConfigDigest",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
            ("buildTrigger", "pull_request_target"),
            (
                "runInvocationURI",
                "https://github.com/malancas/attest-demo/actions/runs/9228858953/attempts/2",
            ),
            ("sourceRepositoryVisibilityAtSigning", "private"),
            (
                "githubWorkflowSHA",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
        ] {
            let mut value = original.clone();
            value[0]["verificationResult"]["signature"]["certificate"][key] = bad.into();
            assert!(check(&value, &signed, &bytes, &expected).is_err(), "{key}");
        }
        let mut value = original.clone();
        value[0]["verificationResult"]["verifiedTimestamps"] = serde_json::json!([]);
        assert!(check(&value, &signed, &bytes, &expected).is_err());
        let mut value = original.clone();
        value.as_array_mut().unwrap().push(original[0].clone());
        assert!(check(&value, &signed, &bytes, &expected).is_err());
        let mut value = original.clone();
        value[0]["verificationResult"]["statement"]["subject"][0]["name"] = "other.whl".into();
        assert!(check(&value, &signed, &bytes, &expected).is_err());
        let wrong_signed = value[0]["verificationResult"]["statement"].clone();
        assert!(check(&value, &wrong_signed, &bytes, &expected).is_err());
        let mut value = original.clone();
        value[0]["verificationResult"]["statement"]["predicate"]["runDetails"]["metadata"]["invocationId"] =
            "different-run".into();
        let wrong_signed = value[0]["verificationResult"]["statement"].clone();
        assert!(check(&value, &wrong_signed, &bytes, &expected).is_err());
        let mut value = original.clone();
        let dep = value[0]["verificationResult"]["statement"]["predicate"]["buildDefinition"]["resolvedDependencies"][0].clone();
        value[0]["verificationResult"]["statement"]["predicate"]["buildDefinition"]["resolvedDependencies"].as_array_mut().unwrap().push(dep);
        let wrong_signed = value[0]["verificationResult"]["statement"].clone();
        assert!(check(&value, &wrong_signed, &bytes, &expected).is_err());
    }
    #[test]
    /// Ensure invalid root transport cannot select a usable subset of supplied records.
    fn root_transport_rejects_duplicates_arrays_empty_and_malformed_records() {
        assert!(
            validate_root_transport(include_bytes!(
                "../../tests/fixtures/sigstore/trusted_root.json"
            ))
            .is_ok()
        );
        assert!(validate_root_transport(b"{\"a\":1}\n{\"b\":2}\n").is_ok());
        for bytes in [
            b"{\"a\":1,\"a\":2}".as_slice(),
            b"[]",
            b"",
            b"{}\ninvalid",
            b"{\"a\":1,\"a\":2}\n{}",
        ] {
            assert!(validate_root_transport(bytes).is_err());
        }
    }
    #[cfg(unix)]
    #[test]
    /// Exercise time/output/exit faults independently of genuine cryptographic verification.
    fn subprocess_faults_have_bounded_output_time_and_no_raw_error_disclosure() {
        /// Build a test-only shell process to simulate bounded child faults; production never uses it.
        fn command(script: &str) -> Command {
            let mut c = Command::new("/bin/sh");
            c.args(["-c", script]).stdin(Stdio::null());
            c
        }
        let limits = Limits {
            timeout: Duration::from_millis(150),
            stdout: 4096,
            stderr: 4096,
        };
        assert_eq!(run_process(command("printf '[]'"), limits).unwrap(), b"[]");
        let error = run_process(
            command("printf 'fixture-sensitive-error' >&2; exit 1"),
            limits,
        )
        .unwrap_err()
        .to_string();
        assert!(!error.contains("fixture-sensitive-error"));
        for script in [
            "while :; do :; done",
            "while :; do printf '0123456789'; done",
            "while :; do printf '0123456789' >&2; done",
        ] {
            let started = Instant::now();
            assert!(run_process(command(script), limits).is_err());
            assert!(started.elapsed() < Duration::from_secs(3));
        }
    }
}
