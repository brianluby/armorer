# Armorer v0.1 completion plan — drafted 2026-11-12

Status snapshot, review findings and sequenced plan to finish Veans project 16
(epic #1 / db 1304). Draft for owner review; no tracker writes, merges or
releases are authorized by this document.

## 1. Where the project stands

Source main: `ab526d7` (PR #27 merged 2026-10-04). Workflows main: `b94969c`
(PR #22 merged + Momus review enablement). Both clean; local gates re-verified
2026-11-12: `cargo fmt --check`, `clippy -D warnings`, full `cargo test --locked`
all pass on macOS arm64 (250 passed, 9 ignored); hosted Development run
37218646841 passes all three
platforms at `ab526d7` including the ignored Apple/Sigstore/CycloneDX/gh gates.

Implementation is substantially complete on both mains: 12 CLI commands,
27 schemas, transactional bootstrap/upgrade, strict complete/historical
consumers, owned-draft publication controller (native writes deliberately
blocked), and the full unprivileged producer/intake/writer workflow stack.
What remains is mostly **operational qualification, human authority
decisions, and rehearsal evidence** — but three concrete implementation
items survive: the catalog pin advance, the attestation/signing producer
jobs, and the two publication gates + controller workflow wiring.

### Ticket state (project index / db id)

| Ticket | Bucket | Reality |
| --- | --- | --- |
| #4 / 1307 check-plan-apply | In Review | Code merged (PR #21). CLI acceptance + rollback evidence exists on unmerged `evidence/*` branches. Remaining: production catalog/runtime approval + end-to-end onboarding evidence. |
| #5 / 1308 upgrades | In Progress | Code merged (PR #9 via #21); bucket stale. Remaining: operational upgrade rehearsal on a real consumer with stored bases. |
| #8 / 1311 attestations+verifier | In Review | Code merged. Evidence audit (2026-11-04) classifies what synthetic vs genuine evidence proves. Remaining: production roots/catalog/verifier approval + own signed producer→consumer rehearsal. |
| #9 / 1312 Apple signing | Todo | Consumer-side native Apple verification merged; **producer-side protected signing workflow does not exist**. Blocked on Apple credentials + protected environment + unsigned payload handoff rehearsal. |
| #10 / 1313 publication | Todo | Controller foundation merged incl. PR #26 fixes. Native writes blocked by two unsupported gates: current-attempt environment enforcement, cross-runner serialization. Remaining: provider trials, those two gates, real draft→publish→verify rehearsal. |
| #11 / 1314 adversarial tests | Todo (stale) | Large adversarial suites already merged (240+ tests). Remaining: rehearsal-bound negatives (real provider ambiguity/race cases) + gate #26 executions. |
| #12 / 1315 Momus adoption | Todo | Not started; momus-review repo has no armorer.toml. |
| #13 / 1316 Rusty Brain adoption | Todo | Not started; 18-crate workspace, no armorer.toml. |
| #14 / 1317 runbooks | Todo (stale) | Onboarding/recovery/upgrades docs largely merged via #22/#25; needs final reconciliation after rehearsals. |
| #15 / 1318 SLSA L3 | Todo (future) | Explicitly deferred; not v0.1 acceptance. |
| #16 / 1319 supply-chain assessment | Done | Spun out optional tickets #17–#24. |
| #17–#24 optional capabilities | Todo | CodeQL, auditable binaries, cargo-vet, Linux egress, rebuild experiment, Scorecard, dist spike, crates.io OIDC spike. None required for L2. |
| #26 / 1480 four native gates | Todo | **3 of 4 executed 2026-11-12** (see §3). Gate 4 blocked on expired CI artifacts + independent spec approval. |
| #27 / 1481 OCR residuals U01–U14 | Todo | Holding ticket; no confirmed bypass. Split-on-selection policy. |

### Branch / worktree inventory

Armorer repo:
- `evidence/acceptance-4`, `evidence/acceptance-8`, `evidence/signing-readiness`,
  `evidence/gates-readiness` — the only branches with content not on main:
  three review receipts under `reviews/` plus one regression test
  (`bootstrap_cli_write_failure_rolls_back_and_retry_succeeds_without_building`,
  re-verified passing 2026-11-12). `gates-readiness` is the superset.
  **Action: one docs+test PR from `evidence/gates-readiness`, then delete all four.**
- `codex/bootstrap-provisioning`, `codex/bootstrap-upgrades` — superseded by
  merged PR #21; no unique src files; 68 commits behind. **Delete after owner OK.**
- `codex/fix-candidate-schema-guide`, `codex/preview-docstrings-successor` —
  PRs #23/#20 closed as superseded; content byte-identical or incorporated.
  **Delete after owner OK.**
- 4 external worktrees (`~/repos/armorer-evidence-*`, `-gates-readiness`,
  `-signing-readiness`) — prune after evidence merge.

Workflows repo: all 19 `codex/*` branches merged into main; PRs #5–#20 still
OPEN but their heads are ancestors of main (stacked PRs never auto-closed).
**Action: close #5–#20 with superseded-by-#21/#22 notes; delete branches.**

### Documentation drift found

- `docs/v01-acceptance.md` + `docs/development-status.md` still describe PR #26
  as unmerged/pending human review; it merged 2026-10-04 (`7b0cca9`) and #27
  landed after. Ledger needs a reconciliation pass (baseline heads, PR #26/#27
  rows, gate #26 progress).
- `docs/v01-provisioning-progress.md` frozen at 2026-10-02; superseded by ledger.
- Cross-repo pin currency: current workflows main builds its trusted intent
  validator from armorer `1615339` (PR #1 era). Verified: config field set and
  plan contract (`schema_version`, `mode`, `config_sha256`, workspace inputs)
  are unchanged on main, so the frozen pin still functions — deliberate
  isolation, not a bug. Refreshing it to `ab526d7`+ is a reviewed workflow
  change to schedule with the catalog pin advance (Phase 1), not before.
- Compiled bootstrap catalog authority (`src/catalog.rs:14` WORKFLOW_COMMIT,
  TOOLS_SHA256, examples/bootstrap-catalog callers) still pins workflows
  `772ca83` — the pre-integration main. The accepted producer stack lives at
  `b94969c`. The advance is a reviewed **successor authority** (`bootstrap-v2`)
  because `bootstrap-v1` IDs are frozen by contract; real implementation
  work, not just an approval (Phase 1, step 6).

### Rehearsal vehicle state (armorer-test)

Remote `brianluby/armorer-test` (public) is further along than ticket comments
suggest: inert dependency-free fixture + single-owner rehearsal exception
documented at `d114999`; environment `release-signing` exists with required
reviewer brianluby (ID 3779002, self-review exception), main-only deployment
branch policy. No workflows, no releases, no catalog/lock/roots approved.
Local clone `~/repos/armorer-test` is an unborn empty checkout — stale; fetch
before use. This is the designated rehearsal consumer for #4/#8/#10.
Still missing there (verified via API 2026-11-12): immutable releases
**disabled**, no tag rulesets, main branch unprotected, no `release-publish`
environment. The controller's native gates require all of these before any
stage/publish trial — add them to Phase 1 owner setup.

## 2. Code review findings

(Four parallel slice reviews: transactions, verification, core/config,
workflows. CodeRabbit CLI could not run: local review fails auth despite
`auth status` OK; `--remote` requires base+source refs and no open PR exists.
Findings below are from the slice reviews + direct inspection.)

### Direct findings (verified this session)

- **Catalog advance must be a successor authority** — `src/catalog.rs:14`
  pins pre-integration workflows `772ca83`; `src/upgrade/authority.rs` freezes
  `bootstrap-v1` semantics, so the advance adds `bootstrap-v2` (new digest,
  callers, fixtures) while retaining v1 for existing state (Phase 1, step 6).
- **Attestation producer not wired** — no `actions/attest` invocation exists
  in armorer-workflows; only consumer-side qualification (Phase 3, step 14).
- **Signing workflow absent, prior art proven** — `brianluby/apple-signing`
  reusable workflow already used by momus-review at pin `bac7bb3` (Phase 3, step 13).
- **Controller job not wired + two hardcoded Unsupported gates** —
  `src/verification/publication/github.rs:344-350` (Phase 4, steps 15–16).
- **Rehearsal repo settings incomplete** — armorer-test: immutable releases
  off, no tag ruleset, main unprotected, no publish environment (Phase 1, step 9).
- **Security policy unpublished** — private vulnerability reporting disabled,
  no SECURITY.md, support policy draft (Phase 6, step 22).
- **Stale documentation** — ledger/status still call PR #26 unmerged; it and
  #27 merged 2026-10-04 (Phase 0, step 3).
- **Stale local checkouts** — `.worktrees/armorer-workflows` 65 commits behind
  (missing the entire producer stack); `~/repos/armorer-test` unborn while
  remote has the fixture. Both mislead future sessions (Phase 0, step 2).
- **Gate 3 not in CI** — workflows CI runs the Python producer but never the
  Rust interop reader with retained receipts (§3).
- **No confirmed security defect found by direct inspection**; the compiled
  validator pin (`1615339`) is old but contract-compatible (verified field
  set + plan shape), deliberate isolation. Minor observation: `asset_name`
  (src/trust/mod.rs:186) admits `.`/`..` as names; harmless today because
  `compare_directory` requires exact set equality with inventory-declared
  names and regular-file type, but worth a one-line guard if the name
  validator is ever reused elsewhere (candidate U09-quality item).
- **Transport contract drift (minor)**: Rust publication adapter pins
  `X-GitHub-Api-Version: 2026-03-10` (github.rs:218,533,627) while the
  workflows Python adapters pin `2022-11-28` (transport_v1.py:41). Both
  fixed, but the two sides observe provider settings under different API
  contracts; docs disagree too (candidate-workflow-prerequisites vs
  platform-prerequisites). Reconcile during Phase 1 qualification.
- **Legacy tool-naming question resolved (no defect)**: legacy trust-v1
  fixtures use bare names (`cargo-cyclonedx`) consistently in lock, catalog
  and evidence; native v2/v3 fixtures use target-qualified pin names
  (`{name}--{target}`) with `material.target` binding. Both paths match;
  the two naming worlds never mix within one context version.
- **Dead code cleanup candidate**: `trust/policy.rs:233`
  `compare_historical_bytes` has no callers and carries a stale "#8" pending
  comment; live path is `HistoricalByteMatch::verify`. Delete in Phase 0.

### Slice reviewer findings

#### Transactions slice (apply/bootstrap/upgrade) — verdict: correct, confidence 0.8

# ReviewTransactions — final report

**Verdict: correct.** No provable unintentional blocking defect in the transactional provisioning slice; consistently fail-closed with sound journal/rollback ordering. Confidence 0.8 (full static read + cross-boundary tracing; nothing executed).

##### Architecture
Three versioned approval-bound writers share one kernel lock + durable-journal pattern:
- v1 (apply.rs/plan.rs/preview.rs): toolchain-only; digest-bound Plan (compact JSON, digest field blanked); re-inspect under lock; journal before writes; before-image checks; ownership State last; committed marker; journal removal; recover finalizes/rolls back.
- v2 bootstrap: fixed 5-target inventory, compiled catalog authority, same journal shape, guard_legacy/guard_v1 cross-version gates.
- upgrade: three-way merge::decide, limited caller_extension import, separate forward/rollback approvals, state.rs generated-vs-applied images re-qualified against compiled catalog, exact reverse images.
Shared: safe_path, read_small (1 MiB), replace (sync temp + atomic rename, persist_noclobber creates), sync_directory, flock TransactionLock with explicit Drop unlock, duplicate-key-rejecting decode with round-trip check.

##### Findings (all minor/info; no blocking bug)
1. [minor, documented] Residual path TOCTOU: safe_path/read_small/replace validate then use without immediate re-validation; concurrent local writer could swap parent to symlink. Mitigated by root.canonicalize() in mutating entries, re-safe_path in replace/ensure_parent, rename replaces (not follows) destination symlink. Threat model excludes concurrent hostile writers (U07).
2. [info, fail-closed] apply.rs:506-510 committed-replay predicate collapses I/O errors into generic conflict message; debugging clarity only.
3. [info, documented] Empty created directories retained on rollback (bootstrap/transaction.rs:203); unrelated bytes preserved.
4. [info] replace copies mode via fs::metadata (follows symlinks) — subsumed by #1; rename still replaces the link.

##### Verified statically
Digest re-derivation vs independent expected everywhere; journal-before-write + ownership-last + both crash windows consistent; rollback validates ALL preimages first, reverse restore, refuses intervening edits; cross-version guards; no build-script execution (snapshot + metadata --no-deps --offline --locked + env_clear); bounded reads/outputs; symlink/traversal rejection; regular-file flock.

##### Not verified
Real power-loss durability; flock under genuine multi-process contention; cargo metadata behavior in practice; File::try_lock MSRV (assumed; parent session built+tested successfully).

Stubs/TODOs: none (only test-module unwrap/expect).

#### Verification slice (release consumer/sigstore/apple/publication) — verdict: no critical/major defect

# ReviewVerification — final report

**Verdict: no confirmed critical or major defect.** Strict complete-release consumer traced end to end (context → Sigstore → inventory → assets → SBOM/graph → Apple → publication) plus historical and runtime paths. Consistently fail-closed, digest-bound, inventory-first.

##### Findings (all info / documented limitation)
- F1 info — sigstore.rs:489-499 `validate_output` doesn't semantically re-validate verifiedTimestamps type/freshness; delegated to pinned hash-qualified gh whose exit must succeed; policy re-validated before/after. Defense-in-depth gap only.
- F2 info — sigstore.rs:640-659 legacy build-type builder relaxation looks loose but cert runnerEnvironment=="github-hosted" enforced unconditionally at :521; self-hosted cannot slip through. Do not "tighten" blindly.
- F3 info — publication/mod.rs:729-742 publish runs before release-attestation verification (attestation only exists post-publish); success claim gated on verification; recover_published re-verifies without re-publishing. Crash-recovery design, not fail-open.
- F4 info — trust/policy.rs:233 `compare_historical_bytes` is DEAD CODE (no callers) with a pending "#8" comment; live path is HistoricalByteMatch::verify. Cleanup candidate.
- F5 documented limitation — historical path can't detect "downgrade of a release that has valid modern attestations"; explicit separate op, allowlist-gated, reports authenticity:not-established; reviewer policy decision, matches contract.

Residual not fully confirmed by reviewer: validate_remote per-asset uploader check (PARENT CONFIRMED present at mod.rs:551-555: allowed_publisher_ids.contains(&asset.uploader)); config loaders (digest-gated; parent confirmed read_small bounded).

##### Architecture
Proof types with private constructors + compile_fail doctests (cannot deserialize from JSON). Expectations only from independently approved context/policy digests. Inventory authenticated before claims parsed; exact directory↔inventory set; preflight budgets; every bundle verified; subject digest/name/single-subject binding; cert SAN/issuer/source/signer/run/trigger equality; --deny-self-hosted-runners + cert extension + policy flag; snapshot re-hash around native calls; env_clear + private tempdirs; bounded JSON/base64/tar/Mach-O.

##### Ignored qualification tests (9)
real_sigstore (qualified gh+root), real_cyclonedx (qualified validator), release_context:392 (gh+CycloneDX), release_context:894 (three exact-head CI artifacts), cargo_graph_v2:286 (retained receipts), github.rs:1017/1066 (qualified gh+GET auth), apple.rs:796 (clang+codesign), apple.rs:868 (public reference+ticket lookup).
Parent executed 3 of the 4 ticket-#26 gates successfully 2026-11-12; CI runs the rest at head.

##### Verified statically
Fail-closed propagation; no build-script execution; exact pinning + re-hash; inventory-first; identity equality; symlink/traversal rejection; no creds in logs; historical separation; repeated time revalidation.

##### Not verified
Crypto soundness of pinned tools (qualification tests cover); live Apple/GitHub services; runtime native_header vs real binary.

Stubs/TODOs: none in src/verification; one pending comment on dead code (F4).

#### Core/config/trust slice — verdict: correct, confidence 0.7

**Method note:** reviewer verified every file against git HEAD ab526d7 via shell after read-tool content divergence; findings grounded in on-disk bytes.

**Overall: correct — no confirmed critical/major unintentional defect in the core/config/trust slice.** Confidence ~0.7 (static only).

Findings:
1. [info, intentional] Publication writes permanently blocked: two required gates hardcoded `GateState::Unsupported` (github.rs:344-350); `require_ready` demands all Enforced. Deferred to #10 by design.
2. [verified fail-closed] 403/404→Unknown→block; capability `satisfies` requires Available+Enforced+evidence for Required. No fail-open found.
3. [minor, documented] `read_small` (lib.rs:124-137) lacks the post-open inode re-check that verification/io.rs has; doc states stable-filesystem assumption. Config/lock reads weaker than release-file reads vs concurrent local attacker (U07 family).
4. [minor, documented] `safe_path` doesn't check supplied root for symlinks; callers canonicalize first (discovery.rs:354). Caller-discipline reliance.

Verified invariants: no build-script execution in discovery (metadata --no-deps --offline --locked, .rs placeholders, env_clear); no command injection (arg arrays, validated identifiers); no secrets in logs (token via child env only, stderr discarded); fail-closed capability/trigger/receipt gates; owned-draft retry/recovery state machine (Creating/Uploading terminal-ambiguous, recover_published read-only); catalog hash-before-parse matches compiled anchor; deny_unknown_fields + bounds throughout; spot-checked schemas match Rust types.

Open question (unconfirmed): legacy catalog↔lock tool naming — render_lock keys `{name}--{target}` vs context.rs matches_tool `pin.name == tool.name`; needs ToolEvidence.name format confirmation. Native v2/v3 use pin.matches instead.

Stubs: none except the two intentional Unsupported gates; `unreachable!()` Plan arms guarded.

Not verified: crypto/Apple internals (other slice), no builds/tests run.

#### Workflows repo (all slices incl. 3 sub-reviews) — verdict: no critical/major fail-open

# ReviewWorkflows — consolidated final report (all slices)

Basis: /tmp/armorer-wf-gate @ b94969c (current origin/main of brianluby/armorer-workflows), clean. Read-only. Three deep-dive scouts (policy/transport, payload/signature, graph-v2/v3); parent line-verified every reported finding; excluded one scout claim that did not reproduce (transport_v1._timestamp already raises Failure correctly).

##### Verdict
**No critical or major fail-open defect.** Consistently fail-closed: exact-key-set gates before field access, returncode enforcement, SHA-pinning of every action/tool/Node/gh distribution, symlink/traversal rejection, duplicate-key JSON rejection, sterile credential-whitelisted envs, every authority flag hardcoded false. No workflow grants id-token:write, release-write scope, or signing credentials; no step executes consuming-repo code with elevated permissions. Producer-OIDC/intake/writer authentication implemented+hardened but deliberately unwired.

**Headline (documented, not bug):** release trust chain terminates at state="layout-complete-unverified"; nothing in the workflows repo cryptographically verifies assembled Sigstore bundles — the strict consumer lives in the armorer Rust repo (merged). L2 requires the complete producer→consumer rehearsal (plan Phase 3).

##### Confirmed defects (all minor)
- D1 schema drift: cargo-graph-v2.json:299-313 omits run_id/run_attempt from required, minimum:0 vs producer/gate requiring both >0. Schema-only consumers weakened; Python gate compensates. Fix: required + minimum 1.
- D2 schema drift: Selection.required omits binary; DependencyContext.required omits target (:213-224, :85-87). Same class; fix both.
- D3 schema drift (fail-closed): ci.py load_policy has no length bounds vs schema maxLength 128/2048; pathological input → CI red, not fail-open. Add matching bounds.
- D4 latent runtime-integrity: build.py:502-505 required_runtime omits build_v3.py (the v3 entry module); untracked rejection only on caller root. Tracked here + fresh hosted checkouts ⇒ latent. One-line fix + untracked rejection on runtime_root.
- D5 installer hardening asymmetry (DoS/cleartext-leg only; all SHA-pinned before extraction, no integrity impact): tools.py plain urlopen; policy_tools no final-HTTPS check; transport_tools no total deadline; producer_launcher strongest. Converge on strictest pattern.
- D6 policy observation: sole write/secret edge is momus.yml (pull-requests:write + TYPESAFE_API_KEY), continue-on-error:true, fail-on-blocking:false — cannot block merges. Attestation-pinned same-repo bot; matches ticket U02 deliberate choice.
- D7 stale checkout: .worktrees/armorer-workflows 65 commits behind (parent finding, confirmed).

##### Verified-safe highlights
metadata_adapter $CARGO shim uses repr literals + exact argv compare; grep shell=True/os.system/eval/exec empty; graph reconciliation exact-compiled-set with cycle/duplicate/foreign rejection; handoff run/attempt binding + transport _run provider authentication; ZIP intake exact 5-leaf set with traversal/symlink/encryption rejection + expansion caps; OIDC full-claim binding; Node pin qualification with gpgv signature check (not executed).

##### Stubs/disabled
None except intentional retired Node-first entrypoints (throw on use) and the unwired credentialed producer path. Sole "not finished" marker: docs/final-payload-v1.md:14 "production integration is still in progress".

##### Not verified
gpgv runtime correctness; Sigstore bundle verification (absent by design, lives in armorer repo); adapter_args vs real cargo-cyclonedx argv; pins vs upstream bytes; live GitHub platform guarantees (code fails closed if absent).

## 3. Gate #26 execution evidence (2026-11-12, this workstation)

1. `real_pinned_gh_release_attestation_matches_frozen_baseline` — **PASS**.
   gh 2.102.0 qualified via `scripts/qualify-test-verifier.py` (digest-pinned),
   existing read-only auth. 1 passed, 5.05s.
2. `real_native_publication_observations_never_grant_write_authority` — **PASS**.
   Same qualified binary; fixed GET routes, write refusal. 1 passed, 3.63s.
3. `native_python_graphs_match_independent_rust_reader` — **PASS**.
   Producer: workflows `b94969c` clone at /tmp/armorer-wf-gate,
   `tests/build_cases_v2.py` with hash-locked py3.14 venv (system py3.15 is
   outside the locked support range — pip correctly refused unhashed wheel).
   Receipts: /tmp/armorer-gate-receipts.BdO4pM. 1 passed.
4. `real_native_runtime_distribution_matches_independent_provider_qualification`
   — **BLOCKED**. All retained CI artifacts expired (14-day retention; repo
   reports 0 artifacts). Needs: fresh Development run at accepted head,
   download three `runtime-candidate-*` artifacts in-window, independently
   approved runtime spec + digest + source identity (human gate).

Full receipt: `reviews/2026-11-12-ticket-26-gate-execution.md` in this
directory. Gates 1–2 also ran in armorer CI at this
head. Workflows CI runs `build_cases_v2.py` (producer side) but never retains
`--receipt-directory` output nor runs the Rust interop reader — the cross-repo
gate 3 exists only as an operator procedure. Consider wiring it into CI.

## 4. Completion plan

Sequenced by dependency. Every step: isolated branch/worktree, local gates +
hosted CI at exact head, evidence receipt, human merge authorization.
Owner decisions are marked **[OWNER]**.

### Phase 0 — hygiene (no authority needed, ~half day)
1. PR from `evidence/gates-readiness`: 3 review receipts + rollback regression
   test + today's gate receipt. Verified 2026-11-12: merges cleanly into main
   (merge-tree), fmt/clippy clean, full suite passes (251 passed, 9 ignored).
   Merge with authorization; delete 4 evidence branches + 4 external worktrees.
2. Close workflows PRs #5–#20 as superseded (heads already in main); delete
   merged branches both repos; prune stale `.worktrees/` checkouts **[OWNER]**
   (they are the user's; confirm before removing).
3. Reconcile `docs/v01-acceptance.md` + `development-status.md` to `ab526d7` /
   `b94969c`: PR #26/#27 merged, gate #26 progress, ticket buckets (#5, #11,
   #14 stale). Update Veans buckets with readback.
4. Triage #27 U01–U14: keep holding ticket; split only selected items.
   Natural splits during this plan: U05 (strict catalog pin-set equality —
   decide with the Phase 1 catalog advance), U06 (max review lifetime —
   decide with Phase 1 policy approval), U09 (positive-control-first fixture
   mutations — fold into Phase 2 #11 work). U07 hostile-FS races stays a
   documented contract limit unless the threat model changes; U01–U04,
   U08, U10–U14 remain revisit-triggered, no v0.1 work.
5. Review-fix batch (small, one PR per repo, from §2 findings):
   - armorer: delete dead `compare_historical_bytes` (trust/policy.rs:233);
     optional one-line `asset_name` guard against `.`/`..`.
   - workflows: schema drift D1/D2 (require run_id/run_attempt minimum 1,
     binary, target in cargo-graph-v2.json — regenerate via the Rust writer
     contract, keep byte-identical fixtures); D3 length bounds in
     ci.py load_policy; D4 add build_v3.py to required_runtime + untracked
     rejection on runtime_root; D5 converge installer download hardening.
   Each fix gets a regression check per repo conventions; no behavior change
   to verified gates.

### Phase 1 — catalog advance + production authority (~2–3 days, blocking)
These mix one reviewed source change with human approvals no agent can
self-grant; everything downstream waits:
6. Implement the catalog advance as a **successor authority**, not an edit:
   `src/upgrade/authority.rs` freezes `bootstrap-v1` ("stable IDs have
   immutable meanings… add a separately reviewed successor, rather than
   moving this ID") and `decode_state` rejects states whose authority
   contradicts the compiled catalog. Add `bootstrap-v2` (revision 2) pinning
   workflows `b94969c`+ with new TOOLS_SHA256; keep the v1 entry for existing
   consumers' state validation and the two-catalog upgrade rehearsal (#5).
   Regenerate example callers/fixtures; refresh the workflows-side validator
   pin in the same reviewed change set; full gates + hosted CI.
   **[OWNER]** approves the resulting catalog bytes as production authority.
7. Approve production **trust roots / verifier identities / publication policy**
   through the independent channel (docs/release-readiness.md §Prepare).
8. Re-run Development CI at accepted head; retain three runtime-candidate
   artifacts; approve runtime spec; execute gate #26.4. Closes #26.
9. Confirm `armorer-test` as the rehearsal vehicle (single-owner exception
   documented at `d114999`; `release-signing` environment already has required
   reviewers + main-only branch policy). Complete its provider setup:
   enable immutable releases, add tag ruleset protecting update/deletion
   with no bypass actors, protect main, create `release-publish` environment
   with required reviewer. All are owner-only settings changes; the
   controller's inspect gate verifies each before any write.

### Phase 2 — close the source-side tickets (~2–3 days)
10. **#4**: end-to-end bootstrap on a disposable consumer with the approved
   catalog (Phase 1, step 6); retain check/plan/apply/replay receipts; merge evidence;
   move #4 → Done.
11. **#5**: upgrade rehearsal on the same consumer. Ledger C3/C4 require
    migration/reversal between **two independently accepted catalog
    revisions** — the Phase 1 pin advance (`772ca83` → `b94969c`+) naturally
    provides exactly that pair; retain both approved catalog bytes. Cover
    edited-managed-file conflict, import and rollback paths; #5 → Done.
12. **#11**: fold gate #26 results + rehearsal negatives into the adversarial
    ledger; remaining provider-bound negatives land with Phase 3/4 rehearsals;
    then #11 → Done.

### Phase 3 — own signed release rehearsal (#8, #9) (~3–5 days, largest block)
All credentialed jobs here must satisfy the producer startup boundary v1
(docs/producer-startup-v1.md in workflows): fixed startup environment before
any earlier action receives credentials, isolated-Python launcher, no
Node-first entry. "No credentialed release job is wired yet" is the current
state; the composite actions exist and are rehearsed unprivileged.
13. **#9 producer signing**: implement the protected signing workflow in
    armorer-workflows (does not exist yet — only unsigned payload intake is
    merged). Prior art exists and is proven: `brianluby/apple-signing`
    reusable `sign-macos.yml` (Developer ID + notarization, one executable
    in tar.gz), already used by momus-review's release.yml at pin `bac7bb3`
    with a `release-signing` environment. Decision **[OWNER]**: qualify and
    SHA-pin that reusable workflow into the armorer catalog vs. build a
    native armorer signing job. Either way: exact-ref environment, named
    credentials only (APPLE_CERTIFICATE_P12_BASE64, APPLE_CERTIFICATE_PASSWORD,
    APPLE_ID, APPLE_APP_SPECIFIC_PASSWORD — provision for armorer-test via
    apple-signing's private configure-secrets helper), build/sign isolation,
    unsigned→signed digest chain, final-byte attestations. Note: apple-signing
    v1 handles one executable per tar.gz; armorer's multi-target CLI release
    needs one call per target or the documented extension.
14. **#8**: the attestation **producer** step is also not wired: no workflow
    in armorer-workflows invokes `actions/attest` (only the consumer-side
    pinned-source qualification exists, docs/offline-verifier.md). Add the
    SHA-pinned attest job over final artifacts + SBOMs + authenticated
    inventory, protected per the whole-job startup contract. Then run the
    complete own producer→consumer rehearsal with approved roots; retain
    per-artifact evidence. #8 → Done only on that positive, not fixtures.

### Phase 4 — publication (#10) (~3–4 days)
15. Implement the two unsupported gates (hardcoded `GateState::Unsupported`
    at `src/verification/publication/github.rs:344-350`): current-attempt
    environment enforcement and cross-runner serialization. Design decision
    **[OWNER]**: GitHub deployment protection + concurrency groups vs.
    external authenticated lock; document the residual trust assumption —
    the controller docs explicitly disclaim atomic compare-and-swap from GETs.
16. Wire the protected controller workflow job in armorer-workflows ("source
    workflow publication is not wired yet" per owned-draft-controller-v1.md):
    the Rust controller exists but no trusted job invokes it with GH_TOKEN.
17. Provider trials on armorer-test: inspect → plan → stage → approve →
    publish → verify immutable release + release attestation; ambiguity/race
    negatives against the real provider; safe-retry journal evidence.
18. #10 → Done; feed provider-bound negatives back into #11 ledger.

### Phase 5 — pilot adoption (#12, #13) (~2–3 days each, parallelizable)
19. **#12 Momus**: isolated worktree of momus-review; author armorer.toml +
    owner-reviewed policy; check/plan/bootstrap; upgrade path; full rehearsal
    per dogfooding.md; never modify Momus implementation incidentally.
    Ticket criteria: preserve pre-attestation **v0.2.0** historical
    compatibility (tag exists; verify-historical-bytes path) and do not
    duplicate/regress its existing three-target protections.
    Friction found: Momus pins Rust **1.98.1** while armorer's own examples use
    1.95.0; the consumer config chooses its exact toolchain (`armorer.toml
    toolchain`, validated x.y.z), so 1.98.1 is expressible — but the catalog's
    pinned CI tools must be qualified against it. Momus already has its own
    release stack (attest.yml, builder.yml, release.yml); adoption must preserve
    those unowned workflows (ownership conflict path) **[OWNER decisions]**.
20. **#13 Rusty Brain**: same flow on the 18-crate workspace; cover
    rusty-brain/rb-hooks/rb-install deliverables, default/local feature sets,
    exact multi-binary inventory + target SBOM pairing; preserve bespoke
    semantic/contract CI; list external model/native downloads and gaps.
    Do not disturb active feature branches (feat/*, codex/*).
    Friction: toolchain is `stable` (mutable) — armorer requires an exact pin,
    an intentional consuming-repo change needing owner sign-off; existing
    deny.toml policy must be reconciled with armorer's reviewed ci-policy.
21. Both pilots publish nothing without separate authorization; receipts cite
    exact heads.

### Phase 6 — docs + release decision (~1 day)
22. **#14**: reconcile onboarding/upgrades/recovery runbooks against rehearsed
    reality; refresh acceptance ledger to final heads. Two policies are still
    drafts needing owner action: security-reporting (private vulnerability
    reporting is **disabled** on the repo; pick channel + enable) and
    support-policy (no version/tag exists; approve scope or keep draft).
    Complete a newcomer operational walkthrough + offline/recovery receipt.
    #14 → Done.
23. Epic #1: assemble per-artifact evidence table; **[OWNER]** decide v0.1
    release (tag, publication) — explicit authorization required.
24. Optional #17–#24 and future #15 (L3) stay backlog; schedule only on owner
    selection. The supply-chain assessment's own priorities: P1 = #17 CodeQL,
    #18 auditable binaries + Syft inventory; P2 = #19 cargo-vet, #20 Linux
    egress, #21 rebuild experiment, #22 Scorecard; P3 = #23 dist, #24 crates.io
    OIDC. If any are pulled forward, follow that order; none blocks v0.1 L2.

### Critical path
Phase 1 approvals → Phase 3 signing credentials → #8 rehearsal → #10 gates →
pilots → release decision. Phases 0/2 run immediately in parallel.

### Effort summary

| Phase | Agent work | Owner gates |
| --- | --- | --- |
| 0 hygiene | ~0.5 day | merge approval, branch deletions |
| 1 catalog + authority | ~1 day | catalog/roots/policy approval, repo settings, Apple credentials |
| 2 source tickets | ~2 days | evidence acceptance |
| 3 signing + attest | ~3–4 days | credential provisioning, environment approval |
| 4 publication | ~3 days | serialization design choice, publish approvals |
| 5 pilots | ~3–4 days (parallel) | pilot policy reviews, toolchain decisions |
| 6 docs + release | ~1 day | security/support policy, release authorization |

Rough total: ~2–3 weeks of agent work interleaved with owner decisions;
the calendar is dominated by the human approval gates, not code.
