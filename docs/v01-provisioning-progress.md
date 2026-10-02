# v0.1 provisioning progress — 2026-10-02

This delivery overlay records source and runtime evidence against the
[full live acceptance ledger](https://github.com/brianluby/armorer/blob/90784cfcbd816567e969ac6a4721ebb58ced5ca0/docs/v01-acceptance.md).
Veans project 16 / epic #1 (database ID 1304); #4 remains In Progress.

| Slice / criterion | Immutable source | Local checks | Hosted / review evidence | Human acceptance |
| --- | --- | --- | --- | --- |
| #4 exact preview C1/C4 | PR #6, 90784cfcbd816567e969ac6a4721ebb58ced5ca0 | 81 tests, pinned build/fmt/strict Clippy; 14 schemas / 54 positive / 7 rejection examples; actionlint/Zizmor | Linux/macOS run 36980365033 success; CodeRabbit portability finding fixed, inline reply 4163806606 and resolved thread PRRT_kwDOU2HMOM6oQxPG | Merge approval requested and pending |
| #4 catalog/caller foundation C5/C6 | PR #7, 3496ad1787bf9adf318d840a397e695315e56ea3 | 75 tests; same build/fmt/lint/schema gates; all 18 publisher asset digests match compiled baseline; caller security scans pass | Linux/macOS run 36980656371 success; CodeRabbit reviewed all 12 files, no actionable findings; docstrings 84.62% | Open; no merge authorization |
| #4 explicit policy foundation | codex/bootstrap-provisioning, based on PR #7 source | 4 new policy tests; primary Python runtime independently agrees on same-day / 90-day / expired / too-far / invalid-date cases; UTC day 20728 agrees with datetime oracle | Pending integrated provisioning candidate and hosted validation | Pending |
| #4 repeated apply/recovery/customization C2/C3 | Existing accepted v1 toolchain engine | Existing tests retained in every suite | Multi-file fault/crash/recovery extension remains required | Full ticket remains open |
| #5 upgrades/migration; #8–#14 runtime/pilots/docs | Accepted schema/builder baseline and ledger references | No new full acceptance earned | Exact crypto, Apple, draft-state, adversarial integration, two pilots and dogfooding remain required | All nine tickets stay open |

## Current work and limitations

`src/ci_policy.rs` requires every explicit policy section; rejects unsupported
fields/versions, empty or duplicate allowlists, unsafe sources/ban IDs and invalid
exceptions; validates owner/reason and quoted Gregorian UTC expiry through the
current day with a 90-day ceiling. SPDX membership and actual dependency/advisory
coverage remain the existing enforced CI runtime's responsibility. It supplies
no default license list or exception and executes no repository code.

ADR 0009 proposes a separately versioned multi-file plan/state/journal sharing
the persistent kernel apply lock. It still needs implementation, exact preview,
ownership/preimage validation, complete meaningful fault/crash/recovery tests,
repeatability, legacy ownership migration handling and hosted provisioning evidence.
Earlier v1 plans must never gain new mutation targets implicitly.

The primary checkout remains clean at d5f048240db04fb40d73515e571a4028f049b973;
workflow checkout stays clean at 772ca83e386c883c88cc3b936d69f8cb3216c91e.
No merge, release/package publication, tag movement, repository-admin change,
pilot edit or delegation occurred. Local CodeRabbit export was rejected by
automatic approval review; it was not run. Available GitHub review findings were
read and adjudicated. Copilot review errors and Codex review quota exhaustion
are recorded as unavailable reviews, not approvals.

Continue #4 multi-file provisioning, then #5. Develop #8/#9 against accepted
contracts; they unlock #10, #10 unlocks #11, and #5/#11 unlock the isolated pilots.
Human merges and protected external validation are separate outstanding gates.
