# Contributing

Bring a focused change. Keep it isolated. Show the evidence that makes it ready
for review. **This is the way.**

Use isolated branches and pull requests. Preserve unrelated edits and pilot
repositories. The development toolchain is Rust 1.95.0 with rustfmt and Clippy.
The owner prefixes shell commands with `rtk`; contributors elsewhere can run the
underlying commands directly.

## Prove the change

Before committing or pushing, run:

```sh
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked
```

If workstation configuration sets an unavailable compiler wrapper, set `RUSTC_WRAPPER` to an empty value. Ensure PATH resolves the pinned toolchain rather than a system installation. CI places the toolchain's bin directory first.

On the owner's workstation, an explicit pinned invocation is
`rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo test --locked`.
Use the same prefix for build, format and lint checks.

Passing local checks earns review evidence. Hosted CI and human acceptance have
their own gates. **This is the way.**

## Keep the contracts exact

Regenerate schemas after changing serialized contracts. Build the checkout's
binary in a fixed target directory and stage all 14 outputs before replacing
committed files. Missing binaries or failed generation preserve the old schemas;
an installed `armorer` on PATH is never used:

```sh
(
  set -eu
  cargo build --locked --target-dir target
  armorer_schema_staging="$(mktemp -d schemas/.armorer-schemas.XXXXXX)"
  trap 'rm -rf "$armorer_schema_staging"' 0
  for kind in config lock plan release-inventory verification-policy artifact-evidence evidence-requirements capability-config catalog capability-observation lifecycle-record github-receipt publish-set registry-receipt; do
    target/debug/armorer schema "$kind" > "$armorer_schema_staging/$kind-v1.json"
  done
  for kind in config lock plan release-inventory verification-policy artifact-evidence evidence-requirements capability-config catalog capability-observation lifecycle-record github-receipt publish-set registry-receipt; do
    mv "$armorer_schema_staging/$kind-v1.json" "schemas/$kind-v1.json"
  done
)
```

This procedure covers all 14 main-branch schemas. After regeneration, run
`cargo test --locked` to check committed-schema equality. Run
`scripts/validate-contract-examples.py` with the development dependencies from
`requirements-schema.txt` to independently validate examples; CI installs those
dependencies using their reviewed hashes.

Contracts remain experimental before v0.1 release. Structural schemas do not
replace semantic validation or upstream pin authentication. Keep changes small;
describe the trigger, resulting behavior, tests and limitations. Keep check/plan
discovery free of repository builds and consuming-file changes. Required
capability failures stay explicit. Never request secret values or expose caller
shell/custom-provenance inputs in reusable builders.

## Write in the creed

Follow [the writing guide](docs/writing-guide.md). Use the Mandalorian voice in
prose and keep technical identifiers exact. Repeat “This is the way” at meaningful
checkpoints. Test copyable instructions in a temporary workspace. Link to shared
contracts, label proposed PR behavior, and preserve dated validation receipts.

Work is tracked in [Veans project 16](https://kanban.luby.us/projects/16/61) under
[epic #1](https://kanban.luby.us/tasks/1304). Distinguish project indexes from
database IDs and read back tracker writes. Merges and publication require explicit
human authorization. v0.1 targets SLSA Build L2; L3 stays future work.

Protect the work. Earn the claim. **This is the way.**
