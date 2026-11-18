//! One reviewed file that names the inputs of `verify-release` and `publication`.
//!
//! The manifest is an alternative to the individual flags, not a trust mechanism. Its
//! exact bytes are hashed and compared with an independently supplied digest before
//! the file is parsed or any path inside it is opened (the order `catalog::authenticate`
//! uses). Reviewing the manifest means reviewing the approval digests inside it.
use super::io;
use crate::{
    Error, Result, config,
    trust::{ByteIdentity, require},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Contract name every manifest must carry.
pub const CONTRACT: &str = "armorer-release-inputs";
/// Largest accepted manifest, in bytes.
pub const MAX_MANIFEST: u64 = 64 * 1024;

/// Frozen catalog semantics a manifest may select; the names match `--context-kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub enum ReleaseContextKind {
    #[serde(rename = "native-v3")]
    NativeV3,
}

/// Inputs shared by `verify-release` and `publication`; one field per command-line flag.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReleaseInputsManifest {
    #[schemars(range(min = 1, max = 1))]
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub schema_version: u32,
    #[schemars(extend("const" = CONTRACT))]
    pub contract: String,
    /// `--directory`; a relative path resolves against the manifest's directory.
    #[schemars(length(min = 1))]
    pub directory: String,
    /// `--trusted-inputs`.
    #[schemars(length(min = 1))]
    pub trusted_inputs: String,
    /// `--expect-context-sha256`.
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub expect_context_sha256: String,
    /// `--context-kind`.
    pub context_kind: ReleaseContextKind,
    /// `--gh`.
    #[schemars(length(min = 1))]
    pub gh: String,
    /// `--trusted-root`.
    #[schemars(length(min = 1))]
    pub trusted_root: String,
    /// `--cyclonedx`.
    #[schemars(length(min = 1))]
    pub cyclonedx: String,
    /// Required by `publication`, ignored by `verify-release`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publication: Option<PublicationManifest>,
}

/// The three inputs only `publication` takes.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublicationManifest {
    /// `--publication-policy`.
    #[schemars(length(min = 1))]
    pub policy: String,
    /// `--expect-publication-policy-sha256`.
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub expect_policy_sha256: String,
    /// `--release-attestation-root`.
    #[schemars(length(min = 1))]
    pub release_attestation_root: String,
}

/// Publication inputs with resolved paths.
#[derive(Debug, Clone)]
pub struct PublicationFiles {
    pub policy: PathBuf,
    pub expect_policy_sha256: String,
    pub release_attestation_root: PathBuf,
}

/// Resolved inputs, from the manifest or from the individual flags.
#[derive(Debug, Clone)]
pub struct ReleaseInputs {
    pub directory: PathBuf,
    pub trusted_inputs: PathBuf,
    pub expect_context_sha256: String,
    pub context_kind: ReleaseContextKind,
    pub gh: PathBuf,
    pub trusted_root: PathBuf,
    pub cyclonedx: PathBuf,
    pub publication: Option<PublicationFiles>,
    /// Byte identity of the authenticated manifest; `None` for the flag form.
    pub manifest: Option<ByteIdentity>,
}

/// Reject empty paths and control characters; resolve relative paths against `base`.
fn resolve(base: &Path, value: &str) -> Result<PathBuf> {
    require(
        !value.is_empty() && !value.chars().any(char::is_control),
        "release-inputs-manifest-invalid-path",
    )?;
    let path = Path::new(value);
    Ok(if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    })
}

impl ReleaseInputs {
    /// Authenticate the manifest bytes against `expected_sha256`, then parse and resolve.
    ///
    /// The digest comparison precedes parsing and every path access; a mismatch reports
    /// only a fixed message, whatever the file contains.
    pub fn open(manifest: &Path, expected_sha256: &str) -> Result<Self> {
        require(
            config::hex_digest(expected_sha256, 64),
            "invalid-release-inputs-manifest-digest",
        )?;
        let bytes = io::read_bounded(manifest, MAX_MANIFEST)?;
        require(
            crate::digest(&bytes) == expected_sha256,
            "release-inputs-manifest-digest-mismatch",
        )?;
        let parsed: ReleaseInputsManifest = io::parse(&bytes)?;
        require(
            parsed.schema_version == 1,
            "unsupported-release-inputs-manifest-version",
        )?;
        require(
            parsed.contract == CONTRACT,
            "unsupported-release-inputs-manifest-contract",
        )?;
        // A bare file name has an empty parent: its directory is the current one.
        let base = manifest.parent().unwrap_or_else(|| Path::new(""));
        let publication = parsed
            .publication
            .map(|p| -> Result<PublicationFiles> {
                Ok(PublicationFiles {
                    policy: resolve(base, &p.policy)?,
                    expect_policy_sha256: p.expect_policy_sha256,
                    release_attestation_root: resolve(base, &p.release_attestation_root)?,
                })
            })
            .transpose()?;
        Ok(Self {
            directory: resolve(base, &parsed.directory)?,
            trusted_inputs: resolve(base, &parsed.trusted_inputs)?,
            expect_context_sha256: parsed.expect_context_sha256,
            context_kind: parsed.context_kind,
            gh: resolve(base, &parsed.gh)?,
            trusted_root: resolve(base, &parsed.trusted_root)?,
            cyclonedx: resolve(base, &parsed.cyclonedx)?,
            publication,
            manifest: Some(ByteIdentity::from_bytes(&bytes)),
        })
    }

    /// The publication block, required by every `publication` subcommand.
    pub fn publication(&self) -> Result<&PublicationFiles> {
        self.publication.as_ref().ok_or_else(|| {
            Error::Invalid("release-inputs-manifest-has-no-publication-block".into())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Without a publication block, publication inputs are refused with the fixed error.
    #[test]
    fn publication_block_is_required_on_demand() {
        let inputs = ReleaseInputs {
            directory: "d".into(),
            trusted_inputs: "t".into(),
            expect_context_sha256: "0".repeat(64),
            context_kind: ReleaseContextKind::NativeV3,
            gh: "g".into(),
            trusted_root: "r".into(),
            cyclonedx: "c".into(),
            publication: None,
            manifest: None,
        };
        assert!(
            matches!(inputs.publication(), Err(Error::Invalid(m)) if m == "release-inputs-manifest-has-no-publication-block")
        );
    }

    /// Relative paths join the base; absolute paths, and only valid text, pass through.
    #[test]
    fn resolve_rules() {
        let base = Path::new("/m");
        assert_eq!(resolve(base, "a/b").unwrap(), Path::new("/m/a/b"));
        assert_eq!(resolve(base, "/x/y").unwrap(), Path::new("/x/y"));
        assert!(resolve(base, "").is_err());
        assert!(resolve(base, "a\nb").is_err());
        assert!(resolve(base, "a\u{7}").is_err());
    }
}
