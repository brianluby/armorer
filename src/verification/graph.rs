//! Version-two selected Cargo graph and SBOM reconciliation.
//!
//! These methods check semantics only. The complete consumer must authenticate
//! both graph and SBOM bytes before using them, and obtain every expected input
//! and selection from independently approved context. Version-one feature-only
//! graph output cannot satisfy this contract.

use crate::{
    Result,
    config::{Config, Profile, hex_digest},
    trust::{InputIdentity, evidence::Selection, require},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const MAX_PACKAGES: usize = 4096;
const MAX_EDGES: usize = 65536;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum GraphScope {
    CompiledCargoTargetAndHostBuildDependencies,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum GraphGap {
    NativeAndSystemLibrariesNotFullyInventoried,
    CargoHostTargetAggregatedByPackageId,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GraphSource {
    pub repository: String,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GraphInputs {
    #[serde(rename = "armorer.toml")]
    pub config_sha256: String,
    #[serde(rename = "armorer.lock")]
    pub lock_sha256: String,
    #[serde(rename = "Cargo.lock")]
    pub cargo_lock_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CargoPackage {
    pub id: String,
    pub name: String,
    pub version: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyKind {
    Normal,
    Build,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DependencyContext {
    pub kind: DependencyKind,
    pub target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CargoDependency {
    pub package: String,
    pub name: String,
    pub contexts: Vec<DependencyContext>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CargoNode {
    pub id: String,
    pub features: Vec<String>,
    pub dependencies: Vec<CargoDependency>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NativeLinkage {
    pub package_id: String,
    pub linked_libraries: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CargoGraphV2 {
    #[schemars(range(min = 2, max = 2))]
    pub schema_version: u32,
    pub source: GraphSource,
    pub runtime_commit: String,
    /// Null identifies an unhosted development fixture and cannot satisfy release verification.
    pub run_id: Option<u64>,
    pub run_attempt: Option<u64>,
    pub input_sha256: GraphInputs,
    pub selection: Selection,
    pub root_component_name: String,
    pub root: String,
    pub packages: Vec<CargoPackage>,
    pub nodes: Vec<CargoNode>,
    pub native_linkage: Vec<NativeLinkage>,
    pub graph_scope: GraphScope,
    pub coverage_gaps: Vec<GraphGap>,
}

/// Borrow validated indexes without copying producer graph contents.
struct GraphIndex<'a> {
    packages: BTreeMap<&'a str, &'a CargoPackage>,
    nodes: BTreeMap<&'a str, &'a CargoNode>,
}

/// Accept bounded opaque metadata text without treating it as a path or command.
fn text(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

/// Read a required JSON string without retaining or displaying untrusted error contents.
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| crate::Error::Invalid("sbom-string-field-missing".into()))
}

/// Require a flat component whose scope matches independently derived dependency paths.
fn flat_component_scope(value: &Value, expected: &str) -> bool {
    value.is_object()
        && value
            .get("scope")
            .map_or(expected == "required", |s| s.as_str() == Some(expected))
        && value
            .get("components")
            .is_none_or(|v| v.as_array().is_some_and(Vec::is_empty))
}

impl CargoGraphV2 {
    /// Bind the retained graph to independently approved source, run, locks and selection.
    /// The reusable workflow commit must come from the independently approved lock.
    pub fn validate_against(
        &self,
        config: &Config,
        selection: &Selection,
        root_component_name: &str,
        inputs: &InputIdentity,
        workflow_commit: &str,
    ) -> Result<()> {
        inputs.validate()?;
        selection.validate_against(config)?;
        require(self.schema_version == 2, "unsupported-cargo-graph-version")?;
        require(
            self.selection == *selection
                && self.root_component_name == root_component_name
                && text(root_component_name, 1024),
            "cargo-graph-selection-mismatch",
        )?;
        require(
            self.source.repository == inputs.source.repository
                && self.source.commit == inputs.source.commit
                && hex_digest(workflow_commit, 40)
                && self.runtime_commit == workflow_commit
                && self.run_id == Some(inputs.run.id)
                && self.run_attempt == Some(inputs.run.attempt)
                && self.input_sha256.config_sha256 == inputs.config_sha256
                && self.input_sha256.lock_sha256 == inputs.lock_sha256
                && self.input_sha256.cargo_lock_sha256 == inputs.cargo_lock_sha256,
            "cargo-graph-input-mismatch",
        )?;
        require(
            self.coverage_gaps
                == [
                    GraphGap::NativeAndSystemLibrariesNotFullyInventoried,
                    GraphGap::CargoHostTargetAggregatedByPackageId,
                ],
            "cargo-graph-coverage-mismatch",
        )?;
        let GraphIndex { packages, nodes } = self.index()?;
        let root = packages
            .get(self.root.as_str())
            .ok_or_else(|| crate::Error::Invalid("cargo-graph-root-missing".into()))?;
        require(
            root.name == selection.package && root.version == selection.package_version,
            "cargo-graph-root-identity-mismatch",
        )?;
        let root_node = nodes
            .get(self.root.as_str())
            .ok_or_else(|| crate::Error::Invalid("cargo-graph-root-node-missing".into()))?;
        require(
            selection
                .features
                .iter()
                .all(|f| root_node.features.contains(f)),
            "cargo-graph-requested-feature-missing",
        )
    }

    /// Construct unique package/node indexes and reject dangling, unreachable or oversized graphs.
    fn index(&self) -> Result<GraphIndex<'_>> {
        require(self.schema_version == 2, "unsupported-cargo-graph-version")?;
        require(
            !self.packages.is_empty()
                && self.packages.len() <= MAX_PACKAGES
                && self.nodes.len() == self.packages.len(),
            "cargo-graph-package-budget",
        )?;
        let mut packages = BTreeMap::new();
        for package in &self.packages {
            require(
                text(&package.id, 4096)
                    && text(&package.name, 100)
                    && text(&package.version, 100)
                    && semver::Version::parse(&package.version)
                        .is_ok_and(|v| v.to_string() == package.version)
                    && packages.insert(package.id.as_str(), package).is_none(),
                "invalid-or-duplicate-cargo-package",
            )?;
        }
        let root_package = packages
            .get(self.root.as_str())
            .ok_or_else(|| crate::Error::Invalid("cargo-graph-root-missing".into()))?;
        require(
            root_package.name == self.selection.package
                && root_package.version == self.selection.package_version
                && text(&self.root_component_name, 1024),
            "cargo-graph-root-identity-mismatch",
        )?;
        let mut nodes = BTreeMap::new();
        let mut edge_count = 0_usize;
        for node in &self.nodes {
            require(
                packages.contains_key(node.id.as_str())
                    && nodes.insert(node.id.as_str(), node).is_none(),
                "invalid-or-duplicate-cargo-node",
            )?;
            let mut features = BTreeSet::new();
            require(
                node.features.len() <= 1024
                    && node
                        .features
                        .iter()
                        .all(|f| text(f, 100) && features.insert(f)),
                "invalid-cargo-feature-inventory",
            )?;
            let mut dependencies = BTreeSet::new();
            for edge in &node.dependencies {
                edge_count += 1;
                require(
                    edge_count <= MAX_EDGES
                        && packages.contains_key(edge.package.as_str())
                        && edge.package != node.id
                        && text(&edge.name, 256)
                        && dependencies.insert((&edge.package, &edge.name)),
                    "invalid-or-duplicate-cargo-edge",
                )?;
                let mut contexts = BTreeSet::new();
                require(
                    !edge.contexts.is_empty()
                        && edge.contexts.len() <= 128
                        && edge.contexts.iter().all(|c| {
                            c.target.as_ref().is_none_or(|t| text(t, 4096)) && contexts.insert(c)
                        }),
                    "invalid-cargo-edge-context",
                )?;
            }
        }
        require(
            nodes.contains_key(self.root.as_str()),
            "cargo-graph-root-node-missing",
        )?;
        let mut reachable = BTreeSet::new();
        let mut pending = vec![self.root.as_str()];
        while let Some(id) = pending.pop() {
            if reachable.insert(id) {
                let node = nodes
                    .get(id)
                    .ok_or_else(|| crate::Error::Invalid("cargo-graph-node-missing".into()))?;
                pending.extend(node.dependencies.iter().map(|e| e.package.as_str()));
            }
        }
        require(
            reachable.len() == packages.len(),
            "cargo-graph-unreachable-package",
        )?;
        require(
            self.native_linkage.len() <= MAX_PACKAGES,
            "cargo-graph-native-budget",
        )?;
        for native in &self.native_linkage {
            require(
                packages.contains_key(native.package_id.as_str())
                    && !native.linked_libraries.is_empty()
                    && native.linked_libraries.len() <= 1024
                    && native.linked_libraries.iter().all(|l| text(l, 4096)),
                "invalid-native-linkage-evidence",
            )?;
        }
        Ok(GraphIndex { packages, nodes })
    }

    /// Match the pinned generator's normal-path classification, including mixed host/runtime use.
    fn runtime_packages<'a>(
        &'a self,
        nodes: &BTreeMap<&'a str, &'a CargoNode>,
    ) -> BTreeSet<&'a str> {
        let mut required = BTreeSet::new();
        let mut pending = vec![self.root.as_str()];
        while let Some(id) = pending.pop() {
            if required.insert(id) {
                pending.extend(
                    nodes[id]
                        .dependencies
                        .iter()
                        .filter(|edge| {
                            edge.contexts
                                .iter()
                                .any(|context| context.kind == DependencyKind::Normal)
                        })
                        .map(|edge| edge.package.as_str()),
                );
            }
        }
        required
    }

    /// Compare one flat CycloneDX 1.5 Cargo SBOM against every retained identity and dependency edge.
    /// This is graph reconciliation; it does not replace independent whole-schema validation.
    pub fn compare_sbom(&self, bom: &Value) -> Result<()> {
        let GraphIndex { packages, nodes } = self.index()?;
        require(
            bom.is_object()
                && string(bom, "bomFormat")? == "CycloneDX"
                && string(bom, "specVersion")? == "1.5"
                && bom
                    .get("version")
                    .and_then(Value::as_u64)
                    .is_some_and(|n| n > 0),
            "invalid-cargo-sbom-format",
        )?;
        let root = bom
            .get("metadata")
            .and_then(|m| m.get("component"))
            .ok_or_else(|| crate::Error::Invalid("cargo-sbom-root-missing".into()))?;
        require(
            string(root, "bom-ref")? == self.root
                && string(root, "name")? == self.root_component_name
                && string(root, "version")? == self.selection.package_version
                && string(root, "type")?
                    == match self.selection.profile {
                        Profile::Library => "library",
                        Profile::Cli | Profile::Service => "application",
                    },
            "cargo-sbom-root-mismatch",
        )?;
        require(
            flat_component_scope(root, "required"),
            "nested-cargo-sbom-component-unsupported",
        )?;
        let empty = Vec::new();
        let components = match bom.get("components") {
            None => &empty,
            Some(v) => v
                .as_array()
                .ok_or_else(|| crate::Error::Invalid("invalid-cargo-sbom-components".into()))?,
        };
        let required_packages = self.runtime_packages(&nodes);
        let mut references = BTreeSet::from([self.root.as_str()]);
        for component in components {
            let id = string(component, "bom-ref")?;
            let expected = packages
                .get(id)
                .ok_or_else(|| crate::Error::Invalid("unexpected-cargo-sbom-component".into()))?;
            require(
                references.insert(id)
                    && string(component, "type")? == "library"
                    && string(component, "name")? == expected.name
                    && string(component, "version")? == expected.version
                    && flat_component_scope(
                        component,
                        if required_packages.contains(id) {
                            "required"
                        } else {
                            "excluded"
                        },
                    ),
                "cargo-sbom-component-mismatch",
            )?;
        }
        require(
            references.len() == packages.len(),
            "cargo-sbom-component-set-mismatch",
        )?;
        let dependencies = bom
            .get("dependencies")
            .and_then(Value::as_array)
            .ok_or_else(|| crate::Error::Invalid("cargo-sbom-dependencies-missing".into()))?;
        require(
            dependencies.len() == nodes.len(),
            "cargo-sbom-dependency-set-mismatch",
        )?;
        let mut dependency_ids = BTreeSet::new();
        for dependency in dependencies {
            let id = string(dependency, "ref")?;
            let expected = nodes
                .get(id)
                .ok_or_else(|| crate::Error::Invalid("unexpected-cargo-sbom-dependency".into()))?;
            require(
                dependency_ids.insert(id)
                    && dependency
                        .as_object()
                        .is_some_and(|o| o.keys().all(|k| k == "ref" || k == "dependsOn")),
                "ambiguous-cargo-sbom-dependency",
            )?;
            let expected_edges: BTreeSet<_> = expected
                .dependencies
                .iter()
                .map(|e| e.package.as_str())
                .collect();
            let edges = match dependency.get("dependsOn") {
                None => &empty,
                Some(v) => v
                    .as_array()
                    .ok_or_else(|| crate::Error::Invalid("invalid-cargo-sbom-edges".into()))?,
            };
            let mut actual_edges = BTreeSet::new();
            for edge in edges {
                let value = edge
                    .as_str()
                    .ok_or_else(|| crate::Error::Invalid("invalid-cargo-sbom-edge".into()))?;
                require(actual_edges.insert(value), "duplicate-cargo-sbom-edge")?;
            }
            require(actual_edges == expected_edges, "cargo-sbom-edge-mismatch")?;
        }
        Ok(())
    }
}
