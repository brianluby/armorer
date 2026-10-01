# Ticket #2 contract validation and acceptance matrix

Local validation on 2026-10-01 in isolated `codex/trust-contracts`, from freshly
fetched Armorer `origin/main` at `3e391a25257bb607ed6ea0e44427be3fa5aae711`.
Inspected workflow checkout identity: `brianluby/armorer-workflows`, clean main at
freshly fetched `772ca83e386c883c88cc3b936d69f8cb3216c91e`.

## Acceptance matrix

| Ticket #2 requirement | Reviewable evidence | Status |
| --- | --- | --- |
| Preserve config/lock/plan and unsigned inventory | Existing schemas/types unchanged; all existing 44 tests retained; upstream unsigned schema's two tests pass | Delivered for review |
| Release inventory roles/bytes/subjects/assets/acyclic authentication | inventory module, generated release-inventory schema, independent expected layout, tamper/missing/extra/duplicate/cycle/detached-bundle tests | Delivered for review |
| Independent source/signer/workflow/predicate/scope and historical compatibility | policy/requirements schemas, explicit release/rehearsal modes, exact reviewed historical allowlist, source/signer/predicate/downgrade tests | Delivered for review |
| Per-artifact build/sign/package inputs, tools/freshness, coverage/outcomes/exceptions | artifact-evidence and evidence-requirements schemas; Apple byte chain and policy/pin/age/exception/coverage failures | Delivered for review |
| Enumerated capabilities/adapters, pins and lifecycle | capability/catalog/observation/lifecycle schemas; required unknown/unsupported/error/reporting rejection, catalog-lock binding, distinct branch versus publication evidence | Delivered for review |
| Separate GitHub/future registry receipts and conflict/DAG identity | github-receipt/publish-set/registry-receipt schemas; conflicting attempt/set, missing gates, partial DAG/index visibility and byte mismatch tests | Delivered for review |
| Proposed ADRs and concrete #8/#9/#10 handoffs | ADR 0006–0008 and trust-contracts-v1.md | Delivered for review |
| Human acceptance/merge | PR review and approval required; original acceptance checkbox remains incomplete | Pending |

## Exact local checks

All shell commands were invoked through `rtk`; Rust commands used
`rtk proxy rustup run 1.95.0 cargo ...` because the workstation Cargo is not a
Rustup proxy. This explicitly selects the repository's pinned compiler.

* `cargo fmt --check`: passed.
* `cargo clippy --locked --all-targets --all-features -- -D warnings`: passed.
* `cargo test --locked`: **65 passed**, no ignored or failed tests: 9 library,
  10 apply, 24 discovery/inspection, 21 trust-contract integration, 1 doctest.
* `cargo build --locked`: passed.
* Schema CLI output matches all **14** committed v1 schemas byte-for-byte in
  `generated_schemas_match_all_committed_version_one_schemas`.
* Independent `jsonschema 4.26.0` Draft 2020-12 validation:
  **14 schemas / 54 positive examples / 7 structural rejection cases** passed.
  Installed development wheels with `--require-hashes --only-binary=:all:` using
  the unchanged reviewed workflow requirements copied as requirements-schema.txt.
* `actionlint .github/workflows/development.yml`: passed after expanding schema
  checks and adding independent example validation.
* Verified workflow checkout:
  `python -m unittest discover -s tests -p test_build_schema.py -v`: **2 passed**.
  No workflow-repository tracked files were edited.
* `git diff --check`: passed.

Fixture regeneration is deterministic and never downloads/runs sample artifacts.
Every trust example is explicitly synthetic, including root/tool pins, bundles,
platform assertions, approval/notary values and registry observations. Local
contract tests do not substitute for hosted crypto/Apple/publication integration.
CI runs Rust and schema validation on Linux/macOS; its result is recorded in the
PR/tracker separately after delivery rather than predicted here.

## Review dispositions and runtime limits

The coordinating contract owner reviewed security boundaries, parsing/resource
limits, byte-cycle relationships, source modes, pin/catalog binding, freshness,
exceptions, lifecycle, retries and examples. Valid findings were fixed:

* Distinguish branch/CI/rehearsal evidence from stable-tag publication; explicit
  verification modes and publication tag guards with rejection coverage.
* Bind catalog identities to actual serialized fixture bytes and lock tool pins
  to catalog distributions, rather than a matching synthetic label alone.
* Require retained platform-evidence references for complete build/sign/package
  chains while keeping cryptographic authentication explicitly separate.
* Reject duplicate nested map keys in a bounded exact-byte loader; avoid copying
  the complete published artifact set during comparisons.
* Reject published/conflict state resumption as an owned draft.

No unresolved code finding was retained at delivery. Substantive reviewer changes
may still alter proposed ADRs or require another contract version before v0.1.
Required runtime work remains in #8/#9/#10: cryptography and offline roots,
verified SBOM predicate/graph content, actual Apple checks, live settings,
protected-ref/actor authorization and remote concurrency/state transitions.
Optional adapters, registry upload implementation, release publication and L3
assessment are outside this PR. No pilot repository was edited.
