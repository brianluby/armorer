//! Whole-document validation with the exact native tool, not synthetic success output.
use armorer::{
    trust::ByteIdentity,
    verification::{
        cyclonedx::{OfflineSbomValidator, qualified_native_validator},
        graph::CargoGraphV2,
    },
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Locate existing shared graph/SBOM fixtures without executing any fixture bytes.
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cargo-graph-v2")
        .join(name)
}
/// Write a synthetic malformed SBOM and use its observed identity solely for this negative schema test.
fn assert_invalid(validator: &OfflineSbomValidator, bytes: &[u8]) {
    let file = tempfile::NamedTempFile::new().unwrap();
    fs::write(file.path(), bytes).unwrap();
    assert!(
        validator
            .validate(file.path(), &ByteIdentity::from_bytes(bytes))
            .is_err()
    );
}
#[test]
/// Independent approval cannot override compiled official pins, even with a caller-selected path.
fn caller_selected_executable_approval_cannot_replace_the_compiled_native_tool() {
    let file = tempfile::NamedTempFile::new().unwrap();
    fs::write(file.path(), b"#!/bin/sh\nexit 0\n").unwrap();
    assert!(
        OfflineSbomValidator::open(
            file.path(),
            &ByteIdentity::from_bytes(b"#!/bin/sh\nexit 0\n")
        )
        .is_err()
    );
    let approval = match qualified_native_validator() {
        Ok(identity) => identity,
        // This is only a negative test identity; unsupported production hosts still fail closed.
        Err(_) => ByteIdentity::from_bytes(b"unsupported-platform"),
    };
    assert!(OfflineSbomValidator::open(file.path(), &approval).is_err());
}
#[test]
#[ignore = "Required explicit native CI gate after compiled-pin distribution qualification"]
/// Run genuine whole-document validation and distinguish malformed metadata from graph consistency.
fn native_cyclonedx_validates_complete_documents_and_rejects_non_graph_field_errors() {
    let executable = PathBuf::from(
        std::env::var_os("ARMORER_TEST_CDX")
            .expect("Explicit native test requires ARMORER_TEST_CDX"),
    );
    let expected_tool = qualified_native_validator().unwrap();
    expected_tool
        .matches(&fs::read(&executable).unwrap())
        .unwrap();
    let validator = OfflineSbomValidator::open(&executable, &expected_tool).unwrap();
    for name in ["minimal", "optional", "zero-library"] {
        let path = fixture(&format!("{name}.cdx.json"));
        let bytes = fs::read(&path).unwrap();
        let proof = validator
            .validate(&path, &ByteIdentity::from_bytes(&bytes))
            .unwrap();
        assert_eq!(proof.bytes(), &ByteIdentity::from_bytes(&bytes));
        assert_eq!(proof.validator(), &expected_tool);
        let document: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(proof.document(), &document);
    }
    let bytes = fs::read(fixture("minimal.cdx.json")).unwrap();
    let graph: CargoGraphV2 =
        serde_json::from_slice(&fs::read(fixture("minimal.graph.json")).unwrap()).unwrap();
    let original: Value = serde_json::from_slice(&bytes).unwrap();
    let mut invalid = original.clone();
    invalid["metadata"]["component"]["licenses"] =
        json!([{"license":{"id":"not-an-SPDX-license"}}]);
    // Graph reconciliation intentionally doesn't validate unrelated schema fields.
    graph.compare_sbom(&invalid).unwrap();
    assert_invalid(&validator, &serde_json::to_vec(&invalid).unwrap());
    let mut invalid = original.clone();
    invalid["metadata"]["timestamp"] = json!("invalid-date-time");
    graph.compare_sbom(&invalid).unwrap();
    assert_invalid(&validator, &serde_json::to_vec(&invalid).unwrap());
    let mut invalid = original.clone();
    invalid["metadata"]["component"]["externalReferences"] =
        json!([{"type":"not-a-reference-type","url":"https://example.invalid"}]);
    graph.compare_sbom(&invalid).unwrap();
    assert_invalid(&validator, &serde_json::to_vec(&invalid).unwrap());
    let mut invalid = original.clone();
    invalid["specVersion"] = json!("1.6");
    assert_invalid(&validator, &serde_json::to_vec(&invalid).unwrap());
    assert_invalid(
        &validator,
        b"{\"bomFormat\":\"CycloneDX\",\"specVersion\":\"1.5\",\"metadata\":{\"a\":1,\"a\":2}}",
    );
    assert_invalid(&validator, b"{} {}");
    let mut wrong = ByteIdentity::from_bytes(&bytes);
    wrong.sha256 = "a".repeat(64);
    assert!(
        validator
            .validate(&fixture("minimal.cdx.json"), &wrong)
            .is_err()
    );
    #[cfg(unix)]
    {
        let directory = tempfile::tempdir().unwrap();
        let symlink = directory.path().join("sbom.json");
        std::os::unix::fs::symlink(fixture("minimal.cdx.json"), &symlink).unwrap();
        assert!(
            validator
                .validate(&symlink, &ByteIdentity::from_bytes(&bytes))
                .is_err()
        );
    }
    let file = tempfile::NamedTempFile::new().unwrap();
    fs::write(file.path(), b"wrong tool").unwrap();
    assert!(OfflineSbomValidator::open(file.path(), &expected_tool).is_err());
}
