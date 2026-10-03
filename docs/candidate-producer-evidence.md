# Bring the whole cargo. Prove who carried it.

Candidate guide: workflow source
[`3fac9a1cef3d9a5c4ac6f5c79c99d9255f63200a`](https://github.com/brianluby/armorer-workflows/tree/3fac9a1cef3d9a5c4ac6f5c79c99d9255f63200a).
Armorer's embedded workflow pin remains `772ca83e386c883c88cc3b936d69f8cb3216c91e`.
These internal interfaces prepare a fixed controller. They are not main CLI
commands or an accepted protected release procedure.

Keep the source, attempt and complete selection together. **This is the way.**

## Carry build and policy evidence together

The candidate's no-input `rust-build-v3.yml` produces unsigned build handoffs.
The separate `rust-policy-v1.yml` produces independent policy observations.
Both reusable calls require `contents: read`, exact reviewed workflow commits and
matching lock/configuration identities. They accept no caller shell or custom
provenance inputs. The policy job never runs consuming builds or tests; it does
not replace the build or CI jobs.

| Handoff | Exact regular leaves | What the reader checks |
| --- | --- | --- |
| Build v3 | Selected `.bin` or `.crate`, `.cdx.json`, `.cargo-graph.json`, `inventory.json`, `handoff-v3.json` | Independently expected source/runtime/run/attempt, selection/root/version, input/tool identities, complete leaf set and observed build interval |
| Policy v1 | `policy-v1.json`, `source-inputs.json`, `actionlint.json`, `zizmor.json`, `gitleaks.json`, `cargo-deny.jsonl`, `advisory-db.json` | Independently expected complete source tree, runtime/catalog/policy/tool identities, exact reports, actual retained advisory bytes and freshness |

The controller supplies those expectations through its independent approval path.
Copying them out of downloaded JSON would let the claim choose its own authority.
The combined reader requires the build's entire source-file digest map to agree
with the independently expected policy source tree. Agreement on only config,
lock or the selected package is insufficient.

The build reader preserves its v2 graph/inventory semantics. The builder's own
whole-document SBOM check does not replace final independent consumer validation
with approved tools. Root-feature authority follows the consumer's selected
[context version](candidate-verification.md#approve-every-activated-root-feature).

Retain every leaf. Approve expectations elsewhere. **This is the way.**

## Collect one complete active attempt

`collect_producer_handoffs` takes a qualified native transport adapter, a
`CombinedRun`, complete `PayloadExpectation` selections and independent source,
runtime, catalog and project-policy records. It cross-binds shared identities
before provider reads. Build and policy must refer to the same repository,
source/head, caller workflow, run/attempt, runtime commit and maximum age.

The run must still be active. Latest-run and explicit-attempt observations must
agree, with both exact immutable reusable workflow references present. For each
selection, the reader requires its build and policy artifacts together. It reads
the complete listing, checks artifact details, verifies whole ZIP digest/size,
rejects unexpected members, and checks the downloaded inert reports.

The current single-page boundary is **1–32 selections / 2–64 artifacts** and
**4 GiB total archive plus expanded staging**. A larger complete set fails as
unsupported. Splitting it or dropping selections does not qualify the original
release. The separate [unsigned Apple intake](candidate-workflow-prerequisites.md)
supports up to 64 selections; that higher ceiling does not expand this collector.

Before yielding, the collector rereads latest and explicit attempts, the complete
listing and every artifact detail. It rehashes local leaves and repeats semantics,
freshness and exception checks. Paths live only within its private context:
directories are mode `0700`, leaves are mode `0400`, and cleanup removes staging.
Keep consuming code inside that lifetime. Audit receipts can be retained; their
paths and observations do not become reusable authorization objects.

Never run payloads or extract inner `.crate` contents to inspect a handoff.
Stable filesystem ancestry and the trusted local account remain assumptions.

One attempt. Every artifact. **This is the way.**

## Watch policy time and advisory outages

The policy job uses fixed native adapters for actionlint, zizmor, gitleaks and
cargo-deny. Native findings must satisfy each fixed success parser; secret
matches are redacted and failed reports are not uploaded as passing evidence.
The effective dependency policy binds the explicit member, target and feature
arguments. Its workspace scope includes development/build dependencies; it does
not claim to be the minimal selected SBOM graph.

Cargo-deny must successfully fetch into a fresh private database before its
offline checks. The retained advisory report carries actual file bytes, hashes,
sizes, Git observations and successful fetch-completion time. Those Git fields
do not prove an upstream signature. Tool archive/member pins likewise do not
establish upstream signature verification or accepted production catalogs.

Policy age is checked from observation start, with a one-hour ceiling, before
and after reading. Advisory exceptions are independently supplied, must remain
valid on the current UTC date and cannot extend more than 90 days from that date
at the initial check. The final check catches expiry during verification,
including a midnight crossing. Build age also starts at build-operation start;
its independently chosen positive ceiling is at most one day. The combined
reader enforces both readers and its shared ceiling.

An outage, expiry, changed source, missing report or changed provider snapshot
stops the gate. Preserve the failure and rerun the entire applicable producer
job in a fresh attempt. Authenticate that new identity before use. Do not refresh
JSON timestamps, reuse an old attempt's permission or manufacture a smaller set.
Follow [evidence maintenance](evidence-maintenance.md) for root/policy renewal and
[release recovery](release-recovery.md) for independently owned draft decisions.

The clock does not bend for the mission. **This is the way.**

## Keep identity proofs separate from receipts

Transport success reports `combined-build-policy-transport-observed` with
`archive_bytes_verified: true`. Producer-job authentication, artifact-producer
authentication, accepted production catalog, cryptographic release authentication,
signing authorization and publication authorization remain false.

Production integration must join the fresh original private mapped-OIDC and
artifact-writer proofs to these exact archives. Mapped source/job/issuer identity
and writer identity establish different prerequisites. Serialized audit JSON
cannot recreate either private proof. Review exact decimal writer size strings
against measured archive integers without lossy numeric conversion.

PR collection requires explicit `qualification_only=True`. A PR fixture may
exercise transport, but cannot enter the production producer join or final
stable-tag release path. Its synthetic run IDs and catalog/context inputs retain
their fixture meaning even when it downloads real provider archives.

The candidate final assembler still rejects unsigned Apple executables in a
complete release selection. Qualified Linux/library transforms cannot earn Apple
parity by omitting those executables. Protected Developer ID finalization,
notarization, final-byte attestation and strict complete consumer verification
remain required, alongside immutable publication, recovery and both pilots.

The preceding `e88e25fc46c1` [integration review](../reviews/2026-10-03-workflow-integration.md) records
eight passing local combined-collector tests and 12 policy boundary tests, with
one real scanner/public-feed control skipped locally. It retains exact hosted
test/build/policy/transport results and the separate combined caller rehearsal's
three passing native collection/writer-join jobs. Their nine-selection fixture
does not supply production mapped-OIDC or own signed release acceptance.
The [preceding review](../reviews/2026-10-02-workflow-prerequisites.md)
keeps its historical source and receipts. This guide adds no live private
producer-OIDC positive, complete signed rehearsal or human acceptance.
The [final payload guide](candidate-final-payloads.md) follows the fixed
transformation/inventory order and its unverified result. Read the exact candidate
[combined contract](https://github.com/brianluby/armorer-workflows/blob/3fac9a1cef3d9a5c4ac6f5c79c99d9255f63200a/docs/combined-handoff-v1.md)
and [policy contract](https://github.com/brianluby/armorer-workflows/blob/3fac9a1cef3d9a5c4ac6f5c79c99d9255f63200a/docs/independent-policy-v1.md)
before qualifying an integration.

Earn each proof. Hold the remaining gates. **This is the way.**

## Use original identity proofs within their lifetimes

The fixed controller must derive source, caller/signer pins, run/attempt, exact
job display names and native runner from independent intent. Map the active job
through source/API observations first. Job ID and check-run ID are separate;
the signed OIDC claim must match the independently mapped check-run identity.
The mapped helper observes again after issuer verification and rejects source,
job or prerequisite changes. An artifact-writer observation separately binds
each completed uploader's native check to its artifact-service backend ID.

| Interface | Required original object and export lifetime | What remains unproven |
| --- | --- | --- |
| `producerContextRecord` | Module-created private issuer proof; before JWT expiry, no clock rollback, at most 300 seconds since observation | Effective protection/approval, artifact producer, accepted catalog and release authority |
| `mappedProducerContextRecord` | Original joined private proof; native observation at most 30 seconds old, plus a still-live issuer proof | Effective protection/approval, artifact producer, accepted catalog and release authority |
| `artifactWriterRecord` | Original process-local observation handle; at most 30,000 milliseconds old and no clock rollback | Producer OIDC, accepted catalog, payload download/verification and signing/publication authority |

OIDC acceptance also bounds token issue age to five minutes and lifetime to ten
minutes, with no clock-skew grace. The export's 300-second window never extends
JWT expiry. The mapped export's 30-second native window never extends the issuer
proof. Seconds and milliseconds are explicit above; do not compare mixed units.

Copied/deserialized records, reconstructed handles and private-object JSON are
not credentials or continuing permits. Do not edit audit times to retry expiry.
Obtain fresh observations through the fixed helper with independent intent and
recheck the complete same-attempt byte set. A new run/attempt must earn its own
source/job/writer binding and approvals; an old draft's owner identity is separate.

The [credential guide](credential-boundaries.md#match-the-candidate-helpers-exact-read-interface)
names the implemented read/runtime/OIDC interfaces without collecting values.
The [identity review](../reviews/2026-10-03-identity-prerequisites.md) records
synthetic issuer/protocol tests. Those fixtures and native PR reader qualification
do not supply an authorized live protected OIDC producer or current-attempt
environment approval.

Keep the original proof. Watch every clock. **This is the way.**


## Retain the failed collection attempt

At the reviewed successor `9d857a80f094`, the
[combined rehearsal](https://github.com/brianluby/armorer-workflows/actions/runs/37106718935)
finishes with failure. All nine build jobs and their policy/CI siblings pass,
but Linux x64 and ARM fail at the collection/writer-binding qualification step;
macOS collection passes. No all-host joined handoff is established by that run.
The preceding `e88e25fc46c1` success keeps its own source/run identity.

Keep the failed attempt and inspect the fixed phase/code or retained receipt.
Do not widen source/job/byte/freshness expectations or remove a target to force
collection through. A new attempt needs a complete new binding; successful build
or policy siblings cannot substitute for failed collection. The
[cleanup review](../reviews/2026-10-03-workflow-cleanup.md) records the verified
step outcomes without attributing an unproven root cause.

Keep the failure visible. Earn the whole handoff again. **This is the way.**
