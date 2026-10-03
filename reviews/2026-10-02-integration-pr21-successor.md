# Review the repaired candidate

Snapshot: 2026-10-02 America/Los_Angeles / 2026-10-03 UTC.
PR #21 advanced to `3a21085a5864b8967f6da9e65e8e25850087d6f0`.
The [earlier finding and receipts](2026-10-02-integration-pr21.md) remain tied to
`c815a065fb08dbbdc0321b4a80bed66f12da9f64`; they were not rewritten.
Main remains `96457ee418439dde097339cfcd374c08f2cc98ad`.

Inspect the repair. Retain the old evidence. **This is the way.**

## Artifact scope repaired at this head

The [retained-reference helper](https://github.com/brianluby/armorer/blob/3a21085a5864b8967f6da9e65e8e25850087d6f0/src/verification/release.rs#L356)
now requires matching bytes, an admitted report role and a subject naming the
current final artifact. The complete consumer calls this helper for every step
platform reference and Apple notarization-log reference. That closes the missing
subject check identified in the earlier review.

Its new regression tests each admitted report role: same-subject acceptance,
other-subject and missing-subject rejection, then wrong-byte and wrong-role
rejection. The test invokes the helper used by the production consumer. It checks
post-authentication scope semantics, not a genuine complete signed producer run.
The exact-head local suite passes, including this regression. No new confirmed
defect was found in the six-file successor diff.

Open PRs #15–19 retain their previous heads and missing subject check. The fix
exists in the combined #21 candidate only. No source worktree was edited and no
hosted review/comment was posted by this documentation review.

## Version authority and migration prose clarified

The successor centralizes `bootstrap-v1` runtime `0.1.0` as immutable authority.
Catalog selection now explicitly rejects a compiled crate version that differs,
with a requirement for a separately reviewed catalog successor. The new unit
control accepts 0.1.0 and rejects patch/minor/prerelease/build-metadata changes.
The declared-lock check uses that same frozen constant. This preserves catalog
meaning and makes the unsupported future runtime boundary explicit; it does not
implement a successor catalog or migration between two accepted catalogs.

The runtime-native and Cargo-graph pages now name the complete consumer/CLI and
explicit context routing. They retain production/rehearsal acceptance limits.
The stale no-complete-verifier claims from the earlier review are addressed.

## Validation boundary

An immutable source export of the new head passed **205 ordinary tests and eight
doctests**, including both new regressions. Seven explicit native integration
groups remain ignored by that local suite. The existing Unix-socket control ran
through authorized execution outside the filesystem sandbox. The previous
candidate's fixture and guide receipts remain intact; bootstrap/upgrade CLI
syntax, ordinary version-0.1.0 behavior and fixtures are unchanged in this diff.

Hosted [run 37100465930](https://github.com/brianluby/armorer/actions/runs/37100465930)
is a distinct exact-head receipt. At the initial successor observation, Linux
x64 passed while Linux ARM/macOS were running. Previous run 37099233555 cannot
qualify the successor. A later dated observation may update the development map;
this initial observation stays visible here.

Later same-day readback confirms that run completed successfully at
`3a21085a5864b8967f6da9e65e8e25850087d6f0` on Linux x64, Linux ARM and macOS 15.
This hosted qualification remains separate from the seven locally ignored groups
and from missing producer/publication/pilot acceptance.

The operational documentation update checks 36 Markdown files, 203 local links
(including 14 heading links), 28 shell blocks and balanced fences. Root-export and
release-metadata command syntax/fields match live CLI help and current primary
GitHub documentation. No production root export, policy approval, remote recovery,
alias write or published-incident exercise was performed.

The [current candidate guides](../docs/candidate-verification.md) now bind this
new head. Whole-release own producer/consumer positive and negative qualification,
production catalog/root approval, protected Apple transformations, effective
approvals, immutable backend transitions/races, both pilots and human acceptance
remain separate gates. This source repair establishes no release publication,
SLSA achievement or merge authorization.

Keep the scope with the artifact. **This is the way.**
