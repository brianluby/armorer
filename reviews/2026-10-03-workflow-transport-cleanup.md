# Check every reader before calling the hunt finished

Snapshot: 2026-10-03 UTC. Workflow candidate
`3fac9a1cef3d9a5c4ac6f5c79c99d9255f63200a` remains under review. Accepted workflow
main is `772ca83e386c883c88cc3b936d69f8cb3216c91e`; Armorer main and its combined
candidate retain the source identities in the [development map](../docs/development-status.md).

Keep the successful receipt. Keep the remaining fault. **This is the way.**

## P2 observed at `3fac9a1cef3d`: shared transport skips exited-leader cleanup

The fixed source/controller/mapped-worker repair does not cover
[`QualifiedGhApi._read`, lines 152–166](https://github.com/brianluby/armorer-workflows/blob/3fac9a1cef3d9a5c4ac6f5c79c99d9255f63200a/armorer_runtime/transport_v1.py#L152).
That shared reader still calls `killpg` only when `process.poll()` is null.
If the native leader exits while a descendant holds stdout/stderr open, the read
hits its deadline, skips group termination and returns a failure with the
descendant still running. A failed read is not proof that its credentialed process
group has been terminated. The combined and policy collectors require this exact
adapter; the [native combined qualification](https://github.com/brianluby/armorer-workflows/blob/3fac9a1cef3d9a5c4ac6f5c79c99d9255f63200a/tests/combined_handoff_cases_v1.py#L186)
constructs it with the explicit ephemeral read token.

The file is byte-identical to the independently inspected caller/runtime pin
`59a2e782150abff3f995682013d514e6865d8dfb`. The successor did not repair or re-pin
this method. Scope the cleanup claim to the specific repaired adapters.

An immutable export reuses the candidate's actual exited-leader regression
helper against `QualifiedGhApi`: its owned inert Python executable forks a
pipe-holding child and exits; the reader's 0.4-second deadline fires, then the
child writes its one-second delayed marker. The assertion fails as expected.
The test uses a synthetic token and bypasses native hash qualification solely
for its owned fixture. It does not prove that pinned genuine `gh` creates this
fault, that offered payloads execute, or that a provider can steal a credential.
The demonstrated consequence is incomplete subprocess cleanup under that native
fault. Test finally cleanup owns only the deliberately spawned process group.

A separate control through `CapabilityGhApi.response`, its actual supported
entry point, times out and prevents the delayed marker. An initial attempt to
reuse the `_read` fixture is rejected as an unsupported operation and is not
counted as that adapter's cleanup test. The earlier four source/controller/worker
controls and 48-test receipt remain in the [cleanup review](2026-10-03-workflow-cleanup.md).

Before claiming complete native transport cleanup, repair the shared reader's
leader-status guard and qualify its actual group termination on supported hosts.
Preserve exact route/credential/output bounds and explicit termination-denied
handling. Add the exited-leader regression for the base adapter and rerun affected
build/policy/combined transport controls. This documentation review changes no
source adapter and posts no hosted review comment.

## Repair at `b2b9fbc9846d`

Successor `b2b9fbc9846d5aa4bf1dbdb8e8618bddad25e23c` changes only the base
transport reader and its cleanup test file. The complete two-file patch removes
the live-leader guard, preserving fixed GET/credential/output/time checks, and
adds `test_artifact_reader_reaps_exited_leader_descendant`. The remaining cleanup
sequence and macOS termination-denied handling match the earlier repaired readers.
It does not contain escaped process groups or independently prove child death
when group signaling is denied. Workflows, action wrappers, schemas, pins, lock,
OIDC helper and collector/policy source bytes are unchanged.

The immutable successor export passes **32 tests**, with no skips: five native
cleanup controls, ten build-transport tests, nine policy-transport tests and eight
combined-handoff tests. The new base-adapter regression fails on the immutable
`3fac9a1cef3d` predecessor and passes on the successor. Tests use owned inert
children and synthetic provider/credential data; native hash qualification is
bypassed only for those owned subprocess fault fixtures. At this exact head,
all three jobs pass in [prerequisite run 37108079026](https://github.com/brianluby/armorer-workflows/actions/runs/37108079026)
and [unsigned-intake run 37108079033](https://github.com/brianluby/armorer-workflows/actions/runs/37108079033).
All 12 jobs pass in [runtime run 37108079030](https://github.com/brianluby/armorer-workflows/actions/runs/37108079030).
The distinct combined run `37108079402` remains live at this observation. No
terminal result is inherited from the predecessor. This reviewer changes no
source runtime; the finding and successor are both retained at exact identities.

## The preceding successor's joined rehearsal finishes

At exact head `3fac9a1cef3d`, [runtime run 37107472105](https://github.com/brianluby/armorer-workflows/actions/runs/37107472105)
passes all 12 test/build/policy/transport jobs. The same formerly queued
[combined run 37107472496](https://github.com/brianluby/armorer-workflows/actions/runs/37107472496)
finishes successfully: all 33 jobs pass, including all three native collection/
writer-join jobs after nine builds, nine policy jobs and nine prerequisite CI
jobs. Its fixed nine-selection/eighteen-archive fixture supplies a joined native
qualification receipt. No archive-by-archive independent byte audit is claimed.

The [preceding snapshot](2026-10-03-workflow-cleanup.md) keeps the earlier queue
observation and the failed `9d857a80f094` run. Later success does not erase that
failure, establish its cause or prove the unrelated cleanup fault absent.
The caller still uses its reviewed immutable producer/runtime pins; a new source
head is not a silent runtime/catalog upgrade.

Protected current-attempt approval, accepted catalog/context/root, live mapped
OIDC producer acceptance, Apple finalization, complete own signed consumer,
immutable publication/recovery, pilots and human acceptance remain open. Private
vulnerability reporting is still disabled and no releases are listed at this
observation. Documentation draft #22 has an automated “draft not reviewed” notice,
not source-review approval. No setting, tag, release, credential, pilot or hosted
reviewer request was changed.

Earn every boundary. Do not let a green run hide a missing guard.
**This is the way.**


## Terminal receipt after the live observation

The same `b2b9fbc9846d` combined run
[37108079402](https://github.com/brianluby/armorer-workflows/actions/runs/37108079402)
now finishes successfully. Exact-head API results show all 33 jobs passing,
including collection/writer joins on Linux x64, Linux ARM and macOS. This closes
that candidate's pending native fixture receipt without erasing the live snapshot
or qualifying a complete own signed release. The caller/runtime/fixture identities
retain their existing scope; no independent archive-by-archive audit is inferred.

Keep the new verdict with its own source. **This is the way.**
