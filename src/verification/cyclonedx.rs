//! Qualified whole-document CycloneDX 1.5 validation, separate from authenticity.
use super::{io, sigstore};
use crate::{
    Error, Result,
    trust::{ByteIdentity, require},
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

pub const VALIDATOR_VERSION: &str = "0.33.1";
pub const VALIDATOR_SOURCE: &str = "b3cfa4b0edc356dad07e0b6e7ab6da0a94af0246";
const MAX_SBOM: u64 = 17 * 1024 * 1024;

/// Return the exact official native distribution; callers cannot replace it with another tool.
pub fn qualified_native_validator() -> Result<ByteIdentity> {
    let (sha256, size) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => (
            "bfc8b2538da86fe239bc53658bbb63c1c8c510a293c1e6891aa5bea5d3c58746",
            80337458,
        ),
        ("linux", "aarch64") => (
            "b2e9fdf9665ef49868a2ec012171c6e785dcd69745bc5869e53e4f4bfb096a5f",
            87752084,
        ),
        ("macos", "aarch64") => (
            "750c148780154833f6401f9067d08c5a4c31567b6ee3c26c062c3a95c62d741c",
            86373536,
        ),
        _ => return Err(Error::Invalid("unsupported-sbom-validator-platform".into())),
    };
    Ok(ByteIdentity {
        sha256: sha256.into(),
        size,
    })
}

/// A complete schema-validated document, not a signature or release-acceptance proof.
/// Its constructor is private; ordinary JSON cannot synthesize a successful native validation.
/// ```compile_fail
/// let proof: armorer::verification::cyclonedx::ValidatedSbom = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Debug)]
pub struct ValidatedSbom {
    bytes: ByteIdentity,
    document: Value,
    validator: ByteIdentity,
}
impl ValidatedSbom {
    /// Return the exact rehashed SBOM bytes validated by this operation.
    pub fn bytes(&self) -> &ByteIdentity {
        &self.bytes
    }
    /// Borrow the duplicate-free document for signed-predicate and graph reconciliation.
    pub fn document(&self) -> &Value {
        &self.document
    }
    /// Return the exact compiled native validator pin used for the complete document.
    pub fn validator(&self) -> &ByteIdentity {
        &self.validator
    }
}

/// Own the qualified validator snapshot; never execute a consuming artifact or caller helper.
pub struct OfflineSbomValidator {
    workspace: tempfile::TempDir,
    executable: PathBuf,
    bundle_directory: PathBuf,
    identity: ByteIdentity,
}
impl OfflineSbomValidator {
    /// Require an independently approved identity and the compiled official native pin before execution.
    pub fn open(executable: &Path, independently_approved: &ByteIdentity) -> Result<Self> {
        require(
            *independently_approved == qualified_native_validator()?,
            "unapproved-sbom-validator",
        )?;
        let workspace = crate::filesystem::private_tempdir("armorer-sbom-validator-", None)?;
        let copy = workspace.path().join("cyclonedx");
        let identity = io::snapshot(
            executable,
            &copy,
            independently_approved.size.min(io::MAX_VERIFIER),
        )?;
        require(
            identity == *independently_approved,
            "unapproved-sbom-validator-bytes",
        )?;
        sigstore::native_executable(&copy)?;
        io::readonly(&copy, true)?;
        // The qualified tool's own runtime cache is private to this validator, while each document stays isolated.
        let bundle_directory = workspace.path().join("native-bundle");
        fs::create_dir(&bundle_directory)?;
        Ok(Self {
            workspace,
            executable: copy,
            bundle_directory,
            identity,
        })
    }

    /// Validate all SBOM fields with fixed JSON/v1.5 flags, sterile environment and bounded native execution.
    /// A matching digest must come from authenticated inventory context in a release consumer.
    pub fn validate(&self, sbom: &Path, expected_bytes: &ByteIdentity) -> Result<ValidatedSbom> {
        expected_bytes.validate()?;
        require(expected_bytes.size <= MAX_SBOM, "sbom-document-size-limit")?;
        require(
            io::identity(&self.executable, io::MAX_VERIFIER)? == self.identity,
            "sbom-validator-snapshot-changed",
        )?;
        let request = crate::filesystem::private_tempdir("request-", Some(self.workspace.path()))?;
        let input = request.path().join("sbom.json");
        let bytes = io::snapshot(sbom, &input, expected_bytes.size)?;
        require(bytes == *expected_bytes, "sbom-document-byte-mismatch")?;
        let document: Value = io::parse(&io::read_bounded(&input, MAX_SBOM)?)?;
        require(
            document.get("bomFormat").and_then(Value::as_str) == Some("CycloneDX")
                && document.get("specVersion").and_then(Value::as_str) == Some("1.5"),
            "unsupported-sbom-document-version",
        )?;
        io::readonly(&input, false)?;
        let home = request.path().join("home");
        fs::create_dir(&home)?;
        let mut command = Command::new(&self.executable);
        command
            .args(["validate", "--input-file"])
            .arg(&input)
            .args([
                "--input-format",
                "json",
                "--input-version",
                "v1_5",
                "--fail-on-errors",
            ])
            .current_dir(request.path())
            .env_clear()
            .env("HOME", &home)
            .env("TMPDIR", request.path())
            .env("DOTNET_BUNDLE_EXTRACT_BASE_DIR", &self.bundle_directory)
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
            .env("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1")
            .stdin(Stdio::null());
        let output = sigstore::run_process(
            command,
            sigstore::Limits {
                timeout: Duration::from_secs(30),
                stdout: 1024 * 1024,
                stderr: 1024 * 1024,
            },
        )
        .map_err(|_| Error::Invalid("sbom-schema-validation-failed".into()))?;
        // Qualification binds this success text to the exact implementation; no autodetection or relaxed exit is used.
        require(
            std::str::from_utf8(&output).is_ok_and(|text| {
                text.lines()
                    .filter(|line| *line == "BOM validated successfully.")
                    .count()
                    == 1
            }),
            "sbom-validator-result-mismatch",
        )?;
        require(
            io::identity(&input, MAX_SBOM)? == bytes
                && io::identity(&self.executable, io::MAX_VERIFIER)? == self.identity,
            "sbom-validation-snapshot-changed",
        )?;
        Ok(ValidatedSbom {
            bytes,
            document,
            validator: self.identity.clone(),
        })
    }
}
