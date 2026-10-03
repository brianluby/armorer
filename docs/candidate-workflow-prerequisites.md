# Inspect the handoff before opening the vault

Candidate source: `brianluby/armorer-workflows`
[`b2b9fbc9846d5aa4bf1dbdb8e8618bddad25e23c`](https://github.com/brianluby/armorer-workflows/tree/b2b9fbc9846d5aa4bf1dbdb8e8618bddad25e23c),
the reviewed successor in workflow integration PR #21, preserving the stack through #20.
The catalog still pins accepted workflow main `772ca83e386c883c88cc3b936d69f8cb3216c91e`.
This guide prepares review of internal candidate interfaces. Armorer's current
CLI and generated callers do not expose these operations.

Inspect every gate. Keep the keys with their owner. **This is the way.**

## Observe configuration without granting authority

`observe_capabilities` takes a qualified `CapabilityGhApi`, independently
reviewed `RepositoryIntent`, and zero to two `EnvironmentIntent` policies.
Repository intent names the exact repository, numeric database ID and default
branch. The adapter reads only fixed GitHub.com GET routes for that repository,
its immutable-release setting and existing `release-signing`/`release-publish`
environments and deployment rules. It surrounds snapshots with identity reads,
repeats the complete snapshot and rejects configuration changes.
The adapter fixes API version `2022-11-28`; changes to that transport contract
require their own source review and qualification.

The following states remain visible. Failed observations cannot enable a weaker
publication path:

| Observation | Candidate state | Next duty |
| --- | --- | --- |
| HTTP 200, immutable `enabled: true` | `configured` | Retain the setting observation; earn the remaining release gates |
| HTTP 200, immutable `enabled: false` | `disabled` | Owner reviews setup; the observer changes no setting |
| HTTP 401 | `denied` | Correct the separately reviewed read identity |
| HTTP 403 | `unknown`, `access-denied-or-throttled` | Resolve permission/rate-limit ambiguity; retain the blocking result |
| HTTP 404 | `unknown`, `not-visible-or-not-enabled` | Establish resource visibility and configuration independently |
| HTTP 429 or other provider error | `error` | Preserve the failed observation and investigate before a fresh bounded read |

GitHub can use 403 for rate limiting; response message text cannot select release
authority. See [GitHub's rate-limit contract](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api).
The immutable-setting endpoint requires Administration read access; ordinary
workflow read permissions do not establish that capability. See
[the endpoint contract](https://docs.github.com/en/rest/repos/repos#check-if-immutable-releases-are-enabled-for-a-repository).

An independently reviewed environment policy binds numeric/node identities,
one to six user reviewer IDs, the exact default branch, `v*` tag restrictions,
self-review prevention, wait timer and disabled administrator bypass. Team
reviewers and unsupported controls are rejected. Without independent policy,
a discovered environment stays `unknown`; its configuration hash is audit data.
Current-attempt approval and effective enforcement remain unsupported even when
configuration matches. An environment name does not unlock credentials.

Keep the observation. Earn the authorization separately. **This is the way.**

## Preserve the exact credential boundary

The candidate qualification workflow uses `contents: read` and `actions: read`.
It supplies its ephemeral `github.token` through `ARMORER_READ_TOKEN`; the fixed
native child receives only that explicit token as `GH_TOKEN`. This is a
qualification-process variable, not a named repository secret to provision.
No OIDC, Apple credential or publication grant belongs to this observer.

`CapabilityGhApi.operator_qualification` is a separate internal path using the
owner's existing native `gh` credential store without reading or exporting its
values. It does not qualify workflow-token isolation or define a production
administrative-read App/secret binding. Exact production provisioning and
rotation remain in [the credential guide's open boundaries](credential-boundaries.md#prepare-each-future-protected-boundary).

Both paths rehash the compiled native `gh` 2.102.0 identity before each request,
close stdin, restrict routes/origin, bound output and discard diagnostics.
Requests have a 60-second bound; a complete observation has a 120-second bound.
The helper terminates its private process group, including descendants of an
exited leader. Wall time earlier than the observation's start or an expired
observation rejects the result.

Every returned observation keeps `run_bound`,
`protected_environment_authenticated`, `signing_authorized`,
`publication_authorized`, `immutable_release_verified` and
`release_attestation_verified` false. A serialized record cannot recover missing
private proofs or authorize a future signing/publication job.

Give the reader access. Keep the writer's keys closed. **This is the way.**

## Freeze the complete unsigned Apple handoff

`prepare_apple_payloads` accepts controller-owned `PayloadExpectation` values
and the preceding collector's live inert directories. There is no public workflow
input, shell command, credential parameter or provenance override. Provider ZIP
bytes still need a separate join to fresh original artifact-writer and mapped
OIDC proofs, plus independently accepted policy/catalog/root authority.

Supply the entire independently expected v3 handoff set, including Linux payloads
and library archives. Every declared target sibling must exist and agree on
package/feature declarations. Tool hashes agree within a target; native targets
can have different tool bytes. Missing, extra, conflicting, expired or changed
members reject the whole intake. Do not substitute an Apple-only subset.

The intake permits at most 64 selections and 4 GiB of staged bytes, including the
temporary copy used by semantic verification. Its private directory is mode 0700
with readonly mode 0400 leaves. Audit access rehashes the whole set; payload access
rehashes all five leaves for the selected artifact. Build age defaults to one
hour; the explicit bound cannot exceed one day. Access also checks a twenty-minute
monotonic stage lifetime and wall time at or after stage start and build finish.
Normal exit rechecks
the complete set; normal and exceptional exits clean up the workspace. Retained
paths become invalid outside the context manager.

The upstream combined collector has a smaller current bound: 1–32 selections and
2–64 artifacts on one page. Its bound governs that composed path. The intake's
64-selection limit cannot expand the collector or justify subset verification.
See the [exact combined collector contract](https://github.com/brianluby/armorer-workflows/blob/b2b9fbc9846d5aa4bf1dbdb8e8618bddad25e23c/docs/combined-handoff-v1.md).

Apple CLI/service payloads must be thin little-endian ARM64 Mach-O executables
with a bounded executable entry, the fixed loader and an explicit macOS platform
command. Foreign platforms/architectures, universal images and unsupported
layouts fail. Unknown load commands receive generic bounds only; this is not a
complete loader or security assessment. An embedded ad-hoc signature range
establishes no Developer ID, team, hardened runtime, secure timestamp or
notarization result. Library source archives remain opaque.

The final assembler still rejects unsigned Apple executables. Intake success
reports `unsigned-apple-payloads-inspected`, retains exact bytes and keeps producer,
writer, catalog, protected-environment, cryptographic, signing and publication
authority false. The [exact intake contract](https://github.com/brianluby/armorer-workflows/blob/b2b9fbc9846d5aa4bf1dbdb8e8618bddad25e23c/docs/apple-payload-intake-v1.md)
defines the supported format and file-reader assumptions.

Carry the whole handoff. Accept no missing sibling. **This is the way.**

## Keep qualification and production evidence separate

At reviewed successor `b2b9fbc9846d`, all three native jobs pass in the
[prerequisite qualification run](https://github.com/brianluby/armorer-workflows/actions/runs/37108079026)
and [unsigned intake run](https://github.com/brianluby/armorer-workflows/actions/runs/37108079033).
The first uses an unprivileged workflow token; the second qualifies unsigned
fixtures and actual native payload shape without executing them. Linux intake
success does not establish an Apple native positive.

The preceding immutable integration export passes 19 capability tests, three tests of the actual
validator-build step and all 24 intake tests. A temporary CPython 3.14 environment
uses the candidate's hash-pinned development wheels; no global dependency or
consuming repository changes are needed. The build tests exercise candidate/
inherited Cargo overrides, scratch-ancestor rejection and fixed-control
substitution. The [integration review](../reviews/2026-10-03-workflow-integration.md)
preserves the preceding source's import failure and the new distinct results.
The [cleanup continuation](../reviews/2026-10-03-workflow-cleanup.md) checks the
seven-file successor patch and passes 48 affected local tests. The source/controller
and mapped-worker readers now attempt to terminate their owned process group even
when its leader has exited. This does not contain a child that escaped that group.
The policy-schema CI check also rejects an empty report set at its required depth;
nonempty schema validity alone cannot prove the complete expected selection set.
The `9d857a80f094` combined rehearsal fails Linux x64/ARM collection while macOS passes;
it is distinct from the preceding all-host success. Successor `3fac9a1cef3d` adds
the [OIDC startup environment guard](credential-boundaries.md#match-the-candidate-helpers-exact-read-interface);
its new hosted receipts are separate. The [transport continuation](../reviews/2026-10-03-workflow-transport-cleanup.md)
records the preceding `3fac9a1cef3d` joined success and its shared-reader fault,
then the two-file `b2b9fbc9846d` cleanup repair and separate hosted receipts. The
capability adapter uses its own `response` entry point and cleanup. Hosted qualification remains
separate from an Armorer own signed rehearsal.

Required next evidence includes accepted controller/context/catalog/root,
current-attempt protected approval, Developer ID transformation/notarization,
final-byte attestations, a complete genuine signed consumer positive, immutable
draft/publication adapters, pilots and human acceptance. These candidate helpers
grant no SLSA level and close no operational ticket. Follow
[the full coverage audit](documentation-coverage.md) and
[release readiness](release-readiness.md).

Inspect first. Sign only after every earned gate. **This is the way.**
