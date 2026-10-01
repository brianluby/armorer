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

Regenerate schemas after changing serialized contracts:

```sh
armorer schema config > schemas/config-v1.json
armorer schema lock > schemas/lock-v1.json
armorer schema plan > schemas/plan-v1.json
```

Contracts remain experimental before v0.1 release. Structural schemas do not replace semantic validation or upstream pin authentication. Keep changes small; describe the trigger, resulting behavior, tests and limitations. Do not request secret values, add caller shell commands or claim release/provenance verification from local previews. Work is tracked in [the epic](https://kanban.luby.us/tasks/1304); merges and publication require separate human authorization.
