# Keep the stack. Qualify the joined source.

Snapshot: 2026-10-03 UTC. Workflow
[PR #21](https://github.com/brianluby/armorer-workflows/pull/21) at
`e88e25fc46c1bb5298579c5cd49f52d43215b9e1` targets accepted main
`772ca83e386c883c88cc3b936d69f8cb3216c91e`. It contains the earlier stack through
`853437cb1c9cbb9a60b2186ebb637ce56d3f6b51`. Armorer main and its embedded workflow
catalog still use their earlier accepted source/pin.

Inspect the source that joined. Keep each receipt separate. **This is the way.**

## Compared source and scope

The exact comparison to `853437cb1c9c` changes eight files: two workflow files
update three unprivileged fixture-validator checkout references to Armorer
`56d7380a5534583b7973b5eedc55a1223b46b60e`; six Python test files add seven
class docstrings. Runtime, schemas, pins and actions are byte-identical in the
immutable Git comparison. No protected signing or publication implementation is
added. The validator remains a review candidate, not accepted production authority.

Reviewed the complete successor patch and the unchanged final-payload assembler,
combined collector and policy reader used by the new guides. This is a scoped
source/documentation continuation. It does not claim an independent exhaustive
audit of every inherited producer/OIDC/writer module or verify every hosted
archive's bytes. The [preceding review](2026-10-02-workflow-prerequisites.md)
retains the earlier source and its local dependency limitation.

Final assembly keeps unsigned Apple executables blocked before staging. Linux
packaging checks a fixed ELF header and measured archive bytes; library copying
never extracts `.crate` contents. The fixed complete asset set, exact subject/
SBOM binding, detached inventory bundle and transition rehashes are preserved.
Structural bundle acceptance deliberately ends at `layout-complete-unverified`;
no signature verifier or authorization adapter is manufactured by assembly.

## Fresh local evidence

A new temporary CPython 3.14 environment installed only binary development wheels
using this candidate's exact `requirements-schema.txt` and `--require-hashes`.
The workstation's global/bundled Python and consuming repositories were unchanged.
The immutable candidate export then passes 85 tests, with one separately skipped
native policy control:

| Suite | Passed | Skipped | Scope |
| --- | ---: | ---: | --- |
| Final payload assembly | 19 | 0 | Complete sets, measured byte chains, fixed bundles, inventory order, mutation/limits and cleanup; synthetic signatures |
| Apple unsigned intake | 24 | 0 | Complete sibling sets, inert native format/byte checks, stage/access/exit integrity and limits |
| Combined handoff | 8 | 0 | Same-attempt complete build/policy collection, source/byte/freshness substitutions and inert files |
| Independent policy | 12 | 1 | Report/source/advisory/expiry boundaries; real scanners/public-feed test requires separately qualified tools/network |
| Native capability observations | 19 | 0 | Closed states, exact intent, bounded real inert child-process isolation |
| Actual validator-build workflow step | 3 | 0 | Candidate/inherited Cargo traps, scratch rejection and fixed control pins |

The earlier 24-test intake import failure is preserved in its dated packet; it
does not describe this fresh environment. These ordinary tests do not authenticate
producer timestamps or prove live private OIDC/writer acceptance.

## Hosted evidence at this exact head

| Run | Verified terminal result |
| --- | --- |
| [37105406885: prerequisite qualification](https://github.com/brianluby/armorer-workflows/actions/runs/37105406885) | Linux x64, Linux ARM and macOS all pass |
| [37105406924: unsigned Apple intake](https://github.com/brianluby/armorer-workflows/actions/runs/37105406924) | Linux x64, Linux ARM and macOS all pass |
| [37105406884: workflow runtime validation](https://github.com/brianluby/armorer-workflows/actions/runs/37105406884) | All 12 test/build/policy/transport jobs pass across the three hosts |
| [37105407257: combined caller rehearsal](https://github.com/brianluby/armorer-workflows/actions/runs/37105407257) | Overall success; all three native collection/writer-join jobs pass after the complete build/policy matrix |

The combined run was initially observed live, then its same handle completed.
Its three collection jobs qualify the fixed nine-selection/eighteen-archive
fixture and writer join on the native hosts. Source/run/catalog fixture identities
retain their declared scope. This does not provide live production mapped-OIDC,
protected Apple finalization or a complete genuine own signed release. Other
caller rehearsals have independent run identities; the four passing runs above
do not qualify them by inheritance.

Effective protected approval, accepted production context/catalog/root,
credentialed Apple finalization, final-byte attestations, a complete genuine own
signed consumer positive, immutable publication/recovery, pilots and human
acceptance remain open. No source repository, tag, provider setting, credential,
pilot or hosted review comment was changed by this documentation pass.

Keep the full acceptance gate. **This is the way.**
