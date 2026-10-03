# Armorer v0.1 acceptance ledger

> **Historical tracker snapshot, 2026-10-02.** The source and ticket states below
> preserve that observation. They are not current implementation or live tracker
> status. Use [the development map](development-status.md) and
> [documentation coverage](documentation-coverage.md) for the current source and
> guide boundaries. Later integration/test evidence does not supply tracker
> closeout or human acceptance. Retain the old receipt. **This is the way.**

Reconciled from live Veans project **16**, epic **#1 / database ID 1304**, on
2026-10-02. Database IDs below are API identities; `#N` is the project index.
This ledger records delivery, human acceptance and operational validation separately.
Keep immutable receipts and previous evidence when updating it across goal continuations.

## Accepted baseline

* Armorer main: `d5f048240db04fb40d73515e571a4028f049b973`;
  [PR #5](https://github.com/brianluby/armorer/pull/5) is merged.
  [Merge-head run 36895204858](https://github.com/brianluby/armorer/actions/runs/36895204858)
  passed Linux and macOS (live API recheck on 2026-10-02).
* Workflow main: `772ca83e386c883c88cc3b936d69f8cb3216c91e` (live remote recheck).
* Live completed tickets: #2, #3, #6, #7 and #16. #4 is in progress;
  #5 and #8–#14 remain open. L3 #15 and optional #17–#24 are deferred/unscheduled.
* Existing contract tests and unsigned builders do not establish Build L2.

## Review and validation rules

Each criterion needs implementation/test references, immutable candidate and accepted
source/PR heads, hosted receipts and any required pilot/runtime receipt. A reviewed
subset never completes its enclosing ticket. A schema or mock is not runtime proof.
Scoped commits/pushes, PRs, tracker reconciliation and approved-environment validation
are authorized. Merges, public release/package publication, tag movement, repository
administration and collecting credential values require separate human authorization.
No delegation is authorized. One owner coordinates contracts and Git state.

## Live criteria and remaining gates

| Criterion | Requirement | Delivery / evidence | Acceptance / operational gate |
| --- | --- | --- | --- |
| [#4 / 1307](https://kanban.luby.us/tasks/1307) C1 | Check is read-only; plan shows exact diffs and manual prerequisites; apply requires matching file preimages and plan digest. | Preview slice: src/preview.rs, tests/preview.rs, docs/apply.md; 81 Rust tests, pinned build/fmt/strict Clippy and 14-schema checks pass | Pending preview PR review/hosted validation; provisioning remains open |
| [#4 / 1307](https://kanban.luby.us/tasks/1307) C2 | Repeated apply is a no-op; partial failures roll back managed files; dirty unrelated files remain byte-identical. | Accepted toolchain-only engine: src/apply.rs, tests/apply.rs; PR #3, baseline main | Revalidate against multi-file provisioning; ticket remains open |
| [#4 / 1307](https://kanban.luby.us/tasks/1307) C3 | Never silently overwrite unowned workflows, deny policy or toolchain customizations; reject symlink/path traversal and concurrent apply. | Accepted toolchain-only engine: src/apply.rs, tests/apply.rs; PR #3, baseline main | Revalidate against multi-file provisioning; ticket remains open |
| [#4 / 1307](https://kanban.luby.us/tasks/1307) C4 | Complete the original reviewable diff criterion: show before and proposed content together, including create/update/conflict cases, while preserving read-only planning and bounded input handling. | Preview slice: src/preview.rs, tests/preview.rs, docs/apply.md; 81 Rust tests, pinned build/fmt/strict Clippy and 14-schema checks pass | Pending preview PR review/hosted validation; provisioning remains open |
| [#4 / 1307](https://kanban.luby.us/tasks/1307) C5 | Track a separate implementation slice under this bootstrap work for reviewed CI/build caller workflow templates, catalog-backed lock/tool pins and explicit project policy provisioning. Current apply intentionally manages only rust-toolchain.toml; preserve custom workflows/jobs and policies and never silently overwrite them. Reviewed upgrades and migration remain #5. | Pending implementation | Pending local, hosted and human acceptance |
| [#4 / 1307](https://kanban.luby.us/tasks/1307) C6 | Reviewed caller CI/build workflows, authenticated catalog-backed lock/tool pins and explicit project policy; transactional multi-file apply and recovery. | Pending implementation | Pending local, hosted and human acceptance |
| [#5 / 1308](https://kanban.luby.us/tasks/1308) C1 | Upgrade previews workflow/tool SHA changes, compatibility and policy changes; no automatic mutable-major pin adoption. | Pending implementation | Pending local, hosted and human acceptance |
| [#5 / 1308](https://kanban.luby.us/tasks/1308) C2 | Three-way merge uses stored generated base; edited managed files yield explicit conflicts rather than replacement. | Pending implementation | Pending local, hosted and human acceptance |
| [#5 / 1308](https://kanban.luby.us/tasks/1308) C3 | Test old config versions, rollback and imported existing workflows; preserve unrelated customized jobs. | Pending implementation | Pending local, hosted and human acceptance |
| [#5 / 1308](https://kanban.luby.us/tasks/1308) C4 | Compatibility/policy changes, explicit downgrade controls, old versions, imported workflows and rollback; preserve custom jobs and policy. | Pending implementation | Pending local, hosted and human acceptance |
| [#8 / 1311](https://kanban.luby.us/tasks/1311) C1 | Use SHA-pinned consolidated actions/attest with explicit paths for final artifacts, SBOM files and authenticated inventory. | Pending implementation | Pending local, hosted and human acceptance |
| [#8 / 1311](https://kanban.luby.us/tasks/1311) C2 | Verify bytes, repository/source ref and commit, signer workflow/repository/digest, hosted runner and predicate types; compare SBOM predicate to published JSON. | Pending implementation | Pending local, hosted and human acceptance |
| [#8 / 1311](https://kanban.luby.us/tasks/1311) C3 | Trust policy is independent of release content; exact historical allowlist only; no authenticity-failure fallback; bundle and offline trust-root paths documented. | Pending implementation | Pending local, hosted and human acceptance |
| [#8 / 1311](https://kanban.luby.us/tasks/1311) C4 | Rehash downloads; compare verified SBOM predicates with published JSON and required graph. Authenticate independent policy/root/catalog expectations and inventory. Genuine signed fixtures and exact historical routing; no authenticity fallback. | Pending implementation | Pending local, hosted and human acceptance |
| [#9 / 1312](https://kanban.luby.us/tasks/1312) C1 | Protected caller signing environment with exact ref restrictions; named credentials only; no secret values requested or logged. | Pending implementation | Pending local, hosted and human acceptance |
| [#9 / 1312](https://kanban.luby.us/tasks/1312) C2 | Isolate build code from signing; verify build handoff, record unsigned/signed executable and archive digests, workflow pins, team identity and notarization evidence. | Pending implementation | Pending local, hosted and human acceptance |
| [#9 / 1312](https://kanban.luby.us/tasks/1312) C3 | Attest final signed/notarized/package bytes; verify codesign/expected team/hardened runtime/timestamp and Apple acceptance on macOS; fail every target on signing failure. | Pending implementation | Pending local, hosted and human acceptance |
| [#9 / 1312](https://kanban.luby.us/tasks/1312) C4 | Authenticate bounded run/attempt/source-bound unsigned handoffs before credentials. Exercise all supported macOS targets and online/offline package limits; required failure blocks release. | Pending implementation | Pending local, hosted and human acceptance |
| [#10 / 1313](https://kanban.luby.us/tasks/1313) C1 | Distinguish supported/unsupported/unknown GitHub capabilities, including public/private attestation eligibility and environment/ruleset availability; fail secure release closed. | Pending implementation | Pending local, hosted and human acceptance |
| [#10 / 1313](https://kanban.luby.us/tasks/1313) C2 | Check immutable setting with narrowly scoped admin-read capability; 403 is unknown and blocks publication; report manual setup without exposing credentials. | Pending implementation | Pending local, hosted and human acceptance |
| [#10 / 1313](https://kanban.luby.us/tasks/1313) C3 | Draft exact inventory, re-download and verify served bytes, approve/publish only after gates, then verify immutable release and release attestation. | Pending implementation | Pending local, hosted and human acceptance |
| [#10 / 1313](https://kanban.luby.us/tasks/1313) C4 | Serialize release/tag operations; identical owned drafts resume by digest; conflicting drafts or published bytes never overwritten; stable tags immutable and moving tags controlled. | Pending implementation | Pending local, hosted and human acceptance |
| [#10 / 1313](https://kanban.luby.us/tasks/1313) C5 | Authenticate exact trigger/ref/ancestry/actor and capability observations. Freeze and re-download exact assets, recheck mutable prerequisites immediately before approval-gated publication; reject cross-run substitution and replacement. Full acceptance requires Apple parity. | Pending implementation | Pending local, hosted and human acceptance |
| [#11 / 1314](https://kanban.luby.us/tasks/1314) C1 | Real Sigstore fixture verification plus pinned-gh integration rejects tampered artifacts/SBOMs, wrong source/signer/predicate, missing or extra assets and invalid manifest signatures. | Pending implementation | Pending local, hosted and human acceptance |
| [#11 / 1314](https://kanban.luby.us/tasks/1314) C2 | Reject unauthorized triggers/refs, injected inputs, cross-run substitution, failed signing, capability denial and historical downgrade attempts. | Pending implementation | Pending local, hosted and human acceptance |
| [#11 / 1314](https://kanban.luby.us/tasks/1314) C3 | Exercise conflicting retries, draft mutation before publish, concurrent tags and failed publication/tag move; assertions show no unauthorized publish or execution. | Pending implementation | Pending local, hosted and human acceptance |
| [#11 / 1314](https://kanban.luby.us/tasks/1314) C4 | Genuine signed Sigstore fixtures and pinned real gh complement mocks. Cover races, concurrent tags, owned-draft retries/mutations, publication failures and historical downgrade; prove rejection causes no unauthorized execution/publication/tag mutation. | Pending implementation | Pending local, hosted and human acceptance |
| [#12 / 1315](https://kanban.luby.us/tasks/1315) C1 | Wait for current Momus work to finish; freshly inspect main and preserve unrelated edits; use isolated adoption branch/worktree. | Pending implementation | Pending local, hosted and human acceptance |
| [#12 / 1315](https://kanban.luby.us/tasks/1315) C2 | Compare Armorer plan against Momus three-target pipeline and consumer contract; do not duplicate or regress its protections. | Pending implementation | Pending local, hosted and human acceptance |
| [#12 / 1315](https://kanban.luby.us/tasks/1315) C3 | Hosted CI and complete signed rehearsal pass; preserve pre-attestation v0.2.0 compatibility and record evidence; no merge or release publication without human authorization. | Pending implementation | Pending local, hosted and human acceptance |
| [#12 / 1315](https://kanban.luby.us/tasks/1315) C4 | Fresh active-work inspection and independent consumer verification of a signed non-publishing Momus rehearsal; preserve three targets and reviewed historical compatibility. | Pending implementation | Pending local, hosted and human acceptance |
| [#13 / 1316](https://kanban.luby.us/tasks/1316) C1 | Use public brianluby/rusty-brain, freshly verified main and isolated worktree; do not disturb active feature branches. | Pending implementation | Pending local, hosted and human acceptance |
| [#13 / 1316](https://kanban.luby.us/tasks/1316) C2 | Cover libraries plus rusty-brain/rb-hooks/rb-install deliverables, default/local feature configurations and native dependency scope; preserve bespoke semantic/contract CI. | Pending implementation | Pending local, hosted and human acceptance |
| [#13 / 1316](https://kanban.luby.us/tasks/1316) C3 | Verify exact multi-binary inventory and target SBOM pairing; hosted CI and release rehearsal with no publication; list external model/native downloads and remaining gaps. | Pending implementation | Pending local, hosted and human acceptance |
| [#13 / 1316](https://kanban.luby.us/tasks/1316) C4 | Exact rusty-brain/rb-hooks/rb-install inventories, default/local features and target-specific SBOM pairing; list actual model/native downloads and limitations. | Pending implementation | Pending local, hosted and human acceptance |
| [#14 / 1317](https://kanban.luby.us/tasks/1317) C1 | MIT licensing, contribution/security policy, version/support contract, examples for every profile and public/private capability matrix. | Pending implementation | Pending local, hosted and human acceptance |
| [#14 / 1317](https://kanban.luby.us/tasks/1317) C2 | Document credential names and setup scopes, signing approvals, onboarding/upgrade, historical verification, outage/retry recovery and manual settings. | Pending implementation | Pending local, hosted and human acceptance |
| [#14 / 1317](https://kanban.luby.us/tasks/1317) C3 | Report configured, CI verified, release rehearsed, published and provenance verified using separate artifact/source-bound evidence; dogfood Armorer before any public release. | Pending implementation | Pending local, hosted and human acceptance |
| [#14 / 1317](https://kanban.luby.us/tasks/1317) C4 | All supported profiles, upgrades, offline/historical verification, outages/retries/recovery, credential names/scopes/approval prerequisites; usable newcomer path and hosted Armorer dogfooding. Reconcile stale proposed/pending labels against accepted evidence. | Pending implementation | Pending local, hosted and human acceptance |

## Current slices and continuation

1. `codex/bootstrap-previews`: additive exact UTF-8 before/proposed views, complete
   linear diffs, stale saved-plan checks and legacy blocker visibility. Existing
   v1 plan/schema/digest and toolchain-only apply semantics are preserved.
2. Next #4 slice: authenticated immutable catalog/pins, explicit reviewed policy,
   caller workflow rendering and bounded transactional multi-file provisioning.
   Review version compatibility before extending the frozen v1 interfaces.
3. #5 follows bootstrap; #8/#9 unlock #10; #10 unlocks #11; #5/#11 unlock pilots.
   Continue independent authorized work while human merge or external gates wait.

Current product states: configuration-valid/toolchain configuration only;
CI evidence exists for the accepted baseline. Full configured/release-rehearsed/
published/provenance-verified v0.1 outcomes remain incomplete. No public release
is required or authorized by this goal.

## Preview slice local validation — 2026-10-02

Pinned Rust 1.95.0: locked build, formatting, strict all-target/all-feature Clippy
and **81 tests** pass (11 library, 10 apply, 24 inspection, 10 preview,
25 trust-contract, 1 doctest; no ignored tests). Existing **14** v1 schemas match
CLI output unchanged. Independent hash-locked jsonschema validation passes
**54 positive examples / 7 structural rejections**. Actionlint and Zizmor strict offline scans pass using native distributions
verified by the accepted workflow catalog installer. Hosted status is recorded
with the exact candidate PR separately.

New tests independently reconstruct candidates with the system patch tool and
cover create, unchanged, owned update, customization conflict, legacy blocker,
CRLF/Unicode/final-newline identity, stale/forged saved plans, symlinks,
non-UTF-8/oversized inputs/views and whole consumer snapshots with build-script
execution markers. These are local preview/transaction evidence, not signing,
publication, credentialed rehearsals or Build L2 evidence.

## Preview hosted receipt and review disposition

[PR #6](https://github.com/brianluby/armorer/pull/6), source
`ead990ecb2832ace5ca03b5288d8b8877ccca420`, passed hosted Linux/macOS validation
in [run 36979744806](https://github.com/brianluby/armorer/actions/runs/36979744806).
CodeRabbit's one inline finding is valid: resolve the test-only patch executable
through PATH for Unix hosts with a different installation layout. The remediation
also documents the three private preview helpers to address the docstring warning.
New candidate local/hosted evidence is retained in the PR and #4 comments before
requesting acceptance. Copilot returned a review error and Codex reported review
quota exhaustion; neither is an independent completed review. No merge occurred.

The next #4 foundation is isolated at `codex/bootstrap-catalog`: embedded catalog
identity from workflow commit `772ca83e386c883c88cc3b936d69f8cb3216c91e`, all
18 primary-source distribution digests reverified on 2026-10-02, fixed unprivileged
caller rendering and explicit native-target pin IDs. Integration with project
policy and transactional provisioning is still required.
