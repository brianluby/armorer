//! Version-one intent and resolved pin contracts.

use crate::{Error, Result, read_small, safe_path};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub const VERSION: u32 = 1;
pub const TARGETS: &[&str] = &[
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-gnu",
];

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema_version: u32,
    pub repository: String,
    pub toolchain: String,
    #[serde(default = "manifest_default")]
    pub manifest: String,
    pub deliverables: Vec<Deliverable>,
    pub feature_sets: BTreeMap<String, FeatureSet>,
    pub policy: Policy,
}

/// Use the workspace-root manifest when configuration omits `manifest`.
fn manifest_default() -> String {
    "Cargo.toml".into()
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Deliverable {
    pub id: String,
    pub profile: Profile,
    pub package: String,
    pub binary: Option<String>,
    pub targets: Vec<String>,
    pub feature_set: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    Library,
    Cli,
    Service,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FeatureSet {
    pub default_features: bool,
    #[serde(default)]
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub license_file: String,
    pub attestations: Attestations,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Attestations {
    Required,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Lock {
    pub schema_version: u32,
    pub config_sha256: String,
    pub runtime_version: String,
    pub workflows: WorkflowPin,
    pub tools: BTreeMap<String, ToolPin>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPin {
    pub repository: String,
    pub commit: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToolPin {
    pub version: String,
    pub sha256: String,
}

/// Accept 1–100 ASCII bytes, starting with an alphanumeric character and
/// continuing with alphanumeric characters, hyphens, or underscores.
pub fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

/// Accept 1–100 ASCII bytes, starting with an alphanumeric character and
/// continuing with alphanumeric characters or `_`, `-`, `+`, and `.`.
fn feature_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-+.".contains(&c))
}

/// Check `owner/name` syntax without verifying that the repository exists.
///
/// Each part must contain 1–100 ASCII bytes, start with an alphanumeric
/// character, and contain only alphanumeric characters, hyphens, underscores,
/// or periods.
pub fn repository_name(value: &str) -> bool {
    let parts: Vec<_> = value.split('/').collect();
    parts.len() == 2
        && parts.iter().all(|p| {
            !p.is_empty()
                && *p != "."
                && *p != ".."
                && p.len() <= 100
                && p.as_bytes()[0].is_ascii_alphanumeric()
                && p.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
        })
}

/// Accept a complete semantic version without prerelease or build metadata.
fn exact_version(value: &str) -> bool {
    semver::Version::parse(value).is_ok_and(|v| v.pre.is_empty() && v.build.is_empty())
}

/// Check for exactly `length` lowercase hexadecimal characters.
///
/// `length` counts encoded characters, not decoded bytes; zero accepts only
/// an empty string. This checks syntax without authenticating the digest.
pub fn hex_digest(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

/// Return `Error::Invalid` with `message` when the condition is false.
fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::Invalid(message.into()))
    }
}

/// Read and validate `armorer.toml` under `root`, returning its exact bytes
/// alongside the configuration for digest calculation.
///
/// # Errors
/// Returns `Error::Toml` for invalid UTF-8, malformed TOML, or a schema mismatch
/// (including unknown fields). Propagates path, I/O, size-limit, and
/// [`Config::validate`] errors.
pub fn load_config(root: &Path) -> Result<(Config, Vec<u8>)> {
    let bytes = read_small(&safe_path(root, "armorer.toml")?)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| Error::Toml)?;
    let config: Config = toml::from_str(text).map_err(|_| Error::Toml)?;
    config.validate(root)?;
    Ok((config, bytes))
}

impl Config {
    /// Check version-one configuration rules and path safety relative to `root`.
    ///
    /// Checks selection syntax, supported targets, uniqueness, and feature-set
    /// references. Referenced files need not exist; package, binary, and feature
    /// availability are checked during discovery.
    ///
    /// # Errors
    /// Returns `Error::Invalid` for a contract violation and propagates path and
    /// filesystem inspection errors from [`safe_path`].
    pub fn validate(&self, root: &Path) -> Result<()> {
        self.validate_shape()?;
        safe_path(root, &self.manifest)?;
        safe_path(root, &self.policy.license_file)?;
        Ok(())
    }

    /// Validate semantic shape without a filesystem lookup or temporary mutation.
    pub(crate) fn validate_shape(&self) -> Result<()> {
        require(self.schema_version == VERSION, "unsupported schema_version")?;
        require(
            repository_name(&self.repository),
            "repository must be owner/name",
        )?;
        require(
            exact_version(&self.toolchain),
            "toolchain must be an exact stable x.y.z version",
        )?;
        require(
            self.deliverables.len() <= 128 && !self.deliverables.is_empty(),
            "expected 1–128 deliverables",
        )?;
        require(
            !self.feature_sets.is_empty() && self.feature_sets.len() <= 128,
            "expected 1–128 feature sets",
        )?;
        require(
            self.manifest == "Cargo.toml",
            "v1 discovery requires the workspace-root Cargo.toml",
        )?;
        for (name, case) in &self.feature_sets {
            require(identifier(name), "invalid feature-set name")?;
            require(
                case.features.len() <= 128,
                "feature set exceeds 128 entries",
            )?;
            let mut features = BTreeSet::new();
            for feature in &case.features {
                require(
                    feature_name(feature),
                    "invalid feature name; select declared package features only",
                )?;
                require(features.insert(feature), "duplicate feature in feature set")?;
            }
        }
        let mut ids = BTreeSet::new();
        let mut selections = BTreeSet::new();
        for d in &self.deliverables {
            require(identifier(&d.id), "invalid deliverable id")?;
            require(ids.insert(&d.id), "duplicate deliverable id")?;
            require(identifier(&d.package), "invalid package name")?;
            match (d.profile, &d.binary) {
                (Profile::Library, None) => {}
                (Profile::Cli | Profile::Service, Some(binary)) => {
                    require(identifier(binary), "invalid binary name")?
                }
                _ => {
                    return Err(Error::Invalid(
                        "library forbids binary; CLI/service requires binary".into(),
                    ));
                }
            }
            require(
                self.feature_sets.contains_key(&d.feature_set),
                "unknown feature set",
            )?;
            require(
                !d.targets.is_empty() && d.targets.len() <= TARGETS.len(),
                "explicit supported targets required",
            )?;
            let mut targets = BTreeSet::new();
            for target in &d.targets {
                require(
                    TARGETS.contains(&target.as_str()),
                    "unsupported target triple",
                )?;
                require(targets.insert(target), "duplicate target triple")?;
                require(
                    selections.insert((&d.package, &d.binary, target, &d.feature_set)),
                    "duplicate artifact selection",
                )?;
            }
        }
        Ok(())
    }
}

/// Read `armorer.lock` under `root`, returning the validated lock and exact
/// bytes from the same read, or `None` when it is absent.
///
/// `config_digest` is the lowercase SHA-256 of the exact configuration bytes.
/// Checks its binding, runtime compatibility, and pin syntax; upstream pins
/// and distribution contents are not authenticated.
///
/// # Errors
/// Returns `Error::Toml` for invalid UTF-8, malformed TOML, or a schema mismatch,
/// and `Error::Invalid` for oversized input or invalid or incompatible lock
/// contents. Propagates path and I/O errors.
pub fn load_lock(root: &Path, config_digest: &str) -> Result<Option<(Lock, Vec<u8>)>> {
    let path = safe_path(root, "armorer.lock")?;
    if !path.try_exists()? {
        return Ok(None);
    }
    let bytes = read_small(&path)?;
    let lock: Lock = toml::from_str(std::str::from_utf8(&bytes).map_err(|_| Error::Toml)?)
        .map_err(|_| Error::Toml)?;
    require(
        lock.schema_version == VERSION,
        "unsupported lock schema_version",
    )?;
    require(
        hex_digest(&lock.config_sha256, 64) && lock.config_sha256 == config_digest,
        "armorer.lock does not match the exact config bytes",
    )?;
    require(
        lock.runtime_version == env!("CARGO_PKG_VERSION"),
        "incompatible runtime_version",
    )?;
    require(
        repository_name(&lock.workflows.repository) && hex_digest(&lock.workflows.commit, 40),
        "workflow pin must contain owner/repository and full lowercase commit SHA",
    )?;
    require(!lock.tools.is_empty(), "lock requires reviewed tools")?;
    for (name, pin) in &lock.tools {
        require(
            identifier(name) && exact_version(&pin.version) && hex_digest(&pin.sha256, 64),
            "tool pin requires a valid name, exact version and distribution SHA-256",
        )?;
    }
    Ok(Some((lock, bytes)))
}
