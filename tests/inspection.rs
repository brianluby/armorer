use armorer::{
    config::{load_config, load_lock},
    plan::inspect,
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

fn fixture() -> TempDir {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "armorer.toml", CONFIG);
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

fn plan_bytes(root: &Path) -> Vec<u8> {
    serde_json::to_vec_pretty(&inspect(root, "plan").unwrap()).unwrap()
}

#[test]
fn discovers_inheritance_profiles_and_native_requirements_without_building() {
    let root = fixture();
    let plan = inspect(root.path(), "plan").unwrap();
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
    assert_eq!(plan.state, "configuration-valid");
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
    let plan = inspect(root.path(), "plan").unwrap();
    assert_eq!(plan.changes[0].disposition, "conflict");
    assert_eq!(
        fs::read_to_string(root.path().join("Cargo.lock")).unwrap(),
        lock
    );
    assert_eq!(
        fs::read_to_string(root.path().join("rust-toolchain.toml")).unwrap(),
        toolchain
    );
}

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
    let plan = inspect(root.path(), "plan").unwrap();
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
    let error = inspect(root.path(), "plan").unwrap_err();
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
        assert_eq!(
            inspect(root.path(), "plan").unwrap_err().code(),
            "invalid-config"
        );
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
    assert_eq!(
        inspect(root.path(), "plan").unwrap_err().code(),
        "invalid-config"
    );
    write(
        root.path(),
        "Cargo.toml",
        &manifest.replace("crates/core\" }", "../outside\" }"),
    );
    assert_eq!(
        inspect(root.path(), "plan").unwrap_err().code(),
        "unsafe-path"
    );
}

#[cfg(unix)]
#[test]
fn rejects_symlink_inputs() {
    let root = fixture();
    fs::remove_file(root.path().join("LICENSE")).unwrap();
    std::os::unix::fs::symlink("Cargo.toml", root.path().join("LICENSE")).unwrap();
    assert!(safe_path(root.path(), "LICENSE").is_err());
    assert_eq!(
        inspect(root.path(), "plan").unwrap_err().code(),
        "unsafe-path"
    );
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
    assert!(load_lock(root.path(), &digest).unwrap().is_some());
    assert!(load_lock(root.path(), &"d".repeat(64)).is_err());
    write(
        root.path(),
        "armorer.lock",
        &lock.replace(&"b".repeat(40), "v1"),
    );
    assert!(load_lock(root.path(), &digest).is_err());
}

#[test]
fn cli_check_fails_closed_plan_is_read_only_and_errors_are_json() {
    let root = fixture();
    let run = |mode: &str| {
        Command::new(env!("CARGO_BIN_EXE_armorer"))
            .args(["--repository", root.path().to_str().unwrap(), mode])
            .output()
            .unwrap()
    };
    let check = run("check");
    assert_eq!(check.status.code(), Some(2));
    let plan = run("plan");
    assert!(plan.status.success());
    assert_eq!(plan.stdout, run("plan").stdout);
    assert!(!root.path().join("rust-toolchain.toml").exists());
    assert!(!root.path().join("Cargo.lock").exists());
    write(root.path(), "armorer.toml", "invalid");
    let error = run("plan");
    assert_eq!(error.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&error.stdout).unwrap();
    assert_eq!(value["error"]["code"], "invalid-toml");
}

#[test]
fn rejects_oversized_inputs_before_metadata() {
    let root = fixture();
    write(root.path(), "armorer.toml", &"x".repeat(1_048_577));
    assert_eq!(
        inspect(root.path(), "plan").unwrap_err().code(),
        "invalid-config"
    );
}

#[test]
fn handles_recursive_feature_cycles_and_transitive_required_features() {
    let root = fixture();
    let manifest = fs::read_to_string(root.path().join("crates/tool/Cargo.toml")).unwrap();
    write(
        root.path(),
        "crates/tool/Cargo.toml",
        &manifest.replace(
            "default = [\"service\"]\nservice = []",
            "default = [\"bundle\"]\nbundle = [\"service\"]\nservice = [\"bundle\"]",
        ),
    );
    assert!(inspect(root.path(), "plan").is_ok());
}

#[test]
fn missing_installed_toolchain_is_an_error_without_installation() {
    let root = fixture();
    write(
        root.path(),
        "armorer.toml",
        &CONFIG.replace("1.95.0", "99.99.99"),
    );
    assert_eq!(
        inspect(root.path(), "plan").unwrap_err().code(),
        "cargo-discovery"
    );
    assert!(!root.path().join("Cargo.lock").exists());
}

#[cfg(unix)]
#[test]
fn preserves_rustup_multicall_executable_name() {
    let root = fixture();
    let tools = tempfile::tempdir().unwrap();
    let installed = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|directory| directory.join("rustup"))
        .find(|path| path.is_absolute() && path.is_file())
        .unwrap();
    fs::copy(installed, tools.path().join("rustup-init")).unwrap();
    std::os::unix::fs::symlink("rustup-init", tools.path().join("rustup")).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_armorer"))
        .env("PATH", tools.path())
        .args(["--repository", root.path().to_str().unwrap(), "plan"])
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
    let marker = root.path().join("executed");
    write(
        root.path(),
        "rustup",
        &format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
    );
    fs::set_permissions(
        root.path().join("rustup"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_armorer"))
        .env("PATH", root.path())
        .args(["--repository", root.path().to_str().unwrap(), "plan"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(!marker.exists());
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["error"]["code"], "cargo-discovery");
}
