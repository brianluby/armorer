# ADR 0014: review evidence lives outside the repository

Status: decision accepted by the repository owner, 2026-11-18.

The `reviews/` directory held 147 tracked files: dated review receipts, rehearsal
exit codes and checksums. Their status drifted from the code they described, and
every new receipt added review cost to a pull request without adding anything a
reviewer could verify from the diff. A receipt is evidence about a commit, not
part of the source that commit builds.

Receipts now live in the git-ignored `code_reviews/` directory on the machine that
produced them. The root `.gitignore` also ignores `/reviews/`. The architecture
and roadmap review is the plan of record cited by ADRs, so it stays tracked at
[`docs/architecture-review-2026-11-16.md`](../architecture-review-2026-11-16.md).
History keeps the earlier tracked copies at commit `292e9b6`; read one with
`git show 292e9b6:reviews/<file>`. This supersedes the sentence in
[ADR 0012](0012-pre-release-contract-consolidation.md) that historical receipts
stay under `reviews/` only as to where they live. Their meaning is unchanged:
they describe the state they recorded and are not edited to match later code.

Documents that linked to a receipt keep the sentence and the link text, and name
the receipt by its original path as plain text. Evidence is no longer reviewable
in a pull request, so a reader who needs a receipt reads it from history or from
the owner's local copy. A receipt still earns credit only for the exact source
commit it names. See [the contributor note](../../CONTRIBUTING.md#review-receipts-and-rehearsal-evidence).
