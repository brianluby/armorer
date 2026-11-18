//! Version-one intent and resolved pin contracts.

use crate::{
    Error, Result,
    filesystem::{optional_bytes, read_managed},
    safe_path,
};
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
    #[schemars(transform = crate::schema_bounds::unsigned)]
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    #[schemars(regex(
        pattern = "^[A-Za-z0-9][A-Za-z0-9_.-]{0,99}/[A-Za-z0-9][A-Za-z0-9_.-]{0,99}$(?![\\s\\S])"
    ))]
    pub repository: String,
    #[schemars(regex(
        pattern = "^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$(?![\\s\\S])"
    ))]
    pub toolchain: String,
    #[serde(default = "manifest_default")]
    #[schemars(regex(pattern = "^Cargo\\.toml$(?![\\s\\S])"))]
    pub manifest: String,
    #[schemars(length(min = 1, max = 128))]
    pub deliverables: Vec<Deliverable>,
    #[schemars(extend("minProperties" = 1, "maxProperties" = 128, "propertyNames" = serde_json::json!({"pattern":"^(?!.*--)[A-Za-z0-9][A-Za-z0-9_-]{0,99}$(?![\\s\\S])"})))]
    pub feature_sets: BTreeMap<String, FeatureSet>,
    pub policy: Policy,
    /// Optional managed repository files. Omitted from every serialization while
    /// all switches are off, so existing configurations, plans and digests keep
    /// their exact bytes.
    #[serde(default, skip_serializing_if = "RepositoryFiles::is_default")]
    pub repository_files: RepositoryFiles,
}

/// Most code owners one configuration may list.
pub const MAX_CODE_OWNERS: usize = 16;

/// Opt-in switches for the optional managed repository files. Every switch is off
/// by default; an absent table selects none of these files.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RepositoryFiles {
    /// Manage `SECURITY.md` from the fixed catalog template.
    #[serde(default)]
    pub security_policy: bool,
    /// Manage `.github/dependabot.yml` from the fixed catalog template.
    #[serde(default)]
    pub dependabot: bool,
    /// Manage `.github/CODEOWNERS` for these owners; an empty list leaves it unmanaged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(length(max = 16))]
    #[schemars(extend("uniqueItems" = true))]
    #[schemars(inner(regex(
        pattern = "^@(?=[A-Za-z0-9-]{1,39}(?:/|$(?![\\s\\S])))[A-Za-z0-9]+(?:-[A-Za-z0-9]+)*(?:/[A-Za-z0-9][A-Za-z0-9_.-]{0,99})?$(?![\\s\\S])"
    )))]
    pub code_owners: Vec<String>,
}

impl RepositoryFiles {
    /// True when no optional file is selected; the table is then never serialized.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Check the owner list: at most 16 unique `@user` or `@org/team` entries.
    ///
    /// # Errors
    /// Returns `Error::Invalid` with a fixed message that never echoes a value.
    pub fn validate(&self) -> Result<()> {
        require(
            self.code_owners.len() <= MAX_CODE_OWNERS,
            "repository_files.code_owners exceeds 16 entries",
        )?;
        let mut seen = BTreeSet::new();
        for owner in &self.code_owners {
            require(
                code_owner(owner),
                "repository_files.code_owners entries must be @user or @org/team",
            )?;
            // GitHub logins and slugs are case-insensitive; one owner listed twice is a duplicate.
            require(
                seen.insert(owner.to_ascii_lowercase()),
                "duplicate repository_files.code_owners entry",
            )?;
        }
        Ok(())
    }
}

/// GitHub user or organization login: 1–39 ASCII alphanumerics and single hyphens,
/// neither starting nor ending with a hyphen.
fn github_login(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 39
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-')
}

/// Accept exactly `@user` or `@org/team`, where a team slug is 1–100 ASCII
/// alphanumerics, `-`, `_` or `.` and starts with an alphanumeric. Nothing else,
/// including e-mail addresses.
pub fn code_owner(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('@') else {
        return false;
    };
    match rest.split_once('/') {
        None => github_login(rest),
        Some((org, team)) => {
            github_login(org)
                && team.len() <= 100
                // A slug starts with an alphanumeric, so `.`, `..` and `-x` are refused.
                && team.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
                && team
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
        }
    }
}

/// Use the workspace-root manifest when configuration omits `manifest`.
fn manifest_default() -> String {
    "Cargo.toml".into()
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Deliverable {
    #[schemars(regex(pattern = "^(?!.*--)[A-Za-z0-9][A-Za-z0-9_-]{0,99}$(?![\\s\\S])"))]
    pub id: String,
    pub profile: Profile,
    #[schemars(regex(pattern = "^[A-Za-z0-9][A-Za-z0-9_-]{0,99}$(?![\\s\\S])"))]
    pub package: String,
    #[schemars(regex(pattern = "^[A-Za-z0-9][A-Za-z0-9_-]{0,99}$(?![\\s\\S])"))]
    pub binary: Option<String>,
    #[schemars(extend("minItems" = 1, "maxItems" = TARGETS.len(), "uniqueItems" = true, "items" = serde_json::json!({"enum": crate::config::TARGETS})))]
    pub targets: Vec<String>,
    #[schemars(regex(pattern = "^(?!.*--)[A-Za-z0-9][A-Za-z0-9_-]{0,99}$(?![\\s\\S])"))]
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
    #[schemars(length(max = 128))]
    #[schemars(extend("uniqueItems" = true))]
    #[schemars(inner(regex(pattern = "^[A-Za-z0-9][A-Za-z0-9_+.-]{0,99}$(?![\\s\\S])")))]
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    #[schemars(length(min = 1))]
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
    #[schemars(transform = crate::schema_bounds::unsigned)]
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub config_sha256: String,
    #[schemars(regex(
        pattern = "^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$(?![\\s\\S])"
    ))]
    pub runtime_version: String,
    pub workflows: WorkflowPin,
    #[schemars(extend("minProperties" = 1, "propertyNames" = serde_json::json!({"pattern":"^[A-Za-z0-9][A-Za-z0-9_-]{0,99}$(?![\\s\\S])"})))]
    pub tools: BTreeMap<String, ToolPin>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowPin {
    #[schemars(regex(
        pattern = "^[A-Za-z0-9][A-Za-z0-9_.-]{0,99}/[A-Za-z0-9][A-Za-z0-9_.-]{0,99}$(?![\\s\\S])"
    ))]
    pub repository: String,
    #[schemars(regex(pattern = "^[0-9a-f]{40}$(?![\\s\\S])"))]
    pub commit: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToolPin {
    #[schemars(regex(
        pattern = "^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$(?![\\s\\S])"
    ))]
    pub version: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
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
pub(crate) fn exact_version(value: &str) -> bool {
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
    let bytes = read_managed(root, "armorer.toml")?;
    let config = parse_config(&bytes, root)?;
    Ok((config, bytes))
}

/// Parse and validate exact `armorer.toml` bytes exactly as [`load_config`] does.
///
/// # Errors
/// Returns `Error::Toml`/`Error::TomlAt` for invalid UTF-8, malformed TOML or a
/// schema mismatch and propagates [`Config::validate`] errors.
pub(crate) fn parse_config(bytes: &[u8], root: &Path) -> Result<Config> {
    let text = std::str::from_utf8(bytes).map_err(|_| Error::Toml)?;
    let config: Config = toml::from_str(text).map_err(toml_error)?;
    config.validate(root)?;
    Ok(config)
}

/// Preserve the failure offset without echoing any untrusted config value.
fn toml_error(error: toml::de::Error) -> Error {
    match error.span() {
        Some(span) => Error::TomlAt { offset: span.start },
        None => Error::Toml,
    }
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
            // Composite asset keys join with `--`; an embedded `--` makes distinct selections collide.
            require(!name.contains("--"), "feature-set name cannot contain --")?;
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
            require(!d.id.contains("--"), "deliverable id cannot contain --")?;
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
        self.repository_files.validate()
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
    // Held-handle read: a symlink anywhere, dangling or not, is an error, never a silent "no lock".
    let Some(bytes) = optional_bytes(root, "armorer.lock")? else {
        return Ok(None);
    };
    let lock: Lock = toml::from_str(std::str::from_utf8(&bytes).map_err(|_| Error::Toml)?)
        .map_err(toml_error)?;
    require(
        lock.schema_version == VERSION,
        "unsupported lock schema_version",
    )?;
    require(
        hex_digest(&lock.config_sha256, 64) && lock.config_sha256 == config_digest,
        "armorer.lock does not match the exact config bytes",
    )?;
    require(
        crate::runtime::compatible(&lock.runtime_version),
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

#[cfg(test)]
mod tests {
    use super::{RepositoryFiles, code_owner};

    #[test]
    /// GitHub's two owner forms are accepted exactly; e-mail and path forms never are.
    fn code_owner_forms_follow_github_rules() {
        let login = format!("@a{}", "b".repeat(38));
        let team = format!("@org/{}", "t".repeat(100));
        for good in [
            "@a",
            "@a-b",
            "@A1",
            &login,
            "@org/team",
            "@org/a.b_c-d",
            &team,
        ] {
            assert!(code_owner(good), "{good}");
        }
        let long_login = format!("@a{}", "b".repeat(39));
        let long_team = format!("@org/{}", "t".repeat(101));
        for bad in [
            "",
            "@",
            "a",
            "@-a",
            "@a-",
            "@a--b",
            "@a_b",
            "@a b",
            &long_login,
            "@org/",
            "@/team",
            "@org/.",
            "@org/..",
            "@org/.hidden",
            "@org/-team",
            "@org/_team",
            "@org/team/x",
            "@org/te am",
            &long_team,
            "@-org/team",
            "a@example.com",
            "@a\n",
            "/.github/",
        ] {
            assert!(!code_owner(bad), "{bad:?}");
        }
    }

    #[test]
    /// Every switch is off by default; at most 16 owners, unique ignoring ASCII case.
    fn repository_files_defaults_and_owner_bounds() {
        let files = RepositoryFiles::default();
        assert!(files.is_default());
        files.validate().unwrap();
        let mut owners = RepositoryFiles {
            code_owners: (0..17).map(|i| format!("@o{i}")).collect(),
            ..RepositoryFiles::default()
        };
        assert!(owners.validate().is_err());
        owners.code_owners.truncate(16);
        owners.validate().unwrap();
        owners.code_owners[1] = owners.code_owners[0].to_ascii_uppercase();
        assert!(owners.validate().is_err());
    }

    #[test]
    /// A prerelease crate version would make the x.y.z lock schema unsatisfiable for every input.
    fn crate_version_matches_lock_schema_pattern() {
        assert!(super::exact_version(env!("CARGO_PKG_VERSION")));
    }
}
