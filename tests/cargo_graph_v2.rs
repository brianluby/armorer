//! Independent consumer semantics for retained graph fixtures; no authentication claim.
use armorer::{
    config::Config,
    trust::{
        ByteIdentity, InputIdentity, RunIdentity, Source, WorkflowIdentity, evidence::Selection,
    },
    verification::graph::CargoGraphV2,
};
use serde_json::{Value, json};

/// Fix expectations independently of the producer graph bytes used by each case.
fn context(name: &str) -> (Config, Selection, InputIdentity, &'static str) {
    let library = name == "zero-library";
    let optional = name == "optional";
    let features = if optional {
        vec!["extra".to_owned()]
    } else {
        vec![]
    };
    let feature_set = if optional { "optional" } else { "minimal" };
    let config: Config = serde_json::from_value(json!({"schema_version":1,"repository":"fixture/graph","toolchain":"1.95.0",
        "deliverables":[{"id":"app","profile":if library {"library"} else {"cli"},"package":"app","binary":if library {None} else {Some("app")},"targets":["x86_64-unknown-linux-gnu"],"feature_set":feature_set}],
        "feature_sets":{feature_set:{"default_features":false,"features":features}},"policy":{"license_file":"LICENSE","attestations":"required"}})).unwrap();
    let selection = Selection {
        deliverable_id: "app".into(),
        profile: config.deliverables[0].profile,
        package: "app".into(),
        package_version: "0.1.0".into(),
        binary: config.deliverables[0].binary.clone(),
        target: "x86_64-unknown-linux-gnu".into(),
        feature_set: feature_set.into(),
        default_features: false,
        features,
        toolchain: "1.95.0".into(),
    };
    let inputs = InputIdentity {
        source: Source {
            repository: "fixture/graph".into(),
            commit: "a".repeat(40),
            git_ref: "refs/heads/main".into(),
        },
        config_sha256: "1".repeat(64),
        lock_sha256: "2".repeat(64),
        cargo_lock_sha256: "3".repeat(64),
        runtime: ByteIdentity {
            sha256: "4".repeat(64),
            size: 7,
        },
        runtime_version: "0.1.0".into(),
        run: RunIdentity {
            id: 17,
            attempt: 2,
            workflow: WorkflowIdentity {
                repository: "fixture/workflows".into(),
                path: ".github/workflows/rust-build-v2.yml".into(),
                commit: "b".repeat(40),
            },
        },
    };
    (
        config,
        selection,
        inputs,
        if library { "custom_library" } else { "app" },
    )
}

/// Read the retained Python-writer outputs; expectations are fixed separately above.
fn fixture(name: &str) -> (CargoGraphV2, Value) {
    let (graph, bom) = match name {
        "minimal" => (
            include_str!("fixtures/cargo-graph-v2/minimal.graph.json"),
            include_str!("fixtures/cargo-graph-v2/minimal.cdx.json"),
        ),
        "optional" => (
            include_str!("fixtures/cargo-graph-v2/optional.graph.json"),
            include_str!("fixtures/cargo-graph-v2/optional.cdx.json"),
        ),
        "zero-library" => (
            include_str!("fixtures/cargo-graph-v2/zero-library.graph.json"),
            include_str!("fixtures/cargo-graph-v2/zero-library.cdx.json"),
        ),
        _ => panic!("unknown fixture"),
    };
    (
        serde_json::from_str(graph).unwrap(),
        serde_json::from_str(bom).unwrap(),
    )
}

/// Require every fixture to match source/run/selection and all published SBOM graph edges.
#[test]
fn retained_python_graphs_match_independent_rust_expectations() {
    for name in ["minimal", "optional", "zero-library"] {
        let (graph, bom) = fixture(name);
        let (config, selection, inputs, root_name) = context(name);
        graph
            .validate_against(&config, &selection, root_name, &inputs, &"b".repeat(40))
            .unwrap();
        graph.compare_sbom(&bom).unwrap();
    }
}

/// Reject source/run/lock/selection substitution even when graph and SBOM agree with each other.
#[test]
fn producer_graph_cannot_select_its_own_consumer_context() {
    let (graph, _) = fixture("optional");
    let (config, selection, inputs, root_name) = context("optional");
    let original = serde_json::to_value(graph).unwrap();
    for (pointer, value) in [
        ("/schema_version", json!(1)),
        ("/source/repository", json!("attacker/graph")),
        ("/source/commit", json!("c".repeat(40))),
        ("/runtime_commit", json!("c".repeat(40))),
        ("/run_id", json!(18)),
        ("/run_attempt", json!(3)),
        ("/run_id", Value::Null),
        ("/input_sha256/armorer.toml", json!("5".repeat(64))),
        ("/input_sha256/armorer.lock", json!("5".repeat(64))),
        ("/input_sha256/Cargo.lock", json!("5".repeat(64))),
        ("/selection/target", json!("aarch64-unknown-linux-gnu")),
        ("/selection/features", json!([])),
        ("/selection/default_features", json!(true)),
        ("/selection/profile", json!("service")),
        ("/selection/deliverable_id", json!("other")),
        ("/selection/binary", json!("other")),
        ("/selection/package_version", json!("0.2.0")),
        ("/selection/toolchain", json!("1.94.0")),
        ("/root_component_name", json!("other")),
        ("/coverage_gaps", json!([])),
        ("/nodes/0/features", json!([])),
    ] {
        let mut mutated = original.clone();
        *mutated.pointer_mut(pointer).unwrap() = value;
        let parsed: CargoGraphV2 = serde_json::from_value(mutated).unwrap();
        assert!(
            parsed
                .validate_against(&config, &selection, root_name, &inputs, &"b".repeat(40))
                .is_err(),
            "{pointer}"
        );
    }
}

/// Reject schema-valid identity/edge tampering, partial inventories and ambiguous references.
#[test]
fn sbom_changes_cannot_hide_behind_valid_existing_package_references() {
    let (graph, bom) = fixture("optional");
    for (pointer, value) in [
        ("/metadata/component/bom-ref", json!("dep")),
        ("/metadata/component/name", json!("other")),
        ("/metadata/component/version", json!("0.2.0")),
        ("/metadata/component/type", json!("file")),
        ("/metadata/component/type", json!("library")),
        ("/components/0/version", json!("0.2.0")),
        ("/components/0/name", json!("other")),
        ("/components/0/bom-ref", json!("app")),
        ("/components/0/type", json!("file")),
        ("/components", json!([])),
        ("/dependencies", json!([])),
        ("/dependencies/0/dependsOn", json!(["app", "dep"])),
        ("/dependencies/0/dependsOn", json!(["dep", "dep"])),
        ("/dependencies/1/ref", json!("app")),
        ("/specVersion", json!("1.6")),
        ("/version", json!(0)),
    ] {
        let mut mutated = bom.clone();
        *mutated.pointer_mut(pointer).unwrap() = value;
        assert!(graph.compare_sbom(&mutated).is_err(), "{pointer}");
    }
    let mut nested = bom.clone();
    nested["components"][0]["components"] = json!([{"bom-ref":"hidden"}]);
    assert!(graph.compare_sbom(&nested).is_err());
    let mut excluded = bom.clone();
    let dependency = excluded["components"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["bom-ref"] == "dep")
        .unwrap();
    dependency["scope"] = json!("excluded");
    assert!(graph.compare_sbom(&excluded).is_err());
    let mut extra = bom.clone();
    extra["dependencies"][0]["provides"] = json!(["hidden"]);
    assert!(graph.compare_sbom(&extra).is_err());
}

/// Reject legacy, duplicate, dangling, unreachable, dev-only and unsupported graph records.
#[test]
fn retained_graph_must_be_complete_and_explicitly_versioned() {
    let (graph, _) = fixture("optional");
    let (config, selection, inputs, root_name) = context("optional");
    let original = serde_json::to_value(graph).unwrap();
    for (pointer, value) in [
        ("/packages", json!([])),
        ("/nodes", json!([])),
        ("/root", json!("missing")),
        ("/root", json!("dep")),
        ("/packages/0/name", json!("other")),
        ("/packages/0/version", json!("0.2.0")),
        ("/packages/1/id", json!("app")),
        ("/nodes/1/id", json!("app")),
        ("/nodes/0/dependencies", json!([])),
        ("/nodes/0/dependencies/0/package", json!("missing")),
        ("/nodes/0/dependencies/0/contexts", json!([])),
        ("/nodes/0/features", json!(["extra", "extra"])),
    ] {
        let mut mutated = original.clone();
        *mutated.pointer_mut(pointer).unwrap() = value;
        let parsed: CargoGraphV2 = serde_json::from_value(mutated).unwrap();
        assert!(
            parsed
                .validate_against(&config, &selection, root_name, &inputs, &"b".repeat(40))
                .is_err(),
            "{pointer}"
        );
    }
    let mut dev = original.clone();
    dev["nodes"][0]["dependencies"][0]["contexts"][0]["kind"] = json!("dev");
    assert!(serde_json::from_value::<CargoGraphV2>(dev).is_err());
    let mut unknown = original.clone();
    unknown["caller_command"] = json!("build");
    assert!(serde_json::from_value::<CargoGraphV2>(unknown).is_err());
    assert!(serde_json::from_value::<CargoGraphV2>(json!({"compiled_packages":{"app":[]},"native_linkage":[],"graph_scope":"compiled-cargo-target-and-host-build-dependencies"})).is_err());
}

/// Bind build-only and mixed runtime/build component scopes to the graph rather than SBOM claims.
#[test]
fn retained_build_paths_control_component_scope() {
    let (mut graph, mut bom) = fixture("optional");
    let host = bom["components"]
        .as_array()
        .unwrap()
        .iter()
        .position(|c| c["bom-ref"] == "host")
        .unwrap();
    assert_eq!(bom["components"][host]["scope"], "excluded");
    for scope in [Some("required"), Some("optional"), None] {
        let mut changed = bom.clone();
        if let Some(scope) = scope {
            changed["components"][host]["scope"] = json!(scope);
        } else {
            changed["components"][host]
                .as_object_mut()
                .unwrap()
                .remove("scope");
        }
        assert!(graph.compare_sbom(&changed).is_err());
    }
    let root = graph.nodes.iter().position(|n| n.id == "app").unwrap();
    let host_node = graph.nodes.iter().position(|n| n.id == "host").unwrap();
    let normal = graph.nodes[root].dependencies.remove(0);
    graph.nodes[host_node].dependencies.push(normal.clone());
    let app_edges = bom["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .position(|n| n["ref"] == "app")
        .unwrap();
    let host_edges = bom["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .position(|n| n["ref"] == "host")
        .unwrap();
    bom["dependencies"][app_edges]["dependsOn"] = json!(["host"]);
    bom["dependencies"][host_edges]["dependsOn"] = json!(["dep"]);
    let dep = bom["components"]
        .as_array()
        .unwrap()
        .iter()
        .position(|c| c["bom-ref"] == "dep")
        .unwrap();
    bom["components"][dep]["scope"] = json!("excluded");
    graph.compare_sbom(&bom).unwrap();
    bom["components"][dep]["scope"] = json!("required");
    assert!(graph.compare_sbom(&bom).is_err());
    graph.nodes[root].dependencies.push(normal);
    bom["dependencies"][app_edges]["dependsOn"] = json!(["host", "dep"]);
    graph.compare_sbom(&bom).unwrap();
}

/// Reconcile real retained Python/Cargo output against separate fixture config and run context.
/// This explicit integration check establishes interoperability, never authentication.
#[test]
#[ignore = "requires retained native fixture outputs; run explicitly with ARMORER_TEST_GRAPH_RECEIPTS"]
fn native_python_graphs_match_independent_rust_reader() {
    use sha2::{Digest, Sha256};
    use std::{fs, path::PathBuf};
    let directory = PathBuf::from(
        std::env::var_os("ARMORER_TEST_GRAPH_RECEIPTS").expect("native fixture receipts required"),
    );
    assert!(directory.is_absolute());
    let receipt: Value =
        serde_json::from_slice(&fs::read(directory.join("receipt.json")).unwrap()).unwrap();
    assert_eq!(receipt["attested"], false);
    assert_eq!(receipt["run_identity"], "synthetic fixture-only 17/2");
    let target = receipt["native_target"].as_str().unwrap();
    assert!(armorer::config::TARGETS.contains(&target));
    let config_bytes = fs::read(directory.join("armorer.toml")).unwrap();
    let config: Config = toml::from_str(std::str::from_utf8(&config_bytes).unwrap()).unwrap();
    config.validate(&directory).unwrap();
    assert_eq!(config.repository, "fixture/build");
    let lock_bytes = fs::read(directory.join("armorer.lock")).unwrap();
    let lock: armorer::config::Lock =
        toml::from_str(std::str::from_utf8(&lock_bytes).unwrap()).unwrap();
    let hash = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
    let workflow_commit = receipt["runtime_commit"].as_str().unwrap();
    assert_eq!(lock.workflows.commit, workflow_commit);
    let inputs = InputIdentity {
        source: Source {
            repository: config.repository.clone(),
            commit: receipt["source_commit"].as_str().unwrap().into(),
            git_ref: "refs/heads/main".into(),
        },
        config_sha256: hash(&config_bytes),
        lock_sha256: hash(&lock_bytes),
        cargo_lock_sha256: hash(&fs::read(directory.join("Cargo.lock")).unwrap()),
        // Synthetic runtime byte identity is unused by the semantic graph check.
        runtime: ByteIdentity {
            sha256: "4".repeat(64),
            size: 7,
        },
        runtime_version: "0.1.0".into(),
        run: RunIdentity {
            id: 17,
            attempt: 2,
            workflow: WorkflowIdentity {
                repository: "brianluby/armorer-workflows".into(),
                path: ".github/workflows/rust-build-v2.yml".into(),
                commit: workflow_commit.into(),
            },
        },
    };
    for (id, feature_set) in [
        ("minimal", "minimal"),
        ("extra", "extra"),
        ("zero", "minimal"),
        ("service", "service"),
    ] {
        let deliverable = config.deliverables.iter().find(|d| d.id == id).unwrap();
        let features = &config.feature_sets[feature_set];
        let selection = Selection {
            deliverable_id: id.into(),
            profile: deliverable.profile,
            package: deliverable.package.clone(),
            package_version: "0.1.0".into(),
            binary: deliverable.binary.clone(),
            target: target.into(),
            feature_set: feature_set.into(),
            default_features: features.default_features,
            features: features.features.clone(),
            toolchain: config.toolchain.clone(),
        };
        let key = format!("{id}--{target}--{feature_set}");
        let graph: CargoGraphV2 = serde_json::from_slice(
            &fs::read(directory.join(id).join(format!("{key}.cargo-graph.json"))).unwrap(),
        )
        .unwrap();
        let bom: Value = serde_json::from_slice(
            &fs::read(directory.join(id).join(format!("{key}.cdx.json"))).unwrap(),
        )
        .unwrap();
        let root_name = if id == "zero" {
            "custom_zero"
        } else {
            "fixture-app"
        };
        graph
            .validate_against(&config, &selection, root_name, &inputs, workflow_commit)
            .unwrap();
        graph.compare_sbom(&bom).unwrap();
        if id != "zero" {
            let mut altered = bom.clone();
            let root = altered["dependencies"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|n| n["ref"] == graph.root)
                .unwrap();
            root["dependsOn"] = json!([]);
            assert!(graph.compare_sbom(&altered).is_err());
        }
    }
}

/// Reject an independently supplied run workflow from another reviewed catalog snapshot.
#[test]
fn graph_and_expected_run_share_the_approved_workflow_commit() {
    let (graph, _) = fixture("optional");
    let (config, selection, mut inputs, root_name) = context("optional");
    inputs.run.workflow.commit = "c".repeat(40);
    inputs.validate().unwrap();
    assert!(
        graph
            .validate_against(&config, &selection, root_name, &inputs, &"b".repeat(40))
            .is_err()
    );
}

/// Literal old contexts reject default/all-feature expansion instead of trusting a requested-name subset.
#[test]
fn feature_expanded_graphs_cannot_satisfy_literal_legacy_selections() {
    for name in ["minimal", "optional", "zero-library"] {
        let (graph, _) = fixture(name);
        let (config, selection, inputs, root_name) = context(name);
        for extra in [
            vec!["default"],
            vec!["unapproved"],
            vec!["default", "unapproved"],
        ] {
            let mut changed = graph.clone();
            let root = changed.root.clone();
            changed
                .nodes
                .iter_mut()
                .find(|node| node.id == root)
                .unwrap()
                .features
                .extend(extra.into_iter().map(str::to_owned));
            assert_eq!(
                changed
                    .validate_against(&config, &selection, root_name, &inputs, &"b".repeat(40))
                    .unwrap_err()
                    .to_string(),
                "invalid configuration: cargo-graph-unapproved-root-feature"
            );
        }
        let mut config = config;
        let mut selection = selection;
        let mut graph = graph;
        config
            .feature_sets
            .get_mut(&selection.feature_set)
            .unwrap()
            .default_features = true;
        selection.default_features = true;
        graph.selection = selection.clone();
        assert_eq!(
            graph
                .validate_against(&config, &selection, root_name, &inputs, &"b".repeat(40))
                .unwrap_err()
                .to_string(),
            "invalid configuration: cargo-graph-resolved-feature-expectation-required"
        );
    }
}

/// Explicit expectations allow legitimate implied features but reject missing, extra and malformed sets.
#[test]
fn independently_approved_feature_closure_is_compared_exactly() {
    let (mut graph, _) = fixture("optional");
    let (config, selection, inputs, root_name) = context("optional");
    let expected = vec!["extra".to_owned(), "implied".to_owned()];
    let root = graph.root.clone();
    graph
        .nodes
        .iter_mut()
        .find(|node| node.id == root)
        .unwrap()
        .features = expected.clone();
    graph
        .validate_against_with_resolved_features(
            &config,
            &selection,
            root_name,
            &inputs,
            &"b".repeat(40),
            &expected,
        )
        .unwrap();
    for actual in [
        vec!["extra"],
        vec!["extra", "implied", "unapproved"],
        vec!["default", "extra", "implied"],
    ] {
        graph
            .nodes
            .iter_mut()
            .find(|node| node.id == root)
            .unwrap()
            .features = actual.into_iter().map(str::to_owned).collect();
        assert!(
            graph
                .validate_against_with_resolved_features(
                    &config,
                    &selection,
                    root_name,
                    &inputs,
                    &"b".repeat(40),
                    &expected
                )
                .is_err()
        );
    }
    for malformed in [
        vec![],
        vec!["implied"],
        vec!["extra", "extra"],
        vec!["implied", "extra"],
        vec!["extra", "bad\nfeature"],
    ] {
        let malformed = malformed.into_iter().map(str::to_owned).collect::<Vec<_>>();
        assert_eq!(
            graph
                .validate_against_with_resolved_features(
                    &config,
                    &selection,
                    root_name,
                    &inputs,
                    &"b".repeat(40),
                    &malformed
                )
                .unwrap_err()
                .to_string(),
            "invalid configuration: invalid-approved-root-features"
        );
    }
}

/// Default activation is accepted only when every actual member matches the separately approved set.
#[test]
fn default_feature_decision_uses_independent_resolved_expectations() {
    let (mut graph, _) = fixture("minimal");
    let (mut config, mut selection, inputs, root_name) = context("minimal");
    config
        .feature_sets
        .get_mut(&selection.feature_set)
        .unwrap()
        .default_features = true;
    selection.default_features = true;
    graph.selection = selection.clone();
    let expected = vec!["base".to_owned(), "default".to_owned()];
    let root = graph.root.clone();
    graph
        .nodes
        .iter_mut()
        .find(|node| node.id == root)
        .unwrap()
        .features = expected.clone();
    graph
        .validate_against_with_resolved_features(
            &config,
            &selection,
            root_name,
            &inputs,
            &"b".repeat(40),
            &expected,
        )
        .unwrap();
    for actual in [
        vec![],
        vec!["default"],
        vec!["base", "default", "unapproved"],
    ] {
        graph
            .nodes
            .iter_mut()
            .find(|node| node.id == root)
            .unwrap()
            .features = actual.into_iter().map(str::to_owned).collect();
        assert!(
            graph
                .validate_against_with_resolved_features(
                    &config,
                    &selection,
                    root_name,
                    &inputs,
                    &"b".repeat(40),
                    &expected
                )
                .is_err()
        );
    }
}

/// Real Cargo metadata resolves implied/default features and an all-features substitution fails the independent reader.
#[test]
fn real_cargo_resolution_distinguishes_requested_flags_from_resolved_features() {
    use std::{fs, process::Command};
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/lib.rs"), b"pub fn owned_fixture() {}\n").unwrap();
    fs::write(
        root.join("build.rs"),
        b"compile_error!(\"metadata must not build this fixture\");\n",
    )
    .unwrap();
    fs::write(root.join("Cargo.toml"), b"[package]\nname=\"app\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[lib]\nname=\"custom_library\"\n[features]\ndefault=[\"base\"]\nbase=[]\nextra=[\"implied\"]\nimplied=[]\n").unwrap();
    let lock = b"version = 4\n[[package]]\nname = \"app\"\nversion = \"0.1.0\"\n";
    fs::write(root.join("Cargo.lock"), lock).unwrap();
    let cargo = std::path::Path::new(env!("CARGO"));
    for (flags, expected) in [
        (vec!["--no-default-features"], vec![]),
        (
            vec!["--no-default-features", "--features", "extra"],
            vec!["extra", "implied"],
        ),
        (
            vec!["--features", "extra"],
            vec!["base", "default", "extra", "implied"],
        ),
        (
            vec!["--no-default-features", "--all-features"],
            vec!["base", "default", "extra", "implied"],
        ),
    ] {
        let output = Command::new(cargo)
            .args(["metadata", "--format-version=1", "--locked", "--offline"])
            .args(flags)
            .current_dir(root)
            .env_clear()
            .env("HOME", root)
            .env("CARGO_HOME", root.join("cargo-home"))
            .env("RUSTC", cargo.with_file_name("rustc"))
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();
        assert!(output.status.success(), "owned feature metadata failed");
        let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
        let actual = metadata["resolve"]["nodes"][0]["features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        let expected = expected.into_iter().map(str::to_owned).collect::<Vec<_>>();
        assert_eq!(actual, expected);
        let (mut graph, _) = fixture("zero-library");
        let (config, selection, inputs, root_name) = context("zero-library");
        graph.nodes[0].features = actual;
        // The independently selected minimal variant expects no activated features.
        assert_eq!(
            graph
                .validate_against_with_resolved_features(
                    &config,
                    &selection,
                    root_name,
                    &inputs,
                    &"b".repeat(40),
                    &[]
                )
                .is_ok(),
            expected.is_empty()
        );
    }
    assert_eq!(fs::read(root.join("Cargo.lock")).unwrap(), lock);
    assert!(!root.join("target").exists());
}
