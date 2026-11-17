# ADR 0012 Removal 1: retire the version-one provisioning path — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove top-level `plan` / `plan --preview` / `preview` / `apply` / `recover`, the `plan-v1` schema, `.armorer/state.json` + `.armorer/journal.json` ownership, `guard_v1`, and the `LegacyV1` upgrade source, leaving `bootstrap` and `upgrade` as the only provisioning paths.

**Architecture:** Shared transaction I/O helpers move out of `src/apply.rs` into `src/filesystem.rs` first (pure move), then the v1 surface is deleted in one coherent cut. `src/plan.rs` shrinks to the shared `Disposition`/`Finding` types that bootstrap and upgrade already import. Top-level `check` is re-pointed at the surviving bootstrap contract before the v1 inspector is deleted.

**Tech Stack:** Rust (edition per Cargo.toml), clap, schemars, serde; Python structural validators under `scripts/`.

**Spec:** `docs/adr/0012-pre-release-contract-consolidation.md` (rows 1–2 of "Removed before 0.1.0"), review finding A4 in `reviews/2026-11-16-architecture-security-roadmap-review.md`.

**Tracking:** Veans project 16, task #31 (db id 1496). Branch `adr0012/remove-v1-provisioning`, worktree `../armorer-adr0012-r1`.

## Global Constraints

- Prefix shell commands with `rtk` (owner workstation rule).
- Gates before every commit: `rtk cargo fmt --check`, `rtk cargo clippy --locked --all-targets --all-features -- -D warnings`, `rtk cargo test --locked`. If the toolchain wrapper interferes: `rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo test --locked` (CONTRIBUTING.md).
- Discovery/planning must never execute repository build scripts or write consuming files (existing invariant; fixtures write `BUILD_EXECUTED` tripwires — keep them).
- Never rename identity-bearing wire strings (`Disposition` variants, finding codes) — plan digests depend on them.
- Removed command names stay reserved; do not reuse them (ADR 0012).
- Schemas are regenerated only via the CONTRIBUTING.md staging loop, never hand-edited.
- No merge, tag, or publication. Commit to the branch only; push only the branch.
- This plan does NOT remove: `bootstrap-v1` compiled catalog, `legacy-v1`/`native-v2` context kinds (separate ADR 0012 removals, separate tasks/PRs).

## Review Focus

1. **Fail-closed preserved:** a repository that still carries `.armorer/state.json` or `.armorer/journal.json` must not be silently adopted, upgraded, or rolled back. After removal these paths are simply unmanaged; confirm bootstrap/upgrade never read them and their presence cannot flip an ownership decision.
2. **No coverage loss:** every discovery/validation behavior tested through v1 `plan::inspect` in `tests/inspection.rs` must survive, re-expressed against `bootstrap::inspect`. Count tests before/after.
3. **Digest stability where required:** bootstrap/upgrade plan digest rules unchanged except where the contract shape itself changes (upgrade preimages lose the v1 key). Committed packets regenerated, not hand-patched.
4. **Helper move is behavior-free:** Task 1 must not change any function body.
5. **Schema/CLI drift:** `schemas/plan-v1.json` deleted, `SchemaKind::Plan` removed, schema-equality test list updated, docs stop promising removed commands.

## File Structure

| File | Action | Responsibility after |
| --- | --- | --- |
| `src/filesystem.rs` | extend | private temp/file helpers (existing) + moved `optional_bytes`, `json`, `text`, `decode`, `TransactionLock`, `lock`, `sync_directory`, `replace` |
| `src/apply.rs` | delete | — |
| `src/preview.rs` | delete | — |
| `src/plan.rs` | shrink | shared `Disposition`, `Finding` only |
| `src/lib.rs` | edit | drop `pub mod apply; pub mod preview;` |
| `src/main.rs` | edit | drop v1 `Operation` variants + `SchemaKind::Plan`; `Check` → bootstrap contract |
| `src/bootstrap/mod.rs` | edit | drop `V1_STATE`, `V1_JOURNAL`, `guard_v1` |
| `src/bootstrap/plan.rs`, `src/bootstrap/transaction.rs` | edit | drop v1 preimage/finding/recovery/guard branches; imports → `filesystem` |
| `src/upgrade/{mod,plan,state,transaction,rollback}.rs` | edit | drop `LegacyV1` source + v1 preimage key; imports → `filesystem` |
| `schemas/plan-v1.json` | delete | — |
| `schemas/{bootstrap-plan-v2,upgrade-plan-v1,upgrade-rollback-v1}.json` | regenerate | shape changes from removed v1 keys |
| `tests/apply.rs`, `tests/preview.rs` | delete | — |
| `tests/inspection.rs` | rewrite | same coverage via `bootstrap::inspect` |
| `tests/{bootstrap,upgrade,integrated_cli,trust_contracts}.rs` | edit | drop v1-only scenarios; update schema list |
| `examples/upgrades/{library,cli,service}/*.json` | regenerate | packets lose v1 preimage entries |
| `examples/bootstrap-v2/**` packets, `examples/armorer-dogfood` | regenerate if shape changed | bootstrap plan loses v1 preserved-input key |
| `docs/apply.md` | delete | — |
| `README.md`, `docs/cli-reference.md`, `docs/contracts-v1.md`, `docs/upgrades.md`, `docs/bootstrap-v2.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md` | edit | describe surviving surface only |

---

### Task 1: Move shared transaction I/O helpers to `src/filesystem.rs`

Pure mechanical move; no behavior change; v1 still works after this task.

**Moves (verbatim bodies) from `src/apply.rs`:** `optional_bytes` (:82), `json` (:92), `text` (:101), `decode` (:205), `TransactionLock` (:291), `lock` (:306), `sync_directory` (:334), `replace` (:349). Keep `pub(crate)` visibility. Move any private helpers/imports they need. `replace` uses `filesystem::private_file`/`private_tempdir` already in the destination module — adjust paths, not logic.

**Import updates:** `src/bootstrap/mod.rs`, `src/bootstrap/plan.rs`, `src/bootstrap/transaction.rs`, `src/upgrade/mod.rs`, `src/upgrade/plan.rs`, `src/upgrade/state.rs`, `src/upgrade/transaction.rs`, `src/upgrade/rollback.rs`, `src/plan.rs` — change `crate::apply::{…}` / `apply::{…}` to the `filesystem` equivalents. `src/apply.rs` itself imports the moved items back from `crate::filesystem`.

- [ ] Step 1: Move the items; update all imports (grep `apply::` and `crate::apply` to find every site, including tests).
- [ ] Step 2: `rtk cargo test --locked` — full suite green, zero test edits beyond import paths.
- [ ] Step 3: fmt + clippy gates.
- [ ] Step 4: Commit `refactor: move shared transaction I/O helpers into filesystem`.

### Task 2: Re-point top-level `check` at the bootstrap contract

TDD. New semantics (ruling, ledgered): `check` = `bootstrap check` with default policy `.armorer/ci-policy.toml`; exit 0 when in sync, exit 2 with sorted drift reasons on stderr, stdout is the bootstrap plan JSON; missing policy file is a fail-closed JSON error (exit 1). No `--release-ready` on top-level check (that flag lives on `bootstrap check`).

- [ ] Step 1: Write failing tests in `tests/inspection.rs` (new fns): (a) unbootstrapped fixture → `check` exits 1 with JSON error, nothing written; (b) bootstrapped fixture (reuse `tests/bootstrap.rs` provisioning pattern) → `check` exits 0, stdout parses as bootstrap plan (`schema_version == 2`); (c) edit an owned file → `check` exits 2, stderr contains `update:` reason, repository bytes unchanged.
- [ ] Step 2: Run them; watch (b)/(c) fail against the old always-2 v1 check.
- [ ] Step 3: Implement in `src/main.rs` `Operation::Check`: `armorer::bootstrap::inspect(&cli.repository, Path::new(".armorer/ci-policy.toml"))`, mirror the `BootstrapOperation::Check` drift/exit logic (no blockers branch). Update the command's doc comment and the crate-level exit-status comment.
- [ ] Step 4: Update the old `cli_check_fails_closed_plan_is_read_only_and_errors_are_json` test's check assertions to the new contract (its `plan` assertions die in Task 3).
- [ ] Step 5: Full gates; commit `feat: top-level check reads the bootstrap contract`.

### Task 3: Delete the version-one surface (one coherent cut)

Everything below lands in one commit; the tree does not compile mid-task.

**CLI (`src/main.rs`):** remove `Operation::{Plan, Preview, Apply, Recover}` variants and their match arms; remove `SchemaKind::Plan` and its arm; drop `plan::{Plan, inspect}` imports (keep nothing from v1).

**Modules:** delete `src/apply.rs`, `src/preview.rs`; remove `pub mod apply; pub mod preview;` from `src/lib.rs`. Shrink `src/plan.rs` to `Disposition` + `Finding` (verbatim, including doc comments and serde attributes) — module path `crate::plan::{Disposition, Finding}` stays valid for bootstrap/upgrade/tests.

**Bootstrap (`src/bootstrap/`):** delete `V1_STATE`, `V1_JOURNAL`, `guard_v1` from `mod.rs`. In `plan.rs`: drop v1 entries from `preserved_inputs` (keep `"rust-toolchain"` — that is the legacy *rustup* file, unrelated), drop the `legacy-ownership-migration-required` finding and its `SETUP_FINDINGS` entry, drop `V1_JOURNAL` from `recovery_required`. In `transaction.rs`: drop the v1 state/journal refusal branches (:193–198) and the v1 preimage recheck (:447); keep `guard_previous` (upgrade-owner guard — that one survives).

**Upgrade (`src/upgrade/`):** remove `SourceFormat::LegacyV1` and every branch keyed on it (`plan.rs` :201–266 source inventory shrinks from 3 preimage keys to 2 — `bootstrap::STATE` + `super::STATE`; :279, :408–410, :502, :568; `state.rs` :161). A repository carrying only `.armorer/state.json` now plans as `Unmanaged` — verify no code path reads that file anymore (`rtk grep -rn "state.json\|journal.json" src/` returns only v2/upgrade names).

**Schemas:** delete `schemas/plan-v1.json`; remove `"plan"` from the kinds list in `tests/trust_contracts.rs` (:577). Regenerate `bootstrap-plan-v2`, `upgrade-plan-v1`, `upgrade-rollback-v1` via the CONTRIBUTING.md staging loop after the code compiles.

**Tests:**
- Delete `tests/apply.rs`, `tests/preview.rs`.
- `tests/inspection.rs`: rewrite every test that calls `plan::inspect(root, "plan")` to `bootstrap::inspect(root, policy)` where `policy` is a written fixture file (copy the reviewed policy text pattern from `tests/bootstrap.rs` / `examples/bootstrap-v2/library`). Field mapping: `workspace`, `findings`, `changes[].disposition`, `plan_sha256`, `config_sha256` exist on the bootstrap plan; v1 `lock_sha256` assertions become digest assertions on `input_preimages["armorer.lock"]`; v1 single-toolchain-change assertions become assertions on the `rust-toolchain.toml` entry among the five targets. Error-code assertions (`invalid-toml`, oversized inputs, symlink rejection, MSRV, feature cycles, rustup multicall, PATH executable) must keep identical codes — they come from shared config/discovery and must not change. Keep the test count ≥ the current count; no scenario dropped.
- `tests/bootstrap.rs`: delete v1-interaction tests (:340–366 v1 apply guard scenarios, :531 v1 CLI plan) — the guard they exercise no longer exists. Keep everything else.
- `tests/upgrade.rs`: delete `legacy_v1_and_declared_unqualified_lock_require_explicit_scoped_import`'s LegacyV1 half (keep the unqualified-lock import coverage if separable; otherwise re-express against `Unmanaged`), delete tests that provision via v1 `apply` (:626, :725, :1005 forged-legacy-base). Where a forged-base scenario is still meaningful against bootstrap-v2 bases, re-express it; ledger which die.
- `tests/integrated_cli.rs` :113: replace `["preview"]` read-only call with `["bootstrap", "plan", "--policy", policy]`.
- Run the full suite; fix any remaining compile references (grep `armorer::apply\|armorer::preview\|plan::inspect\|LegacyV1\|guard_v1`).

- [ ] Step 1: Make all deletions/edits above.
- [ ] Step 2: `rtk cargo test --locked` green; confirm inspection test count preserved.
- [ ] Step 3: Regenerate the three changed schemas (staging loop); rerun schema-equality test.
- [ ] Step 4: fmt + clippy; commit `refactor!: remove version-one provisioning path per ADR 0012`.

### Task 4: Regenerate committed packets and examples

Packets are generated by running the CLI against fixture copies, never hand-edited (docs/candidate-upgrades.md flow).

- [ ] Step 1: For each of `library`, `cli`, `service`: copy `examples/bootstrap-v2/<profile>` to a temp dir, run `upgrade plan` (default catalog, no imports) and `upgrade rollback plan` per `docs/candidate-upgrades.md`, write outputs to `examples/upgrades/<profile>/{upgrade-plan,rollback-plan}.json`. The `service` packet's v1 `state.json` preimage content disappears; its source becomes `unmanaged` or `bootstrap-v2` per the fixture's actual files — inspect the fixture first and ledger the resulting `source_format`.
- [ ] Step 2: Regenerate bootstrap example packets under `examples/bootstrap-v2/` and `examples/armorer-dogfood` if they embed plan JSON (preserved_inputs shape changed). Use `scripts/validate-bootstrap-examples.py` to find what is validated.
- [ ] Step 3: `rtk python3 scripts/validate-upgrade-examples.py`, `rtk python3 scripts/validate-bootstrap-examples.py`, `rtk python3 scripts/validate-contract-examples.py` all pass; `rtk cargo test --locked` green (packet tests at `tests/upgrade.rs`:914).
- [ ] Step 4: Commit `chore: regenerate review packets without version-one ownership`.

### Task 5: Documentation sweep and final gates

- [ ] Step 1: Delete `docs/apply.md`. Update `README.md` (:11, :81–82), `docs/cli-reference.md` (v1 rows, `check` semantics, schema table), `docs/contracts-v1.md` (drop plan-v1 section), `docs/upgrades.md` (drop LegacyV1 source), `docs/bootstrap-v2.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md` (schema loop list if it names `plan`). Grep `armorer apply\|armorer recover\|armorer preview\|armorer plan\|plan-v1\|state.json\|LegacyV1\|legacy-v1 ownership` across docs/README — remaining hits must be historical ADR/review text only.
- [ ] Step 2: Note in `docs/adr/0012-…md` status line that removals 1–2 (rows 1–2) are implemented on this branch (do not mark the whole ADR done — catalogs/contexts remain).
- [ ] Step 3: Full gates: fmt, clippy, `cargo test --locked`, all `scripts/validate-*.py`, `scripts/test-review-gates.py` if runnable locally.
- [ ] Step 4: Re-run Armorer on itself: `rtk target/debug/armorer bootstrap check --policy .armorer/ci-policy.toml` in the worktree — expected exit 2 with drift only if template bytes changed (they must not in this removal); ledger the result. Do NOT re-bootstrap or mutate this repository's managed files.
- [ ] Step 5: Commit `docs: describe the surviving provisioning surface`; push branch; update Veans #31 with summary + read back.

---

## Ledger

(Executor records rulings here as `Ruling: … — why — cost if wrong`.)

- Ruling: top-level `check` defaults to policy `.armorer/ci-policy.toml` and mirrors `bootstrap check` exit semantics without `--release-ready` — ADR says "reads the surviving plan contract" but names no policy source; the managed policy path is the only non-hand-written candidate — cost if wrong: unbootstrapped repos get exit 1 instead of a setup report; acceptable fail-closed, revisit at F1 (`init`).
- Ruling: shared helpers land in `src/filesystem.rs` rather than a new module — it already owns private file I/O; A1 will extract the real engine later — cost if wrong: one more import move during A1.
- Ruling: check policy path is resolved against `--repository` (`cli.repository.join(...)`) — relative default was resolved against process cwd, breaking `--repository` runs — cost if wrong: none, tested both ways.
- Ruling: upgrade transaction unit fixtures' `"legacy"` kind becomes `"bootstrap"` — prior-owner deletion/recovery coverage survives via the v2 owner; v1-only path lists shrink — cost if wrong: v1 journal-mixing coverage gone with the format itself (intended).
- Ruling: v1-based upgrade integration tests re-expressed: unsupported-version and forged-base against bootstrap-v2 state; declared-import tests against Unmanaged; patch-line case list 3→2 — cost if wrong: none, same failure modes exercised on surviving formats.
- Ruling: packet regeneration (Task 4) folded into the Task 3 commit — the schema/packet equality tests cannot pass mid-cut; ADR row removal + packet shape change are one reviewed unit — cost if wrong: larger review diff, noted in commit message.
- Ruling: `service` upgrade packet source_format is `unmanaged` — the fixture carries no ownership state; old packet's v1 state was fabricated by the removed v1 apply — cost if wrong: service profile no longer demonstrates owned-source migration; bootstrap-v2-owned coverage exists in other tests.
- Ruling: declared-import rollback test drops the persistent `.armorer/apply.lock` from the restored snapshot — same pattern already used at tests/upgrade.rs:586/638; lock file persists by design — cost if wrong: none.
- Ruling: schema-validator venv uses python3.14 — default python3 is 3.15, outside the pinned-wheel support range (3.11–3.14); hash-pinned install fails closed on 3.15 — cost if wrong: none, CI uses supported interpreters.
- Verification: fmt clean; clippy -D warnings clean; cargo test 258 passed / 9 ignored; all six validate-*.py OK; test-review-gates.py 7 tests OK; self `bootstrap check` exit 0 (no drift, templates unchanged).
- Ruling (post-review self-audit): ADR 0012 rule 1 requires leftover v1 ownership to be REFUSED, not silently unmanaged — the initial cut missed this. Fixed in 8abb27c: shared `reject_v1` guard called from `guard_previous` (bootstrap plan/apply) and upgrade `inspect_bytes`/`guard` (plan/apply/recover/rollback). Test `leftover_version_one_ownership_is_refused_not_treated_as_unmanaged` watched fail then pass; suite 259 passed — cost if wrong: repos with orphaned v1 files need explicit owner removal before adopting, which is the ADR's intent.
