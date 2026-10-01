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
binary in a fixed target directory and stage all three outputs before replacing
committed files. Missing binaries or failed generation preserve the old schemas;
an installed `armorer` on PATH is never used:

```sh
(
  set -eu
  cargo build --locked --target-dir target
  armorer_schema_staging="$(mktemp -d schemas/.armorer-schemas.XXXXXX)"
  trap 'rm -rf "$armorer_schema_staging"' 0
  for kind in config lock plan; do
    target/debug/armorer schema "$kind" > "$armorer_schema_staging/$kind-v1.json"
  done
  for kind in config lock plan; do
    mv "$armorer_schema_staging/$kind-v1.json" "schemas/$kind-v1.json"
  done
)
```

Contracts remain experimental before v0.1 release. Structural schemas do not replace semantic validation or upstream pin authentication. Keep changes small; describe the trigger, resulting behavior, tests and limitations. Do not request secret values, add caller shell commands or claim release/provenance verification from local previews. Work is tracked in [the epic](https://kanban.luby.us/tasks/1304); merges and publication require separate human authorization.
