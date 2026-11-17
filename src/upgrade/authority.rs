//! Reviewed immutable catalog selection. No consuming document supplies roots.
use crate::{Error, Result, catalog};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Authority {
    pub id: String,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub revision: u32,
    #[schemars(regex(
        pattern = "^[A-Za-z0-9][A-Za-z0-9_.-]{0,99}/[A-Za-z0-9][A-Za-z0-9_.-]{0,99}$(?![\\s\\S])"
    ))]
    pub workflow_repository: String,
    #[schemars(regex(pattern = "^[0-9a-f]{40}$(?![\\s\\S])"))]
    pub workflow_commit: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$(?![\\s\\S])"))]
    pub tools_sha256: String,
    #[schemars(regex(
        pattern = "^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$(?![\\s\\S])"
    ))]
    pub runtime_version: String,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub config_schema_version: u32,
    #[schemars(transform = crate::schema_bounds::unsigned)]
    pub policy_schema_version: u32,
}

/// Runtime identity frozen by the reviewed bootstrap catalogs.
pub(super) const BOOTSTRAP_RUNTIME_VERSION: &str = catalog::RUNTIME_VERSION;

/// Accept patch releases of the frozen runtime line. A minor or major bump is
/// rejected until a separate catalog successor has been reviewed.
fn require_bootstrap_runtime(runtime_version: &str) -> Result<()> {
    if !crate::runtime::compatible_with(BOOTSTRAP_RUNTIME_VERSION, runtime_version) {
        return Err(Error::Transaction(
            "compiled runtime requires a separately reviewed upgrade catalog successor",
        ));
    }
    Ok(())
}

/// Stable IDs have immutable meanings. A future source update must retain these
/// entries and add a separately reviewed successor, rather than moving an ID.
/// ADR 0012 applies this rule from the first tagged release; before that tag a
/// superseded entry with no adopter is removed, never reinterpreted.
pub fn select(id: &str) -> Result<Authority> {
    let revision = match id {
        #[cfg(feature = "test-predecessor")]
        "bootstrap-v1" => 1,
        "bootstrap-v2" => 2,
        _ => {
            return Err(Error::Transaction(
                "unknown upgrade catalog; no moving pin or fallback is supported",
            ));
        }
    };
    let catalog = catalog::reviewed_for(id)?;
    require_bootstrap_runtime(env!("CARGO_PKG_VERSION"))?;
    Ok(Authority {
        id: id.into(),
        revision,
        workflow_repository: catalog.workflow().repository.clone(),
        workflow_commit: catalog.workflow().commit.clone(),
        tools_sha256: catalog::TOOLS_SHA256.into(),
        runtime_version: BOOTSTRAP_RUNTIME_VERSION.into(),
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

    /// A future crate version must not silently redefine the immutable bootstrap-v1 authority.
    #[test]
    fn runtime_bump_requires_a_reviewed_catalog_successor() {
        // Patch releases keep the frozen contract and therefore the same authority.
        for runtime in ["0.1.0", "0.1.1", "0.1.42"] {
            require_bootstrap_runtime(runtime).unwrap();
        }
        for runtime in [
            "0.0.9",
            "0.2.0",
            "1.1.0",
            "0.1.0-rc.1",
            "0.1.1-rc.1",
            "0.1.0+unreviewed",
        ] {
            assert!(require_bootstrap_runtime(runtime).is_err(), "{runtime}");
        }
        let authority = select("bootstrap-v1").unwrap();
        // The recorded identity is the frozen contract, never the running patch.
        assert_eq!(authority.runtime_version, "0.1.0");
        assert!(crate::runtime::compatible(&authority.runtime_version));
    }
    /// Verify that registry identity is independent of claims and unknown downgrades stay blocked.
    #[test]
    fn registry_identity_is_independent_of_claims_and_unknown_downgrades_stay_blocked() {
        let v1 = select("bootstrap-v1").unwrap();
        assert_eq!(v1.revision, 1);
        assert_eq!(v1.workflow_commit, catalog::WORKFLOW_COMMIT);
        assert_eq!(v1.tools_sha256, catalog::TOOLS_SHA256);
        let v2 = select("bootstrap-v2").unwrap();
        assert_eq!(v2.revision, 2);
        assert_eq!(v2.workflow_commit, catalog::WORKFLOW_COMMIT_V2);
        // The successor reuses the byte-identical tool catalog; only the workflow pin moves.
        assert_eq!(v2.tools_sha256, catalog::TOOLS_SHA256);
        assert_eq!(v2.workflow_repository, catalog::WORKFLOW_REPOSITORY);
        assert_eq!(v2.runtime_version, v1.runtime_version);
        for id in [
            "main",
            "v1",
            "v2",
            "bootstrap-v0",
            "bootstrap-v3",
            catalog::WORKFLOW_COMMIT,
            catalog::WORKFLOW_COMMIT_V2,
            "caller-root",
        ] {
            assert!(select(id).is_err());
        }
        assert_eq!(direction(0, 1, false).unwrap(), "upgrade");
        assert_eq!(direction(1, 1, false).unwrap(), "refresh");
        assert_eq!(direction(1, 2, false).unwrap(), "upgrade");
        assert_eq!(direction(2, 2, false).unwrap(), "refresh");
        assert!(direction(2, 1, false).is_err());
        assert_eq!(direction(2, 1, true).unwrap(), "downgrade");
        assert!(select("bootstrap-v0").is_err());
    }
}
