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
binary in a fixed target directory and stage all 27 outputs before replacing
committed files. Missing binaries or failed generation preserve the old schemas;
an installed `armorer` on PATH is never used. Stage the whole set. Keep the old
schemas if generation fails. **This is the way.**

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
    verification-context-v2:verification-context-v2 \
    verification-context-v3:verification-context-v3 \
    publication-policy:publication-policy-v1 publication-plan:publication-plan-v1 \
    publication-approval:publication-approval-v1; do
    kind="${entry%%:*}"
    filename="${entry#*:}"
    target/debug/armorer schema "$kind" > "$armorer_schema_staging/$filename.json"
  done
  for staged_schema in "$armorer_schema_staging"/*.json; do
    mv "$staged_schema" "schemas/$(basename "$staged_schema")"
  done
)
```

This procedure covers all 27 current schemas. After regeneration, run
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
database IDs and read back tracker writes. Merging PRs and publishing releases
require explicit human authorization. v0.1 targets SLSA Build L2; L3 stays future work.

Protect the work. Earn the claim. **This is the way.**
