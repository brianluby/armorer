use armorer::ci_policy::{parse_at, utc_day};
const POLICY: &str = "schema_version = 1\n[licenses]\nallow = [\"MIT\", \"Apache-2.0\"]\n[advisories]\nexceptions = []\n[sources]\nallow_git = []\n[bans]\nmultiple_versions = \"deny\"\ndeny = []\n";
fn with_exception(date: &str) -> String {
    POLICY.replace("exceptions = []", &format!("exceptions = [{{ id = \"RUSTSEC-2026-0001\", owner = \"security-maintainer\", reason = \"Reviewed and tracked\", expires = \"{date}\" }}]"))
}
#[test]
fn explicit_complete_policy_has_no_permissive_defaults_or_extra_fields() {
    let today = utc_day("2026-10-02").unwrap();
    assert!(parse_at(POLICY.as_bytes(), today).is_ok());
    for invalid in [
        POLICY.replace("allow = [\"MIT\", \"Apache-2.0\"]", "allow = []"),
        POLICY.replace("[sources]\nallow_git = []\n", ""),
        POLICY.replace("schema_version = 1", "schema_version = 2"),
        POLICY.replace(
            "schema_version = 1",
            "schema_version = 1\nshell = \"DO-NOT-ECHO\"",
        ),
        POLICY.replace(
            "multiple_versions = \"deny\"",
            "multiple_versions = \"allow\"",
        ),
        POLICY.replace("Apache-2.0", "MIT"),
        POLICY.replace("MIT", "MIT; execute"),
    ] {
        let error = parse_at(invalid.as_bytes(), today).unwrap_err();
        assert!(!error.to_string().contains("DO-NOT-ECHO"));
    }
}
#[test]
fn advisory_exceptions_enforce_exact_ids_owner_reason_and_current_utc_expiry() {
    let today = utc_day("2026-10-02").unwrap();
    for date in ["2026-10-02", "2026-12-31"] {
        assert!(
            parse_at(with_exception(date).as_bytes(), today).is_ok(),
            "{date}"
        );
    }
    for date in [
        "2026-10-01",
        "2027-01-01",
        "2026-2-03",
        "2026-02-30",
        "2026-10-02T00:00:00Z",
    ] {
        assert!(
            parse_at(with_exception(date).as_bytes(), today).is_err(),
            "{date}"
        );
    }
    for bad in [
        with_exception("2026-10-02").replace("RUSTSEC-2026-0001", "RUSTSEC-*"),
        with_exception("2026-10-02").replace("security-maintainer", " "),
        with_exception("2026-10-02").replace("Reviewed and tracked", ""),
    ] {
        assert!(parse_at(bad.as_bytes(), today).is_err());
    }
}
#[test]
fn date_validation_handles_gregorian_centuries_and_boundaries() {
    assert_eq!(utc_day("1970-01-01").unwrap(), 0);
    assert_eq!(utc_day("2000-02-29").unwrap(), 11016);
    assert_eq!(utc_day("2026-10-02").unwrap(), 20728);
    assert!(utc_day("2100-02-29").is_err());
    assert!(utc_day("2000-02-29").is_ok());
    for date in [
        "1969-12-31",
        "2026-13-01",
        "2026-00-01",
        "2026-01-00",
        "2026-04-31",
        "２０２６-01-01",
    ] {
        assert!(utc_day(date).is_err());
    }
}
#[test]
fn source_and_ban_injection_unknown_keys_and_duplicates_are_rejected() {
    let today = utc_day("2026-10-02").unwrap();
    let good = POLICY
        .replace(
            "allow_git = []",
            "allow_git = [\"https://github.com/example/project.git\"]",
        )
        .replace("deny = []", "deny = [\"unsafe-crate\"]");
    assert!(parse_at(good.as_bytes(), today).is_ok());
    for bad in [
        good.replace(
            "https://github.com/example/project.git",
            "https://example.invalid/source",
        ),
        good.replace(
            "https://github.com/example/project.git",
            "https://github.com/example/project.git?command=bad",
        ),
        good.replace("unsafe-crate", "../../outside"),
        good.replace("unsafe-crate\"]", "unsafe-crate\", \"unsafe-crate\"]"),
        good.replace("[bans]", "[bans]\ncustom_hook = \"DO-NOT-ECHO\""),
    ] {
        assert!(parse_at(bad.as_bytes(), today).is_err());
    }
}
