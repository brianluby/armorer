# Tickets #4 and #5 — end-to-end rehearsal on the approved bootstrap-v2 authority

**Status: rehearsal PASS on disposable consumers with the OWNER-APPROVED catalog.**
This supersedes the 2026-11-12 rehearsal, which used the frozen bootstrap-v1
authority because production approval was still pending. The owner approved
production trust authority on 2026-11-13; the approved catalog bytes are what
this rehearsal exercised. No publication, tag, release or settings mutation
was performed. armorer-test and both pilot primaries were untouched.

- Date (UTC): 2026-11-13
- Host: macOS darwin arm64
- Disposable consumers: `/tmp/armorer-45-rehearsal/{create,cli,recover,conflict,upgrade}`
  (copies of `examples/bootstrap-v2/{library,cli}`). `examples/` never edited —
  `git status --porcelain -- examples/` empty (`examples-untouched.txt`).
- Plans/policies stored OUTSIDE consumer roots, in this evidence directory.
- **No consuming build executed anywhere**: final sweep
  `find /tmp/armorer-45-rehearsal \( -name target -o -name BUILD_EXECUTED \)`
  → 0 results (`final-no-build-artifacts.txt`). The `recover` consumer carried a
  hostile `build.rs` that writes `BUILD_EXECUTED` and panics if ever run; it
  never ran.

## 1. Build identity (`build-identity.txt`)

```
head: 3eb6071af77701f0ea83c8aa6f04f9d4f3bea793   (main; PR #34 catalog-v2 merged)
binary: /Users/bluby/repos/armorer/target/debug/armorer
binary_sha256: 9393b19609027a400a7dd27b0c2e3889d40d8d9baa31d189d1cf09fd6a1d68ae
rustc: rustc 1.95.0 (59807616e 2026-04-14)
cargo: cargo 1.95.0 (f2d3ce0bd 2026-03-21)
```

Command: `RUSTC_WRAPPER= rustup run 1.95.0 cargo build --locked`.

## 2. Approved authority actually in force (`catalog.json`, `catalog.exit` = 0)

`armorer catalog` reports workflow `brianluby/armorer-workflows` @
`e5e9c0d31baff87e357046d81068b646a39a9f23`, tools_sha256
`e21e6cbd2cdd1125dbb9817116b1f46d043807b4d92128abc9dc07256dbb4b13` — the exact
digest the owner approved. Every new bootstrap plan in this rehearsal carried
`catalog_id: bootstrap-v2` with that same `catalog_sha256`.

## 3. Ticket #4 — bootstrap lifecycle

| Case | Command | Exit | Observed |
| --- | --- | --- | --- |
| read-only check | `bootstrap check` | 2 | `capability_state: unknown`; findings `github-capabilities-unchecked`, `release-runtime-unavailable`. All 7 consumer files byte-identical before/after. |
| exact preview | `bootstrap plan` | 0 | 5 changes, all `create`; each carries `before_content`, `proposed_content`, `unified_diff`, `before_sha256`, `after_sha256`, `owned`. |
| wrong digest | `bootstrap apply --expect-plan-sha256 000…` | 1 | `transaction blocked: bootstrap plan does not match approved digest, catalog or runtime`. |
| correct apply | `bootstrap apply` | 0 | `outcome: applied`, `configured: true`, `ci_verified/provenance_verified/published/release_rehearsed: false`. |
| replay | same apply again | 0 | `outcome: unchanged`; all 5 managed digests byte-identical (`create-managed-after.sha256`). |
| unrelated bytes | before/after compare | — | 7 unrelated files compared, **0 changed**, 0 unexpected additions. |
| unowned conflict | plant custom `armorer-ci.yml`, `rust-toolchain.toml` (1.98.1), `armorer.lock`; plan | 0 | all three reported `conflict`. |
| conflict apply | `bootstrap apply` | 1 | `transaction blocked: bootstrap conflict or unfinished transaction requires explicit resolution`. Unowned files **byte-identical**; no `.armorer` state created (no partial writes). |
| hostile discovery | `build.rs` + `build = "build.rs"`; check/plan | 2 / 0 | `BUILD_EXECUTED` absent, no `target/` — consuming code never executed. |
| symlink | replace managed file with a symlink; plan | 1 | `unsafe-path: symlinks require migration before discovery`. |
| concurrent apply | hold `.armorer/apply.lock` exclusively (`flock LOCK_EX`) in a second process, then `bootstrap apply` | 1 | `transaction blocked: another Armorer transaction is active`. All five managed digests unchanged under contention (`concurrent-apply.json`, `concurrent-apply.exit`). |

## 4. Ticket #5 — cross-revision migration, rollback, conflict, import

The two independently accepted catalog revisions required by ledger C3/C4 are
`bootstrap-v1` (frozen, workflows `772ca83e…`) and `bootstrap-v2` (approved,
workflows `e5e9c0d…`). Both were exercised on one consumer with stored bases.

| Step | Command | Exit | Observed |
| --- | --- | --- | --- |
| v1 baseline | `upgrade plan --catalog bootstrap-v1` → `upgrade apply` | 0 / 0 | State authority `bootstrap-v1`, revision 1, workflow `772ca83e…`; `armorer.lock` pinned to `772ca83e…`. Real stored bases retained (`upgrade-v1-managed.sha256`). |
| v1→v2 preview | `upgrade plan --catalog bootstrap-v2` | 0 | `direction: upgrade`. Three-way merge: `base_content` matches the stored v1 bytes; `before_content` has `772ca83`, `proposed_content` has `e5e9c0d`; decisions `update` for lock + both callers. Carries `candidate_diff`, `proposed_diff`, `base_sha256`, `compatibility`. |
| v1→v2 apply | `upgrade apply` | 0 | `outcome: applied`. Authority now `bootstrap-v2`, revision 2, workflow `e5e9c0d…`; lock advanced to `e5e9c0d…`. |
| rollback without intent | `upgrade rollback-plan` (no flag) | 1 | `transaction blocked: historical restoration requires explicit allow-downgrade`. |
| rollback preview | `upgrade rollback-plan --allow-downgrade` | 0 | Reverse packet with `original_upgrade`, `limitations`, `allow_downgrade`. |
| rollback apply | `upgrade rollback-apply` | 0 | `outcome: rolled-back`, `configured: false`. **All five managed files restored to the exact v1 digests** (`diff upgrade-v1-managed.sha256 rollback-managed.sha256` empty); authority back to `bootstrap-v1` rev 1 `772ca83e…`. |
| edited managed file | append an owner marker to managed `armorer-ci.yml`; plan v2 | 0 | That path reports `conflict` (not replacement); other paths `update`/`unchanged`. |
| conflict apply | `upgrade apply` | 1 | `transaction blocked: upgrade compatibility or merge conflict requires review`. Owner edit **preserved byte-identically**; every other managed file still at its v1 bytes (no partial writes). |
| import existing workflow | plant generated-identical `armorer-ci.yml`; `upgrade plan --import-file` | 0 | `import_files: [".github/workflows/armorer-ci.yml"]`, decision `import`, `imported: true`. |
| import apply | `upgrade apply` | 0 | `outcome: applied`; imported file adopted **byte-identically** (not rewritten); state records `bootstrap-v2` @ `e5e9c0d…`. |

No automatic mutable-major pin adoption occurred: every pin change was an
explicit catalog-sourced SHA, and the historical direction required an explicit
`--allow-downgrade`.

## 5. Observed exit codes

`exit-summary.txt` retains every raw exit code. Expected non-zero results are
the deliberate negative cases: `create-check`/`recover-check` = 2 (release gates
open), `create-tamper-apply`/`conflict-apply`/`edited-apply`/`rollback-noflag`/
`symlink-plan` = 1 (fail-closed).

## 6. What this does NOT establish

- No GitHub capability preflight, hosted CI run, signing, attestation,
  publication or tag operation was performed. `ci_verified`,
  `provenance_verified`, `published`, `release_rehearsed` remain `false` in every
  receipt.
- Production Sigstore roots, verifier identities and publication-policy bytes
  still do not exist; only the catalog authority is approved. Tickets #8/#9/#10
  are unaffected by this rehearsal.
- The rehearsal used a reviewed example policy on disposable copies, not a
  pilot adoption. Momus and Rusty Brain remain untouched pending M/R decisions.