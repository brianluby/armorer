# Selected Cargo graph version 2

`verification::graph::CargoGraphV2` checks a retained selected Cargo graph and
its paired CycloneDX 1.5 SBOM. These are semantic checks. Authenticate both
files before release acceptance; obtain the expected source, workflow, run,
configuration/lock digests, selection and root target name independently.
The complete `AuthenticatedReleaseFiles::verify` consumer authenticates the
inventory before applying `validate_against` and `compare_sbom` to every selected
graph and its paired SBOM. `armorer verify-release` exposes that consumer through
[an explicitly approved context](verify-release-cli.md). Operational Build L2
acceptance still requires the producer, protected signing and rehearsal gates.

## Explicit successor

`armorer schema cargo-graph-v2` emits
[`cargo-graph-v2.json`](../schemas/cargo-graph-v2.json). Frozen v1 schemas retain
their original bytes. The workflow repository's explicit `rust-build-v2.yml`
entry point writes build-inventory version 2 with `cargo_graph_version: 2`;
the original `rust-build.yml` and feature-only graph format remain version 1.
Adopting the successor requires a reviewed full workflow commit and lock update.
No reader infers v2 from an unversioned v1 graph or retries v1 after rejection.

The v2 graph retains source repository/commit, reusable runtime commit, run and
attempt, SHA-256 for `armorer.toml`, `armorer.lock` and `Cargo.lock`, exact typed
selection, actual root Cargo target name, opaque Cargo package IDs, canonical
package versions, activated features and normal/build dependency contexts.
It includes every selected compiled package and its dependency edges. A
zero-dependency library remains one root package/node; its Cargo library name
may differ from its package name. Dependency versions may have legitimate
prerelease/build metadata; the selected root follows v1's stable version policy.

`validate_against` requires hosted run/attempt identities and compares the
expected source, input digests, workflow commit and selection. The expected
run workflow commit must equal the approved graph workflow commit: the v1
catalog binds baseline adapters to one workflow repository commit. Null run values
identify unhosted development output and cannot pass this release-context check.
The graph's run identity does not itself establish GitHub platform authenticity.

## SBOM reconciliation

`compare_sbom` checks all root and dependency identities, the complete component
and dependency-node sets, and every edge. Duplicate, missing, dangling, extra or
nested graph components fail. It does not replace whole-document CycloneDX
schema validation: the writer also uses the independently pinned native
CycloneDX CLI, and a complete consumer must retain that separate gate.

The pinned [cargo-cyclonedx 0.5.9 generator](https://github.com/CycloneDX/cyclonedx-rust-cargo/blob/e58bd5590212f82c5b7e16dd3e2e819b0dbea5b1/cargo-cyclonedx/src/generator.rs)
marks packages reachable through only normal dependencies as `required`.
Build-only packages and their descendants have `excluded` runtime scope, while
remaining mandatory components and nodes in the build graph and SBOM. A package
with both build and normal paths is `required`. Both readers derive this rule
from graph edges, rejecting scope substitutions and unsupported `optional`
scope. An omitted scope defaults to `required`; it cannot disguise a build-only
package. No build package is omitted from reconciliation because it is excluded
from runtime scope.

## Bounds and coverage

Semantic limits are 4,096 packages, 65,536 edges, 1,024 features per node,
128 contexts per edge and 4,096 native linkage records. Opaque IDs are at most
4,096 bytes; package names/versions and features are bounded. The trusted
consumer must additionally use bounded, duplicate-free regular-file parsing
and authenticate the exact graph/SBOM bytes; deserializing this public type alone
does not provide the offline verifier's file/JSON bounds.

The graph always declares `native-and-system-libraries-not-fully-inventoried`
and `cargo-host-target-aggregated-by-package-id`. Compiler messages come from
the consuming build environment. Retaining them does not provide platform
isolation or a complete native inventory. Unsigned macOS builds need the
separate protected signing/notarization and final-byte attestation gates.

## Validation

Ordinary `cargo test --locked --test cargo_graph_v2` checks byte-identical
Python-writer fixtures with independent Rust expectations and graph/SBOM
mutations. `scripts/validate-graph-v2.py` checks the generated schema and
fixtures using the existing hash-locked development schema environment.

For an explicit real native interoperability check, first run the workflow
repository's `tests/build_cases_v2.py` with a reviewed Armorer binary, a new tool
directory and `--receipt-directory /absolute/path/to/new-receipts`. It builds
minimal and optional CLI, service and zero-dependency library cases, including
a prerelease host build dependency. Then run:

```sh
ARMORER_TEST_GRAPH_RECEIPTS=/absolute/path/to/new-receipts cargo test --locked --test cargo_graph_v2 native_python_graphs_match_independent_rust_reader -- --ignored --test-threads=1
```

This test requires retained native outputs and separate fixture configuration;
it fails when those inputs are absent. The receipt explicitly records unsigned
output and synthetic fixture run 17/attempt 2. No fixture artifact is executed,
no release/tag API is invoked, and these tests do not authenticate a release.
