# Ticket #8 evidence audit

Date: 2026-11-04 UTC. Scope: Armorer project 16 ticket **#8** (database ID
**1311**) on branch `evidence/acceptance-8`, rebased onto acceptance receipt A
commit `dc566c1411bd6adeb212c890dbdfc086c6673f68`. This is an evidence-only
source audit: it changes no production code, catalog, pin, schema, trust module,
root, fixture, tracker state, release, tag, or pilot repository. It is not a
signed rehearsal or a claim that ticket #8 is complete.

The ticket was read from the configured Veans root with `rtk veans show #8`.
Ticket **#26** was also read there; it is project index 26, database ID 1480.
No Veans write was made. The acceptance ledger at `docs/v01-acceptance.md`
remains the current source for C1--C4 and their remaining gates.

## Boundary and evidence classification

The implementation has three materially different evidence kinds. They must not
be conflated.

| Kind | Precise material | What it establishes | What it does not establish |
| --- | --- | --- | --- |
| Genuine cryptographic integration | `tests/fixtures/sigstore/reusable-workflow-artifact` and its retained `reusable-workflow-attestation.sigstore.json`; provenance/fixture receipt `source-receipt.json`; ignored `tests/real_sigstore.rs::genuine_pinned_gh_verifies_and_every_offered_tamper_or_identity_error_fails` | When run with a hash-qualified native `gh`, real offline signature verification and identity/tamper rejection for the retained external 2024 fixture | An Armorer producer, production roots/catalog, private-backend positive, or complete release consumer positive |
| Synthetic/schema/semantic fixtures | `tests/historical.rs`; `tests/cargo_graph_v2.rs`; `tests/fixtures/cargo-graph-v2/*`; context fixtures made by `tests/release_context.rs::fixture`; CycloneDX documents in the latter fixture directory | Explicit historical byte-only routing, independent-context rejection, graph/SBOM semantic reconciliation, and schema validation boundaries | Signature cryptography, provider authentication, production approval, or an own signed release |
| Genuine-native negative boundary | ignored `tests/release_context.rs::real_inventory_authentication_failure_precedes_json_and_asset_processing`, which uses the genuine unrelated Sigstore bundle plus qualified `gh` and CycloneDX | A real verifier failure stops before malformed inventory JSON, path claims, other assets, or payload handling | A successful inventory signature or complete own producer/consumer positive |

The externally retained Sigstore material is genuine signature material, but the
current checkout contains no native verifier path, so this audit did not execute
it. `tests/fixtures/sigstore/verified-result.json` is retained native output used
for post-cryptographic consistency only; it is not a proof minted by a mock, nor
a current execution receipt. The graph fixture receipt intentionally says
`attested: false` and `synthetic fixture-only 17/2`; graph interoperability is
not authentication.

## C1 — SHA-pinned actions/attest, explicit final paths, SBOMs, inventory

**Implementation.** The consumer's fixed naming/layout is in
`src/verification/release.rs:183-271`: for every independently selected target it
requires `<selection>.cdx.json`, `<selection>.cargo-graph.json`, final
`<selection>.tar.gz` or `.crate`, corresponding SBOM predicate bundle, and the
fixed build/package (plus applicable Apple) evidence leaves. Before this,
`src/verification/release.rs:81-131` snapshots only the fixed
`armorer-release-inventory.json` and detached
`armorer-release-inventory.provenance.sigstore.json`, verifies the latter, then
requires the exact directory leaf set. Each listed asset is rehashed at
`src/verification/release.rs:136-148`; each declared attestation bundle is
verified at `149-181`. The expected signer/predicate/scope comes from independent
context through `context.attestation`, never from offered release content.

`src/verification/sigstore.rs:183-241` rejects an unapproved policy, non-compiled
verifier pin, changed executable, or changed root before invocation.
`src/verification/sigstore.rs:283-392` supplies fixed `gh attestation verify`
arguments including exact source/ref/digest, signer workflow/digest, predicate,
local bundle, custom root, and `--deny-self-hosted-runners`; it clears the
environment and does not use PATH. The exact current compiled `gh` identities are
in `qualified_native_verifier` at `src/verification/sigstore.rs:27-49`.

The producer contract records the required final-artifact/SBOM/graph/evidence
layout and inventory-last detached proof in
`docs/candidate-final-payloads.md:58-101`. It explicitly says that its local
assembler only checks shape and can reach synthetic
`layout-complete-unverified` (`103-114`); it is not a signature producer. The
accepted design records SHA-pinned `actions/attest` and pinned `gh` in
`docs/adr/0002-platform-attestations-and-level-claims.md:5-11`. This checkout's
own development workflow pins checkout/upload actions, but is not a final
attestation producer; no audit inference treats ordinary CI artifact upload as an
attestation.

**Tests/fixtures.** `tests/real_sigstore.rs:143-341` is the genuine external
signature integration and adversarial identity/tamper coverage. The independent
complete-consumer rejection boundary is
`tests/release_context.rs:391-491`. Synthetic context authority substitution and
no-build execution are in `tests/release_context.rs:130-389`; its reviews and
inputs are explicitly fixture-owned. The fixed inventory layout itself has
source tests in `src/verification/release.rs` consumers; complete own positive
material is absent.

## C2 — bytes, source/ref/commit, signer/repository/digest, hosted runner,
predicates, and published SBOM comparison

**Implementation.** `ExpectedAttestation` in `src/verification/sigstore.rs:72-110`
holds independently supplied source repository/commit/ref, caller workflow,
reusable signer workflow, exact run/attempt, trigger, predicate, scope, and
subject basename. Its validation calls the independent verification policy;
release/rehearsal trigger mismatch is rejected. Fixed native arguments are at
`322-364`. After native success, `validate_output` at `src/verification/sigstore.rs`
(validated from line 470 onward) requires issuer, signer URI/digest,
`github-hosted` runner, source repository/ref/commit, caller workflow/digest,
trigger, run invocation, backend visibility, one exact subject digest, and exact
predicate type. It also validates SLSA provenance source/workflow/event/builder
claims. A successful native exit alone is therefore insufficient.

The complete consumer rehashes every inventory-selected download
(`src/verification/release.rs:136-148`) and later rehashes private snapshots
(`282-307`). It validates each whole SBOM through the independently qualified
native validator (`189-197`), requires the signed SBOM predicate to equal the
published SBOM JSON and target the authenticated final bytes (`206-215`), and
reconciles the selected graph with that exact SBOM (`216-238`).
`src/verification/context.rs` authenticates the independently approved context
before it reads release data; `src/main.rs:515-576` wires those steps into
`verify-release` and emits success only after all required checks, with
`publication_authorized: false` and `slsa_build_level: null`.

**Tests/fixtures.** The genuine ignored Sigstore test mutates signature, signed
predicate, certificate, artifact, repository, commit, ref, signer repository/
workflow/digest, predicate, run, attempt, caller workflow, root and policy
(`tests/real_sigstore.rs:146-341`). The exact whole-document SBOM native gate is
`tests/real_cyclonedx.rs:50-126`; it validates genuine native tool behavior but
uses retained fixture SBOM documents, not signed release output. Semantic graph
and SBOM mismatch rejection is in `tests/cargo_graph_v2.rs:92-280`; those
fixtures are explicitly unauthenticated. The C2 implementation is present, but
an own complete signed producer/consumer positive has not been supplied.

## C3 — independent trust, exact historical allowlist, no fallback, offline
roots and freshness

**Implementation.** `OfflineVerifier::open` authenticates the independently
approved policy digest before parsing, validates reviews against the actual clock,
and snapshots exact policy-bound native verifier and root bytes
(`src/verification/sigstore.rs:183-241`). `verify_one` revalidates policy time and
all snapshots before/after native verification (`283-392`). Roots are bounded,
local, and parsed as a single object or bounded JSONL only
(`412-428`); no network root refresh exists in this path.

Historical comparison is a separate CLI operation only:
`src/main.rs:579-610` calls `HistoricalByteMatch::verify`, returns
`authenticity: "not-established"`, `provenance_verified: false`, and no SLSA
level. `src/verification/historical.rs:40-124` requires an independently approved
policy digest, release mode and stable tag, and an exact matching source entry in
the historical allowlist. It rehashes the complete fixed leaf set. It rejects
modern inventory/attestation names regardless of an attempted legacy allowlist
(`148-184`). `verify-release` has no call to `HistoricalByteMatch`; failed
inventory authentication propagates an error before inventory parsing at
`src/verification/release.rs:102-114`. Thus there is no authenticity-failure
fallback or automatic downgrade.

**Tests/fixtures.** Synthetic `tests/historical.rs` covers independent digest
precedence, wrong repository/commit/tag/rehearsal/modern collision rejection,
expired/future review rejection, byte/set mismatch, and forbidden modern evidence
(`94-263`), plus explicit CLI output semantics (`305-359`). These are synthetic
allowlist fixtures by design; their result is intentionally weaker. The genuine
native rejection test named above asserts a real unrelated bundle produces
`attestation-authentication-failed` before malformed inventory or fallback
processing (`tests/release_context.rs:435-490`).

**Offline policy/root bounds and clock enforcement.** The verification policy
input is limited to 1 MiB by `OfflineVerifier::open`
(`src/verification/sigstore.rs:195`); the root is limited to 4 MiB and one JSON
object or at most 32 JSONL records (`src/verification/io.rs:14-17` and
`src/verification/sigstore.rs:412-428`). The same adapter bounds bundles at 64
MiB, decoded signed payloads at 17 MiB, the native verifier at 128 MiB, each
subject at 1 GiB, and each native invocation at 30 seconds
(`src/verification/io.rs:14-17`, `src/verification/sigstore.rs:302-305`, and
`docs/offline-verifier.md:139-147`). These are input/process limits, not root
freshness durations.

There is deliberately no implicit root TTL: `Review::validate_at` requires
`reviewed_at > 0`, `reviewed_at <= now`, and `now < expires_at`
(`src/trust/mod.rs:211-224`). `VerificationPolicy::validate` applies that actual
clock check to both policy and root review (`src/trust/policy.rs:95-105`);
`wall_time` fails closed if the clock is unavailable
(`src/verification/sigstore.rs:396-402`). The complete consumer calls
`context.validate_at` at entry, before each bundle, and before success
(`src/verification/release.rs:74-76`, `149-156`, `282-284`), so review expiry
during verification cannot pass. Independent evidence requirements additionally
require a positive `max_age_seconds` (`src/verification/context.rs:431-439`),
but its value is independently selected rather than globally defaulted.

**Documentation alignment.** `docs/offline-verifier.md:139-160` states the same
64 MiB/17 MiB/4 MiB/128 MiB/1 GiB/30-second limits, 32-record root transport,
no silent root refresh, and no built-in offline-key expiry; it identifies policy
review expiry as the separate clock gate. `docs/evidence-maintenance.md:34-65`
requires a new approved policy identity when roots change and states the same
actual-clock relation (`review time <= actual time < expiry`) plus independently
selected age ceilings. Its outage guidance (`84-125`) says expiry/authentication
failure blocks rather than downgrades. `docs/historical-verification.md:1-76`
matches the explicit-only historical implementation and weaker output.
`docs/verify-release-cli.md:1-90` and `docs/authenticated-inventory.md:54-142`
accurately describe context-first, inventory-first, offline-root and fail-closed
behavior. Stable filesystem and no hostile same-account mutation are documented
implementation limits, not omitted trust checks.

## Related #10 capability observation boundary (audit only)

This is not #10 implementation work; it traces the existing blocking behavior
that remains a delivery blocker. `NativeGithub::capability_snapshot` observes
immutable-release and publish-environment routes twice
(`src/verification/publication/github.rs:251-351`). Any non-200 route goes through
`unavailable`; HTTP 403 and 404 map to `GateState::Unknown`, 401 to `Denied`, 410
to `Unsupported`, and other statuses to `Error`
(`src/verification/publication/github.rs:745-752`). Missing enforceable
environment reviewers also yields `Unknown` (`301-339`), and the unavailable
current-attempt-environment and runner-wide-serialization gates are explicitly
`Unsupported` (`344-350`).

A report can authorize no write merely by existing: `CapabilityReport::require_ready`
requires a nonfuture observation within the independently approved policy's
`max_observation_age`, every named required gate, and every reported gate to be
`Enforced` (`src/verification/publication/github.rs:12-62`). The publication
policy accepts only `1..=300` seconds for that observation-age window
(`src/verification/publication/mod.rs:94-118`), and controller preflight invokes
this check before every mutation (`487-504`). Therefore a 403/unknown capability
observation blocks with `publication-prerequisites-unproven`; it cannot become a
fallback or reporting-only success. Exact tests are
`src/verification/publication/github.rs:999-1015`, which asserts 403/404 become
`Unknown`, and `src/verification/publication/tests.rs:264-286`, which makes every
`Unknown`, missing, disabled, denied, unsupported, or error required gate fail
before any create. `tests/trust_contracts.rs:439-460` independently verifies that
unknown/unsupported/error required capability observations fail. The in-review
controller guide says the same at `docs/owned-draft-controller-v1.md:1-25`; its
required current-attempt and runner-serialization gates remain unsupported.

## C4 — download rehash, verified SBOM/JSON/graph, independent policy/root/catalog
and exact historical routing

**Implementation.** The rehash, verified predicate equality, graph comparison,
and authenticated reference checks are all in
`src/verification/release.rs:136-148`, `206-269`, and `282-307`. Context binding
of independently approved config/lock/catalog/policy, source, caller/signer,
run/attempt, selections, and evidence requirements begins in
`src/verification/context.rs:171-260`; catalog/schema shape does not confer
upstream authenticity. The complete verifier documentation says the same at
`docs/verify-release-cli.md:17-49`; `docs/complete-sbom-validation.md:20-45`
separates SBOM schema proof from authentication. `docs/cargo-graph-v2.md:3-11,
32-45` expressly says graph run identity/interoperability does not establish
GitHub platform authenticity.

**Tests/fixtures.** `tests/real_sigstore.rs` is the genuine signature adapter
gate; `tests/real_cyclonedx.rs` is a genuine pinned-validator gate over retained
SBOM fixture bytes; `tests/cargo_graph_v2.rs` is semantic interoperability only.
Its ignored `native_python_graphs_match_independent_rust_reader` explicitly
asserts `attested: false` and synthetic run `17/2`
(`tests/cargo_graph_v2.rs:283-334`). It must never be reported as authentication.
The release-context native negative test proves failure ordering, not a successful
complete set. No code gap was found in the C1--C4 consumer contract; the missing
item is the owner-controlled genuine positive evidence described below, not a
safe source fix for this audit.

## Ticket #26: four ignored qualification gates

Only genuine qualified environment material may run these tests. The audit found
none of the required environment inputs set, so all four remain **unrun**; no
credential values were requested, inspected, printed, or attached.

| Gate | Exact test/source | Result and missing genuine environment |
| --- | --- | --- |
| Pinned public release attestation | `src/verification/publication/github.rs::real_pinned_gh_release_attestation_matches_frozen_baseline` | **Unrun** — missing `ARMORER_TEST_GH`, qualified native `gh`, existing read-only authentication, and network |
| GET-only publication observation/write boundary | `src/verification/publication/github.rs::real_native_publication_observations_never_grant_write_authority` | **Unrun** — missing `ARMORER_TEST_GH`, pin-qualified `gh`, existing GET-only authentication, independently qualified inputs, and network |
| Native Python graph reader interoperability | `tests/cargo_graph_v2.rs::native_python_graphs_match_independent_rust_reader` | **Unrun** — missing `ARMORER_TEST_GRAPH_RECEIPTS` retained native fixture outputs; if run it remains interoperability, never authentication |
| Native runtime provider qualification | `tests/release_context.rs::real_native_runtime_distribution_matches_independent_provider_qualification` | **Unrun** — missing `ARMORER_TEST_RUNTIME_ARCHIVE`, `ARMORER_TEST_RUNTIME_SPEC`, `ARMORER_TEST_RUNTIME_SPEC_SHA256`, `ARMORER_TEST_RUNTIME_SOURCE_COMMIT`, and `ARMORER_TEST_RUNTIME_SOURCE_REF` for all three exact-head native CI artifacts and an independently approved spec |

The first two are wired as explicit hosted development workflow gates at
`.github/workflows/development.yml:52-69`. Their absence locally is not replaced
by a mock, fixture, generic `gh`, or a credential-bearing workaround. The exact
safe presence-only command used in this audit was (it tests set/nonempty values
and required file/directory types but prints no environment value):

```sh
rtk bash -lc '
if test -n "${ARMORER_TEST_GH+x}" && test -n "$ARMORER_TEST_GH" && test -f "$ARMORER_TEST_GH"; then echo "gate-1-2: environment-present"; else echo "gate-1-2: unrun; missing ARMORER_TEST_GH qualified native gh and existing read-only/GET-only authentication"; fi
if test -n "${ARMORER_TEST_GRAPH_RECEIPTS+x}" && test -n "$ARMORER_TEST_GRAPH_RECEIPTS" && test -d "$ARMORER_TEST_GRAPH_RECEIPTS"; then echo "gate-3: environment-present"; else echo "gate-3: unrun; missing ARMORER_TEST_GRAPH_RECEIPTS retained native fixture outputs"; fi
if test -n "${ARMORER_TEST_RUNTIME_ARCHIVE+x}" && test -n "$ARMORER_TEST_RUNTIME_ARCHIVE" && test -f "$ARMORER_TEST_RUNTIME_ARCHIVE" && test -n "${ARMORER_TEST_RUNTIME_SPEC+x}" && test -n "$ARMORER_TEST_RUNTIME_SPEC" && test -f "$ARMORER_TEST_RUNTIME_SPEC" && test -n "${ARMORER_TEST_RUNTIME_SPEC_SHA256+x}" && test -n "$ARMORER_TEST_RUNTIME_SPEC_SHA256" && test -n "${ARMORER_TEST_RUNTIME_SOURCE_COMMIT+x}" && test -n "$ARMORER_TEST_RUNTIME_SOURCE_COMMIT" && test -n "${ARMORER_TEST_RUNTIME_SOURCE_REF+x}" && test -n "$ARMORER_TEST_RUNTIME_SOURCE_REF"; then echo "gate-4: environment-present"; else echo "gate-4: unrun; missing one or more ARMORER_TEST_RUNTIME_ARCHIVE, ARMORER_TEST_RUNTIME_SPEC, ARMORER_TEST_RUNTIME_SPEC_SHA256, ARMORER_TEST_RUNTIME_SOURCE_COMMIT, ARMORER_TEST_RUNTIME_SOURCE_REF"; fi
'
```

Observed output was each corresponding `unrun; missing ...` line. No values were
expanded into the command output or retained in this receipt.

## Remaining blockers and honest delivery state

The absent own complete signed producer/consumer positive is blocked by: **#9**
protected Apple identity/signing/notarization and final-byte evidence; **#10**
effective current-attempt approval (and its required provider gates); and owner
approval of production roots/catalog and associated policy/context identities.
It also needs a qualified final producer and a genuine complete signed
non-publishing rehearsal. Existing external signature fixtures, unsigned
handoffs, synthetic state-machine/schema tests, retained graphs, and successful
interoperability cannot substitute for that proof. The requested delivery state is
**In Review**, not complete acceptance.

## Commands and results

Discovery/audit commands used only source, docs, test names, fixtures, and
read-only tracker reads. Environment availability was checked with the
presence-only command above and without displaying any environment values. The
required native qualification gates above were not run because their genuine
material was absent.

## Validation results

The following prescribed commands were rerun after this receipt update and before
its amend; all passed:

| Command | Result |
| --- | --- |
| `rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo fmt --check` | Passed; no output/differences. |
| `rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo clippy --locked --all-targets --all-features -- -D warnings` | Passed; no warnings. |
| `rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo test --locked` | Passed; ordinary suites passed, with only explicitly ignored native qualification tests unrun for their documented missing qualified material. |
| `rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo build --locked` | Passed. |

## Proposed single tracker comment (not posted)

`Request In Review delivery for #8 / DB1311: commit <COMMIT> adds reviews/2026-11-04-ticket-8-evidence-audit.md, an evidence-only C1-C4 map to precise consumer code, tests, and fixtures. It distinguishes the retained genuine external Sigstore fixture/native gates from synthetic historical/context/schema/graph fixtures, confirms inventory authentication failure blocks before parsing with no historical fallback, and records graph interoperability is not authentication. It traces #10's existing HTTP 403/404 -> Unknown capability mapping and require_ready preflight block (no #10 implementation change), plus policy/root bounds and clock enforcement: 1 MiB policy, 4 MiB/32-record roots, 64 MiB bundles, 17 MiB payload, 128 MiB verifier, 1 GiB subjects, 30-second native calls, and review time <= actual time < expiry. #26's four ignored gates were not run because genuine qualified environment material was absent: ARMORER_TEST_GH plus read-only/GET-only auth/network; ARMORER_TEST_GRAPH_RECEIPTS; and all ARMORER_TEST_RUNTIME_* archive/spec/identity inputs. The receipt includes the exact value-safe presence command; no credential values were read or printed. The own complete signed producer/consumer positive remains blocked on #9 Apple identity/signing, #10 current-attempt approval, and owner production roots/catalog approval. Required validations passed: rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo fmt --check — passed; rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo clippy --locked --all-targets --all-features -- -D warnings — passed; rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo test --locked — passed; rtk proxy env RUSTC_WRAPPER= rustup run 1.95.0 cargo build --locked — passed. Files: reviews/2026-11-04-ticket-8-evidence-audit.md. Blockers remain; no completion claim.`
