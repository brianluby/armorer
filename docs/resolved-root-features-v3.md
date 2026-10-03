# Independently approved resolved root features

`verification-context-v3` is an explicit successor for complete verification with
the unchanged native catalog v2 and Cargo graph v2. Its `release` keeps the frozen
v1 expectation shape, `runtime_source` keeps v2 semantics, and `root_features`
adds one exact sorted, unique resolved feature list for every selection key
`deliverable_id--target--feature_set`. Existing schema bytes remain unchanged.

Approve this outer context digest independently before downloading release
assets. Resolve each list from the separately reviewed immutable source, selected
package, target, default-feature decision and requested feature flags. Include
every activated root feature, including implied features and default members.
Never use the offered release graph to choose its own expected list. The retained
producer graph already derives activated features from actual compiler messages;
its host/target aggregation limitation remains explicit and must be accounted for
in the independently reviewed expectations.

[Cargo feature semantics](https://doc.rust-lang.org/cargo/reference/features.html)
allow requested features to enable other local features. Defaults can also enable
additional members. Comparing the graph only with requested names permits a
feature-expanded artifact; comparing it with only literal requests rejects valid
closures. V3 compares the complete activated root set with the separately approved
set, rejecting additions and omissions before SBOM reconciliation.

```sh
armorer schema verification-context-v3 > /path/outside-project/context-v3-schema.json
armorer verify-release --context-kind native-v3 --trusted-inputs /path/to/approved-inputs --expect-context-sha256 INDEPENDENTLY_APPROVED_CONTEXT_SHA256 --directory /path/to/downloads --gh /path/to/approved/gh --trusted-root /path/to/approved/root.json --cyclonedx /path/to/approved/cyclonedx
```

The trusted-input directory contains `armorer-verification-context-v3.json` and
the same independently approved config, lock, Cargo.lock, catalog and policy
files as native-v2. The library entry point is
`TrustedReleaseContext::open_native_v3`. Missing/extra selection keys, unsorted or
duplicate features, control characters, oversized sets and omitted requested
features reject. Approval digest checking still precedes JSON decoding. A v3
failure never retries v1/v2 or historical verification.

Legacy-v1/native-v2 contain no independent resolved-feature expectation. They now
support only disabled defaults and root features exactly equal to literal
requests. Any expansion rejects; default-enabled selections report
`cargo-graph-resolved-feature-expectation-required`. Migrate such selections by
separately reviewing a v3 context, rather than changing their requested flags or
silently reducing verification. Packages without a default feature may have an
empty approved resolved set with defaults enabled; a separately approved empty
set is distinct from guessing that a default definition is absent.

[Synthetic v3 context](../examples/runtime-native-v2/context-v3.json) illustrates
the shape and grants no runtime/root approval. Tests include real offline Cargo
metadata for an owned package with implied/default features and a build-script
trap, feature-expanded substitutions, exact feature-set adversaries, malformed
context maps and explicit context-first CLI rejection for all three versions.
These tests do not establish an own signed complete release, protected Apple
finalization, pilot acceptance or SLSA Build L2.
