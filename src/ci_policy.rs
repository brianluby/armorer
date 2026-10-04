//! Explicit project decisions compatible with the reviewed CI policy runtime.
use crate::{Error, Result, config::identifier, read_small};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CiPolicy {
    #[schemars(transform = crate::schema_bounds::unsigned)]
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    pub licenses: Licenses,
    pub advisories: Advisories,
    pub sources: Sources,
    pub bans: Bans,
}
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Licenses {
    pub allow: Vec<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Advisories {
    pub exceptions: Vec<Exception>,
}
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Exception {
    pub id: String,
    pub owner: String,
    pub reason: String,
    pub expires: String,
}
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Sources {
    pub allow_git: Vec<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Bans {
    pub multiple_versions: DuplicatePolicy,
    pub deny: Vec<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DuplicatePolicy {
    Warn,
    Deny,
}

/// Return an invalid-policy error when the required semantic condition is false.
fn require(condition: bool) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::Invalid(
            "invalid or unsupported explicit CI policy".into(),
        ))
    }
}
/// Reject duplicate policy entries rather than silently deduplicating them.
fn distinct(values: &[String], max: usize) -> bool {
    values.len() <= max && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}
/// Validate the bounded syntax of one policy license token without claiming SPDX recognition.
fn license_token(value: &str) -> bool {
    value
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".+-".contains(&c))
}
/// Validate the supported license expression syntax before CI enforces actual dependency coverage.
fn license(value: &str) -> bool {
    let parts: Vec<_> = value.split(" WITH ").collect();
    matches!(parts.len(), 1 | 2) && parts.iter().all(|part| license_token(part))
}
/// Require the supported explicit Git source syntax and reject injected or ambiguous values.
fn git_source(value: &str) -> bool {
    let Some(path) = value.strip_prefix("https://github.com/") else {
        return false;
    };
    let parts: Vec<_> = path.split('/').collect();
    parts.len() == 2
        && parts.iter().all(|part| {
            !part.is_empty()
                && *part != "."
                && *part != ".."
                && part
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
        })
}
/// Validate a quoted canonical ISO date and convert it to UTC days since Unix epoch.
/// Bounded iteration avoids locale, timezone and external executable dependencies.
pub fn utc_day(value: &str) -> Result<u64> {
    let b = value.as_bytes();
    require(
        b.len() == 10
            && b[4] == b'-'
            && b[7] == b'-'
            && b.iter()
                .enumerate()
                .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()),
    )?;
    let year = value[..4]
        .parse::<u32>()
        .map_err(|_| Error::Invalid("invalid policy date".into()))?;
    let month = value[5..7]
        .parse::<usize>()
        .map_err(|_| Error::Invalid("invalid policy date".into()))?;
    let day = value[8..]
        .parse::<u64>()
        .map_err(|_| Error::Invalid("invalid policy date".into()))?;
    require((1970..=9999).contains(&year) && (1..=12).contains(&month))?;
    let leap = |y: u32| y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400));
    let mut months = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if leap(year) {
        months[1] = 29;
    }
    require(day > 0 && day <= months[month - 1])?;
    let years = (1970..year)
        .map(|y| if leap(y) { 366_u64 } else { 365 })
        .sum::<u64>();
    Ok(years + months[..month - 1].iter().sum::<u64>() + day - 1)
}

/// Read exact bounded policy bytes and validate them against trusted UTC clock time.
/// Syntax errors omit values; this checks license syntax, not SPDX registry membership.
/// Cargo-deny's existing enforced gate validates recognized identifiers later.
pub fn load(path: &Path) -> Result<(CiPolicy, Vec<u8>)> {
    let bytes = read_small(path)?;
    let policy = parse_at(&bytes, today()?)?;
    Ok((policy, bytes))
}

/// Read UTC time independently of consuming configuration.
pub(crate) fn today() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Invalid("UTC clock precedes epoch".into()))?
        .as_secs()
        / 86400)
}

/// Parse without defaults. `today` is an independently supplied trusted UTC day;
/// it must never come from a consuming policy or downloaded evidence record.
/// No build, dependency resolution, network or consumer mutation occurs.
pub fn parse_at(bytes: &[u8], today: u64) -> Result<CiPolicy> {
    require(bytes.len() <= 1_048_576)?;
    let policy: CiPolicy = toml::from_str(std::str::from_utf8(bytes).map_err(|_| Error::Toml)?)
        .map_err(|_| Error::Toml)?;
    require(
        policy.schema_version == 1
            && !policy.licenses.allow.is_empty()
            && distinct(&policy.licenses.allow, 128)
            && policy.licenses.allow.iter().all(|v| license(v)),
    )?;
    require(policy.advisories.exceptions.len() <= 32)?;
    let mut seen = BTreeSet::new();
    for e in &policy.advisories.exceptions {
        let b = e.id.as_bytes();
        require(
            b.len() == 17
                && b.starts_with(b"RUSTSEC-")
                && b[12] == b'-'
                && b[8..12].iter().all(u8::is_ascii_digit)
                && b[13..].iter().all(u8::is_ascii_digit)
                && seen.insert(&e.id),
        )?;
        for value in [&e.owner, &e.reason] {
            require(
                !value.trim().is_empty()
                    && value.chars().count() <= 512
                    && !value.chars().any(|c| u32::from(c) < 32),
            )?;
        }
        let expiry = utc_day(&e.expires)?;
        require(expiry >= today && expiry - today <= 90)?;
    }
    require(
        distinct(&policy.sources.allow_git, 32)
            && policy.sources.allow_git.iter().all(|v| git_source(v)),
    )?;
    require(distinct(&policy.bans.deny, 128) && policy.bans.deny.iter().all(|v| identifier(v)))?;
    Ok(policy)
}
