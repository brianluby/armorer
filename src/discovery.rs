//! Bounded, offline Cargo discovery from an isolated manifest snapshot.

use crate::{Error, Result, config::Config, digest, read_small};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Debug, Serialize, JsonSchema)]
pub struct Workspace {
    pub packages: Vec<Package>,
    pub inputs: BTreeMap<String, String>,
    pub cargo_config_present: bool,
    pub cargo_lock_present: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub license: Option<String>,
    pub rust_version: Option<String>,
    pub links: Option<String>,
    pub features: BTreeMap<String, Vec<String>>,
    pub targets: Vec<Target>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Target {
    pub name: String,
    pub kind: Vec<String>,
    #[serde(default, rename = "required-features")]
    pub required_features: Vec<String>,
}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<MetadataPackage>,
    workspace_members: Vec<String>,
    workspace_root: PathBuf,
}

#[derive(Deserialize)]
struct MetadataPackage {
    id: String,
    manifest_path: PathBuf,
    #[serde(flatten)]
    package: Package,
}

// Never execute a binary selected by repository-local PATH entries.
/// Find the first rustup file in an absolute PATH directory outside `root`.
///
/// `root` must be canonical. Relative or unresolvable directories and candidates
/// resolving inside `root` are skipped. The returned path retains the rustup
/// filename for multicall installations.
///
/// Returns `Error::Cargo` when PATH is absent or no candidate qualifies;
/// canonicalizing a candidate can propagate `Error::Io`.
fn trusted_rustup(root: &Path) -> Result<PathBuf> {
    let search = std::env::var_os("PATH").ok_or(Error::Cargo("rustup-path"))?;
    for directory in std::env::split_paths(&search) {
        if !directory.is_absolute() {
            continue;
        }
        let Ok(directory) = directory.canonicalize() else {
            continue;
        };
        if directory.starts_with(root) {
            continue;
        }
        let candidate = directory.join(if cfg!(windows) {
            "rustup.exe"
        } else {
            "rustup"
        });
        if !candidate.is_file() {
            continue;
        }
        let resolved = candidate.canonicalize()?;
        if candidate.starts_with(root) || resolved.starts_with(root) {
            continue;
        }
        return Ok(candidate);
    }
    Err(Error::Cargo("rustup-path"))
}

/// Return a UTF-8 path relative to `root`, using forward slashes.
///
/// Returns `Error::Metadata` if the prefix does not match or the relative path
/// is not UTF-8. Paths are not canonicalized.
fn relative(root: &Path, path: &Path) -> Result<String> {
    path.strip_prefix(root)
        .ok()
        .and_then(Path::to_str)
        .map(|s| s.replace('\\', "/"))
        .ok_or(Error::Metadata)
}

/// Check a manifest path lexically against a workspace-relative base directory.
///
/// Parent components may ascend as far as the workspace root. Returns
/// `Error::Path` for an escape, absolute path, backslash, or control character;
/// does not inspect filesystem entries.
fn contained_path(base: &Path, value: &str) -> Result<()> {
    if value.contains('\\') || value.chars().any(char::is_control) {
        return Err(Error::Path("unsupported manifest path".into()));
    }
    let mut depth = base.components().count();
    for part in Path::new(value).components() {
        match part {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir if depth > 0 => depth -= 1,
            _ => return Err(Error::Path("manifest refers outside the workspace".into())),
        }
    }
    Ok(())
}

/// Recursively check recognized manifest path fields relative to `base`, the
/// manifest's workspace-relative directory, propagating [`contained_path`] errors.
///
/// Only string paths and string entries in member/exclusion arrays are checked;
/// other value types are left to Cargo.
fn validate_paths(value: &toml::Value, base: &Path) -> Result<()> {
    if let Some(table) = value.as_table() {
        for (key, value) in table {
            if matches!(
                key.as_str(),
                "path" | "workspace" | "build" | "license-file" | "readme"
            ) && let Some(path) = value.as_str()
            {
                contained_path(base, path)?;
            }
            if matches!(key.as_str(), "members" | "default-members" | "exclude")
                && let Some(paths) = value.as_array()
            {
                for path in paths {
                    if let Some(path) = path.as_str() {
                        contained_path(base, path)?;
                    }
                }
            }
            validate_paths(value, base)?;
        }
    } else if let Some(values) = value.as_array() {
        for value in values {
            validate_paths(value, base)?;
        }
    }
    Ok(())
}

struct Snapshot {
    inputs: BTreeMap<String, String>,
    count: usize,
    bytes: usize,
    cargo_config_present: bool,
}

impl Snapshot {
    /// Populate `destination` from `directory` under `root` without changing inputs.
    ///
    /// Copies Cargo manifests and lockfiles, creates empty Rust source placeholders,
    /// and records hashes or source-path presence. Skips `.cargo` entries and common
    /// build/VCS directories, recording whether a `.cargo` entry was encountered.
    ///
    /// # Errors
    /// Rejects more than 20,000 visited entries, Cargo inputs over 1 MiB, or more than
    /// 8 MiB of manifest/lockfile bytes with `Error::Invalid`. Returns `Error::Path`
    /// for non-UTF-8 names, unskipped symlinks, or unsafe manifest paths, and
    /// `Error::Toml` for invalid manifest UTF-8 or TOML. Propagates I/O and relative
    /// path errors. A failure can leave a partial snapshot and updated counters.
    fn copy(&mut self, root: &Path, directory: &Path, destination: &Path) -> Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(directory)?.collect::<std::io::Result<_>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            self.count += 1;
            if self.count > 20_000 {
                return Err(Error::Invalid(
                    "workspace exceeds discovery entry limit".into(),
                ));
            }
            let path = entry.path();
            let name = entry.file_name();
            let name = name
                .to_str()
                .ok_or_else(|| Error::Path("non-UTF-8 filename".into()))?;
            let kind = entry.file_type()?;
            if kind.is_dir()
                && matches!(
                    name,
                    ".git" | ".worktrees" | "target" | "node_modules" | ".armorer"
                )
            {
                continue;
            }
            if name == ".cargo" {
                self.cargo_config_present = true;
                continue;
            }
            if kind.is_symlink() {
                return Err(Error::Path(
                    "symlinks require migration before discovery".into(),
                ));
            }
            let rel = relative(root, &path)?;
            let output = destination.join(&rel);
            if kind.is_dir() {
                std::fs::create_dir_all(&output)?;
                self.copy(root, &path, destination)?;
            } else if kind.is_file() && (name == "Cargo.toml" || name == "Cargo.lock") {
                let bytes = read_small(&path)?;
                self.bytes += bytes.len();
                if self.bytes > 8_388_608 {
                    return Err(Error::Invalid("manifest snapshot exceeds 8 MiB".into()));
                }
                if name == "Cargo.toml" {
                    let value: toml::Value =
                        toml::from_str(std::str::from_utf8(&bytes).map_err(|_| Error::Toml)?)
                            .map_err(|_| Error::Toml)?;
                    let parent = Path::new(&rel).parent().ok_or(Error::Metadata)?;
                    validate_paths(&value, parent)?;
                }
                self.inputs.insert(rel, digest(&bytes));
                std::fs::write(output, bytes)?;
            } else if kind.is_file() && path.extension().is_some_and(|extension| extension == "rs")
            {
                // Names preserve implicit targets. Source contents are never needed by metadata.
                self.inputs.insert(rel, "target-path-present".into());
                std::fs::write(output, [])?;
            }
        }
        Ok(())
    }
}

/// Read metadata without dependencies, networking, installation, or repository Cargo configuration.
///
/// Validates the configuration and selected packages, targets, features, and
/// minimum Rust versions. Returns workspace members sorted by name, sorted
/// targets, input hashes/source-path markers, and lock/config presence flags.
/// Creates a temporary snapshot and runs installed rustup and Cargo; repository
/// source and build scripts are not executed and repository files are not written.
///
/// # Errors
/// Propagates configuration, path, I/O, snapshot-size, and manifest parsing
/// errors. Returns `Error::Cargo` for tool lookup or execution failures and
/// metadata process limits, `Error::Metadata` for unsupported metadata, and
/// `Error::Invalid` for incompatible selections. Oversized completed metadata
/// output also returns `Error::Invalid`.
pub fn discover(root: &Path, config: &Config) -> Result<Workspace> {
    let root = root.canonicalize()?;
    config.validate(&root)?;
    let rustup = trusted_rustup(&root)?;
    let temporary = tempfile::tempdir()?;
    let snapshot_root = temporary.path().join("workspace");
    std::fs::create_dir(&snapshot_root)?;
    let snapshot_root = snapshot_root.canonicalize()?;
    let mut snapshot = Snapshot {
        inputs: BTreeMap::new(),
        count: 0,
        bytes: 0,
        cargo_config_present: false,
    };
    snapshot.copy(&root, &root, &snapshot_root)?;
    let cargo_home = temporary.path().join("cargo-home");
    std::fs::create_dir(&cargo_home)?;
    let stdout_path = temporary.path().join("metadata.json");
    let stdout = std::fs::File::create(&stdout_path)?;
    let mut lookup = Command::new(&rustup);
    lookup
        .args(["which", "--toolchain", &config.toolchain, "cargo"])
        .current_dir(&snapshot_root)
        .env_clear()
        .env("RUSTUP_AUTO_INSTALL", "0")
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    if cfg!(unix) {
        lookup.env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
    }
    for name in ["HOME", "RUSTUP_HOME", "SYSTEMROOT"] {
        if let Some(value) = std::env::var_os(name) {
            lookup.env(name, value);
        }
    }
    let installed = lookup
        .output()
        .map_err(|_| Error::Cargo("toolchain-lookup"))?;
    if !installed.status.success() {
        return Err(Error::Cargo("toolchain-lookup"));
    }
    let cargo = PathBuf::from(
        std::str::from_utf8(&installed.stdout)
            .map_err(|_| Error::Cargo("toolchain-lookup"))?
            .trim(),
    )
    .canonicalize()?;
    if cargo.starts_with(&root) {
        return Err(Error::Cargo("toolchain-path"));
    }
    let bin = cargo.parent().ok_or(Error::Cargo("toolchain-path"))?;
    let rustc = bin.join(if cfg!(windows) { "rustc.exe" } else { "rustc" });
    if !rustc.is_file() {
        return Err(Error::Cargo("compiler-path"));
    }
    let mut command = Command::new(&cargo);
    command
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
            "--locked",
        ])
        .current_dir(&snapshot_root)
        .env_clear()
        .env("CARGO_HOME", cargo_home)
        .env("RUSTC", rustc)
        .env("RUSTUP_AUTO_INSTALL", "0")
        .stdout(stdout)
        .stderr(Stdio::null())
        .stdin(Stdio::null());
    if cfg!(unix) {
        let system_path = std::env::join_paths([
            bin,
            Path::new("/usr/bin"),
            Path::new("/bin"),
            Path::new("/usr/sbin"),
            Path::new("/sbin"),
        ])
        .map_err(|_| Error::Cargo("toolchain-path"))?;
        command.env("PATH", system_path);
    }
    for name in ["HOME", "RUSTUP_HOME", "SYSTEMROOT"] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let mut child = command
        .spawn()
        .map_err(|_| Error::Cargo("metadata-spawn"))?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                return Err(Error::Cargo("metadata-exit"));
            }
            break;
        }
        if started.elapsed() > Duration::from_secs(15)
            || std::fs::metadata(&stdout_path)?.len() > 1_048_576
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::Cargo("metadata-limit"));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let metadata: Metadata =
        serde_json::from_slice(&read_small(&stdout_path)?).map_err(|_| Error::Metadata)?;
    if metadata.workspace_root != snapshot_root {
        return Err(Error::Metadata);
    }
    let members: BTreeSet<_> = metadata.workspace_members.iter().collect();
    let mut packages = Vec::new();
    for mut entry in metadata.packages {
        if !members.contains(&entry.id) {
            continue;
        }
        relative(&snapshot_root, &entry.manifest_path)?;
        entry
            .package
            .targets
            .sort_by(|a, b| (&a.name, &a.kind).cmp(&(&b.name, &b.kind)));
        packages.push(entry.package);
    }
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    if packages.is_empty() {
        return Err(Error::Metadata);
    }
    validate_selection(config, &packages)?;
    Ok(Workspace {
        packages,
        cargo_lock_present: snapshot.inputs.contains_key("Cargo.lock"),
        inputs: snapshot.inputs,
        cargo_config_present: snapshot.cargo_config_present,
    })
}

/// Check selected packages, minimum Rust versions, and targets against metadata.
///
/// Requires a validated configuration. Required target features are checked
/// against the transitive closure of declared package features, including
/// `default` when requested; cycles terminate without an error.
///
/// Returns `Error::Invalid` for missing or ambiguous packages, incompatible
/// Rust versions, unknown selected features, or unavailable targets/features.
/// Returns `Error::Metadata` for unparseable compiler or minimum Rust versions.
///
/// # Panics
/// Panics if a deliverable references an absent feature set.
fn validate_selection(config: &Config, packages: &[Package]) -> Result<()> {
    let compiler = semver::Version::parse(&config.toolchain).map_err(|_| Error::Metadata)?;
    for deliverable in &config.deliverables {
        let candidates: Vec<_> = packages
            .iter()
            .filter(|p| p.name == deliverable.package)
            .collect();
        if candidates.len() != 1 {
            return Err(Error::Invalid(
                "selected package is missing or ambiguous".into(),
            ));
        }
        let package = candidates[0];
        if let Some(msrv) = &package.rust_version {
            let msrv = if msrv.split('.').count() == 2 {
                format!("{msrv}.0")
            } else {
                msrv.clone()
            };
            if semver::Version::parse(&msrv).map_err(|_| Error::Metadata)? > compiler {
                return Err(Error::Invalid(
                    "selected package requires a newer Rust toolchain".into(),
                ));
            }
        }
        let case = &config.feature_sets[&deliverable.feature_set];
        let mut active: BTreeSet<String> = case.features.iter().cloned().collect();
        if case
            .features
            .iter()
            .any(|f| !package.features.contains_key(f))
        {
            return Err(Error::Invalid(
                "selected feature is not declared by the package".into(),
            ));
        }
        if case.default_features && package.features.contains_key("default") {
            active.insert("default".into());
        }
        loop {
            let before = active.len();
            for feature in active.clone() {
                if let Some(edges) = package.features.get(&feature) {
                    for edge in edges {
                        if package.features.contains_key(edge) {
                            active.insert(edge.clone());
                        }
                    }
                }
            }
            if active.len() == before {
                break;
            }
        }
        let valid = package.targets.iter().any(|target| {
            let selected = match &deliverable.binary {
                Some(binary) => {
                    target.name == *binary && target.kind.iter().any(|kind| kind == "bin")
                }
                None => target.kind.iter().any(|kind| {
                    matches!(
                        kind.as_str(),
                        "lib" | "rlib" | "cdylib" | "dylib" | "staticlib" | "proc-macro"
                    )
                }),
            };
            selected && target.required_features.iter().all(|f| active.contains(f))
        });
        if !valid {
            return Err(Error::Invalid(
                "selected library/binary is missing or its required features are disabled".into(),
            ));
        }
    }
    Ok(())
}
