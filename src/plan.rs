//! Shared plan vocabulary. The lowercase wire strings are identity-bearing:
//! plan digests and stored packets depend on them, so variants must never be renamed.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Typed file outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Disposition {
    Create,
    Unchanged,
    Update,
    Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub code: String,
    pub detail: String,
}
