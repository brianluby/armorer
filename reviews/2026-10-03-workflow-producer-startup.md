# Close startup before the credential arrives

Snapshot: 2026-10-03 America/Los_Angeles / UTC. Workflow PR #21 advances from
`b2b9fbc9846d5aa4bf1dbdb8e8618bddad25e23c` to
`9980f975d4cf222a3a52d0eece718bbfb79ca126`, then
`4cdf5d786aebb637c7b41afe53c4d19a8be08ffa`.
Workflow main remains `772ca83e386c883c88cc3b936d69f8cb3216c91e`.
Armorer's combined candidate remains `56d7380a5534583b7973b5eedc55a1223b46b60e`.

Seal the startup path. Keep each receipt with its source. **This is the way.**

## Source boundary and retired interfaces

The first successor changes 13 files. It adds an isolated Python launcher,
fixed composite action and fresh Node entry; moves issuer/mapped logic into
internal worker modules; retires the two original credentialed Node APIs; and
adds native startup tests and guidance. The issuer worker retains its prior
implementation bytes apart from a header. The mapped worker changes its internal
issuer import and adds SIGTERM cancellation forwarding/removal around the native
observer. The separate artifact-writer interface is unchanged.

The launcher approves the exact bounded request digest/envelope, checks and
privately copies one compiled-pin native Node executable, then constructs the
child environment. The internal worker validates full intent before provider
reads. Node 24.21.0 delivery pins are new proposals inside the launcher, not an
accepted production catalog. Schema bytes, existing `pins` files, `armorer.lock`
and reusable build/policy workflows remain unchanged from `b2b9fbc9846d`.
The returned audit explicitly keeps production Node catalog, signing and
publication authority false. It cannot transfer a private original proof to a
later process. The [startup guide](../docs/candidate-producer-startup.md) records
these interfaces and their limits.

Local public preparation compares all three compiled archive digests with the
[official fixed checksum manifest](https://nodejs.org/dist/v24.21.0/SHASUMS256.txt).
The extracted macOS ARM64 Node has size 122129232 and SHA-256
`e4b5a3af0e05c75de2eae013904145f40fe7fc2a6e6f17510128bf45cca4e79b`.
This validates the compiled delivery pin and native fixture; it does not
establish upstream release-signature authentication.

## Direct startup controls and initial hosted gap

All seven startup controls pass locally with the exact pinned macOS Node:
preload exclusion with a synthetic RSA positive, exact child environment,
wrong approval/substituted bytes before credential access, unsupported envelope,
retired exports, fixed composite interpreter configuration and exited-leader
pipe-holder cleanup. The direct Node negative control executes its preload and
reads only a deliberately fake credential; the isolated launch does not execute
that hook. The test interposes a fixed synthetic issuer/entry harness; it does
not request real platform credentials or prove live protected OIDC.

The new internal issuer/mapped suites also pass **25 tests** on that Node with
no skips. The local Python launcher runs in isolated Python 3.14. Runtime bytes
are identical between `9980f975d4cf` and the later wiring-only successor.

**[P2 at `9980f975d4cf`] Supply the startup fixture in the build-evidence job.**
`test_build*.py` discovers all seven new cases, but its build step sets neither
`ARMORER_TEST_NODE` nor `ARMORER_TEST_NETWORK`. The latter variable appears only
in a different job's boundary-test step. Reproducing that build environment
finds seven cases but executes zero: `setUpClass` skips the entire class.

[Runtime run 37111137859](https://github.com/brianluby/armorer-workflows/actions/runs/37111137859)
passes all 12 jobs at `9980f975d4cf`. Its three native build logs each explicitly
report that class skip, **201 tests**, and `OK (skipped=1)`. A green run therefore
does not qualify the new startup controls. The OIDC/mapped fixture actions also
retain their older source pins at that head.
Its distinct [combined run 37111138177](https://github.com/brianluby/armorer-workflows/actions/runs/37111138177)
is cancelled, with 31 materialized jobs and a cancelled collection job. The old
33-job success belongs to `b2b9fbc9846d` and cannot qualify this attempt.

## Wiring successor and renewed qualification

`4cdf5d786aeb` changes only `.github/workflows/development.yml`: it enables the
public native fixture preparation in the actual build-evidence test step and
pins the two issuer/mapped fixture actions to `9980f975d4cf`. Runtime, schemas,
existing tool pins and reusable callers are byte-identical to the first successor.
That resolves the discovery gap without changing the tests or their assertions.

[Runtime run 37111479943](https://github.com/brianluby/armorer-workflows/actions/runs/37111479943)
passes all **12 jobs** at the exact wiring successor. Each Linux x64, Linux ARM64
and macOS ARM64 build log names all seven startup controls and reports **208 tests,
OK**, with no class skip. This is hosted native startup-fixture qualification,
not a live privileged action/producer or accepted production Node catalog.
The separate [prerequisite run 37111479935](https://github.com/brianluby/armorer-workflows/actions/runs/37111479935)
and [unsigned intake run 37111479967](https://github.com/brianluby/armorer-workflows/actions/runs/37111479967)
complete successfully at that head.

At this observation, [combined run 37111480272](https://github.com/brianluby/armorer-workflows/actions/runs/37111480272)
has materialized 33 jobs and all three native collection/writer-join jobs are
running. Its terminal result is not established by the runtime receipt above.
Keep that live observation until a later exact-head readback supplies the result.

Trusted job startup before any earlier executable, effective approval, accepted
roots/catalog, live protected OIDC, complete own signed producer/consumer,
Apple production and publication/recovery remain required. No new actionable
source defect was confirmed after the wiring repair; this source review changes
no implementation branch, provider setting, credential or pilot checkout.

Earn the next gate. **This is the way.**

Later same-day readback confirms combined run `37111480272` completed successfully
at `4cdf5d786aebb637c7b41afe53c4d19a8be08ffa`: all 33 jobs and all three native
collection/writer joins pass for the fixed nine-selection/eighteen-archive fixture.
The earlier running observation and cancelled predecessor remain visible. This
joined unsigned fixture establishes no complete own signed-release acceptance.
