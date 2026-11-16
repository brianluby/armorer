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

## Low tier (93 findings) — result

~70 fixed across commits 92eef2f..48c8316 (batches 1, 2, 3a–3d, B, C, D, E +
schema_bounds). All fixes verified: 274 tests pass, clippy/fmt clean, schemas
regenerated, library fixture regenerated via CLI after the MSRV addition.

### Invalid (1)

- .github/workflows/momus.yml (timeout finding): self-invalidating — the finding's
  own text notes `timeout-minutes` on a reusable-workflow `uses:` job makes the
  workflow invalid. Upstream (momus-review) fix only; no local action.

### Deferred — closed in branch ocr-low-deferrals (commit 47f17c6) except as noted

Closed:
- tests/bootstrap.rs:159 (select by path), :135 (inode+mtime idempotence)
- tests/apply.rs:202 (cross-process flock via self-spawned holder), :236 (symlinked plan
  file via load_plan + CLI; recovery through symlinked .armorer)
- tests/preview.rs:294 + tests/upgrade.rs:965 (GNU patch probe/visible skip)
- tests/upgrade.rs:592 (clock tick before no-op replay)
- tests/release_context.rs:47 (single clock sample); :598 tar layout guard shipped in PR #37
- tests/cargo_graph_v2.rs:252 (edge selected by package)
- tests/inspection.rs:411 (toolchain-lookup stage pinned)
- tests/integrated_cli.rs:247 (blocked workflow located dynamically)
- tests/historical.rs:334 (shipped in PR #37)
- src/verification/io.rs:184 (shared stream_hash helper)
- src/upgrade/state.rs:42 (per-invariant gate messages), :91 (borrowed source_bytes,
  dead Ownership.preimages field removed)
- src/verification/release.rs:161 (by_name index for both scan loops)

Still open:
- tests/cargo_graph_v2.rs:313 — independent source_commit cross-check needs a second
  retained commit artifact from the producer workflow (workflows repo change); the
  fixture set today has only the receipt's own claim. The ARMORER_TEST_GRAPH_RECEIPTS
  pipeline wiring is likewise a workflows-repo concern.
- remaining release_context pointer/error-code pinning beyond the tar-layout guard

## PR #36 automated review (momus / codex / Copilot / CodeRabbit)

20 inline findings triaged; 15 fixed (commits 8c0f956, 1bb514b), inline replies posted on all 20.

### Already covered — no change (2)
- src/upgrade/authority.rs:46 — unknown-id no-fallback branch already asserted for 8 forged ids
  by `registry_identity_is_independent_of_claims_and_unknown_downgrades_stay_blocked`.
- src/verification/cyclonedx.rs:155 — the `attestation-authentication-failed` →
  `sbom-schema-validation-failed` mapping already driven by real-validator negative cases in
  `native_cyclonedx_validates_complete_documents_and_rejects_non_graph_field_errors`.

### Invalid by design (2)
- src/catalog.rs:179 (compatibility) — exact-toolchain guard is deliberate hardening: the string
  is interpolated into generated TOML; loose strings were the injection surface being closed.
- scripts/qualify-test-verifier.py:78 (compatibility) — refusing group/world-accessible output
  directories is the fail-closed security contract for a pinned executable; CI caller unaffected.

### Followup round (commit 0c47086)
- CodeRabbit pushbacks accepted (2): inherited-stdout pipe hang — fixed at the root by
  redirecting lookup stdout to a file (same pattern as cargo metadata); rollback-phase
  journal-sync message — now phase-neutral, outer error checks journal presence.
- New momus findings (2): rollback.rs:64 empty-deletion diff pinned via extracted
  `reverse_diff` + test; trust/mod.rs:330 duplicate-key compatibility ruled invalid by
  design (deliberate tamper signal, reply posted).
