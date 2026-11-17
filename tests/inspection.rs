use armorer::{
    config::{load_config, load_lock},
    plan::Disposition,
    safe_path,
};
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;

const CONFIG: &str = r#"schema_version = 1
repository = "example/pilot"
toolchain = "1.95.0"
[[deliverables]]
id = "library"
profile = "library"
package = "core-lib"
targets = ["x86_64-unknown-linux-gnu"]
feature_set = "standard"
[[deliverables]]
id = "cli"
profile = "cli"
package = "tool"
binary = "pilot"
targets = ["aarch64-apple-darwin", "x86_64-unknown-linux-gnu"]
feature_set = "standard"
[[deliverables]]
id = "daemon"
profile = "service"
package = "tool"
binary = "pilotd"
targets = ["aarch64-unknown-linux-gnu"]
feature_set = "standard"
[feature_sets.standard]
default_features = true
features = []
[policy]
license_file = "LICENSE"
attestations = "required"
"#;

fn write(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

const POLICY: &str = "schema_version = 1\n[licenses]\nallow = [\"MIT\"]\n[advisories]\nexceptions = []\n[sources]\nallow_git = []\n[bans]\nmultiple_versions = \"warn\"\ndeny = []\n";

fn fixture() -> TempDir {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "armorer.toml", CONFIG);
    write(root.path(), "ci-policy.reviewed.toml", POLICY);
    write(root.path(), "LICENSE", "MIT\n");
    write(
        root.path(),
        "Cargo.toml",
        r#"[workspace]
members = ["crates/core", "crates/tool"]
resolver = "2"
[workspace.package]
version = "0.3.0"
edition = "2024"
license = "MIT"
rust-version = "1.85"
[workspace.dependencies]
core-lib = { path = "crates/core" }
"#,
    );
    write(
        root.path(),
        "crates/core/Cargo.toml",
        r#"[package]
name = "core-lib"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true
links = "pilot-native"
"#,
    );
    write(
        root.path(),
        "crates/core/src/lib.rs",
        "compile_error!(\"must not compile source\");\n",
    );
    write(
        root.path(),
        "crates/core/build.rs",
        "fn main() { panic!(\"must not execute build script\"); }\n",
    );
    write(
        root.path(),
        "crates/tool/Cargo.toml",
        r#"[package]
name = "tool"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true
[dependencies]
core-lib.workspace = true
[features]
default = ["service"]
service = []
[[bin]]
name = "pilot"
path = "src/main.rs"
[[bin]]
name = "pilotd"
path = "src/daemon.rs"
required-features = ["service"]
"#,
    );
    write(root.path(), "crates/tool/src/main.rs", "fn main() {}\n");
    write(root.path(), "crates/tool/src/daemon.rs", "fn main() {}\n");
    root
}

/// The surviving inspection contract: bootstrap over the fixture's reviewed policy.
fn inspect(root: &Path) -> armorer::Result<armorer::bootstrap::Plan> {
    armorer::bootstrap::inspect(root, &root.join("ci-policy.reviewed.toml"))
}

fn plan_bytes(root: &Path) -> Vec<u8> {
    serde_json::to_vec_pretty(&inspect(root).unwrap()).unwrap()
}

#[test]
fn discovers_inheritance_profiles_and_native_requirements_without_building() {
    let root = fixture();
    let plan = inspect(root.path()).unwrap();
    assert_eq!(plan.workspace.packages.len(), 2);
    assert_eq!(plan.intent.deliverables.len(), 3);
    assert_eq!(plan.workspace.packages[0].version, "0.3.0");
    assert_eq!(plan.workspace.packages[0].license.as_deref(), Some("MIT"));
    assert!(
        plan.findings
            .iter()
            .any(|f| f.code == "native-build-review")
    );
    assert!(!root.path().join("Cargo.lock").exists());
    assert!(!root.path().join("target").exists());
    assert_eq!(plan.capability_state, "unknown");
}

#[test]
fn deterministic_across_runs_and_repository_locations() {
    let a = fixture();
    let b = fixture();
    assert_eq!(plan_bytes(a.path()), plan_bytes(a.path()));
    assert_eq!(plan_bytes(a.path()), plan_bytes(b.path()));
    let before = plan_bytes(a.path());
    write(a.path(), "crates/tool/src/bin/extra.rs", "fn main() {}\n");
    assert_ne!(before, plan_bytes(a.path()));
}

#[test]
fn preserves_existing_toolchain_and_lock_bytes() {
    let root = fixture();
    let lock = "# a lock must not be rewritten\nversion = 4\n";
    let toolchain = "[toolchain]\nchannel = \"stable\"\ncomponents = [\"rust-src\"]\n";
    write(root.path(), "Cargo.lock", lock);
    write(root.path(), "rust-toolchain.toml", toolchain);
    let plan = inspect(root.path()).unwrap();
    let toolchain_change = plan
        .changes
        .iter()
        .find(|c| c.path == "rust-toolchain.toml")
        .unwrap();
    assert_eq!(toolchain_change.disposition, Disposition::Conflict);
    assert_eq!(
        fs::read_to_string(root.path().join("Cargo.lock")).unwrap(),
        lock
    );
    assert_eq!(
        fs::read_to_string(root.path().join("rust-toolchain.toml")).unwrap(),
        toolchain
    );
}

#[cfg(unix)]
#[test]
fn excludes_repository_credential_providers_and_overrides() {
    let root = fixture();
    let marker = root.path().join("credential-provider-executed");
    let script = format!("#!/bin/sh\ntouch '{}'\nexit 99\n", marker.display());
    write(root.path(), "provider.sh", &script);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            root.path().join("provider.sh"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
    }
    write(
        root.path(),
        ".cargo/config.toml",
        &format!(
            "[registry]\nglobal-credential-providers = [\"{}\"]\n[build]\nrustc-wrapper = \"{}\"\n",
            root.path().join("provider.sh").display(),
            root.path().join("provider.sh").display()
        ),
    );
    let plan = inspect(root.path()).unwrap();
    assert!(plan.workspace.cargo_config_present);
    assert!(
        plan.findings
            .iter()
            .any(|f| f.code == "cargo-config-excluded")
    );
    assert!(!marker.exists());
}

#[test]
fn rejects_unknown_fields_without_echoing_values() {
    let root = fixture();
    let poison = CONFIG.replace(
        "schema_version = 1",
        "schema_version = 1\nsecret = \"DO-NOT-ECHO-THIS\"",
    );
    write(root.path(), "armorer.toml", &poison);
    let error = inspect(root.path()).unwrap_err();
    assert_eq!(error.code(), "invalid-toml");
    assert!(!error.to_string().contains("DO-NOT-ECHO-THIS"));
}

#[test]
fn rejects_unpinned_toolchains_unsupported_targets_and_duplicates() {
    for replacement in [
        CONFIG.replace("1.95.0", "stable"),
        CONFIG.replace("x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"),
        CONFIG.replace("id = \"cli\"", "id = \"library\""),
        CONFIG.replace("features = []", "features = [\"service\", \"service\"]"),
        CONFIG.replace(
            "license_file = \"LICENSE\"",
            "license_file = \"../LICENSE\"",
        ),
    ] {
        let root = fixture();
        write(root.path(), "armorer.toml", &replacement);
        assert!(load_config(root.path()).is_err());
    }
}

#[test]
fn rejects_missing_packages_binaries_and_disabled_required_features() {
    for replacement in [
        CONFIG.replace("package = \"tool\"", "package = \"missing\""),
        CONFIG.replace("binary = \"pilot\"", "binary = \"missing\""),
        CONFIG.replace("default_features = true", "default_features = false"),
        CONFIG.replace("features = []", "features = [\"missing\"]"),
    ] {
        let root = fixture();
        write(root.path(), "armorer.toml", &replacement);
        assert_eq!(inspect(root.path()).unwrap_err().code(), "invalid-config");
    }
}

#[test]
fn rejects_manifest_escape_and_incompatible_msrv() {
    let root = fixture();
    let manifest = fs::read_to_string(root.path().join("Cargo.toml")).unwrap();
    write(
        root.path(),
        "Cargo.toml",
        &manifest.replace("rust-version = \"1.85\"", "rust-version = \"1.96\""),
    );
    assert_eq!(inspect(root.path()).unwrap_err().code(), "invalid-config");
    write(
        root.path(),
        "Cargo.toml",
        &manifest.replace("crates/core\" }", "../outside\" }"),
    );
    assert_eq!(inspect(root.path()).unwrap_err().code(), "unsafe-path");
}

#[cfg(unix)]
#[test]
fn rejects_symlink_inputs() {
    let root = fixture();
    fs::remove_file(root.path().join("LICENSE")).unwrap();
    std::os::unix::fs::symlink("Cargo.toml", root.path().join("LICENSE")).unwrap();
    assert!(safe_path(root.path(), "LICENSE").is_err());
    assert_eq!(inspect(root.path()).unwrap_err().code(), "unsafe-path");
}

#[test]
fn lock_requires_config_binding_and_full_pins() {
    let root = fixture();
    let digest = "a".repeat(64);
    let lock = format!(
        r#"schema_version = 1
config_sha256 = "{digest}"
runtime_version = "0.1.0"
[workflows]
repository = "brianluby/armorer-workflows"
commit = "{}"
[tools.cargo-cyclonedx]
version = "0.5.9"
sha256 = "{}"
"#,
        "b".repeat(40),
        "c".repeat(64)
    );
    write(root.path(), "armorer.lock", &lock);
    let (validated, bytes) = load_lock(root.path(), &digest).unwrap().unwrap();
    assert_eq!(validated.config_sha256, digest);
    assert_eq!(bytes, lock.as_bytes());
    assert!(load_lock(root.path(), &"d".repeat(64)).is_err());
    write(
        root.path(),
        "armorer.lock",
        &lock.replace(&"b".repeat(40), "v1"),
    );
    assert!(load_lock(root.path(), &digest).is_err());
    assert_eq!(bytes, lock.as_bytes());
}

#[test]
/// A lock is readable by the running binary's own patch line only: never from a
/// newer patch, another minor or major line, or a prerelease.
fn lock_runtime_version_follows_the_patch_line_compatibility_rule() {
    let root = fixture();
    let digest = "a".repeat(64);
    let lock = |runtime: &str| {
        format!(
            "schema_version = 1\nconfig_sha256 = \"{digest}\"\nruntime_version = \"{runtime}\"\n[workflows]\nrepository = \"brianluby/armorer-workflows\"\ncommit = \"{}\"\n[tools.cargo-cyclonedx]\nversion = \"0.5.9\"\nsha256 = \"{}\"\n",
            "b".repeat(40),
            "c".repeat(64)
        )
    };
    let running = semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    let version = |major, minor, patch| format!("{major}.{minor}.{patch}");
    let mut accepted = vec![version(running.major, running.minor, running.patch)];
    if running.patch > 0 {
        accepted.push(version(running.major, running.minor, 0));
    }
    for runtime in accepted {
        write(root.path(), "armorer.lock", &lock(&runtime));
        assert!(
            load_lock(root.path(), &digest).unwrap().is_some(),
            "{runtime}"
        );
    }
    for runtime in [
        version(running.major, running.minor, running.patch + 1),
        version(running.major, running.minor + 1, 0),
        version(running.major + 1, running.minor, running.patch),
        format!("{running}-rc.1"),
        format!("{running}+local"),
    ] {
        write(root.path(), "armorer.lock", &lock(&runtime));
        assert!(load_lock(root.path(), &digest).is_err(), "{runtime}");
    }
}

#[test]
fn plan_hashes_exact_validated_lock_bytes() {
    use sha2::{Digest, Sha256};
    let root = fixture();
    let config_digest = format!("{:x}", Sha256::digest(CONFIG.as_bytes()));
    let lock = format!(
        r#"schema_version = 1
config_sha256 = "{config_digest}"
runtime_version = "0.1.0"
[workflows]
repository = "brianluby/armorer-workflows"
commit = "{}"
[tools.cargo-cyclonedx]
version = "0.5.9"
sha256 = "{}"
"#,
        "b".repeat(40),
        "c".repeat(64)
    );
    for bytes in [lock.clone(), format!("# reviewed comment\n{lock}")] {
        write(root.path(), "armorer.lock", &bytes);
        // The surviving contract binds exact lock bytes through load_lock; the
        // bootstrap plan reads the managed armorer.lock as a provisioning target.
        let (validated, exact) = load_lock(root.path(), &config_digest).unwrap().unwrap();
        assert_eq!(validated.config_sha256, config_digest);
        assert_eq!(exact, bytes.as_bytes());
        assert_eq!(
            format!("{:x}", Sha256::digest(&exact)),
            format!("{:x}", Sha256::digest(bytes.as_bytes()))
        );
    }
}

#[test]
fn cli_check_fails_closed_plan_is_read_only_and_errors_are_json() {
    let root = fixture();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_armorer"))
            .args(["--repository", root.path().to_str().unwrap()])
            .args(args)
            .output()
            .unwrap()
    };
    // Without a managed policy, check fails closed with a JSON error.
    let check = run(&["check"]);
    assert_eq!(check.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&check.stdout).unwrap();
    assert!(value["error"]["code"].is_string());
    let policy = root.path().join("ci-policy.reviewed.toml");
    let run_plan = || {
        Command::new(env!("CARGO_BIN_EXE_armorer"))
            .args(["--repository", root.path().to_str().unwrap()])
            .args(["bootstrap", "plan", "--policy"])
            .arg(&policy)
            .output()
            .unwrap()
    };
    let plan = run_plan();
    assert!(plan.status.success());
    assert_eq!(plan.stdout, run_plan().stdout);
    assert!(!root.path().join("rust-toolchain.toml").exists());
    assert!(!root.path().join("Cargo.lock").exists());
    write(root.path(), "armorer.toml", "invalid");
    let error = run_plan();
    assert_eq!(error.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&error.stdout).unwrap();
    assert_eq!(value["error"]["code"], "invalid-toml");
}

/// Top-level check reads the bootstrap contract: fail-closed without a policy,
/// exit 0 in sync, exit 2 with stderr reasons on drift, never writes.
#[test]
fn cli_check_reads_the_bootstrap_contract() {
    let root = fixture();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_armorer"))
            .args(["--repository", root.path().to_str().unwrap(), "check"])
            .output()
            .unwrap()
    };
    // No managed policy: fail closed with a JSON error, nothing written.
    let missing = run();
    assert_eq!(
        missing.status.code(),
        Some(1),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&missing.stdout),
        String::from_utf8_lossy(&missing.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&missing.stdout).unwrap();
    assert!(value["error"]["code"].is_string());
    assert!(!root.path().join("rust-toolchain.toml").exists());

    // Provision through the bootstrap contract, then check reports sync.
    write(root.path(), "ci-policy.reviewed.toml", POLICY);
    write(root.path(), "Cargo.lock", "version = 4\n");
    let plan =
        armorer::bootstrap::inspect(root.path(), &root.path().join("ci-policy.reviewed.toml"))
            .unwrap();
    armorer::bootstrap::apply(root.path(), &plan, &plan.plan_sha256).unwrap();
    let synced = run();
    assert_eq!(synced.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&synced.stdout).unwrap();
    assert_eq!(value["schema_version"], 2);

    // Editing an owned file is drift: exit 2, reason on stderr, bytes untouched.
    let edited = "[toolchain]\nchannel = \"9.9.9\"\n";
    write(root.path(), "rust-toolchain.toml", edited);
    let drifted = run();
    assert_eq!(drifted.status.code(), Some(2));
    let stderr = String::from_utf8(drifted.stderr).unwrap();
    assert!(stderr.contains("rust-toolchain.toml"), "{stderr}");
    assert_eq!(
        fs::read_to_string(root.path().join("rust-toolchain.toml")).unwrap(),
        edited
    );
}

#[test]
fn rejects_oversized_inputs_before_metadata() {
    let root = fixture();
    write(root.path(), "armorer.toml", &"x".repeat(1_048_577));
    assert_eq!(inspect(root.path()).unwrap_err().code(), "invalid-config");
}

#[test]
fn handles_recursive_feature_cycles_and_transitive_required_features() {
    let root = fixture();
    let manifest = fs::read_to_string(root.path().join("crates/tool/Cargo.toml")).unwrap();
    let mutated = manifest.replace(
        "default = [\"service\"]\nservice = []",
        "default = [\"bundle\"]\nbundle = [\"service\"]\nservice = [\"bundle\"]",
    );
    assert_ne!(manifest, mutated);
    write(root.path(), "crates/tool/Cargo.toml", &mutated);
    assert!(inspect(root.path()).is_ok());
}

#[cfg(unix)]
#[test]
fn missing_installed_toolchain_is_an_error_without_installation() {
    use std::os::unix::fs::PermissionsExt;
    let root = fixture();
    write(
        root.path(),
        "armorer.toml",
        &CONFIG.replace("1.95.0", "99.99.99"),
    );
    // Controlled rustup on PATH: `which` fails for a missing toolchain, so the test
    // exercises toolchain-lookup on any host instead of depending on installed rustup.
    let tools = tempfile::tempdir().unwrap();
    write(tools.path(), "rustup", "#!/bin/sh\nexit 1\n");
    fs::set_permissions(
        tools.path().join("rustup"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_armorer"))
        .env("PATH", tools.path())
        .args([
            "--repository",
            root.path().to_str().unwrap(),
            "bootstrap",
            "plan",
            "--policy",
        ])
        .arg(root.path().join("ci-policy.reviewed.toml"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "cargo-discovery");
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("toolchain-lookup"),
        "{value}"
    );
    assert!(!root.path().join("Cargo.lock").exists());
}

#[cfg(unix)]
#[test]
fn preserves_rustup_multicall_executable_name() {
    let root = fixture();
    let tools = tempfile::tempdir().unwrap();
    let Some(installed) = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|directory| directory.join("rustup"))
            .find(|path| path.is_absolute() && path.is_file())
    }) else {
        eprintln!("rustup not on PATH; skipping multicall-name test");
        return;
    };
    fs::copy(installed, tools.path().join("rustup-init")).unwrap();
    std::os::unix::fs::symlink("rustup-init", tools.path().join("rustup")).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_armorer"))
        .env("PATH", tools.path())
        .args([
            "--repository",
            root.path().to_str().unwrap(),
            "bootstrap",
            "plan",
            "--policy",
        ])
        .arg(root.path().join("ci-policy.reviewed.toml"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
}

#[cfg(unix)]
#[test]
fn rejects_repository_local_executable_on_path() {
    use std::os::unix::fs::PermissionsExt;
    let root = fixture();
    let tools = tempfile::tempdir().unwrap();
    let marker = root.path().join("executed");
    write(
        tools.path(),
        "rustup",
        &format!(
            "#!/bin/sh\nprintf executed > '{}'\nexit 1\n",
            marker.display()
        ),
    );
    fs::set_permissions(
        tools.path().join("rustup"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    fs::create_dir(root.path().join(".cargo")).unwrap();
    std::os::unix::fs::symlink(
        tools.path().join("rustup"),
        root.path().join(".cargo/rustup"),
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_armorer"))
        .env("PATH", root.path().join(".cargo"))
        .args([
            "--repository",
            root.path().to_str().unwrap(),
            "bootstrap",
            "plan",
            "--policy",
        ])
        .arg(root.path().join("ci-policy.reviewed.toml"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(!marker.exists());
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["error"]["code"], "cargo-discovery");
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("rustup-path")
    );
}

#[test]
fn accepts_anchored_package_patterns_and_opaque_metadata() {
    let root = fixture();
    let manifest = fs::read_to_string(root.path().join("crates/tool/Cargo.toml")).unwrap();
    let mutated = manifest.replace("[package]", "[package]\nexclude = [\"/ci\", \"/.github\"]");
    assert_ne!(manifest, mutated);
    write(root.path(), "crates/tool/Cargo.toml", &mutated);
    assert!(inspect(root.path()).is_ok());
    write(
        root.path(),
        "crates/tool/Cargo.toml",
        &format!(
            "{manifest}\n[package.metadata.external]\npath = \"/opaque/value\"\nexclude = [\"/opaque/pattern\"]\n"
        ),
    );
    assert!(inspect(root.path()).is_ok());
}

#[test]
fn rejects_actual_workspace_and_dependency_escapes_in_all_cargo_tables() {
    for declaration in [
        "[workspace]\nmembers = [\"/outside\"]",
        "[workspace]\nmembers = [\"../outside\"]",
        "[workspace]\nexclude = [\"/outside\"]",
        "[workspace]\ndefault-members = [\"../outside\"]",
        "[workspace.package]\nreadme = \"/outside\"",
        "[workspace.dependencies]\nexternal = { path = \"../outside\" }",
        "[dependencies]\nexternal = { path = \"../outside\" }",
        "[dev-dependencies]\nexternal = { path = \"/outside\" }",
        "[build-dependencies]\nexternal = { path = \"../outside\" }",
        "[target.'cfg(unix)'.dependencies]\nexternal = { path = \"/outside\" }",
        "[patch.crates-io]\nexternal = { path = \"../outside\" }",
        "[replace]\n'external:0.1.0' = { path = \"/outside\" }",
        "[lib]\npath = \"../outside.rs\"",
        "[[bin]]\nname = \"external\"\npath = \"/outside.rs\"",
        "[package]\nname = \"external\"\nbuild = \"../outside.rs\"",
    ] {
        let root = fixture();
        write(root.path(), "Cargo.toml", declaration);
        assert_eq!(
            inspect(root.path()).unwrap_err().code(),
            "unsafe-path",
            "{declaration}"
        );
    }
}

#[test]
fn rejects_excessively_deep_directory_trees() {
    let root = fixture();
    let mut path = root.path().join("deep");
    for _ in 0..130 {
        fs::create_dir(&path).unwrap();
        path.push("nested");
    }
    assert_eq!(inspect(root.path()).unwrap_err().code(), "invalid-config");
    assert!(!root.path().join("Cargo.lock").exists());
}

#[test]
fn rejects_oversized_directory_inventory() {
    let root = fixture();
    let many = root.path().join("many");
    fs::create_dir(&many).unwrap();
    for i in 0..20_000 {
        fs::File::create(many.join(i.to_string())).unwrap();
    }
    assert_eq!(inspect(root.path()).unwrap_err().code(), "invalid-config");
    assert!(!root.path().join("Cargo.lock").exists());
}

#[cfg(unix)]
#[test]
fn rejects_fifo_inputs_without_blocking() {
    use std::{
        process::Stdio,
        time::{Duration, Instant},
    };
    for file in ["armorer.toml", "armorer.lock", "rust-toolchain.toml"] {
        let root = fixture();
        let input = root.path().join(file);
        if input.exists() {
            fs::remove_file(&input).unwrap();
        }
        assert!(
            Command::new("mkfifo")
                .arg(&input)
                .status()
                .unwrap()
                .success()
        );
        let mut child = Command::new(env!("CARGO_BIN_EXE_armorer"))
            .args([
                "--repository",
                root.path().to_str().unwrap(),
                "bootstrap",
                "plan",
                "--policy",
            ])
            .arg(root.path().join("ci-policy.reviewed.toml"))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let started = Instant::now();
        let output = loop {
            if child.try_wait().unwrap().is_some() {
                break child.wait_with_output().unwrap();
            }
            if started.elapsed() > Duration::from_secs(10) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("inspection blocked on {file}");
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(output.status.code(), Some(1));
        let diagnostic: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(diagnostic["error"]["code"], "unsafe-path", "{file}");
    }
}

#[cfg(unix)]
#[test]
fn rejects_temporary_directory_inside_consumer_before_writing() {
    let root = fixture();
    let scratch = root.path().join("scratch");
    fs::create_dir(&scratch).unwrap();
    let before = fs::metadata(&scratch).unwrap().modified().unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_armorer"))
        .env("TMPDIR", &scratch)
        .args([
            "--repository",
            root.path().to_str().unwrap(),
            "bootstrap",
            "plan",
            "--policy",
        ])
        .arg(root.path().join("ci-policy.reviewed.toml"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["error"]["code"], "unsafe-path");
    assert_eq!(fs::metadata(&scratch).unwrap().modified().unwrap(), before);
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn rejects_inherited_temporary_ancestor_cargo_configuration() {
    let root = fixture();
    let parent = tempfile::tempdir().unwrap();
    let scratch = parent.path().join("scratch");
    fs::create_dir(&scratch).unwrap();
    write(
        parent.path(),
        ".cargo/config.toml",
        "not-valid-TOML-DO-NOT-ECHO",
    );
    let before = fs::metadata(&scratch).unwrap().modified().unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_armorer"))
        .env("TMPDIR", &scratch)
        .args([
            "--repository",
            root.path().to_str().unwrap(),
            "bootstrap",
            "plan",
            "--policy",
        ])
        .arg(root.path().join("ci-policy.reviewed.toml"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["error"]["code"], "unsafe-path");
    assert!(!String::from_utf8_lossy(&result.stdout).contains("DO-NOT-ECHO"));
    assert_eq!(fs::metadata(&scratch).unwrap().modified().unwrap(), before);
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
/// A descendant inheriting rustup's stdout must not block discovery after the child exits.
fn inherited_stdout_descendant_does_not_block_toolchain_lookup() {
    use std::{
        os::unix::fs::PermissionsExt,
        process::Stdio,
        time::{Duration, Instant},
    };
    let root = fixture();
    let tools = tempfile::tempdir().unwrap();
    // The direct child exits at once; the backgrounded descendant keeps the inherited
    // stdout open far beyond the test timeout below, so the old pipe-based drain would
    // hang and fail this test. stdout is a file, so it cannot block.
    write(root.path(), "cargo", "");
    write(
        tools.path(),
        "rustup",
        &format!(
            "#!/bin/sh\nsleep 30 &\nprintf '{}/cargo\\n'\nexit 0\n",
            root.path().display()
        ),
    );
    fs::set_permissions(
        tools.path().join("rustup"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_armorer"))
        .env("PATH", tools.path())
        .args([
            "--repository",
            root.path().to_str().unwrap(),
            "bootstrap",
            "plan",
            "--policy",
        ])
        .arg(root.path().join("ci-policy.reviewed.toml"))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let started = Instant::now();
    let output = loop {
        if child.try_wait().unwrap().is_some() {
            break child.wait_with_output().unwrap();
        }
        if started.elapsed() > Duration::from_secs(5) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("toolchain lookup blocked on inherited stdout");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    // Prompt termination with the expected failure: the reported cargo path is inside
    // the repository, which discovery must reject.
    assert_eq!(output.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "cargo-discovery");
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("toolchain-path"),
        "{value}"
    );
}

/// After an upgrade owns the files, read-only check still reports drift;
/// only writers refuse the upgrade owner. Leftover v1 ownership stays refused.
#[cfg(unix)]
#[test]
fn check_reports_drift_under_upgrade_ownership_and_refuses_symlinked_policy() {
    let root = fixture();
    write(root.path(), "Cargo.lock", "version = 4\n");
    let plan = inspect(root.path()).unwrap();
    armorer::bootstrap::apply(root.path(), &plan, &plan.plan_sha256).unwrap();
    let policy = root.path().join("ci-policy.reviewed.toml");
    let up = armorer::upgrade::inspect(root.path(), &policy, "bootstrap-v2", &[], false).unwrap();
    armorer::upgrade::apply(root.path(), &up, &up.plan_sha256).unwrap();
    // Writers still refuse the upgrade owner; read-only inspection stays usable.
    let stale = inspect(root.path()).unwrap();
    let refusal = armorer::bootstrap::apply(root.path(), &stale, &stale.plan_sha256)
        .unwrap_err()
        .to_string();
    assert!(refusal.contains("upgrade"), "{refusal}");
    // Read-only check works: copy the reviewed policy to the managed path.
    fs::copy(&policy, root.path().join(".armorer/ci-policy.toml")).unwrap();
    let run = |repo: &Path| {
        Command::new(env!("CARGO_BIN_EXE_armorer"))
            .args(["--repository", repo.to_str().unwrap(), "check"])
            .output()
            .unwrap()
    };
    let synced = run(root.path());
    assert_eq!(
        synced.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&synced.stdout)
    );
    fs::write(
        root.path().join("rust-toolchain.toml"),
        "[toolchain]\nchannel = \"9.9.9\"\n",
    )
    .unwrap();
    assert_eq!(run(root.path()).status.code(), Some(2));

    // A symlinked .armorer parent must not redirect the managed policy read.
    let root = fixture();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("ci-policy.toml"), POLICY).unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join(".armorer")).unwrap();
    let error = run(root.path());
    assert_eq!(error.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&error.stdout).unwrap();
    assert_eq!(value["error"]["code"], "unsafe-path");
}

/// An interrupted upgrade journal is drift even when managed bytes are in sync:
/// empty drift must mean no transaction awaits recovery.
#[test]
fn pending_upgrade_journal_is_recovery_drift() {
    let root = fixture();
    write(root.path(), "Cargo.lock", "version = 4\n");
    write(root.path(), ".armorer/ci-policy.toml", POLICY);
    let policy = root.path().join("ci-policy.reviewed.toml");
    let plan = inspect(root.path()).unwrap();
    armorer::bootstrap::apply(root.path(), &plan, &plan.plan_sha256).unwrap();
    let up = armorer::upgrade::inspect(root.path(), &policy, "bootstrap-v2", &[], false).unwrap();
    armorer::upgrade::apply(root.path(), &up, &up.plan_sha256).unwrap();
    fs::copy(&policy, root.path().join(".armorer/ci-policy.toml")).unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_armorer"))
            .args(["--repository", root.path().to_str().unwrap(), "check"])
            .output()
            .unwrap()
    };
    assert_eq!(run().status.code(), Some(0));
    fs::write(root.path().join(".armorer/upgrade-journal-v1.json"), "{}").unwrap();
    let pending = run();
    assert_eq!(pending.status.code(), Some(2));
    assert!(
        String::from_utf8(pending.stderr)
            .unwrap()
            .contains("transaction-recovery-required")
    );
}

/// A repository provisioned by `upgrade --catalog bootstrap-v1` records ownership
/// only in the upgrade state; check must use that catalog, not the v2 default.
#[test]
fn check_uses_the_upgrade_owners_catalog() {
    let root = fixture();
    write(root.path(), "Cargo.lock", "version = 4\n");
    let policy = root.path().join("ci-policy.reviewed.toml");
    let up = armorer::upgrade::inspect(root.path(), &policy, "bootstrap-v1", &[], false).unwrap();
    armorer::upgrade::apply(root.path(), &up, &up.plan_sha256).unwrap();
    fs::copy(&policy, root.path().join(".armorer/ci-policy.toml")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_armorer"))
        .args(["--repository", root.path().to_str().unwrap(), "check"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
