# Fuzz targets

Each target feeds untrusted bytes to one internal parser through
`armorer::fuzzing` (compiled only with the `fuzzing` feature). A panic, abort or
hang is a finding; a parse error is the normal outcome.

| Target | Parser |
| --- | --- |
| `verification_json` | strict duplicate-key-rejecting JSON reader (`verification/io.rs`) |
| `http_response` | native CLI HTTP response parser (`verification/publication/github.rs`) |
| `trust_json` | bounded duplicate-free contract reader (`trust/mod.rs`) |
| `ci_policy` | CI policy parser, fixed trusted day (`ci_policy.rs`) |
| `config_toml` | `armorer.toml` deserialization and shape validation |
| `lock_toml` | `armorer.lock` deserialization |
| `sigstore_bundle` | unverified Sigstore bundle transport reader (`verification/bundle.rs`) |

Run one target (needs a nightly toolchain and `cargo install cargo-fuzz`):

```sh
cargo +nightly fuzz run <name> -- -max_total_time=60
```

CI only compile-checks this crate on the pinned stable toolchain
(`cargo check --manifest-path fuzz/Cargo.toml`, deliberately without `--locked`); it does not run the
fuzzers. `corpus/` and `artifacts/` stay untracked.

`fuzz/Cargo.lock` is committed so local fuzzing runs are repeatable, but no update
bot maintains it. After a dependency bump in the root `Cargo.toml`, refresh it with
`cargo check --manifest-path fuzz/Cargo.toml` and commit the result when convenient.
CI does not pass `--locked` here, so a stale fuzz lockfile never blocks a root bump.
