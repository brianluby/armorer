use armorer::verification::bundle::load_one;
use std::{fs, path::Path};

#[test]
/// Oversized encoded input must be rejected before the decoder inspects or allocates its payload.
fn oversized_encoded_payload_is_rejected_before_base64_decoding() {
    let mut bundle: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture()).unwrap()).unwrap();
    bundle["dsseEnvelope"]["payload"] = "!"
        .repeat((17_usize * 1024 * 1024).div_ceil(3) * 4 + 4)
        .into();
    let file = tempfile::NamedTempFile::new().unwrap();
    fs::write(file.path(), serde_json::to_vec(&bundle).unwrap()).unwrap();
    assert!(
        matches!(load_one(file.path()).unwrap_err(), armorer::Error::Invalid(code) if code == "attestation-payload-size-limit")
    );
}

#[test]
/// Preserve a legitimate exact-limit statement and reject a decoded overrun sharing its encoded length.
fn payload_size_boundary_preserves_valid_base64_and_checks_decoded_size() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let mut bundle: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture()).unwrap()).unwrap();
    for extra in [0, 1] {
        let payload = format!(
            "{{\"padding\":\"{}\"}}",
            "x".repeat(17 * 1024 * 1024 - 14 + extra)
        );
        assert_eq!(payload.len(), 17 * 1024 * 1024 + extra);
        bundle["dsseEnvelope"]["payload"] = STANDARD.encode(payload).into();
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), serde_json::to_vec(&bundle).unwrap()).unwrap();
        if extra == 0 {
            load_one(file.path()).unwrap();
        } else {
            assert!(
                matches!(load_one(file.path()).unwrap_err(), armorer::Error::Invalid(code) if code == "attestation-payload-size-limit")
            );
        }
    }
}
/// Locate the genuine bare upstream Sigstore bundle without executing its artifact.
fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/sigstore/reusable-workflow-attestation.sigstore.json")
}
#[test]
/// Keep original transport bytes while exposing no signature-authentication result.
fn genuine_transport_is_retained_exactly_and_never_produces_a_verified_claim() {
    let path = fixture();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(load_one(&path).unwrap().bytes(), bytes);
}
#[test]
/// Reject inputs that could otherwise conceal or discard offered evidence.
fn multiple_entries_unknown_fields_and_nested_duplicate_keys_cannot_select_a_verified_subset() {
    let bytes = fs::read_to_string(fixture()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&bytes).unwrap();
    let mut unknown = value.clone();
    unknown["verified"] = true.into();
    let mut duplicate_sig = value.clone();
    duplicate_sig["dsseEnvelope"]["signatures"]
        .as_array_mut()
        .unwrap()
        .push(value["dsseEnvelope"]["signatures"][0].clone());
    for content in [
        format!("[{bytes}]"),
        format!("{bytes}\n{bytes}"),
        serde_json::to_string(&unknown).unwrap(),
        serde_json::to_string(&duplicate_sig).unwrap(),
        serde_json::to_string(&value).unwrap().replacen(
            "\"verificationMaterial\":{",
            "\"verificationMaterial\":{\"duplicate\":1,\"duplicate\":2,",
            1,
        ),
    ] {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), content).unwrap();
        assert!(load_one(file.path()).is_err());
    }
}

#[test]
/// Reject ambiguity within signed bytes and explicit download wrappers before invoking gh.
fn signed_payload_duplicates_and_download_ambiguity_are_rejected_before_verification() {
    use armorer::verification::bundle::load_download;
    use base64::{Engine, engine::general_purpose::STANDARD};
    let original: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture()).unwrap()).unwrap();
    let mut duplicated_payload = original.clone();
    let payload = String::from_utf8(
        STANDARD
            .decode(original["dsseEnvelope"]["payload"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    let changed = payload.replacen("{", "{\"duplicate\":1,\"duplicate\":2,", 1);
    duplicated_payload["dsseEnvelope"]["payload"] = STANDARD.encode(changed).into();
    let file = tempfile::NamedTempFile::new().unwrap();
    fs::write(
        file.path(),
        serde_json::to_vec(&duplicated_payload).unwrap(),
    )
    .unwrap();
    assert!(load_one(file.path()).is_err());
    let wrapper = serde_json::json!({"bundle":original,"bundle_url":"https://untrusted.invalid/metadata-only","initiator":"metadata-only"});
    fs::write(file.path(), serde_json::to_vec(&wrapper).unwrap()).unwrap();
    assert_eq!(load_download(file.path()).unwrap().len(), 1);
    for content in [
        format!("{}\n{}", wrapper, wrapper),
        format!("[{}]", wrapper),
        "".into(),
        serde_json::json!({"bundle":original,"verified":true}).to_string(),
    ] {
        fs::write(file.path(), content).unwrap();
        assert!(load_download(file.path()).is_err());
    }
}
