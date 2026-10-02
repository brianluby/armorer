use armorer::verification::bundle::load_one;
use std::{fs, path::Path};
fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/sigstore/reusable-workflow-attestation.sigstore.json")
}
#[test]
fn genuine_transport_is_retained_exactly_and_never_produces_a_verified_claim() {
    let path = fixture();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(load_one(&path).unwrap().bytes(), bytes);
}
#[test]
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
