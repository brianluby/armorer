# Invalid findings — OCR review 2026-11-14

Findings verified against the source and rejected, with evidence. Deferred-but-valid
items are listed separately at the bottom; they are NOT invalid.

## Invalid

### src/verification/context.rs:287-288 — "Cargo.lock authenticated bytes not retained (TOCTOU)"

Claim: downstream consumers must re-read `Cargo.lock` from disk after the digest
check, so the file can be swapped post-check.

Evidence: `Cargo.lock` is read exactly once, at this site, and hashed from that same
read (`io::read_bounded(...)` → `crate::digest`). Grep of every consumer shows no
second read anywhere:

- `src/verification/graph.rs:260` compares only `cargo_lock_sha256` digests
  (graph inputs vs context inputs); it never opens the file.
- No other code in `src/` or `src/main.rs` opens `Cargo.lock` during verification.

There is no check-then-use window today. Retaining the bytes plus adding an accessor
no caller uses would be speculative API for a hypothetical future consumer. If a
future consumer needs the bytes, retain them at that time (the config/lock/catalog/
policy pattern is right there to copy).

Verdict: INVALID as a live bug; valid only as a note for future consumers.

## Scope corrections (finding partially valid, implemented narrower)

### src/verification/publication/github.rs:962-965 — statusless deployment fail-open (HIGH, fixed in a75ef4d)

The fail-open was real for the serialization `active_count` (competing deployments
with an empty status list vanished from the count) and is fixed: absent status now
counts as active. The finding's implied extension to `bound_active` was NOT applied:
an existing test (`bound_deployment_without_status_is_not_active`) documents that a
bound deployment without an observed status must stay Unknown — enforcement requires
positive evidence of activity, and treating absence as active there would fail OPEN
for enforcement. Only the serialization count was corrected.

## Deferred (valid, not implemented in this pass)

Large refactors or changes needing live-provider/native-host validation; left for a
dedicated change rather than a blind patch:

- src/verification/apple.rs:502-503 — watchdog/deadline around in-process
  Security.framework validation (needs macOS-native testing; 20-min budget check
  exists but only post-return).
- src/verification/sigstore.rs:682-683 — hard teardown bound for run_process_status
  (D-state child / inherited pipe write-ends; pinned single-process gh makes this
  unlikely; needs careful non-blocking reap design).
- src/verification/publication/github.rs:729-729 — streaming upload to avoid ~2x
  asset-size RSS (perf; needs provider-contract testing).
- src/verification/publication/github.rs:701-704 — canonical security-field
  projection instead of whole-body hashes for double-observation equality
  (behavior change against live provider responses; needs rehearsal evidence).
- src/main.rs:387-389, 530-530 — single-match Plan handling / shared
  verify-vs-freeze helper (maintainability refactors, no live bug).
- src/plan.rs:45-45, 120-120 — disposition enum + legacy-flag reuse
  (schema/identity-bearing refactor across apply/bootstrap).
- tests/preview.rs, tests/integrated_cli.rs, tests/upgrade.rs, tests/cargo_graph_v2.rs,
  tests/release_context.rs (tar builder), tests/trust_contracts.rs (pin-error codes),
  tests/historical.rs:89-91 (child-process markers) — remaining test-hardening
  findings; each is a valid improvement, batched for a follow-up test-only change.
