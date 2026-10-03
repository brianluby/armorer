# Bind the observation. Release the lock.

Snapshot: 2026-10-03 UTC. PR #21 advances from
`0ef1145bf7d6fc8a92a276ace0fe13dc6dab0baa` to
`56d7380a5534583b7973b5eedc55a1223b46b60e`, one commit changing five files.
Main remains `96457ee418439dde097339cfcd374c08f2cc98ad`. This continuation
reviews that successor diff; the earlier full/stack packets retain their scope.

Trust approved observations. Leave no completed lock behind. **This is the way.**

## Exact independent evidence requirements

`ArtifactEvidence::validate_against_requirements` now compares each producer
tool/database `observed_at` with its independent requirement, alongside the
existing kind/version/bytes/authentication/freshness-ceiling checks. A producer
cannot replace an approved older observation with its own fresh timestamp. Even
a different timestamp within the permitted age window fails exact comparison.

Required coverage now also compares the exact `exception_ids` vector. An allowed
exception record alone does not authorize its use for a coverage requirement
approved without that exception. Producer additions or removals fail; independent
requirements must explicitly retain the intended use. Existing review/expiry and
nonwaivable checks remain in force. These changes tighten validation without
changing serialized schema shapes or introducing a producer-derived approval.

The two new adversarial tests first establish structural validity, then require
independent-policy rejection. Their accepted controls bind the genuinely matching
timestamp or explicitly expected exception use.

## Transaction lifetime

The shared lock now returns a `TransactionLock` guard with an explicit unlock on
drop, before its file closes. Closing only one descriptor could retain the lock
while an inherited duplicate remained open. Apply/recover, bootstrap and upgrade
retain the guard for their operation. The new test keeps a duplicate descriptor
open, rejects another active transaction, drops the first guard, reacquires
immediately and confirms closing the old duplicate cannot release the new lock.

This addresses the demonstrated descriptor-lifetime mechanism while retaining
writer exclusion. It does not serialize arbitrary editors or authorize deleting
lock files/journals. The earlier ARM failure remains in the
[resolved-feature packet](2026-10-02-pr21-resolved-features.md); a new head's
result does not overwrite that failed receipt.

## Validation and remaining finding

An immutable export of `56d7380a5534` passes 73 focused local tests:

| Suite | Passed | Ignored |
| --- | ---: | ---: |
| Apply transaction unit tests, including the new descriptor regression | 10 | 0 |
| Bootstrap | 15 | 0 |
| Upgrade | 21 | 0 |
| Trust contracts, including the two new evidence regressions | 27 | 0 |

The guard regression also passed separately before that suite. It is counted
once above. These are affected-scope results, not a fresh full-suite, native
integration or complete signed-release qualification.

**[P3] The contributor schema-loop typo remains.** The literal `+` is still at
[CONTRIBUTING.md line 39](https://github.com/brianluby/armorer/blob/56d7380a5534583b7973b5eedc55a1223b46b60e/CONTRIBUTING.md#L39).
The earlier exact-binary proof explains how that separate loop entry stops the
`set -eu` export before v3/final replacements. This commit leaves the file
unchanged. The workflow's separate loop is unaffected.

Hosted [run 37104573003](https://github.com/brianluby/armorer/actions/runs/37104573003)
is bound to this exact head and completes successfully on Linux x64, Linux ARM
and macOS. The ARM Test step now passes; later native, schema and workflow-audit
steps also complete successfully. MacOS additionally completes its genuine
Developer ID reference and native adversary steps. This is a fresh candidate
receipt, separate from the failed preceding run and the documentation PR's CI.
It does not exercise the contributor guide's separate copyable schema loop or
supply the missing complete own signed producer positive.

Complete own signed producer/consumer acceptance, effective protected approvals,
Apple finalization, immutable publication/recovery, both pilots and human
acceptance remain open. No source, schema, provider setting, credential, tag or
pilot is changed by this documentation continuation.

Keep each receipt with its source. **This is the way.**
