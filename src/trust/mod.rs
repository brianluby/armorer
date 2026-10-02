//! Version-one release contracts. Validation checks consistency, never signatures.
//!
//! Consumers must load policy independently and authenticate platform evidence
//! through the verifier in ticket #8. Deserializing a record does not trust it.

pub mod capability;
pub mod evidence;
pub mod inventory;
pub mod native;
pub mod policy;
pub mod publication;

use crate::{
    Error, Result,
    config::{hex_digest, identifier, repository_name},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub(crate) fn require(ok: bool, code: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(code.into()))
    }
}

/// An exact byte identity, independent of a filename or producer claim.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ByteIdentity {
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub sha256: String,
    #[schemars(range(min = 1, max = 1073741824))]
    pub size: u64,
}

impl ByteIdentity {
    pub fn validate(&self) -> Result<()> {
        require(
            hex_digest(&self.sha256, 64) && (1..=1_073_741_824).contains(&self.size),
            "invalid-byte-identity",
        )
    }
    /// Hash the supplied bytes. This provides integrity comparison, not authenticity.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            sha256: crate::digest(bytes),
            size: bytes.len() as u64,
        }
    }
    pub fn matches(&self, bytes: &[u8]) -> Result<()> {
        self.validate()?;
        require(*self == Self::from_bytes(bytes), "byte-mismatch")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub repository: String,
    #[schemars(regex(pattern = "^[0-9a-f]{40}$"))]
    pub commit: String,
    pub git_ref: String,
}

impl Source {
    pub fn validate(&self) -> Result<()> {
        require(
            repository_name(&self.repository) && hex_digest(&self.commit, 40),
            "invalid-source",
        )?;
        require(valid_source_ref(&self.git_ref), "invalid-source-ref")
    }
}

pub(crate) fn release_tag(tag: &str) -> bool {
    tag.strip_prefix('v')
        .and_then(|s| semver::Version::parse(s).ok())
        .is_some_and(|v| v.pre.is_empty() && v.build.is_empty() && tag == format!("v{v}"))
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowIdentity {
    pub repository: String,
    pub path: String,
    #[schemars(regex(pattern = "^[0-9a-f]{40}$"))]
    pub commit: String,
}

impl WorkflowIdentity {
    pub fn validate(&self) -> Result<()> {
        let file = self.path.strip_prefix(".github/workflows/");
        require(
            repository_name(&self.repository)
                && hex_digest(&self.commit, 40)
                && file.is_some_and(|s| s.ends_with(".yml") && asset_name(s)),
            "invalid-workflow-identity",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunIdentity {
    #[schemars(range(min = 1))]
    pub id: u64,
    #[schemars(range(min = 1))]
    pub attempt: u64,
    pub workflow: WorkflowIdentity,
}

impl RunIdentity {
    pub fn validate(&self) -> Result<()> {
        self.workflow.validate()?;
        require(self.id > 0 && self.attempt > 0, "invalid-run-identity")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InputIdentity {
    pub source: Source,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub config_sha256: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub lock_sha256: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub cargo_lock_sha256: String,
    pub runtime: ByteIdentity,
    pub runtime_version: String,
    pub run: RunIdentity,
}

impl InputIdentity {
    pub fn validate(&self) -> Result<()> {
        self.source.validate()?;
        self.runtime.validate()?;
        self.run.validate()?;
        require(
            [
                &self.config_sha256,
                &self.lock_sha256,
                &self.cargo_lock_sha256,
            ]
            .iter()
            .all(|v| hex_digest(v, 64))
                && stable_version(&self.runtime_version),
            "invalid-input-identity",
        )
    }
}

pub(crate) fn stable_version(s: &str) -> bool {
    semver::Version::parse(s)
        .is_ok_and(|v| v.pre.is_empty() && v.build.is_empty() && s == v.to_string())
}

pub(crate) fn asset_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 255
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
}

/// Review records bind a policy decision; their contents do not prove review occurred.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub owner: String,
    pub rationale: String,
    pub reviewed_at: u64,
    pub expires_at: u64,
    pub record: ByteIdentity,
}

impl Review {
    pub fn validate_at(&self, now: u64) -> Result<()> {
        self.record.validate()?;
        require(
            identifier(&self.owner)
                && !self.rationale.trim().is_empty()
                && self.rationale.len() <= 4096
                && self.reviewed_at > 0
                && self.reviewed_at <= now
                && now < self.expires_at,
            "invalid-or-expired-review",
        )
    }
}

/// Read a strict, bounded JSON contract and return the exact bytes read once.
/// Semantic validators remain explicit; this function establishes no trust.
pub fn load_json<T: serde::de::DeserializeOwned>(
    root: &std::path::Path,
    relative: &str,
) -> Result<(T, Vec<u8>)> {
    let bytes = crate::read_small(&crate::safe_path(root, relative)?)?;
    let value = parse_json(&bytes)?;
    Ok((value, bytes))
}

/// Reject duplicate keys at every depth, including maps inside reviewed policy.
struct UniqueJson(serde_json::Value);
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueJson;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: serde::de::Error>(
                self,
                v: bool,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| UniqueJson(n.into()))
                    .ok_or_else(|| E::custom("invalid JSON number"))
            }
            fn visit_str<E: serde::de::Error>(
                self,
                v: &str,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_string<E: serde::de::Error>(
                self,
                v: String,
            ) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<Self::Value, E> {
                Ok(UniqueJson(serde_json::Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(v) = a.next_element::<UniqueJson>()? {
                    values.push(v.0);
                }
                Ok(UniqueJson(values.into()))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if values.contains_key(&k) {
                        return Err(serde::de::Error::custom("duplicate JSON key"));
                    }
                    values.insert(k, a.next_value::<UniqueJson>()?.0);
                }
                Ok(UniqueJson(values.into()))
            }
        }
        d.deserialize_any(Visitor)
    }
}

pub(crate) fn parse_json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    require(bytes.len() <= 1_048_576, "contract-exceeds-1-mib")?;
    let strict: UniqueJson = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
    serde_json::from_value(strict.0).map_err(|_| Error::Json)
}

/// Exact stable v-prefixed release tag ref, shared by every publication gate.
pub(crate) fn release_ref(value: &str) -> bool {
    value.strip_prefix("refs/tags/").is_some_and(release_tag)
}

pub(crate) fn valid_source_ref(value: &str) -> bool {
    if value.starts_with("refs/tags/") {
        return release_ref(value);
    }
    if let Some(branch) = value.strip_prefix("refs/heads/") {
        return !branch.is_empty()
            && branch.len() <= 255
            && !branch.contains("..")
            && branch
                .split('/')
                .all(|s| asset_name(s) && !s.ends_with('.') && !s.ends_with(".lock"));
    }
    if let Some(pull) = value
        .strip_prefix("refs/pull/")
        .and_then(|s| s.strip_suffix("/merge"))
    {
        return pull
            .parse::<u64>()
            .is_ok_and(|n| n > 0 && pull == n.to_string());
    }
    false
}
