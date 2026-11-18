//! `--format text` is an opt-in reviewer view; JSON stays the byte-exact default contract.
use serde_json::Value;
use std::{
    ffi::OsString,
    fs,
    io::Write,
    path::Path,
    process::{Command, Output},
};

const POLICY: &str = "schema_version = 1\n[licenses]\nallow = [\"MIT\"]\n[advisories]\nexceptions = []\n[sources]\nallow_git = []\n[bans]\nmultiple_versions = \"warn\"\ndeny = []\n";
const FALLBACK: &str = "armorer: no text view for this output; showing JSON\n";
const CI: &str = ".github/workflows/armorer-ci.yml";

/// A fresh repository plus the arguments of one command run against it.
struct Case {
    root: tempfile::TempDir,
    policy: tempfile::NamedTempFile,
    /// Scratch file that holds a saved JSON plan; removed with the case.
    plan: tempfile::NamedTempFile,
}

/// Same fixture shape as tests/bootstrap.rs: an inert CLI package with armorer.toml and a license.
fn case() -> Case {
    let root = tempfile::tempdir().unwrap();
    let path = root.path();
    fs::create_dir(path.join("src")).unwrap();
    fs::write(
        path.join("src/main.rs"),
        "compile_error!(\"must not build\");",
    )
    .unwrap();
    fs::write(
        path.join("Cargo.toml"),
        "[package]\nname = \"pilot\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(path.join("LICENSE"), "MIT\n").unwrap();
    fs::write(
        path.join("Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"pilot\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(path.join("armorer.toml"), "schema_version = 1\nrepository = \"example/pilot\"\ntoolchain = \"1.95.0\"\n[[deliverables]]\nid = \"primary\"\nprofile = \"cli\"\npackage = \"pilot\"\nbinary = \"pilot\"\ntargets = [\"x86_64-unknown-linux-gnu\", \"aarch64-apple-darwin\"]\nfeature_set = \"standard\"\n[feature_sets.standard]\ndefault_features = true\nfeatures = []\n[policy]\nlicense_file = \"LICENSE\"\nattestations = \"required\"\n").unwrap();
    let mut policy = tempfile::NamedTempFile::new().unwrap();
    policy.write_all(POLICY.as_bytes()).unwrap();
    Case {
        root,
        policy,
        plan: tempfile::NamedTempFile::new().unwrap(),
    }
}

/// A repository with no armorer.toml, for `init`.
fn init_case() -> Case {
    let case = case();
    fs::remove_file(case.root.path().join("armorer.toml")).unwrap();
    case
}

fn armorer(root: &Path, args: &[OsString]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_armorer"))
        .arg("--repository")
        .arg(root)
        .args(args)
        .output()
        .unwrap()
}

fn os(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

fn with(args: &[OsString], extra: &[&str]) -> Vec<OsString> {
    let mut all = args.to_vec();
    all.extend(os(extra));
    all
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

fn plan_args(case: &Case) -> Vec<OsString> {
    let mut args = os(&["bootstrap", "plan", "--policy"]);
    args.push(case.policy.path().into());
    args
}

/// Write the JSON plan to a file and apply it through the CLI.
fn apply_args(case: &Case) -> (Vec<OsString>, Value) {
    let plan = armorer(case.root.path(), &plan_args(case));
    assert!(plan.status.success());
    let digest = json(&plan)["plan_sha256"].as_str().unwrap().to_owned();
    fs::write(case.plan.path(), &plan.stdout).unwrap();
    let mut args = os(&["bootstrap", "apply", "--plan"]);
    args.push(case.plan.path().into());
    args.extend(os(&["--expect-plan-sha256", &digest]));
    (args, json(&plan))
}

/// Run the same command with no flag, `--format json` and `--format text`, each on a fresh
/// fixture. JSON must be byte-identical to the default; text must share status and stderr
/// (plus the documented fallback line). Returns (json output, text output).
fn trio(make: impl Fn() -> (Case, Vec<OsString>), fallback: bool) -> (Output, Output) {
    let run = |extra: &[&str]| {
        let (case, args) = make();
        let output = armorer(case.root.path(), &with(&args, extra));
        (output, case)
    };
    let (default, _keep_a) = run(&[]);
    let (as_json, _keep_b) = run(&["--format", "json"]);
    let (as_text, _keep_c) = run(&["--format", "text"]);
    assert_eq!(default.stdout, as_json.stdout, "json is the default format");
    assert_eq!(default.stderr, as_json.stderr);
    assert_eq!(default.status.code(), as_json.status.code());
    assert_eq!(default.status.code(), as_text.status.code());
    let mut expected = String::from_utf8(default.stderr.clone()).unwrap();
    if fallback {
        expected.push_str(FALLBACK);
    }
    assert_eq!(String::from_utf8(as_text.stderr.clone()).unwrap(), expected);
    assert!(as_text.stdout.ends_with(b"\n") && !as_text.stdout.ends_with(b"\n\n"));
    (as_json, as_text)
}

fn text_of(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

/// Every change path appears with its disposition on one line, every finding code appears.
fn assert_plan_facts(text: &str, plan: &Value) {
    assert!(text.contains(plan["plan_sha256"].as_str().unwrap()));
    assert!(text.contains(plan["catalog_id"].as_str().unwrap()));
    for change in plan["changes"].as_array().unwrap() {
        let (path, disposition) = (
            change["path"].as_str().unwrap(),
            change["disposition"].as_str().unwrap(),
        );
        assert!(
            text.lines()
                .any(|line| line.contains(path) && line.contains(disposition)),
            "{path} {disposition}\n{text}"
        );
    }
    for finding in plan["findings"].as_array().unwrap() {
        assert!(text.contains(&format!("{}: ", finding["code"].as_str().unwrap())));
    }
}

#[test]
fn bootstrap_plan_text_summarizes_the_json_plan() {
    let (as_json, as_text) = trio(
        || {
            let case = case();
            let args = plan_args(&case);
            (case, args)
        },
        false,
    );
    let plan = json(&as_json);
    let text = text_of(&as_text);
    assert_plan_facts(&text, &plan);
    assert!(text.contains("create"));
    assert!(text.contains(&format!(
        "bootstrap apply --plan <file> --expect-plan-sha256 {}",
        plan["plan_sha256"].as_str().unwrap()
    )));
    assert!(text.contains("the JSON document is what the digest covers"));
    // Every new file has a verbatim diff.
    for change in plan["changes"].as_array().unwrap() {
        assert!(text.contains(change["unified_diff"].as_str().unwrap()));
    }
}

#[test]
fn bootstrap_check_text_keeps_status_and_stderr() {
    let (as_json, as_text) = trio(
        || {
            let case = case();
            let mut args = os(&["bootstrap", "check", "--policy"]);
            args.push(case.policy.path().into());
            (case, args)
        },
        false,
    );
    assert_eq!(as_json.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&as_json.stderr).contains("provisioning drift"));
    assert_plan_facts(&text_of(&as_text), &json(&as_json));
}

#[test]
fn top_level_check_text_after_apply() {
    let (as_json, as_text) = trio(
        || {
            let case = case();
            let (apply, _) = apply_args(&case);
            assert!(armorer(case.root.path(), &apply).status.success());
            (case, os(&["check"]))
        },
        false,
    );
    assert_eq!(
        as_json.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&as_json.stderr)
    );
    let plan = json(&as_json);
    let text = text_of(&as_text);
    assert_plan_facts(&text, &plan);
    assert!(text.contains("unchanged"));
    assert!(!text.contains("Diff for"));
}

#[test]
fn init_proposal_and_write_text() {
    let proposal_args = os(&[
        "init",
        "--repo-name",
        "example/pilot",
        "--toolchain",
        "1.95.0",
    ]);
    let (as_json, as_text) = trio(|| (init_case(), proposal_args.clone()), false);
    let proposal = json(&as_json);
    let text = text_of(&as_text);
    let digest = proposal["proposal_sha256"].as_str().unwrap();
    assert!(text.contains(digest));
    assert!(text.contains("example/pilot") && text.contains("1.95.0"));
    for file in proposal["files"].as_array().unwrap() {
        assert!(text.contains(&format!("File {}:", file["path"].as_str().unwrap())));
        assert!(text.contains(file["content"].as_str().unwrap()));
    }
    for finding in proposal["findings"].as_array().unwrap() {
        assert!(text.contains(&format!("{}: ", finding["code"].as_str().unwrap())));
    }
    assert!(text.contains(&format!("--write --expect-proposal-sha256 {digest}")));

    let mut write_args = proposal_args.clone();
    write_args.extend(os(&["--write", "--expect-proposal-sha256", digest]));
    let (as_json, as_text) = trio(|| (init_case(), write_args.clone()), false);
    let receipt = json(&as_json);
    let text = text_of(&as_text);
    assert!(text.starts_with(&format!(
        "outcome: {}\n",
        receipt["outcome"].as_str().unwrap()
    )));
    assert!(text.contains(digest));
    for path in receipt["written"].as_array().unwrap() {
        assert!(text.contains(path.as_str().unwrap()));
    }
}

#[test]
fn bootstrap_apply_text_is_a_receipt() {
    let (as_json, as_text) = trio(
        || {
            let case = case();
            let (apply, _) = apply_args(&case);
            (case, apply)
        },
        false,
    );
    let receipt = json(&as_json);
    let text = text_of(&as_text);
    assert!(text.starts_with(&format!(
        "outcome: {}\n",
        receipt["outcome"].as_str().unwrap()
    )));
    assert!(text.contains(&format!(
        "plan_sha256: {}",
        receipt["plan_sha256"].as_str().unwrap()
    )));
    assert!(text.contains("configured: true"));
    assert!(text.contains("published: false"));
}

#[test]
fn catalog_text_lists_tools() {
    let (as_json, as_text) = trio(|| (case(), os(&["catalog"])), false);
    let catalog = json(&as_json);
    let text = text_of(&as_text);
    assert!(text.contains(catalog["workflow"]["repository"].as_str().unwrap()));
    assert!(text.contains(catalog["workflow"]["commit"].as_str().unwrap()));
    for (name, tool) in catalog["tools"].as_object().unwrap() {
        let version = tool["version"].as_str().unwrap();
        assert!(
            text.lines()
                .any(|l| l.starts_with(name.as_str()) && l.contains(version)),
            "{name}"
        );
        for platform in tool["platforms"].as_object().unwrap().keys() {
            assert!(text.contains(platform.as_str()));
        }
    }
}

#[test]
fn error_text_keeps_status_and_stderr() {
    let args = || {
        let mut args = os(&[
            "bootstrap",
            "apply",
            "--plan",
            "/nonexistent/armorer-plan.json",
        ]);
        args.extend(os(&["--expect-plan-sha256", &"0".repeat(64)]));
        (case(), args)
    };
    let (as_json, as_text) = trio(args, false);
    assert_eq!(as_json.status.code(), Some(1));
    let error = &json(&as_json)["error"];
    assert_eq!(
        text_of(&as_text),
        format!(
            "error [{}]: {}\n",
            error["code"].as_str().unwrap(),
            error["message"].as_str().unwrap()
        )
    );
}

#[test]
fn schema_text_falls_back_to_json_with_one_stderr_line() {
    let (as_json, as_text) = trio(|| (case(), os(&["schema", "config"])), true);
    assert_eq!(as_json.stdout, as_text.stdout);
    assert!(as_json.stderr.is_empty());
    assert_eq!(String::from_utf8_lossy(&as_text.stderr), FALLBACK);
}

#[test]
fn format_flag_works_before_and_after_the_subcommand() {
    let case = case();
    let (apply, _) = apply_args(&case);
    assert!(armorer(case.root.path(), &apply).status.success());
    let before = armorer(case.root.path(), &os(&["--format", "text", "check"]));
    let after = armorer(case.root.path(), &os(&["check", "--format", "text"]));
    let plain = armorer(case.root.path(), &os(&["check"]));
    assert_eq!(before.stdout, after.stdout);
    assert_eq!(before.status.code(), after.status.code());
    assert_ne!(before.stdout, plain.stdout);
    assert!(text_of(&before).starts_with("Bootstrap plan "));
}

#[test]
fn text_escapes_terminal_control_characters_but_json_is_untouched() {
    let hostile = "name: hostile\n\x1b[2K\rfake line\0nul \u{202e}reversed\n";
    let make = || {
        let case = case();
        fs::create_dir_all(case.root.path().join(".github/workflows")).unwrap();
        fs::write(case.root.path().join(CI), hostile).unwrap();
        let args = plan_args(&case);
        (case, args)
    };
    let (as_json, as_text) = trio(make, false);

    // JSON still carries the exact bytes (escaped by the serializer, round-tripping losslessly).
    let plan = json(&as_json);
    let change = plan["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["path"] == CI)
        .unwrap();
    assert_eq!(change["before_content"], hostile);
    assert_eq!(change["disposition"], "conflict");
    assert!(!as_json.stdout.contains(&0x1b) && !as_json.stdout.contains(&0));

    // Text has no raw ESC, CR, NUL or U+202E, and shows visible escapes in the diff.
    let bytes = &as_text.stdout;
    assert!(!bytes.contains(&0x1b), "raw ESC");
    assert!(!bytes.contains(&b'\r'), "raw CR");
    assert!(!bytes.contains(&0), "raw NUL");
    let text = text_of(&as_text);
    assert!(!text.contains('\u{202e}'), "raw RLO");
    assert!(text.contains("\\x1b[2K"));
    assert!(text.contains("\\r"));
    assert!(text.contains("\\x00"));
    assert!(text.contains("\\u{202e}"));
    assert!(text.contains("conflict"));
}
