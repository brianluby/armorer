//! ADR 0015: optional managed repository files (`SECURITY.md`, `.github/dependabot.yml`
//! and `.github/CODEOWNERS`), each switched on in `[repository_files]` and off by default.
use armorer::{
    Error, bootstrap, catalog,
    config::{Config, RepositoryFiles, load_config},
    plan::Disposition,
    upgrade::{self, merge::Decision},
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    process::{Command, Output},
};

const POLICY: &str = "ci-policy.reviewed.toml";
const WIDE: &str = "bootstrap-test-wide";
const SECURITY: &str = "SECURITY.md";
const DEPENDABOT: &str = ".github/dependabot.yml";
const CODEOWNERS: &str = ".github/CODEOWNERS";
const OPTIONAL: [&str; 3] = [CODEOWNERS, DEPENDABOT, SECURITY];
const BASE: [&str; 5] = [
    ".armorer/ci-policy.toml",
    ".github/workflows/armorer-build.yml",
    ".github/workflows/armorer-ci.yml",
    "armorer.lock",
    "rust-toolchain.toml",
];
const BOOTSTRAP_STATE: &str = ".armorer/bootstrap-state-v2.json";
const UPGRADE_STATE: &str = ".armorer/upgrade-state-v1.json";
const OWNERS: &str = "code_owners = [\"@brianluby\", \"@example-org/platform\"]\n";
const ALL: &str = "security_policy = true\ndependabot = true\ncode_owners = [\"@brianluby\", \"@example-org/platform\"]\n";
const TURNED_OFF: &str = "an owned managed file is no longer selected by the configuration; releasing or removing it is not supported yet";

/// Lowercase hex SHA-256, independent of the digest type's formatting traits.
fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The committed library example's configuration, unchanged.
fn example_config() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/bootstrap-v2/library/armorer.toml"),
    )
    .unwrap()
}

/// Rewrite `armorer.toml` as the example plus an optional `[repository_files]` body.
fn configure(root: &Path, table: &str) {
    let mut text = example_config();
    if !table.is_empty() {
        text.push_str("\n[repository_files]\n");
        text.push_str(table);
    }
    fs::write(root.join("armorer.toml"), text).unwrap();
}

/// Copy the committed library profile into isolated storage, with a build canary.
fn fixture(table: &str) -> tempfile::TempDir {
    fn copy(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for e in fs::read_dir(from).unwrap() {
            let e = e.unwrap();
            let dest = to.join(e.file_name());
            if e.path().is_dir() {
                copy(&e.path(), &dest);
            } else {
                fs::copy(e.path(), dest).unwrap();
            }
        }
    }
    let root = tempfile::tempdir().unwrap();
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/bootstrap-v2/library"),
        root.path(),
    );
    fs::write(
        root.path().join("build.rs"),
        "fn main(){std::fs::write(\"BUILD_EXECUTED\",\"bad\").unwrap();panic!(\"no builds\");}",
    )
    .unwrap();
    configure(root.path(), table);
    root
}

/// Every regular file as exact bytes, ignoring the persistent kernel lock.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, map: &mut BTreeMap<String, Vec<u8>>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(root, &p, map);
            } else {
                map.insert(
                    p.strip_prefix(root).unwrap().to_str().unwrap().into(),
                    fs::read(p).unwrap(),
                );
            }
        }
    }
    let mut map = BTreeMap::new();
    walk(root, root, &mut map);
    map.remove(".armorer/apply.lock");
    map
}

fn inspect(root: &Path) -> armorer::Result<bootstrap::Plan> {
    bootstrap::inspect(root, &root.join(POLICY))
}

/// Plan and apply a bootstrap, returning the applied plan.
fn provision(root: &Path) -> bootstrap::Plan {
    let p = inspect(root).unwrap();
    assert_eq!(
        bootstrap::apply(root, &p, &p.plan_sha256).unwrap().outcome,
        "applied"
    );
    p
}

fn preview(root: &Path, id: &str) -> armorer::Result<upgrade::Plan> {
    upgrade::inspect(root, &root.join(POLICY), id, &[], false)
}

fn run(root: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_armorer"))
        .arg("--repository")
        .arg(root)
        .args(arguments)
        .output()
        .unwrap()
}

/// Top-level `check` over the managed policy: 0 in sync, 2 on drift, 1 on error.
fn check(root: &Path) -> Output {
    run(root, &["check"])
}

/// The five unconditional paths plus `extra`, sorted.
fn expected_paths(extra: &[&str]) -> Vec<String> {
    let mut paths: Vec<String> = BASE.iter().chain(extra).map(|p| (*p).to_owned()).collect();
    paths.sort();
    paths
}

fn paths(plan: &bootstrap::Plan) -> Vec<String> {
    plan.changes.iter().map(|c| c.path.clone()).collect()
}

/// Managed keys of an ownership state document.
fn owned(root: &Path, state: &str) -> Vec<String> {
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(state)).unwrap()).unwrap();
    value["managed"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

const EXPECTED_CODEOWNERS: &str = "# Managed by Armorer from repository_files.code_owners in armorer.toml; edit that list instead of this file.\n\
# Code-owner review takes effect only when a ruleset or branch protection requires it.\n\
* @brianluby @example-org/platform\n\
/.github/ @brianluby @example-org/platform\n\
/.armorer/ @brianluby @example-org/platform\n\
/armorer.toml @brianluby @example-org/platform\n\
/armorer.lock @brianluby @example-org/platform\n\
/rust-toolchain.toml @brianluby @example-org/platform\n\
/SECURITY.md @brianluby @example-org/platform\n";

#[test]
fn default_off_is_the_five_file_plan_and_serializes_no_table() {
    let root = fixture("");
    let p = inspect(root.path()).unwrap();
    assert_eq!(paths(&p), expected_paths(&[]));
    assert_eq!(
        catalog::reviewed_v2()
            .unwrap()
            .selected_paths(&RepositoryFiles::default()),
        BASE
    );
    // The intent of a configuration without the table carries no repository_files key.
    let plan_json = serde_json::to_string(&p).unwrap();
    assert!(!plan_json.contains("repository_files"));

    let (config, _) = load_config(root.path()).unwrap();
    assert!(config.repository_files.is_default());
    let toml_text = toml::to_string(&config).unwrap();
    let json_text = serde_json::to_string(&config).unwrap();
    assert!(!toml_text.contains("repository_files"), "{toml_text}");
    assert!(!json_text.contains("repository_files"), "{json_text}");
    let reparsed: Config = toml::from_str(&toml_text).unwrap();
    assert!(reparsed.repository_files.is_default());
    let reparsed: Config = serde_json::from_str(&json_text).unwrap();
    assert!(reparsed.repository_files.is_default());

    // An explicit table with every switch off is the same configuration and serializes no table.
    for table in [
        "",
        "security_policy = false\ndependabot = false\ncode_owners = []\n",
    ] {
        let mut text = example_config();
        text.push_str("\n[repository_files]\n");
        text.push_str(table);
        fs::write(root.path().join("armorer.toml"), text).unwrap();
        let (config, _) = load_config(root.path()).unwrap();
        assert!(config.repository_files.is_default());
        assert!(
            !toml::to_string(&config)
                .unwrap()
                .contains("repository_files")
        );
        assert_eq!(paths(&inspect(root.path()).unwrap()), expected_paths(&[]));
    }
    // Unknown fields in the table are rejected.
    configure(root.path(), "security_policy = true\nfunding = true\n");
    assert!(load_config(root.path()).is_err());
}

#[test]
fn each_switch_alone_and_all_together_provision_exactly_the_selected_set() {
    for (table, extra) in [
        ("security_policy = true\n", vec![SECURITY]),
        ("dependabot = true\n", vec![DEPENDABOT]),
        (OWNERS, vec![CODEOWNERS]),
        (ALL, OPTIONAL.to_vec()),
    ] {
        let root = fixture(table);
        let p = inspect(root.path()).unwrap();
        assert_eq!(paths(&p), expected_paths(&extra), "{table}");
        assert!(
            p.changes
                .iter()
                .all(|c| c.disposition == Disposition::Create),
            "{table}"
        );
        // The saved packet reloads against its independently supplied digest.
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(file.path(), serde_json::to_vec(&p).unwrap()).unwrap();
        bootstrap::load_plan(file.path(), &p.plan_sha256).unwrap();

        provision(root.path());
        for change in &p.changes {
            assert_eq!(
                fs::read_to_string(root.path().join(&change.path)).unwrap(),
                change.proposed_content,
                "{}",
                change.path
            );
        }
        assert_eq!(owned(root.path(), BOOTSTRAP_STATE), expected_paths(&extra));
        if extra.contains(&SECURITY) {
            let text = fs::read_to_string(root.path().join(SECURITY)).unwrap();
            assert!(text.contains(
                "https://github.com/example/bootstrap-library/security/advisories/new\n"
            ));
        }
        if extra.contains(&DEPENDABOT) {
            let text = fs::read_to_string(root.path().join(DEPENDABOT)).unwrap();
            assert!(text.contains(&format!(
                "- dependency-name: {}*\n",
                catalog::WORKFLOW_REPOSITORY
            )));
            assert!(text.starts_with("# Managed by Armorer"));
        }
        if extra.contains(&CODEOWNERS) {
            assert_eq!(
                fs::read_to_string(root.path().join(CODEOWNERS)).unwrap(),
                EXPECTED_CODEOWNERS
            );
        }
        let output = check(root.path());
        assert_eq!(
            output.status.code(),
            Some(0),
            "{table}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let after = snapshot(root.path());
        assert_eq!(
            bootstrap::apply(root.path(), &p, &p.plan_sha256)
                .unwrap()
                .outcome,
            "unchanged"
        );
        assert_eq!(snapshot(root.path()), after);
        let again = inspect(root.path()).unwrap();
        assert!(again.drift().is_empty(), "{:?}", again.drift());
        assert!(!root.path().join("BUILD_EXECUTED").exists());
    }
}

#[test]
fn existing_unowned_files_conflict_or_stay_unowned() {
    let rendered = inspect(fixture(ALL).path()).unwrap();
    for path in OPTIONAL {
        // Different bytes: a conflict that blocks apply and is never overwritten.
        let root = fixture(ALL);
        let target = root.path().join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "hand-written\n").unwrap();
        let p = inspect(root.path()).unwrap();
        let change = p.changes.iter().find(|c| c.path == path).unwrap();
        assert_eq!(change.disposition, Disposition::Conflict, "{path}");
        assert!(!change.owned);
        let before = snapshot(root.path());
        assert!(bootstrap::apply(root.path(), &p, &p.plan_sha256).is_err());
        assert_eq!(snapshot(root.path()), before, "{path}");
        assert_eq!(fs::read(&target).unwrap(), b"hand-written\n");

        // Identical bytes: unchanged, and the file stays unowned.
        let root = fixture(ALL);
        let target = root.path().join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        let content = &rendered
            .changes
            .iter()
            .find(|c| c.path == path)
            .unwrap()
            .proposed_content;
        fs::write(&target, content).unwrap();
        let p = inspect(root.path()).unwrap();
        let change = p.changes.iter().find(|c| c.path == path).unwrap();
        assert_eq!(change.disposition, Disposition::Unchanged, "{path}");
        provision(root.path());
        assert!(!owned(root.path(), BOOTSTRAP_STATE).contains(&path.to_owned()));
        assert_eq!(fs::read_to_string(&target).unwrap(), *content);
    }
}

#[test]
fn invalid_owners_are_rejected_without_echoing_the_value() {
    let root = fixture("");
    let many: Vec<String> = (1..=17).map(|i| format!("\"@zzmarker{i}\"")).collect();
    let long_login = format!("\"@zzmarker{}\"", "a".repeat(32));
    let long_team = format!("\"@org/zzmarker{}\"", "a".repeat(93));
    for owners in [
        "\"zzmarker\"".to_owned(),
        "\"@zz marker\"".to_owned(),
        "\"@zzmarker-\"".to_owned(),
        "\"@-zzmarker\"".to_owned(),
        "\"@zz--marker\"".to_owned(),
        long_login,
        long_team,
        "\"@zzmarker\", \"@zzmarker\"".to_owned(),
        "\"@zzmarker\", \"@ZZMARKER\"".to_owned(),
        many.join(", "),
        "\"@zz\\nmarker\"".to_owned(),
        "\"@zzmarker\\u0007\"".to_owned(),
        "\"../zzmarker\"".to_owned(),
        "\"@org/zzmarker/x\"".to_owned(),
        "\"@org/../zzmarker\"".to_owned(),
        "\"@zzmarker/\"".to_owned(),
        "\"@/zzmarker\"".to_owned(),
        "\"zzmarker@example.com\"".to_owned(),
        "\"@zz_marker\"".to_owned(),
    ] {
        configure(root.path(), &format!("code_owners = [{owners}]\n"));
        let error = load_config(root.path()).unwrap_err();
        assert!(matches!(error, Error::Invalid(_)), "{owners}: {error}");
        let message = error.to_string();
        assert!(
            !message.to_ascii_lowercase().contains("marker"),
            "{owners}: {message}"
        );
        assert!(message.contains("code_owners"), "{owners}: {message}");
        let plan = inspect(root.path()).unwrap_err();
        assert!(!plan.to_string().to_ascii_lowercase().contains("marker"));
    }
    let max_login = format!("\"@a{}\"", "b".repeat(38));
    let max_team = format!("\"@org/{}\"", "t".repeat(100));
    let sixteen: Vec<String> = (1..=16).map(|i| format!("\"@owner{i}\"")).collect();
    for owners in [
        "\"@a\"".to_owned(),
        "\"@a-b-c\"".to_owned(),
        max_login,
        "\"@example-org/plat.form_x-1\"".to_owned(),
        max_team,
        sixteen.join(", "),
    ] {
        configure(root.path(), &format!("code_owners = [{owners}]\n"));
        load_config(root.path()).unwrap_or_else(|error| panic!("{owners}: {error}"));
    }
}

/// Recompute a forged plan's digest and load it through the approval boundary.
fn load_forged(plan: &mut bootstrap::Plan) -> armorer::Result<bootstrap::Plan> {
    plan.plan_sha256 = plan.digest().unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    fs::write(file.path(), serde_json::to_vec(&*plan).unwrap()).unwrap();
    bootstrap::load_plan(file.path(), &plan.plan_sha256)
}

#[test]
fn plans_must_list_exactly_the_selected_set_for_their_intent() {
    const MISMATCH: &str = "bootstrap plan does not match approved digest, catalog or runtime";
    let on = inspect(fixture(ALL).path()).unwrap();
    let off = inspect(fixture("").path()).unwrap();
    let security = on.changes.iter().find(|c| c.path == SECURITY).unwrap();

    // Selected but missing from the change list.
    let mut forged = on.clone();
    forged.changes.retain(|c| c.path != SECURITY);
    let error = load_forged(&mut forged).unwrap_err();
    assert!(error.to_string().contains(MISMATCH), "{error}");

    // Present in the change list but not selected by the plan's intent.
    let mut forged = off.clone();
    let at = forged
        .changes
        .iter()
        .position(|c| c.path.as_str() > SECURITY)
        .unwrap();
    forged.changes.insert(at, security.clone());
    let error = load_forged(&mut forged).unwrap_err();
    assert!(error.to_string().contains(MISMATCH), "{error}");

    // The intent switched off while the optional file is still listed.
    let mut forged = on.clone();
    forged.intent.repository_files.security_policy = false;
    assert!(load_forged(&mut forged).is_err());

    // The intent switched on while the optional file is not listed.
    let mut forged = off.clone();
    forged.intent.repository_files.security_policy = true;
    assert!(load_forged(&mut forged).is_err());

    // The same count with one optional file swapped for another.
    let mut forged = on.clone();
    forged.intent.repository_files.dependabot = false;
    forged.intent.repository_files.security_policy = true;
    forged.changes.retain(|c| c.path != SECURITY);
    let error = load_forged(&mut forged).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("unsupported bootstrap content, preimage or target"),
        "{error}"
    );

    // Genuine plans reload.
    for mut genuine in [on, off] {
        load_forged(&mut genuine).unwrap();
    }
}

#[test]
fn turning_a_switch_off_after_bootstrap_ownership_fails_closed() {
    let root = fixture(ALL);
    provision(root.path());
    for table in [
        "dependabot = true\ncode_owners = [\"@brianluby\", \"@example-org/platform\"]\n",
        "security_policy = true\ncode_owners = [\"@brianluby\", \"@example-org/platform\"]\n",
        "security_policy = true\ndependabot = true\n",
        "",
    ] {
        configure(root.path(), table);
        let before = snapshot(root.path());
        let error = inspect(root.path()).unwrap_err();
        assert!(matches!(error, Error::Transaction(_)), "{error}");
        assert!(error.to_string().contains(TURNED_OFF), "{error}");
        let output = check(root.path());
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stdout).contains(TURNED_OFF));
        let bootstrap_cli = run(
            root.path(),
            &[
                "bootstrap",
                "check",
                "--policy",
                root.path().join(POLICY).to_str().unwrap(),
            ],
        );
        assert_eq!(bootstrap_cli.status.code(), Some(1));
        // The upgrade path shows the owned file as a blocking conflict and deletes nothing.
        let up = preview(root.path(), "bootstrap-v2").unwrap();
        let blocked: Vec<_> = up
            .changes
            .iter()
            .filter(|c| c.decision == Decision::Conflict)
            .collect();
        assert!(!blocked.is_empty());
        for c in &blocked {
            assert!(OPTIONAL.contains(&c.path.as_str()), "{}", c.path);
            assert!(c.proposed_content.is_none() && c.candidate_content.is_empty());
            assert!(c.before_content.is_some());
        }
        assert!(upgrade::apply(root.path(), &up, &up.plan_sha256).is_err());
        assert_eq!(snapshot(root.path()), before, "{table}");
        for path in OPTIONAL {
            assert!(root.path().join(path).exists(), "{path}");
        }
    }
    // Switching it back on restores a clean, in-sync plan.
    configure(root.path(), ALL);
    assert!(inspect(root.path()).unwrap().drift().is_empty());
}

#[test]
fn turning_a_switch_off_after_upgrade_ownership_fails_closed() {
    let root = fixture(ALL);
    provision(root.path());
    let p = preview(root.path(), "bootstrap-v2").unwrap();
    upgrade::apply(root.path(), &p, &p.plan_sha256).unwrap();
    assert!(!root.path().join(BOOTSTRAP_STATE).exists());
    configure(root.path(), "dependabot = true\n");
    let before = snapshot(root.path());
    // Read-only bootstrap inspection follows the upgrade owner and refuses as well.
    let error = inspect(root.path()).unwrap_err();
    assert!(error.to_string().contains(TURNED_OFF), "{error}");
    assert_eq!(check(root.path()).status.code(), Some(1));
    let up = preview(root.path(), "bootstrap-v2").unwrap();
    let conflicts: Vec<_> = up
        .changes
        .iter()
        .filter(|c| c.decision == Decision::Conflict)
        .map(|c| c.path.as_str())
        .collect();
    assert_eq!(conflicts, [CODEOWNERS, SECURITY]);
    assert!(
        up.compatibility
            .iter()
            .any(|c| c.state == "blocked" && c.code == "three-way-conflict")
    );
    assert!(upgrade::apply(root.path(), &up, &up.plan_sha256).is_err());
    assert_eq!(snapshot(root.path()), before);
}

/// One forged edit of an upgrade ownership state document.
type Mutation = Box<dyn Fn(&mut serde_json::Value)>;

#[test]
fn upgrade_state_records_and_checks_the_optional_inputs() {
    // Without optional files the upgrade state carries neither input.
    let plain = fixture("");
    provision(plain.path());
    let p = preview(plain.path(), "bootstrap-v2").unwrap();
    upgrade::apply(plain.path(), &p, &p.plan_sha256).unwrap();
    let text = fs::read_to_string(plain.path().join(UPGRADE_STATE)).unwrap();
    assert!(!text.contains("\"repository\"") && !text.contains("repository_files"));

    // With them, bootstrap v2 then a real upgrade to the wider catalog.
    let root = fixture(ALL);
    provision(root.path());
    let p = preview(root.path(), WIDE).unwrap();
    assert_eq!(p.direction, "upgrade");
    assert!(
        p.changes
            .iter()
            .filter(|c| OPTIONAL.contains(&c.path.as_str()))
            .all(|c| c.decision == Decision::Unchanged)
    );
    upgrade::apply(root.path(), &p, &p.plan_sha256).unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(root.path().join(UPGRADE_STATE)).unwrap()).unwrap();
    assert_eq!(value["repository"], "example/bootstrap-library");
    assert_eq!(value["repository_files"]["security_policy"], true);
    assert_eq!(value["repository_files"]["dependabot"], true);
    assert_eq!(
        value["repository_files"]["code_owners"],
        serde_json::json!(["@brianluby", "@example-org/platform"])
    );
    assert_eq!(owned(root.path(), UPGRADE_STATE).len(), 9);
    // The state validates: a refresh previews cleanly and check is in sync.
    let refresh = preview(root.path(), WIDE).unwrap();
    assert!(
        refresh
            .changes
            .iter()
            .all(|c| c.decision == Decision::Unchanged)
    );
    assert_eq!(check(root.path()).status.code(), Some(0));

    let state = root.path().join(UPGRADE_STATE);
    let original = value;
    let other =
        "# Security policy\n\nReport to https://github.com/other/repo/security/advisories/new\n";
    let mutations: Vec<Mutation> = vec![
        // A recorded base that differs from the bytes re-derived from the recorded inputs.
        Box::new(move |v| {
            let image = &mut v["managed"][SECURITY];
            image["generated"] = other.into();
            image["generated_sha256"] = sha256_hex(other.as_bytes()).into();
            image["applied"] = other.into();
            image["applied_sha256"] = sha256_hex(other.as_bytes()).into();
        }),
        // Recorded inputs that no longer derive the recorded bases.
        Box::new(move |v| v["repository"] = "other/repo".into()),
        Box::new(move |v| v["repository_files"]["code_owners"] = serde_json::json!(["@someone"])),
        // Optional files listed without the inputs.
        Box::new(move |v| {
            let object = v.as_object_mut().unwrap();
            object.remove("repository");
            object.remove("repository_files");
        }),
        // Inputs recorded only partly.
        Box::new(move |v| {
            v.as_object_mut().unwrap().remove("repository");
        }),
        Box::new(move |v| {
            v.as_object_mut().unwrap().remove("repository_files");
        }),
        // Invalid recorded inputs.
        Box::new(move |v| v["repository"] = "../escape".into()),
        Box::new(move |v| v["repository_files"]["code_owners"] = serde_json::json!(["owner"])),
        Box::new(move |v| v["repository_files"]["unknown"] = true.into()),
        // A customized approved image of an optional file.
        Box::new(move |v| {
            let custom = format!(
                "{}# customized\n",
                v["managed"][CODEOWNERS]["generated"].as_str().unwrap()
            );
            v["managed"][CODEOWNERS]["applied"] = custom.clone().into();
            v["managed"][CODEOWNERS]["applied_sha256"] = sha256_hex(custom.as_bytes()).into();
        }),
    ];
    for (index, mutate) in mutations.iter().enumerate() {
        let mut value = original.clone();
        mutate(&mut value);
        fs::write(&state, serde_json::to_vec(&value).unwrap()).unwrap();
        let before = snapshot(root.path());
        assert!(preview(root.path(), WIDE).is_err(), "mutation {index}");
        assert!(inspect(root.path()).is_err(), "mutation {index}");
        assert_eq!(snapshot(root.path()), before, "mutation {index}");
    }

    // A state without optional files that gains inputs it does not need is rejected too.
    let path = plain.path().join(UPGRADE_STATE);
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["repository"] = "example/bootstrap-library".into();
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(preview(plain.path(), "bootstrap-v2").is_err());
}

#[test]
fn bootstrap_state_bases_for_optional_files_must_have_their_rendered_shape() {
    let root = fixture(ALL);
    provision(root.path());
    let path = root.path().join(BOOTSTRAP_STATE);
    let original: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    // The genuine state migrates into upgrade ownership.
    assert!(preview(root.path(), "bootstrap-v2").is_ok());
    for optional in OPTIONAL {
        let mut value = original.clone();
        let forged = "forged base\n";
        value["managed"][optional] =
            serde_json::json!({ "content": forged, "sha256": sha256_hex(forged.as_bytes()) });
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        let before = snapshot(root.path());
        let error = preview(root.path(), "bootstrap-v2").unwrap_err();
        assert!(
            error
                .to_string()
                .contains("generated base contradicts independently reviewed catalog"),
            "{optional}: {error}"
        );
        assert_eq!(snapshot(root.path()), before);
    }
}

#[test]
fn optional_files_cannot_be_imported_and_guard_the_license_path() {
    let root = fixture(ALL);
    for path in OPTIONAL {
        let error = upgrade::inspect(
            root.path(),
            &root.path().join(POLICY),
            "bootstrap-v2",
            &[path.to_owned()],
            false,
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("import decisions must be unique paths"),
            "{path}: {error}"
        );
    }
    // Every compiled optional path is a generated target the license file may not alias.
    for path in OPTIONAL {
        assert!(catalog::all_managed_paths().contains(&path));
        let text = example_config().replace(
            "license_file = \"LICENSE\"",
            &format!("license_file = \"{path}\""),
        );
        fs::write(root.path().join("armorer.toml"), text).unwrap();
        let error = inspect(root.path()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("license input overlaps bootstrap control or generated files"),
            "{path}: {error}"
        );
        assert!(preview(root.path(), "bootstrap-v2").is_err());
    }
}
