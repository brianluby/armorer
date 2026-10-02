//! Unverified transport. Every supplied download record must be verified;
//! selecting only successful records would conceal offered invalid evidence.
use super::io::{MAX_BUNDLE, MAX_PAYLOAD, parse, read_bounded};
use crate::{Error, Result, trust::require};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeSet, path::Path};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Envelope {
    payload_type: String,
    payload: String,
    signatures: Vec<Signature>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Signature {
    sig: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    keyid: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Bundle {
    media_type: String,
    verification_material: Value,
    dsse_envelope: Envelope,
}

/// No authenticity claims can be constructed from this transport type.
#[derive(Debug)]
pub struct UnverifiedBundle {
    bytes: Vec<u8>,
    statement: Value,
}
impl UnverifiedBundle {
    /// Return the exact unverified bare transport bytes; this establishes no authenticity.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    /// Expose the strictly parsed signed payload only to the cryptographic adapter.
    pub(crate) fn statement(&self) -> &Value {
        &self.statement
    }
}

/// Validate bounded single-signature transport and reject ambiguity inside its signed payload.
fn bare(bytes: Vec<u8>) -> Result<UnverifiedBundle> {
    let bundle: Bundle = parse(&bytes)?;
    require(
        matches!(
            bundle.media_type.as_str(),
            "application/vnd.dev.sigstore.bundle+json;version=0.2"
                | "application/vnd.dev.sigstore.bundle.v0.3+json"
        ) && bundle.dsse_envelope.payload_type == "application/vnd.in-toto+json"
            && !bundle.dsse_envelope.payload.is_empty()
            && bundle.dsse_envelope.signatures.len() == 1
            && !bundle.dsse_envelope.signatures[0].sig.is_empty()
            && bundle.verification_material.is_object(),
        "unsupported-or-ambiguous-attestation-bundle",
    )?;
    let payload = STANDARD
        .decode(&bundle.dsse_envelope.payload)
        .map_err(|_| Error::Json)?;
    require(
        payload.len() <= MAX_PAYLOAD,
        "attestation-payload-size-limit",
    )?;
    // Reject duplicate keys in the signed payload before gh's JSON conversion
    // can erase that ambiguity. This still authenticates nothing.
    let statement: Value = parse(&payload)?;
    require(statement.is_object(), "invalid-attestation-statement")?;
    Ok(UnverifiedBundle { bytes, statement })
}

/// One exact bare Sigstore v0.2/v0.3 bundle, bounded to 64 MiB. No array or
/// JSONL subset selection; published inventory slots use this representation.
pub fn load_one(path: &Path) -> Result<UnverifiedBundle> {
    bare(read_bounded(path, MAX_BUNDLE)?)
}

/// Explicit gh-download JSONL transport, at most 32 records and 64 MiB total.
/// Bundle URLs and initiators are metadata only; they are never followed or used
/// as authority. Bare records and known gh wrappers may be mixed, but each record
/// must be verified against the independent same-slot expectations by the backend.
pub fn load_download(path: &Path) -> Result<Vec<UnverifiedBundle>> {
    let bytes = read_bounded(path, MAX_BUNDLE)?;
    let mut records = Vec::new();
    let mut seen = BTreeSet::new();
    for line in bytes
        .split(|b| *b == b'\n')
        .filter(|l| !l.iter().all(u8::is_ascii_whitespace))
    {
        require(records.len() < 32, "too-many-downloaded-bundles")?;
        let mut value: Value = parse(line)?;
        if value.get("bundle").is_some() {
            let object = value.as_object_mut().ok_or(Error::Json)?;
            require(
                object
                    .keys()
                    .all(|k| matches!(k.as_str(), "bundle" | "bundle_url" | "initiator"))
                    && ["bundle_url", "initiator"]
                        .iter()
                        .all(|k| object.get(*k).is_none_or(Value::is_string)),
                "unknown-downloaded-attestation-fields",
            )?;
            value = object.remove("bundle").ok_or(Error::Json)?;
        }
        let normalized = serde_json::to_vec(&value).map_err(|_| Error::Json)?;
        require(
            seen.insert(crate::digest(&normalized)),
            "duplicate-downloaded-bundle",
        )?;
        records.push(bare(normalized)?);
    }
    require(!records.is_empty(), "downloaded-bundle-missing")?;
    Ok(records)
}
