# A1: single transaction engine — Implementation Plan

> Executor: superpowers:executing-plans, inline. Spec: review A1 (reviews/2026-11-16-architecture-security-roadmap-review.md). Veans #34 (1499). Branch a1/single-transaction-engine.

**Goal:** one journaled write/recovery engine; bootstrap and upgrade keep their approval contracts, pre-lock validation order, receipts and journal wire bytes.

**Design (ruling):** the duplicated mechanical core (~500 lines each) moves to `src/transaction.rs`:
- `Write { path, before: Option<String>, after: Option<String> }` — superset; serde encodes `Some` identically to bootstrap's current `String`, so journal bytes don't move. Both Journal structs embed it (payload field stays `plan` vs tagged `approval`, per-module structs kept).
- `Messages` struct of `&'static str` — every caller-visible error string preserved exactly (tests + momus behavior contract).
- `execute(root, journal_path, journal_bytes, operations, complete_check, rollback_precheck, messages, after_write, commit_marker)` — lock-free core: journal persist → per-write before-check/ensure_parent/replace → commit marker → complete check → remove journal; on failure reverse-restore + preserve journal. Caller owns lock + pre-lock ordering (differs legitimately between contracts).
- `recover_committed` / `restore` / `write_one` / `remove_journal` shared helpers; per-module `recover()` keeps its validate/guard/lock ordering, delegates the mechanical half.
- `ensure_parent` moves to the engine (upgrade already calls bootstrap's copy).

**Not in scope (ledgered):** S6 directory-handle replace stays deferred (Later tier; separate reviewed change). No receipt/journal/plan wire changes. No public API renames.

## Tasks
1. Engine module + upgrade delegation. Embedded upgrade fault tests (interruption at every boundary, wrong digest, intervening edits, commit-marker uncertainty) must pass unchanged — they call `upgrade::transaction::transaction(root, approval, expected, hooks...)`; keep that signature as a thin delegate. Gates; commit.
2. Bootstrap delegation, same pattern (`transaction()` signature kept for its fault tests). `operations()` wraps after in `Some`. `replay_matches`/`validate_journal` stay in bootstrap (contract logic). Gates; commit.
3. Verify journal byte-compatibility explicitly: one test asserting a bootstrap journal serialized through the engine equals the pre-refactor fixture bytes (golden from main). Docs/ARCHITECTURE touch-up if it names two engines. Ledger; push; PR.

## Global constraints
rtk prefix; fmt/clippy(-D warnings)/test --locked gates each commit; no wire-byte drift (digests bind packets); exact error strings; no merge/tag.

## Ledger
- Ruling: caller keeps lock + pre-lock ordering — the two contracts' failure precedence differs (journal-refuse vs unchanged-replay order); unifying order would change observable behavior — cost: ~40 lines of ordering stay per-module, which is planner logic anyway.
- Ruling: Messages struct over generic strings — error text is caller-visible and test-asserted — cost: one small struct per contract.
- Ruling: S6 deferred — separate security change, keeps this diff attributable.
- Ruling (execution): first engine draft had generic execute/Failure/Messages — cut it; the contracts' commit/cleanup error classification differs and is test-asserted, so only the mechanical loops moved (Write/matches/ensure_parent/write_image/apply_writes/restore). Net −106 lines, all 262 pre-existing tests untouched and green — cost: sequencing stays per-caller, which is planner logic.
- Ruling: ensure_parent kept bootstrap's verbatim body + error strings (draft rewrite would have changed test-asserted messages) — cost: "bootstrap" wording now shared, cosmetic.
- Verification: 263 tests (golden journal wire-bytes test added), fmt/clippy clean; upgrade/bootstrap fault matrices pass unchanged against the shared loops.
