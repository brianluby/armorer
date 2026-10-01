# Experimental version-one contracts

This slice establishes typed config, pin-lock and preview shapes for review. They remain experimental until v0.1 ships. Release inventory, verification policy and build/sign/package evidence contracts belong to later slices; architecture requirements are not an already supported protocol.

## Intent: armorer.toml

The structural [config schema](../schemas/config-v1.json) disallows unknown fields. Runtime validation requires schema version 1, owner/repository, an exact stable Rust x.y.z version, relative input paths without symlinks, unique deliverable IDs, explicit package/binary selections, supported triples and declared package features. Nested workspace members are supported through a workspace-root `Cargo.toml`; selecting a nested manifest as the discovery root is deferred.

Feature sets contain `default_features` and package-local declared features. All selected features must exist in the deliverable's package. Binary `required-features` must be enabled, including transitive local/default edges. Dependency resolution and target-specific dependency closure are not performed here. Initial targets are x86_64/aarch64 Linux GNU and aarch64 macOS. Windows, musl and other adapters require explicit future support.

`policy.license_file` identifies a repository license document. Presence does not constitute a dependency license allowlist. `policy.attestations = "required"` is the only secure-release policy currently accepted. A later explicit CI-only mode will not imply release readiness. There are no shell hooks, arbitrary command arguments, secret values, caller predicates or provenance claims.

## Reviewed pins: armorer.lock

The [lock schema](../schemas/lock-v1.json) contains schema version, config digest, runtime version, workflow pin and tools map. Workflow pins require a repository and full lowercase 40-character Git SHA. Tools require exact stable versions and 64-character distribution SHA-256 digests. The config digest binds **exact TOML bytes**, including comments. Runtime compatibility is exact in this slice.

Parsing validates syntax and config binding only. The loader returns the exact bytes it validated, and the plan hashes those bytes without reopening the lock. Hash syntax does not authenticate upstream bytes or identity. Check/plan report `pins-not-authenticated`. There is no resolved catalog, lock generation or upgrade mechanism yet. Do not populate placeholder hashes to satisfy checks.

## Deterministic preview

The [plan schema](../schemas/plan-v1.json) reports mode, validated intent including selected deliverables/features, config/optional lock digests, relative inputs, sorted metadata, proposed changes, findings and capability state. It excludes temporary paths, timestamps and random IDs. Inputs hash Cargo manifests/locks; `.rs` records indicate **path presence only** to bind implicit target discovery. They do not hash source or constitute release source evidence.

The first preview proposes `rust-toolchain.toml` with existing/proposed digests. Different existing contents or legacy `rust-toolchain` produce `conflict`; matching contents are `unchanged`, missing contents are `create`. Customizations are preserved. No workflow calls are generated before a reviewed implementation exists. Future apply must recheck preconditions and ownership rather than trust a stale plan.

`state = "configuration-valid"` means local intent passed validation. It does not mean configured, CI verified, release rehearsed, published or provenance verified. `capability_state = "unknown"` blocks secure readiness. Later states require evidence from their own gates.
