# Combined source review: PR #21

Inspect the combined source. Keep the original receipts.
**This is the way.**

Snapshot: 2026-10-02 America/Los_Angeles / 2026-10-03 UTC. Reviewed
[PR #21](https://github.com/brianluby/armorer/pull/21) at
`c815a065fb08dbbdc0321b4a80bed66f12da9f64`, based on main
`96457ee418439dde097339cfcd374c08f2cc98ad`.
All seven currently open PR heads (#15–21) were checked against the live API;
#15–20 retain the heads in the [continuation packet](2026-10-02-pr-review-continuation.md).
The [original 13-head packet](2026-10-02-open-prs.md) remains a historical record.
No source PR, pilot, hosted comment/review, setting, tag or release was changed.

## [P1] Bind every retained platform reference to its selected final artifact

The [lookup at release.rs lines 247–258](https://github.com/brianluby/armorer/blob/c815a065fb08dbbdc0321b4a80bed66f12da9f64/src/verification/release.rs#L247)
requires reference byte equality and an allowed report role but does not check
`asset.subjects` against the current selection's `final_name`. Inventory validation
enforces each asset's own declared subject, and bundle authentication verifies
that retained report's bytes. Neither makes that report applicable to a different
selection that references it.

With two selections, an otherwise accepted evidence record for selection B can
reference selection A's retained authenticated build/diagnostic report. The
[evidence-chain validator](https://github.com/brianluby/armorer/blob/c815a065fb08dbbdc0321b4a80bed66f12da9f64/src/trust/evidence.rs#L344)
checks reference identities structurally, workflows, chain, selected final bytes
and requirements; it does not resolve the referenced asset's subject. The lookup
then accepts A's report while the CLI can ultimately report `provenance_verified:
true`. This misses the per-artifact scope promised by complete verification.

Require each matching allowed-role asset to name the current final artifact in
its subjects. Add an adversarial multi-selection regression using a retained
report scoped only to the other artifact, and retain an accepted same-subject
control. Gate complete-provenance acceptance on the fix and qualification.

This finding originated in a
[same-head Codex comment](https://github.com/brianluby/armorer/pull/21#discussion_r4171821542)
and was independently checked against the exact source. An isolated synthetic
probe added a second allowed target to the existing Linux contract fixture,
derived its complete inventory relationships, pointed the first selection's
build step at the second selection's retained build-evidence bytes, and confirmed:

1. `ReleaseInventory::validate_against` accepts the complete two-selection asset relationships.
2. `ArtifactEvidence::validate_against_requirements` accepts the changed reference.
3. The exact current byte/role lookup accepts it.
4. Adding the current-final subject condition rejects it.

The probe passed in a temporary export. It is a semantic/lookup demonstration,
not a forged cryptographic bundle or a full signed-release end-to-end positive.
The remaining genuine authentication prerequisites still apply. Verification
sources are byte-identical to #19 at `6c0c3d192360`; this integration inherits
the defect from that stack rather than introducing it through merge resolution.
It narrows the earlier packets' scoped “no additional confirmed defect” findings.
Exact-head inspection confirms the same missing lookup subject condition in all
open verification-stack PRs #15–19 as well as #21. PR #20 changes comments only
and does not contain this proposed complete-release consumer on its main base.

## [P3] Refresh integrated migration documentation

[`docs/runtime-native-v2.md` lines 17–18](https://github.com/brianluby/armorer/blob/c815a065fb08dbbdc0321b4a80bed66f12da9f64/docs/runtime-native-v2.md#L17)
still says no complete-release CLI exists, and
[`docs/cargo-graph-v2.md` lines 7–8](https://github.com/brianluby/armorer/blob/c815a065fb08dbbdc0321b4a80bed66f12da9f64/docs/cargo-graph-v2.md#L7)
says the library lacks complete-release verification. The integrated main.rs
routes explicit `native-v2`/`legacy-v1` contexts into `verify-release`; the
authenticated consumer runs selected graph validation and SBOM comparison after
inventory authentication. Refresh both pages while keeping runtime approval,
Build L2 acceptance and this review's artifact-scope blocker explicit.

The [candidate verification guide](../docs/candidate-verification.md) describes
that implemented routing and limitation. It does not promote the commands to main
or silently rewrite the source PR's historical pages.

## Runtime-version follow-up before a successor release

CodeRabbit's hardcoded-version observation is confirmed in source:
`declared_lock` and `authority::select` use `0.1.0`, whereas candidate rendering
and current plan/state checks use `CARGO_PKG_VERSION`. A crate-only version bump
would make candidate-lock validation fail with `compiled lock invalid` and
reject newly generated bootstrap bases. This does not break the reviewed
0.1.0 candidate; no accepted successor catalog/runtime is currently provided.

Before a new runtime version, define explicit current-versus-historical runtime
authority and regression coverage. Preserve immutable `bootstrap-v1` meaning;
blindly moving its runtime identity is not a migration decision. This is a
successor-version gate, not evidence of current operational upgrade coverage.

## Integration and validation evidence

The candidate reconciles preview, catalog, bootstrap, upgrade, verification,
schema and module routes. It retains the development/native gates and uses
distinct ADRs 0009 (bootstrap), 0010 (upgrades) and 0011 (native runtime).
The original structural conflict and ADR-number follow-ups are addressed in this
combined candidate; none of these future interfaces is yet on the main source.

| Evidence | Result and boundary |
| --- | --- |
| Exact-head offline local suite in immutable source export | 203 ordinary tests plus eight doctests passed; seven explicitly ignored native integrations |
| Integrated CLI regressions | All three profiles, build-script traps, bespoke workflow/Cargo.lock preservation, bootstrap replay, upgrade/reversal and context-first rejection passed |
| Locked offline local build | Passed on macOS ARM with Rust 1.95.0 |
| Temporary cross-artifact scope probe | One semantic/lookup probe passed; no full genuine signed success fabricated |
| Candidate guide procedures in fresh temporary consumers | Library, CLI and service: read-only bootstrap/upgrade/reversal plans, unchanged wrong-digest rejection, local apply and exact reversal; original inputs preserved |
| Documentation checks | 32 Markdown files, 171 local links and 26 shell blocks checked; no missing local targets, unbalanced fences or shell syntax errors |
| [Hosted run 37099233555](https://github.com/brianluby/armorer/actions/runs/37099233555) | Linux x64, Linux ARM and macOS 15 Rust/native jobs report success at `c815a065fb08` |
| Review state | CodeRabbit status success; unresolved source findings remain. Status is not human approval |

The ordinary suite's ignored groups are native Apple reference/adversary tests,
Sigstore, CycloneDX, native Cargo graph, inventory negative-ordering and actual
runtime distribution. They were not rerun locally. The existing historical
Unix-socket test required authorized execution outside the filesystem sandbox;
the isolated offline suite passed there. The temporary scope probe ran with the
compiler cache wrapper disabled after its sandbox startup was blocked.

Training approvals were mechanically supplied only for isolated synthetic
validation. They establish no owner approval for real adoption. The documentation
branch retains main's previously passed formatting, strict Clippy, locked build
and 95-test source boundary; this update changes guides and review records only.

The two imported migration documents and future version gate remain follow-ups.
Production catalog/root approval, final-byte producer, own complete signed
non-publishing positive/negative controls, protected Apple production, immutable
publication, both pilots and human acceptance remain open. Hosted fixture success
does not close them. No merge or release is recommended while the P1 stands.

Keep the scope bound to the cargo. **This is the way.**
