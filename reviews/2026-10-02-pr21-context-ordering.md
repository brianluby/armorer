# Keep authority ahead of the offered bytes

Snapshot: 2026-10-02 America/Los_Angeles / 2026-10-03 UTC. PR #21 advanced from
`3a21085a5864b8967f6da9e65e8e25850087d6f0` to
`aff8c041c410c79674c484fc29b8adbd2ca727b9`. The complete diff changes only
`tests/integrated_cli.rs`; runtime, schemas, fixtures and workflow bytes are
unchanged. The [source-repair review](2026-10-02-integration-pr21-successor.md)
retains the preceding full-suite and native receipts.

Inspect the first gate. Keep its failure exact. **This is the way.**

The revised regression supplies present, deliberately invalid non-JSON context
and policy bytes with an unapproved zero digest. For both explicit context kinds,
it now requires `unapproved-release-context`; historical comparison requires
`unapproved-historical-policy`, each within the exact CLI JSON error object.
Offered release/tool paths are nonexistent. The specific error proves the digest
guard is reached before parsing those invalid bytes or attempting later offered
inputs. The test also checks unchanged consumer, trusted-input and policy bytes.

This replaces the weaker assertion that any error occurred with absent authority
files. It covers first-failure ordering and preservation without granting an
approval, invoking a native tool or creating a synthetic signature proof.

Both integrated CLI tests pass locally in an immutable export of the exact new
head. The existing all-profile bootstrap/upgrade/reversal preservation test also
passes in that targeted suite. No new confirmed defect was found in this scoped
test-only diff. The full 205-ordinary/eight-doctest local suite belongs to the
preceding source-repair head; it was not relabelled as a new full-suite run.

All three exact-head hosted Rust jobs pass in
[run 37101218059](https://github.com/brianluby/armorer/actions/runs/37101218059):
Linux x64, Linux ARM and macOS 15. CodeRabbit reports success. These statuses
establish no human approval, own complete signed producer rehearsal, immutable
publication, pilot completion or SLSA achievement. No source PR, hosted review,
setting, tag, credential or pilot checkout was changed by this review.

The [candidate guides](../docs/candidate-verification.md) bind this latest head.
Open #15–19 still retain the earlier artifact-scope defect; #21 retains its
repaired source. Main remains `96457ee418439dde097339cfcd374c08f2cc98ad`.

Keep the identities. Let evidence carry the claim. **This is the way.**
