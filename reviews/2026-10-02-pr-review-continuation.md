# Armorer PR review continuation

Snapshot: 2026-10-02 America/Los_Angeles / 2026-10-03 UTC. Main is
`96457ee418439dde097339cfcd374c08f2cc98ad`. This is a read-only source review;
no hosted review/comment, merge, tag, release or pilot mutation was performed.
The [original packet](2026-10-02-open-prs.md) retains the earlier 13-head review.

## Current open set

| PR | Exact head | Hosted Rust evidence |
| --- | --- | --- |
| #15 | `005f08099a2b08681080a4e532b3615f3232d449` | [run 37011354685](https://github.com/brianluby/armorer/actions/runs/37011354685), Linux x64/ARM64 and macOS |
| #16 | `c649051a04e49c25885502db65b1aeb6f14a6bba` | [run 37028663996](https://github.com/brianluby/armorer/actions/runs/37028663996), Linux x64/ARM64 and macOS |
| #17 | `cd47288b3b17268af079e882614431f7520caec3` | [run 37053468605](https://github.com/brianluby/armorer/actions/runs/37053468605), Linux x64/ARM64 and macOS |
| #18 | `a5d19efa10b3b5596930ad5eedb75ce7c18598a0` | [run 37072086688](https://github.com/brianluby/armorer/actions/runs/37072086688), Linux x64/ARM64 and macOS |
| #19 | `6c0c3d192360b9dc983533f427babe323eb0baeb` | [run 37086525867](https://github.com/brianluby/armorer/actions/runs/37086525867), Linux x64/ARM64 and macOS |
| #20 | `29c8e6fadf7f2c4e98eb67cbae42938e4e8b2b16` | [run 37088196339](https://github.com/brianluby/armorer/actions/runs/37088196339), Linux x64 and macOS |

All listed jobs report success at their exact heads. All six PRs have zero
unresolved inline threads in the live GraphQL response. CodeRabbit status is
success; several automated reviews failed or skipped work. Copilot explicitly
failed for both new PRs. These observations are not human acceptance.

## Review findings and follow-ups

No new confirmed runtime/security defect was found in the scoped #19/#20 review.
Original integration conflicts and duplicate ADR 0009 remain follow-ups until
resolved and validated against a combined source. Parent-branch merges do not
establish that combined source on main.

**[P3] Refresh #19's hosted qualification wording with a dated receipt.**
The [qualification section at the reviewed head](https://github.com/brianluby/armorer/blob/6c0c3d192360b9dc983533f427babe323eb0baeb/docs/apple-native-verification-v1.md#qualification-and-acceptance)
still says hosted macOS 15 qualification is pending. The same head's macOS job
now passes, including explicit native reference/adversary steps. Add a dated
receipt for that run while preserving the earlier fresh-host failure and keeping
independent source review, protected production signing, complete own rehearsal
and human acceptance open. The stale wording can make readers repeat a completed
fixture qualification or confuse it with the genuinely incomplete release gates.

## #19 source review

Compared the complete diff against #18. Reviewed private context/proof binding,
all-required-selection behavior, Linux rejection, library exemption, bounded
gzip/USTAR parsing, fixed private snapshot, Mach-O/signature bounds, native
Developer ID/team requirement, hardened runtime, CMS leaf/timestamp binding,
ticket lookup, post-call rehash/expiry and CLI success placement. Reviewed the
fixture qualifier, dependency additions and native workflow steps.

The consumer authenticates the complete file set before Apple inspection and
returns CLI success only after every required native check. Offered paths are
never extracted; the payload is never executed. Missing/unsupported native
checks fail closed. The same independent context must bind both private results.

The certificate digest is authenticated producer evidence; independent policy
supplies the expected team. Current-certificate CMS validation intentionally
rejects expired signer certificates. Online lookup requests do not guarantee a
fresh successful service response or authenticate producer submission/log origin.
The 60-second subprocess deadline is enforced; the 20-minute native-stage limit
is checked after API returns, not an interrupt. These are documented limitations,
not stronger claims inferred from a passing fixture.

Local ordinary tests passed on an immutable export of the exact #19 head.
Seven explicit tests were ignored: two Apple native groups, Sigstore, CycloneDX,
native Cargo graph, inventory failure ordering and complete runtime distribution.
The historical Unix-socket rejection test required execution outside the filesystem
sandbox. No credentials or pilot checkout were used. Genuine native groups were
not rerun locally in this review; their hosted receipts are separate evidence.

## #20 source review

Compared the patch to its parent. It adds exactly 15 Rust documentation lines in
`src/preview.rs` and `tests/preview.rs`, with no removed or changed executable
source. Comments match fixture isolation, exact consumer snapshots, preimage
checks, saved-plan identity, diff reconstruction and preview alias regressions.
No new behavioral defect was identified. Its exact-head hosted Linux/macOS jobs
pass; CodeRabbit reports completed same-head coverage with no actionable findings.

## Changed integration boundary

Ancestry confirms #6, #7 and #10 on the main source above. Original #8/#9 and
#11–14 implementation heads are outside it even though #9/#11–14 now show merged
in GitHub: their targets were parent branches. #8 is closed without a merge
record. The [development map](../docs/development-status.md) records this boundary.
The original packet remains a historical snapshot and was not rewritten.

The documentation branch was rebased onto this already merged main source.
Its current source passes Rust 1.95.0 formatting, strict all-target/all-feature
Clippy, locked build and **95 tests**; one genuine Sigstore test remains ignored
by the ordinary suite. Documentation checks cover all 14 schema outputs, local
links, shell syntax, source-bound branch-guide links and the temporary onboarding
and exact-preview walkthrough. This is documentation/local source evidence,
not complete producer, publication, pilot or Build L2 acceptance.

Preserve each boundary and receipt. **This is the way.**
