# Contributing

Use isolated branches and pull requests. The development toolchain is Rust 1.95.0 with rustfmt and Clippy. The owner uses `rtk` as a command proxy; contributors elsewhere can run underlying commands directly.

Before submitting:

```sh
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked
```

If workstation configuration sets an unavailable compiler wrapper, set `RUSTC_WRAPPER` to an empty value. Ensure PATH resolves the pinned toolchain rather than a system installation. CI places the toolchain's bin directory first.

Regenerate schemas after changing serialized contracts. Build the checkout's
binary in a fixed target directory and stage all 23 outputs before replacing
committed files. Missing binaries or failed generation preserve the old schemas;
an installed `armorer` on PATH is never used:

```sh
(
  set -eu
  cargo build --locked --target-dir target
  armorer_schema_staging="$(mktemp -d schemas/.armorer-schemas.XXXXXX)"
  trap 'rm -rf "$armorer_schema_staging"' 0
  for entry in \
    config:config-v1 lock:lock-v1 plan:plan-v1 \
    release-inventory:release-inventory-v1 verification-policy:verification-policy-v1 \
    artifact-evidence:artifact-evidence-v1 evidence-requirements:evidence-requirements-v1 \
    capability-config:capability-config-v1 catalog:catalog-v1 \
    capability-observation:capability-observation-v1 lifecycle-record:lifecycle-record-v1 \
    github-receipt:github-receipt-v1 publish-set:publish-set-v1 registry-receipt:registry-receipt-v1 \
    ci-policy:ci-policy-v1 bootstrap-plan:bootstrap-plan-v2 \
    upgrade-plan:upgrade-plan-v1 upgrade-rollback:upgrade-rollback-v1 \
    cargo-graph-v2:cargo-graph-v2 verification-context:verification-context-v1 \
    native-catalog-v2:native-catalog-v2 runtime-distribution-v1:runtime-distribution-v1 \
    verification-context-v2:verification-context-v2; do
    kind="${entry%%:*}"
    filename="${entry#*:}"
    target/debug/armorer schema "$kind" > "$armorer_schema_staging/$filename.json"
  done
  for staged_schema in "$armorer_schema_staging"/*.json; do
    mv "$staged_schema" "schemas/$(basename "$staged_schema")"
  done
)
```

Contracts remain experimental before v0.1 release. Structural schemas do not replace semantic validation or upstream pin authentication. Keep changes small; describe the trigger, resulting behavior, tests and limitations. Do not request secret values, add caller shell commands or claim release/provenance verification from local previews. Work is tracked in [the epic](https://kanban.luby.us/tasks/1304); merges and publication require separate human authorization.
