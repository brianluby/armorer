use armorer::{config::Config, init};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::Path,
    process::{Command, Output},
};

const INERT: &str = "compile_error!(\"must not build\");";
const BUILD_TRAP: &str =
    "fn main() { std::fs::write(\"BUILD_EXECUTED\", \"bad\").unwrap(); panic!(\"no builds\"); }";
const TOOLCHAIN: &str = "1.95.0";

/// Create a temporary repository whose sources fail to compile and whose build script
/// leaves a marker, so any build or execution would be detected.
fn repository(files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    for (path, content) in files {
        let path = root.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    root
}

fn package(name: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n")
}

/// Single binary crate `pilot` with a license file.
fn single_binary() -> tempfile::TempDir {
    repository(&[
        ("Cargo.toml", &package("pilot")),
        ("src/main.rs", INERT),
        ("build.rs", BUILD_TRAP),
        ("LICENSE", "MIT\n"),
    ])
}

/// Capture every entry's exact bytes without following symlinks.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            let kind = fs::symlink_metadata(&path).unwrap().file_type();
            if kind.is_symlink() {
                files.insert(
                    relative,
                    format!("symlink:{}", fs::read_link(&path).unwrap().display()).into_bytes(),
                );
            } else if kind.is_dir() {
                files.insert(format!("{relative}/"), b"directory".to_vec());
                walk(root, &path, files);
            } else {
                files.insert(relative, fs::read(path).unwrap());
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

/// Nothing was built: no marker and no target directory anywhere in the repository.
fn assert_inert(root: &Path) {
    for path in snapshot(root).keys() {
        assert!(!path.contains("BUILD_EXECUTED"), "{path}");
        assert!(
            !path.starts_with("target") && !path.contains("/target/"),
            "{path}"
        );
    }
}

fn run(root: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_armorer"))
        .arg("--repository")
        .arg(root)
        .args(arguments)
        .output()
        .unwrap()
}

/// Run the CLI, require the status and return parsed stdout JSON.
fn cli(root: &Path, arguments: &[&str], status: i32) -> Value {
    let output = run(root, arguments);
    assert_eq!(
        output.status.code(),
        Some(status),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn propose(root: &Path, repository: &str) -> Value {
    cli(
        root,
        &["init", "--repo-name", repository, "--toolchain", TOOLCHAIN],
        0,
    )
}

fn write(root: &Path, repository: &str, digest: &str, status: i32) -> Value {
    cli(
        root,
        &[
            "init",
            "--repo-name",
            repository,
            "--toolchain",
            TOOLCHAIN,
            "--write",
            "--expect-proposal-sha256",
            digest,
        ],
        status,
    )
}

fn file<'a>(proposal: &'a Value, path: &str) -> &'a str {
    proposal["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["path"] == path)
        .unwrap()["content"]
        .as_str()
        .unwrap()
}

fn proposed_config(proposal: &Value) -> Config {
    toml::from_str(file(proposal, init::CONFIG_PATH)).unwrap()
}

fn codes(proposal: &Value) -> Vec<&str> {
    proposal["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["code"].as_str().unwrap())
        .collect()
}

fn summary(config: &Config) -> Vec<(String, String, String, Option<String>)> {
    config
        .deliverables
        .iter()
        .map(|d| {
            (
                d.id.clone(),
                serde_json::to_value(d.profile)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned(),
                d.package.clone(),
                d.binary.clone(),
            )
        })
        .collect()
}

fn error(value: &Value) -> (&str, &str) {
    (
        value["error"]["code"].as_str().unwrap(),
        value["error"]["message"].as_str().unwrap(),
    )
}

#[test]
/// A binary crate yields one cli deliverable over every target with the default feature set,
/// and the read-only proposal changes no byte.
fn single_binary_crate_is_proposed_read_only() {
    let root = single_binary();
    let before = snapshot(root.path());
    let output = run(
        root.path(),
        &[
            "init",
            "--repo-name",
            "example/pilot",
            "--toolchain",
            TOOLCHAIN,
        ],
    );
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.ends_with(b"}\n"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("nothing was written"));
    let proposal: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(snapshot(root.path()), before);
    assert_inert(root.path());
    assert_eq!(proposal["contract"], "armorer-init-proposal");
    assert_eq!(proposal["schema_version"], 1);
    assert_eq!(proposal["repository"], "example/pilot");
    assert_eq!(proposal["toolchain"], TOOLCHAIN);
    let paths: Vec<_> = proposal["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, ["armorer-policy.draft.toml", "armorer.toml"]);
    assert_eq!(
        codes(&proposal),
        [
            "policy-draft-incomplete",
            "profile-review",
            "targets-review"
        ]
    );
    let config = proposed_config(&proposal);
    assert_eq!(
        summary(&config),
        [(
            "pilot".into(),
            "cli".into(),
            "pilot".into(),
            Some("pilot".into())
        )]
    );
    assert_eq!(config.deliverables[0].targets, armorer::config::TARGETS);
    assert_eq!(config.deliverables[0].feature_set, "default");
    assert!(config.feature_sets["default"].default_features);
    assert!(config.feature_sets["default"].features.is_empty());
    assert_eq!(config.policy.license_file, "LICENSE");
    assert!(file(&proposal, init::CONFIG_PATH).starts_with("# Proposed by `armorer init`"));
    // The digest is the compact typed JSON with the digest field blank.
    let typed: init::Proposal = serde_json::from_value(proposal.clone()).unwrap();
    assert_eq!(typed.digest().unwrap(), proposal["proposal_sha256"]);
    for f in &typed.files {
        assert_eq!(
            f.sha256,
            format!(
                "{:x}",
                <sha2::Sha256 as sha2::Digest>::digest(f.content.as_bytes())
            )
        );
    }
}

#[test]
/// A library-only crate yields a library deliverable; a missing license is a finding.
fn library_only_crate_without_license() {
    let root = repository(&[
        ("Cargo.toml", &package("corelib")),
        ("src/lib.rs", INERT),
        ("build.rs", BUILD_TRAP),
    ]);
    let before = snapshot(root.path());
    let proposal = propose(root.path(), "example/corelib");
    assert_eq!(snapshot(root.path()), before);
    assert_inert(root.path());
    let config = proposed_config(&proposal);
    assert_eq!(
        summary(&config),
        [("corelib".into(), "library".into(), "corelib".into(), None)]
    );
    assert_eq!(config.policy.license_file, "LICENSE");
    assert_eq!(
        codes(&proposal),
        [
            "license-file-missing",
            "policy-draft-incomplete",
            "targets-review"
        ]
    );
}

/// A workspace whose `tools` member has two bins and whose `core` member is a library.
fn two_member_workspace() -> tempfile::TempDir {
    let tools = format!(
        "{}[[bin]]\nname = \"alpha\"\npath = \"src/alpha.rs\"\n[[bin]]\nname = \"beta\"\npath = \"src/beta.rs\"\n",
        package("tools")
    );
    repository(&[
        (
            "Cargo.toml",
            "[workspace]\nresolver = \"3\"\nmembers = [\"tools\", \"core\"]\n",
        ),
        ("tools/Cargo.toml", &tools),
        ("tools/src/alpha.rs", INERT),
        ("tools/src/beta.rs", INERT),
        ("tools/build.rs", BUILD_TRAP),
        ("core/Cargo.toml", &package("core")),
        ("core/src/lib.rs", INERT),
        ("core/build.rs", BUILD_TRAP),
        ("COPYING", "license\n"),
    ])
}

#[test]
/// Members are ordered by package name, bins by target name; license candidates are searched.
fn workspace_with_two_members() {
    let root = two_member_workspace();
    let before = snapshot(root.path());
    let proposal = propose(root.path(), "example/workspace");
    assert_eq!(snapshot(root.path()), before);
    assert_inert(root.path());
    let config = proposed_config(&proposal);
    assert_eq!(
        summary(&config),
        [
            ("core".into(), "library".into(), "core".into(), None),
            (
                "alpha".into(),
                "cli".into(),
                "tools".into(),
                Some("alpha".into())
            ),
            (
                "beta".into(),
                "cli".into(),
                "tools".into(),
                Some("beta".into())
            ),
        ]
    );
    assert_eq!(config.policy.license_file, "COPYING");
    assert_eq!(
        proposal["workspace"]["packages"].as_array().unwrap().len(),
        2
    );
}

#[test]
/// A bin whose required features default features do not enable is skipped and counted, not named.
fn unmet_required_features_are_skipped() {
    let manifest = format!(
        "{}[features]\nextra = []\n[[bin]]\nname = \"pilot\"\npath = \"src/main.rs\"\n[[bin]]\nname = \"gated-tool\"\npath = \"src/gated.rs\"\nrequired-features = [\"extra\"]\n",
        package("pilot")
    );
    let root = repository(&[
        ("Cargo.toml", &manifest),
        ("src/main.rs", INERT),
        ("src/gated.rs", INERT),
        ("build.rs", BUILD_TRAP),
        ("LICENSE", "MIT\n"),
    ]);
    let proposal = propose(root.path(), "example/pilot");
    let config = proposed_config(&proposal);
    assert_eq!(
        summary(&config),
        [(
            "pilot".into(),
            "cli".into(),
            "pilot".into(),
            Some("pilot".into())
        )]
    );
    let skipped = proposal["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["code"] == "required-features-skipped")
        .unwrap();
    let detail = skipped["detail"].as_str().unwrap();
    assert!(detail.starts_with("1 binary target"));
    assert!(!detail.contains("gated"));
    // With the feature enabled by default the bin is proposed.
    fs::write(
        root.path().join("Cargo.toml"),
        manifest.replace("extra = []", "default = [\"extra\"]\nextra = []"),
    )
    .unwrap();
    let ids: Vec<_> = proposed_config(&propose(root.path(), "example/pilot"))
        .deliverables
        .into_iter()
        .map(|d| d.id)
        .collect();
    assert_eq!(ids, ["gated-tool", "pilot"]);
}

#[test]
/// Identical repository states in two locations produce byte-identical proposals.
fn proposals_are_deterministic_across_runs_and_locations() {
    let first = two_member_workspace();
    let second = two_member_workspace();
    let arguments = [
        "init",
        "--repo-name",
        "example/workspace",
        "--toolchain",
        TOOLCHAIN,
    ];
    let a = run(first.path(), &arguments);
    let b = run(first.path(), &arguments);
    let c = run(second.path(), &arguments);
    assert!(a.status.success());
    assert_eq!(a.stdout, b.stdout);
    assert_eq!(a.stdout, c.stdout);
    let text = String::from_utf8(a.stdout).unwrap();
    for location in [first.path(), second.path()] {
        let canonical = location.canonicalize().unwrap();
        assert!(!text.contains(canonical.to_str().unwrap()));
        assert!(!text.contains(location.to_str().unwrap()));
    }
}

#[test]
/// Written configuration loads with the normal loader and passes full discovery.
fn written_config_loads_and_discovers() {
    for root in [single_binary(), two_member_workspace()] {
        let proposal = init::propose(root.path(), "example/written", Some(TOOLCHAIN)).unwrap();
        let receipt = init::write(
            root.path(),
            "example/written",
            Some(TOOLCHAIN),
            &proposal.proposal_sha256,
        )
        .unwrap();
        assert_eq!(receipt.outcome, "written");
        assert!(!receipt.configured && !receipt.policy_reviewed && !receipt.ci_verified);
        for f in &proposal.files {
            assert_eq!(
                fs::read(root.path().join(&f.path)).unwrap(),
                f.content.as_bytes()
            );
        }
        let (config, _) = armorer::config::load_config(root.path()).unwrap();
        let workspace = armorer::discovery::discover(root.path(), &config).unwrap();
        assert_eq!(
            serde_json::to_value(workspace).unwrap(),
            serde_json::to_value(&proposal.workspace).unwrap()
        );
        assert_inert(root.path());
    }
}

#[test]
/// The complete CLI path: propose, write with the digest, complete the draft, plan the bootstrap.
fn init_write_then_bootstrap_plan_end_to_end() {
    let root = single_binary();
    let proposal = propose(root.path(), "example/pilot");
    let digest = proposal["proposal_sha256"].as_str().unwrap();
    let receipt = write(root.path(), "example/pilot", digest, 0);
    assert_eq!(receipt["outcome"], "written");
    assert_eq!(receipt["proposal_sha256"], digest);
    assert_eq!(receipt["configured"], false);
    assert_eq!(
        receipt["written"],
        serde_json::json!(["armorer-policy.draft.toml", "armorer.toml"])
    );
    // The draft itself is not accepted as reviewed policy.
    let draft = root.path().join(init::POLICY_DRAFT_PATH);
    let refused = cli(
        root.path(),
        &["bootstrap", "plan", "--policy", draft.to_str().unwrap()],
        1,
    );
    assert_eq!(error(&refused).0, "invalid-config");
    let mut reviewed = tempfile::NamedTempFile::new().unwrap();
    reviewed
        .write_all(
            fs::read_to_string(&draft)
                .unwrap()
                .replace("allow = []", "allow = [\"MIT\"]")
                .as_bytes(),
        )
        .unwrap();
    let plan = cli(
        root.path(),
        &[
            "bootstrap",
            "plan",
            "--policy",
            reviewed.path().to_str().unwrap(),
        ],
        0,
    );
    assert_eq!(
        plan["intent"],
        serde_json::to_value(proposed_config(&proposal)).unwrap()
    );
    assert_eq!(plan["workspace"], proposal["workspace"]);
    assert_inert(root.path());
    // A written repository is initialized.
    let again = cli(
        root.path(),
        &[
            "init",
            "--repo-name",
            "example/pilot",
            "--toolchain",
            TOOLCHAIN,
        ],
        1,
    );
    assert!(error(&again).1.contains("already initialized"));
}

#[test]
/// An existing or symlinked armorer.toml refuses without echoing values or changing files.
fn existing_or_symlinked_config_is_refused() {
    let root = single_binary();
    fs::write(root.path().join("armorer.toml"), "anything").unwrap();
    let before = snapshot(root.path());
    let value = cli(
        root.path(),
        &[
            "init",
            "--repo-name",
            "secret-owner/secret-name",
            "--toolchain",
            TOOLCHAIN,
        ],
        1,
    );
    let (code, message) = error(&value);
    assert_eq!(code, "invalid-config");
    assert!(message.contains("already initialized"));
    assert!(!message.contains("secret"));
    assert_eq!(snapshot(root.path()), before);
    #[cfg(unix)]
    {
        let root = single_binary();
        std::os::unix::fs::symlink("elsewhere.toml", root.path().join("armorer.toml")).unwrap();
        let value = cli(
            root.path(),
            &[
                "init",
                "--repo-name",
                "example/pilot",
                "--toolchain",
                TOOLCHAIN,
            ],
            1,
        );
        assert_eq!(error(&value).0, "unsafe-path");
        assert!(!root.path().join("elsewhere.toml").exists());
    }
}

#[test]
/// Without --toolchain only an exact rust-toolchain.toml channel is used; nothing is guessed.
fn toolchain_must_be_exact() {
    let root = single_binary();
    let arguments = ["init", "--repo-name", "example/pilot"];
    let value = cli(root.path(), &arguments, 1);
    let (code, message) = error(&value);
    assert_eq!(code, "invalid-config");
    assert!(message.contains("--toolchain"));
    let pin = root.path().join("rust-toolchain.toml");
    for channel in ["stable", "1.95", "nightly-2026-01-01", "1.95.0-beta.1"] {
        fs::write(&pin, format!("[toolchain]\nchannel = \"{channel}\"\n")).unwrap();
        let value = cli(root.path(), &arguments, 1);
        assert!(error(&value).1.contains("--toolchain"));
        assert!(!error(&value).1.contains(channel) || channel == "stable");
    }
    let value = cli(
        root.path(),
        &[
            "init",
            "--repo-name",
            "example/pilot",
            "--toolchain",
            "stable",
        ],
        1,
    );
    assert!(error(&value).1.contains("exact stable"));
    fs::write(&pin, "[toolchain]\nchannel = \"1.95.0\"\n").unwrap();
    let before = snapshot(root.path());
    let proposal = cli(root.path(), &arguments, 0);
    assert_eq!(proposal["toolchain"], "1.95.0");
    assert_eq!(snapshot(root.path()), before);
}

#[test]
/// An invalid repository name is refused before discovery without echoing it.
fn bad_repo_name_is_refused_without_echo() {
    let root = single_binary();
    for name in [
        "secretvalue",
        "secret/value/extra",
        "../secret",
        "secret/",
        "https://secret.invalid/x",
    ] {
        let value = cli(
            root.path(),
            &["init", "--repo-name", name, "--toolchain", TOOLCHAIN],
            1,
        );
        let (code, message) = error(&value);
        assert_eq!(code, "invalid-config");
        assert!(!message.contains("secret"), "{message}");
    }
}

#[test]
/// A malformed or wrong digest writes nothing, not even the lock directory.
fn write_with_wrong_digest_is_refused() {
    let root = single_binary();
    let before = snapshot(root.path());
    let proposal = propose(root.path(), "example/pilot");
    let digest = proposal["proposal_sha256"].as_str().unwrap();
    let wrong = format!(
        "{}{}",
        &digest[..63],
        if digest.ends_with('0') { '1' } else { '0' }
    );
    for candidate in [wrong.as_str(), "ABC", &digest.to_uppercase()] {
        let value = write(root.path(), "example/pilot", candidate, 1);
        assert_eq!(error(&value).0, "transaction");
        assert_eq!(snapshot(root.path()), before);
    }
    // The digest binds the repository name too.
    let value = write(root.path(), "example/other", digest, 1);
    assert_eq!(error(&value).0, "transaction");
    assert_eq!(snapshot(root.path()), before);
}

#[test]
/// A workspace change between review and write makes the reviewed digest stale.
fn write_after_workspace_change_is_refused() {
    let root = single_binary();
    let proposal = propose(root.path(), "example/pilot");
    fs::create_dir_all(root.path().join("src/bin")).unwrap();
    fs::write(root.path().join("src/bin/extra.rs"), INERT).unwrap();
    let before = snapshot(root.path());
    let value = write(
        root.path(),
        "example/pilot",
        proposal["proposal_sha256"].as_str().unwrap(),
        1,
    );
    assert_eq!(error(&value).0, "transaction");
    assert!(error(&value).1.contains("fresh proposal"));
    assert_eq!(snapshot(root.path()), before);
    let fresh = proposed_config(&propose(root.path(), "example/pilot"));
    assert_eq!(fresh.deliverables.len(), 2);
}

#[test]
/// An existing policy draft is never overwritten and armorer.toml is then not created.
fn write_never_overwrites_an_existing_draft() {
    let root = single_binary();
    fs::write(root.path().join(init::POLICY_DRAFT_PATH), "owner bytes").unwrap();
    let proposal = propose(root.path(), "example/pilot");
    assert!(codes(&proposal).contains(&"policy-draft-exists"));
    let before = snapshot(root.path());
    let value = write(
        root.path(),
        "example/pilot",
        proposal["proposal_sha256"].as_str().unwrap(),
        1,
    );
    assert_eq!(error(&value).0, "transaction");
    assert_eq!(snapshot(root.path()), before);
    assert_eq!(
        fs::read(root.path().join(init::POLICY_DRAFT_PATH)).unwrap(),
        b"owner bytes"
    );
    assert!(!root.path().join("armorer.toml").exists());
    // A draft appearing after review also changes the digest.
    let root = single_binary();
    let proposal = propose(root.path(), "example/pilot");
    fs::write(root.path().join(init::POLICY_DRAFT_PATH), "late").unwrap();
    write(
        root.path(),
        "example/pilot",
        proposal["proposal_sha256"].as_str().unwrap(),
        1,
    );
    assert_eq!(
        fs::read(root.path().join(init::POLICY_DRAFT_PATH)).unwrap(),
        b"late"
    );
    assert!(!root.path().join("armorer.toml").exists());
}

#[test]
/// A library package and another package's bin with the same name get distinct ids,
/// and the result still passes discovery.
fn id_collisions_are_suffixed_deterministically() {
    let zeta = format!(
        "{}[[bin]]\nname = \"tool\"\npath = \"src/main.rs\"\n",
        package("zeta")
    );
    let root = repository(&[
        (
            "Cargo.toml",
            "[workspace]\nresolver = \"3\"\nmembers = [\"tool\", \"zeta\"]\n",
        ),
        ("tool/Cargo.toml", &package("tool")),
        ("tool/src/lib.rs", INERT),
        ("zeta/Cargo.toml", &zeta),
        ("zeta/src/main.rs", INERT),
        ("LICENSE", "MIT\n"),
    ]);
    let proposal = propose(root.path(), "example/collide");
    let config = proposed_config(&proposal);
    assert_eq!(
        summary(&config),
        [
            ("tool".into(), "library".into(), "tool".into(), None),
            (
                "tool-2".into(),
                "cli".into(),
                "zeta".into(),
                Some("tool".into())
            ),
        ]
    );
    write(
        root.path(),
        "example/collide",
        proposal["proposal_sha256"].as_str().unwrap(),
        0,
    );
    let (config, _) = armorer::config::load_config(root.path()).unwrap();
    armorer::discovery::discover(root.path(), &config).unwrap();
}

#[test]
/// A package name that cannot become an id fails closed without a partial proposal or echo.
fn unsupported_package_name_fails_closed() {
    let root = repository(&[
        ("Cargo.toml", &package("secret--name")),
        ("src/lib.rs", INERT),
        ("LICENSE", "MIT\n"),
    ]);
    let before = snapshot(root.path());
    let value = cli(
        root.path(),
        &[
            "init",
            "--repo-name",
            "example/pilot",
            "--toolchain",
            TOOLCHAIN,
        ],
        1,
    );
    let (code, message) = error(&value);
    assert_eq!(code, "invalid-config");
    assert!(message.contains("not supported"));
    assert!(!message.contains("secret"));
    assert!(value.get("files").is_none());
    assert_eq!(snapshot(root.path()), before);
}

#[test]
/// The committed schema matches the runtime contract and the binary's output byte for byte.
fn committed_init_proposal_schema_matches_runtime() {
    let committed =
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/init-proposal-v1.json"))
            .unwrap();
    let schema: Value = serde_json::from_slice(&committed).unwrap();
    assert_eq!(
        serde_json::to_value(schemars::schema_for!(init::Proposal)).unwrap(),
        schema
    );
    let output = Command::new(env!("CARGO_BIN_EXE_armorer"))
        .args(["schema", "init-proposal"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, committed);
}
