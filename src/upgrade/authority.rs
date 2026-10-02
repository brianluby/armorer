//! Reviewed immutable catalog selection. No consuming document supplies roots.
use crate::{Error, Result, catalog};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Authority {
    pub id: String,
    pub revision: u32,
    pub workflow_repository: String,
    pub workflow_commit: String,
    pub tools_sha256: String,
    pub runtime_version: String,
    pub config_schema_version: u32,
    pub policy_schema_version: u32,
}

/// Stable IDs have immutable meanings. A future source update must retain this
/// entry and add a separately reviewed successor, rather than moving this ID.
pub fn select(id: &str) -> Result<Authority> {
    if id != "bootstrap-v1" {
        return Err(Error::Transaction(
            "unknown upgrade catalog; no moving pin or fallback is supported",
        ));
    }
    catalog::reviewed()?;
    Ok(Authority {
        id: id.into(),
        revision: 1,
        workflow_repository: catalog::WORKFLOW_REPOSITORY.into(),
        workflow_commit: catalog::WORKFLOW_COMMIT.into(),
        tools_sha256: catalog::TOOLS_SHA256.into(),
        runtime_version: "0.1.0".into(),
        config_schema_version: 1,
        policy_schema_version: 1,
    })
}

/// Direction is decided only after both identities belong to the compiled registry.
/// An explicit downgrade flag never makes an unknown identity trusted.
pub(super) fn direction(current: u32, target: u32, allow_downgrade: bool) -> Result<&'static str> {
    if target < current && !allow_downgrade {
        return Err(Error::Transaction(
            "reviewed downgrade requires an explicit downgrade decision",
        ));
    }
    Ok(if target < current {
        "downgrade"
    } else if target > current {
        "upgrade"
    } else {
        "refresh"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_identity_is_independent_of_claims_and_unknown_downgrades_stay_blocked() {
        let entry = select("bootstrap-v1").unwrap();
        assert_eq!(entry.workflow_commit, catalog::WORKFLOW_COMMIT);
        assert_eq!(entry.tools_sha256, catalog::TOOLS_SHA256);
        for id in [
            "main",
            "v1",
            "bootstrap-v0",
            "bootstrap-v2",
            catalog::WORKFLOW_COMMIT,
            "caller-root",
        ] {
            assert!(select(id).is_err());
        }
        assert_eq!(direction(0, 1, false).unwrap(), "upgrade");
        assert_eq!(direction(1, 1, false).unwrap(), "refresh");
        assert!(direction(2, 1, false).is_err());
        assert_eq!(direction(2, 1, true).unwrap(), "downgrade");
        // Direction alone never qualifies a catalog: these ordinal cases are
        // policy unit tests, not evidence of a second live accepted catalog.
        assert!(select("bootstrap-v0").is_err());
    }
}
