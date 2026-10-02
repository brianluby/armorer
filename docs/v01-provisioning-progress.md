# v0.1 provisioning progress — 2026-10-02

This delivery overlay records source and runtime evidence against the
[full live acceptance ledger](https://github.com/brianluby/armorer/blob/90784cfcbd816567e969ac6a4721ebb58ced5ca0/docs/v01-acceptance.md).
Veans project 16 / epic #1 (database ID 1304); #4 and #5 remain In Progress.

| Slice / criterion | Immutable source | Local checks | Hosted / review evidence | Human acceptance |
| --- | --- | --- | --- | --- |
| #4 exact preview C1/C4 | PR #6, 90784cfcbd816567e969ac6a4721ebb58ced5ca0 | 81 tests, pinned build/fmt/strict Clippy; 14 schemas / 54 positive / 7 rejection examples; actionlint/Zizmor | Linux/macOS run 36980365033 success; CodeRabbit portability finding fixed, inline reply 4163806606 and resolved thread PRRT_kwDOU2HMOM6oQxPG | Merge approval requested and pending |
| #4 catalog/caller foundation C5/C6 | PR #7, 3496ad1787bf9adf318d840a397e695315e56ea3 | 75 tests; same build/fmt/lint/schema gates; all 18 publisher asset digests match compiled baseline; caller security scans pass | Linux/macOS run 36980656371 success; CodeRabbit reviewed all 12 files, no actionable findings; docstrings 84.62% | Open; no merge authorization |
| #4 C1–C6 integrated bootstrap | codex/bootstrap-provisioning, based on PR #7 source and policy foundation fb31d8057dd97b2f540ebbda4e3b187e06cf0dbe | 103 tests; all five targets, three profiles, independent patch reconstruction, every write/crash/commit boundary, recovery conflicts, replay and version guards; frozen 14 schema bytes plus new schemas/examples; catalog workflow audits pass | Final PR #8 head 241d09e28a6405cf2f0f215fde698c4f60f1b190: Linux/macOS run 36984121102 success; three-profile workflow discovery/policy walkthrough passes; CodeRabbit skipped the stacked base (not a source review), Copilot failed and Codex quota was unavailable; tracker comments 771/772 read back | Delivered candidate; acceptance pending |
| #4 repeated apply/recovery/customization C2/C3 | Accepted v1 plus proposed multi-file engine | Existing tests retained; 24 new bootstrap transaction/integration tests cover multiple scenarios and boundaries | Final PR #8 head and run above pass hosted checks; no unresolved GitHub inline threads, available bot reviews were unavailable/skipped | Full ticket remains open |
| #5 C1–C3 local migration implementation | codex/bootstrap-upgrades, based on PR #8 final head 241d09e28a6405cf2f0f215fde698c4f60f1b190; ADR 0010 and separately versioned upgrade/rollback schemas | Read-only base/current/candidate/pin/policy preview; explicit scoped imports; v1/v2 owner migration; preserved jobs/policies; every forward/reverse write, owner deletion and commit-marker boundary; exact reversal, replay, forged journals, versions, symlinks and concurrent writers; three real CLI profile walkthroughs restore original snapshots without builds | Exact final source, test counts and hosted/review receipts will be recorded in #5/epic comments and the durable goal ledger; second accepted catalog successor remains required for real between-catalog migration evidence | Implementation in review; ticket open, #4 prerequisite unaccepted |
| #8–#14 runtime/pilots/docs | Accepted schema/builder baseline and ledger references | No new full acceptance earned | Exact crypto, Apple, draft-state, adversarial integration, two pilots and dogfooding remain required | All nine tickets stay open |

## Current work and limitations

`src/ci_policy.rs` requires every explicit policy section; rejects unsupported
fields/versions, empty or duplicate allowlists, unsafe sources/ban IDs and invalid
exceptions; validates owner/reason and quoted Gregorian UTC expiry through the
current day with a 90-day ceiling. SPDX membership and actual dependency/advisory
coverage remain the existing enforced CI runtime's responsibility. It supplies
no default license list or exception and executes no repository code.

ADR 0009 is implemented for review with a separate version-two plan/state/journal
sharing the persistent kernel apply lock. Exact plans bind all five targets,
config/discovery/license/policy bytes, catalog, ownership and managed preimages.
Updates require matching owned bases; edits/deletions and differing unowned files
conflict. Recovery reconstructs every operation and ownership byte from the
approved plan; fault/crash coverage spans six writes and both commit-marker states.
The separately reviewed #5 upgrade engine now implements v1/v2 ownership migration,
three-way customization previews and exact historical reversal; it has not been
accepted/merged and has no second accepted catalog to qualify a real catalog change.
Earlier v1 plans must never gain new mutation targets implicitly.

The primary checkout remains clean at d5f048240db04fb40d73515e571a4028f049b973;
workflow checkout stays clean at 772ca83e386c883c88cc3b936d69f8cb3216c91e.
No merge, release/package publication, tag movement, repository-admin change,
pilot edit or delegation occurred. Local CodeRabbit export was rejected by
automatic approval review; it was not run. Available GitHub review findings were
read and adjudicated. Copilot review errors and Codex review quota exhaustion
are recorded as unavailable reviews, not approvals.

Finish #4/#5 human acceptance and catalog-successor integration. Develop #8/#9 against accepted
contracts; they unlock #10, #10 unlocks #11, and #5/#11 unlock the isolated pilots.
Human merges and protected external validation are separate outstanding gates.
