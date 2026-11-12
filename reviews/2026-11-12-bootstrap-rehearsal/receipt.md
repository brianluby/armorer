# Ticket #4 — bootstrap end-to-end acceptance rehearsal receipt

**Status: rehearsal PASS on disposable consumers. This is NOT ticket completion.**
Production catalog approval remains an owner gate. This rehearsal used the
**compiled bootstrap-v1 catalog** embedded in armorer main (workflows pin
`772ca83e386c883c88cc3b936d69f8cb3216c91e`), a **REHEARSAL-ONLY policy**, granted
**no production authority**, and performed **no publication** of any kind.
The bootstrap-v2 catalog successor is being implemented separately; this
rehearsal exercises what main ships today.

- Date (UTC): 2026-11-12 (run ~19:26–19:50Z)
- Host: macOS darwin arm64
- Disposable consumers: `/tmp/armorer-4-rehearsal/{create,cli,recover,conflict}` (copies of `examples/bootstrap-v2/{library,cli}`; repo `examples/` never edited — `git status --porcelain -- examples/` empty, see `examples-untouched.txt`)
- Plans/policies stored OUTSIDE consumer roots, in this evidence directory.
- No consuming build executed anywhere: final sweep `find /tmp/armorer-4-rehearsal \( -name target -o -name BUILD_EXECUTED \)` → 0 results (`final-no-build-artifacts.txt`). The recover consumer carried a hostile `build.rs` that would write `BUILD_EXECUTED` and panic if ever executed; it never ran.
- No tracker writes, no merges, no pushes. armorer-test and pilots untouched.

## 1. Build identity (`build-identity.txt`)

```
head: 57edd3271a20260b02b8f9c895f7cd0f4a6304af   (main, PRs #28/#29 merged)
binary: /Users/bluby/repos/armorer/target/debug/armorer
binary_sha256: 5d08a0be3073512385311ba5b2f9b390cf462d79273336e300b30222f5e138a0
rustc: rustc 1.95.0 (59807616e 2026-04-14)
cargo: cargo 1.95.0 (f2d3ce0bd 2026-03-21)
```

Command: `RUSTC_WRAPPER= rustup run 1.95.0 cargo build --locked` →
`Finished dev profile [unoptimized + debuginfo] target(s) in 0.25s` (freshness
verified: `find src Cargo.toml Cargo.lock -type f -newer target/debug/armorer`
→ empty; binary mtime 2026-11-12 08:48:31 PST, no source newer). `armorer
--version` → `armorer 0.1.0`.

Catalog authentication (read-only): `armorer catalog` exit 0 (`catalog.json`):
- `workflow.repository = brianluby/armorer-workflows`
- `workflow.commit = 772ca83e386c883c88cc3b936d69f8cb3216c91e` ← compiled pin
- `catalog_sha256 = e21e6cbd2cdd1125dbb9817116b1f46d043807b4d92128abc9dc07256dbb4b13`, 6 tools.

Policy: `rehearsal-policy.toml` / `rehearsal-policy-cli.toml` = fixture
`ci-policy.reviewed.toml` bytes with a 3-line REHEARSAL-ONLY header prepended
(header is part of the reviewed plan bytes; no production approval implied).

Notation below: `A=/Users/bluby/repos/armorer/target/debug/armorer`,
`E=` this evidence dir, `R=/tmp/armorer-4-rehearsal`. Every command's exit code
is recorded in the matching `*.exit` file; full stdout JSON is saved alongside.

## 2. Consumer `create` (library profile, create path) — C1–C5 happy path

Setup: `cp -R examples/bootstrap-v2/library $R/create`; removed the fixture's
pre-baked `bootstrap-plan.json` (fixture artifact, not a consumer input);
added unrelated files:
- `unrelated-sentinel` = `unrelated sentinel bytes\r\n`, sha256 `b61d2c79e37b8c28219a762db27839a740f2281e32ce0e8d52ad8e24d100c996`
- `.github/workflows/bespoke.yml` = `bespoke bytes\r\n`, sha256 `20bb890d820e38273bb6459e60e353f66b5756128471f3d0923b66d8aab74553`

Note: `bespoke.yml` is NOT one of the five fixed bootstrap targets, so it never
appears as a plan change — it is preserved as an unrelated file. The
differing-unowned-file **conflict disposition** is demonstrated separately in
§5 on an actual target path.

### 2.1 check — read-only, exit 2 (C1)

```
$A --repository $R/create bootstrap check --policy $E/rehearsal-policy.toml
exit=2   (create-check.json, create-check.exit)
```

Findings (both expected open gates):
```json
[{"code":"github-capabilities-unchecked","detail":"Authenticated GitHub capability preflight remains required."},
 {"code":"release-runtime-unavailable","detail":"Callers provide CI and manual unsigned builds; secure publication, final-byte attestations and Apple finalization remain required."}]
```
Plan view: 5 changes, all `disposition=create, owned=false`; `plan_sha256=4e5e6e15cb0c5023b95e0dc6813a582497d5add6a5a81bf588aa149f27348551`; `recovery_required=false`. File-set diff before/after check: empty — check wrote nothing.

### 2.2 plan — read-only, exit 0, exact diffs (C2)

```
$A --repository $R/create bootstrap plan --policy $E/rehearsal-policy.toml > $E/create-plan.json
exit=0   (create-plan.exit)
```
File-set diff before/after plan: empty — plan wrote nothing. Same digest as check.
Every change carries `before_content` (null for missing), `proposed_content`,
`before_sha256`/`after_sha256`, `owned`, and a complete `unified_diff`. Example:

```diff
--- /dev/null
+++ b/rust-toolchain.toml
@@ -0,0 +1,4 @@
+[toolchain]
+channel = "1.95.0"
+profile = "minimal"
+components = ["clippy", "rustfmt"]
```

Catalog pin visible in proposed bytes: both callers
`uses: brianluby/armorer-workflows/.github/workflows/rust-{ci,build}.yml@772ca83e386c883c88cc3b936d69f8cb3216c91e`;
`armorer.lock` `[workflows] commit = "772ca83e…"`. Embedded policy = 500 bytes
(the REHEARSAL-ONLY policy). `bespoke.yml` appears nowhere in the plan
(`grep -c bespoke create-check.json` → 0).

### 2.3 apply with independently supplied digest (C3)

```
DIG=4e5e6e15cb0c5023b95e0dc6813a582497d5add6a5a81bf588aa149f27348551
$A --repository $R/create bootstrap apply --plan $E/create-plan.json --expect-plan-sha256 $DIG
exit=0
```
```json
{"schema_version":2,"plan_sha256":"4e5e6e15…","outcome":"applied","configured":true,
 "ci_verified":false,"release_rehearsed":false,"published":false,"provenance_verified":false}
```
All 5 managed files match the plan's `after_sha256` byte-for-byte
(`create-managed-after.sha256`, all MATCH):
`.armorer/ci-policy.toml 9cab242b…`, `.github/workflows/armorer-build.yml d54919c9…`,
`.github/workflows/armorer-ci.yml a21d50ba…`, `armorer.lock d2e7125f…`,
`rust-toolchain.toml 24ef3b9d…`.
Ownership receipt `.armorer/bootstrap-state-v2.json` records exact generated
bases + `last_plan_sha256`. Sentinel + bespoke digests unchanged
(`create-unrelated-before/after.sha256` identical). No `target/`, no journal
left behind (`create-no-build-artifacts.txt` empty).

### 2.4 replay — no-op receipt (C4)

Same apply command again → exit 0, `outcome:"unchanged"` (`create-replay.json`).
All file mtimes identical before/after (`create-mtimes-{before,after}-replay.txt`,
diff empty) — replay rewrote nothing.

### 2.5 tamper + stale-plan apply — preimage rejection (C3/C6)

Appended `# rehearsal tamper\n` to owned `rust-toolchain.toml`
(sha256 became `b40438bd…`, `tampered-file.sha256`; original preserved in
`tamper-original-rust-toolchain.toml`), then re-applied the SAME approved plan:

```
exit=1
{"error":{"code":"transaction","message":"transaction blocked: committed bootstrap receipt conflicts with files or ownership"}}
```
Nothing written by the rejected apply (tampered file, sentinel, bespoke all
unchanged afterwards). Restored the original bytes → replay again returned
`outcome:"unchanged"` exit 0 (`create-restore-replay.json`). A fresh plan on the
restored consumer shows all five targets `disposition:"unchanged", owned:true`
(`create-plan-2.json`).

## 3. Consumer `cli` (cli profile) — brief repeat

Setup: copy of `examples/bootstrap-v2/cli`, fixture plan removed, sentinel
`cli rehearsal sentinel\n` (sha256 in `cli-unrelated-before.sha256`). Policy:
`rehearsal-policy-cli.toml` (cli fixture policy + REHEARSAL-ONLY header).

| step | command | exit | artifact |
|---|---|---|---|
| check | `bootstrap check --policy …` | 2 | `cli-check.json` (5 creates) |
| plan | `bootstrap plan --policy … > cli-plan.json` | 0 | file-set unchanged; digest `8631a8ed3a94942759ef7c3f70f59270949719c1a0e5bbc2a49ecfc6c39aeff3` |
| apply | `--expect-plan-sha256 8631a8ed…` | 0 | `outcome:"applied"`; all 5 files MATCH plan hashes (`cli-managed-after.sha256`); sentinel unchanged; no target/ |
| replay | same apply | 0 | `outcome:"unchanged"` (`cli-replay.json`) |
| wrong digest | `--expect-plan-sha256 1111…1111` | 1 | `{"error":{"code":"transaction","message":"transaction blocked: bootstrap plan does not match approved digest, catalog or runtime"}}` (`cli-wrong-digest.json`) |

Build caller is manual-only (`on: workflow_dispatch`), `permissions: contents: read`,
no OIDC/secrets; pinned `@772ca83e…`.

## 4. Consumer `recover` — live interruption + recover (C5)

Setup: library copy + sentinel + bespoke + hostile `build.rs`
(`fn main(){std::fs::write("BUILD_EXECUTED","bad").unwrap();panic!("must not build");}`),
digests in `recover-unrelated-before.sha256`. check exit 2, plan exit 0,
digest `25b31fd981066f984a1acbc6d8282a66b9ffb8c5f78003afc4162d0f5ab9ad87`
(`recover-plan.json`).

### 4.1 Live interruption (SIGKILL mid-transaction)

Started apply in background; polled for the journal plus a partial target set;
`kill -9` fired with **1/5 targets written** (`.armorer/ci-policy.toml` present,
other four absent). Apply process exit **137** (`recover-interrupt.summary`).
State at interruption (`recover-at-interruption-files.txt`): journal
`.armorer/bootstrap-journal-v2.json` present with `committed:false` and 6
operations (5 targets + state file, all `before:null`); saved as
`recover-journal-at-interruption.json`. No `target/`, no `BUILD_EXECUTED`.

### 4.2 Guards while journal present

- New apply attempt → exit 1: `"transaction blocked: approved bootstrap inputs or preimages changed; review a new plan"` (`recover-apply-while-journal.json`) — the partially-written inventory no longer matches the approved preimages, so apply refuses; recovery is the only path.
- `bootstrap recover --expect-plan-sha256 0000…0000` → exit 1: `"transaction blocked: bootstrap plan does not match approved digest, catalog or runtime"` (`recover-wrong-digest.json`).

### 4.3 recover — rolled-back

```
$A --repository $R/recover bootstrap recover --expect-plan-sha256 25b31fd9…
exit=0
{"outcome":"rolled-back","configured":false,"plan_sha256":"25b31fd9…", …all gates false}
```
(`recover-receipt.json`). Verified afterwards:
- File set equals the pre-apply set except the persistent 0-byte advisory lock
  `.armorer/apply.lock` (`recover-recovery-fileset.diff`; the lock is a known
  preserved input, `src/bootstrap/plan.rs:153`). The written policy file was
  removed; no state file; journal removed.
- All six pre-existing fixture files byte-identical to `examples/` (`cmp`,
  `recover-fixture-integrity.txt`); sentinel/bespoke/build.rs digests unchanged
  (`recover-unrelated-{before,after}.sha256` identical).
- No `target/`, no `BUILD_EXECUTED` — the build script never executed.

### 4.4 retry after recovery

Same approved plan apply → exit 0 `outcome:"applied"`; all 5 managed files
MATCH plan hashes (`recover-managed-after.sha256`); sentinel/bespoke still
unchanged; no journal/target/BUILD_EXECUTED remain.

Corroborating merged regression (not re-run here):
`tests/integrated_cli.rs:227 bootstrap_cli_write_failure_rolls_back_and_retry_succeeds_without_building`
covers the deterministic permission-based write failure with the same
rollback/retry/no-build assertions.

## 5. Consumer `conflict` — differing unowned target file (C6)

Setup: library copy with preexisting unowned `rust-toolchain.toml`
(`channel = "1.90.0"`, sha256 in `conflict-unowned-before.sha256`).

- plan → exit 0; `rust-toolchain.toml` `disposition:"conflict", owned:false`
  with full unified diff showing exact before/proposed bytes
  (`conflict-plan.json`); other four targets `create`.
- apply with that plan's digest → exit 1:
  `"transaction blocked: bootstrap conflict or unfinished transaction requires explicit resolution"`
  (`conflict-apply.json`). Unowned file bytes untouched (digest diff empty).
  No silent overwrite.

## 6. Acceptance-criteria mapping (rehearsal evidence only)

| Criterion | Evidence | Result |
|---|---|---|
| C1 read-only check, exit 2 + findings | §2.1, §3 | observed |
| C2 exact plan diffs | §2.2, §5 (full unified_diff + before/proposed bytes) | observed |
| C3 digest+preimage-bound apply; stale/tamper rejection | §2.3, §2.5, §3 wrong-digest | observed |
| C4 repeated apply no-op | §2.4, §3 replay (`unchanged`, mtimes identical) | observed |
| C5 partial-failure rollback preserving unrelated bytes | §4 live SIGKILL interruption → `rolled-back`, byte-identical unrelated files; merged regression cited | observed live |
| C6 no silent overwrite of unowned files | §5 conflict disposition + apply refusal; bespoke/sentinel preserved throughout | observed |

Symlink/traversal/concurrency rejection (part of C6's full scope) is covered by
the merged unit/integration tests, not exercised live in this rehearsal.

## 7. Explicit limitations

- Compiled **bootstrap-v1** catalog, workflows pin `772ca83e…`; bootstrap-v2
  successor not used (separate workstream).
- Policy is REHEARSAL-ONLY fixture content; **no owner production approval**.
- Receipt gates all false: `ci_verified`, `release_rehearsed`, `published`,
  `provenance_verified` — bootstrap alone grants none of them.
- **No publication, no tracker writes, no merges, no pushes.** armorer-test and
  pilots untouched.
- This rehearsal does **not** complete ticket #4; production catalog approval
  remains an owner gate.

## Artifact index

All JSON/exit/digest artifacts listed above live in this directory:
`build-identity.txt`, `catalog.json`, `rehearsal-policy{,-cli}.toml`,
`create-*` (check/plan/apply/replay/tamper/plan-2 + file lists + digests + mtimes),
`cli-*`, `recover-*` (incl. `recover-journal-at-interruption.json`,
`recover-interrupt.summary`), `conflict-*`, `tamper-original-rust-toolchain.toml`,
`final-no-build-artifacts.txt`, `examples-untouched.txt`.