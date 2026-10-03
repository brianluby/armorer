# Approve the features that actually stand

Snapshot: 2026-10-02 America/Los_Angeles / 2026-10-03 UTC. PR #21 advances from
`aff8c041c410c79674c484fc29b8adbd2ca727b9` to
`0ef1145bf7d6fc8a92a276ace0fe13dc6dab0baa`. The new commit changes 18 files and
adds an explicit verification-context v3. Main remains
`96457ee418439dde097339cfcd374c08f2cc98ad`.

Bring independent authority. Compare the entire set. **This is the way.**

## Source and validation

The previous graph reader required requested root features to be present but
could accept additional activated root features. The successor's old-context
reader requires defaults disabled and exact equality with literal requests.
Native v3 adds an independently approved resolved set for every selection key;
the context rejects missing/extra keys and malformed sets before retaining that
authority. Complete verification compares actual root features with that set
before SBOM reconciliation. Context approval still precedes parsing, and explicit
version selection never falls back to another format or historical mode.

This permits separately reviewed default/implied feature closures while rejecting
expanded substitutions. It leaves the producer's host/target aggregation limit
visible. It does not derive expectations from offered release graphs or execute
Cargo on the consumer. Native catalog v2 and Cargo graph v2 keep their serialized
meanings; all 23 earlier schema files remain byte-identical. The new v3 exporter
matches its committed schema, bringing the candidate's total to 24.

An immutable export of this exact head passes formatting and these local suites:

| Suite | Passed | Ignored | Boundary |
| --- | ---: | ---: | --- |
| Cargo graph v2 | 10 | 1 | Exact/implied/default feature sets, expansion adversaries and real offline metadata with a build trap |
| Release context | 18 | 2 | Independent v3 authority, malformed maps, no fallback and context-first rejection |
| Integrated CLI | 2 | 0 | All three profiles, preserved customizations, transaction reversal and all three context modes |
| Trust contracts | 25 | 0 | Existing semantic contracts and committed schema checks |
| Bootstrap | 15 | 0 | Existing transaction suite, including the replay test that fails on hosted ARM |

These focused results are not a new full-suite or genuine signed-release receipt.
The ignored native controls require their distinct qualifications.

## Confirmed contributor-guide finding

**[P3] Remove the literal `+` from the schema loop.**
At [CONTRIBUTING.md line 39](https://github.com/brianluby/armorer/blob/0ef1145bf7d6fc8a92a276ace0fe13dc6dab0baa/CONTRIBUTING.md#L39),
the new schema entry begins with a literal `+`. The continued `for entry in`
list therefore includes `+` as its own entry. The exact candidate binary rejects
`armorer schema +` with Clap exit 2. Under the guide's `set -eu`, this stops the
staged export before the v3 entry or final schema replacements. Remove that
character and verify every entry matches one of the 24 schema names. The normal
workflow's separate schema loop has no corresponding typo; a passing schema CI
step does not validate this copyable contributor procedure.

## Hosted failure remains evidence

[Run 37102828787](https://github.com/brianluby/armorer/actions/runs/37102828787)
is bound to this exact candidate head. Its Linux ARM test job fails at
`tests/bootstrap.rs:130` in
`bootstrap_all_profiles_is_read_only_then_provisions_all_five_files_with_no_builds`:
the unwrap receives `Transaction("another Armorer transaction is active")`.
The bootstrap suite reports 14 passes and one failure. This file and its
transaction implementation are unchanged by the resolved-feature commit.
That ancestry establishes scope, not a root cause or permission to disregard
the failure. Linux x64 and macOS complete successfully; the overall run fails.
ARM's later native/schema/runtime steps are skipped.

Keep the failed run. Diagnose the transaction contention and earn a new exact-head
receipt before reporting a fully passing candidate. Do not weaken the lock or
relabel the preceding candidate's green run as this head's result.

The [candidate verification guide](../docs/candidate-verification.md) describes
native v3 and the older-context migration boundary. Complete own signed
producer/consumer qualification, protected Apple production, publication/retry
adapters, both pilots and human acceptance remain open. This review changes no
source PR, hosted review comment, credential, setting, tag or pilot checkout.

Keep the failed gate visible. **This is the way.**
