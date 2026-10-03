# The Armorer's field guide

Know your task. Take the guide that serves it. Keep the evidence close.

**This is the way.**

## Learn the craft

Read [onboarding](onboarding.md) for your first inspection, then
[the CLI reference](cli-reference.md) for exact commands and status codes.
Use [the profile walkthrough](profiles.md) for library, CLI and service selections.
Check [compatibility](compatibility.md) before changing versions and
[troubleshooting](troubleshooting.md) when inspection or a transaction stops.
Use [apply and recovery](apply.md) when you are ready to approve a local change.
Read [the bootstrap catalog](bootstrap-catalog.md) to inspect reviewed pins and
[the offline verifier](offline-verifier.md) to understand individual evidence proofs.
These guides describe the implementation on `main`; the
[development map](development-status.md) marks work still under PR review.

For the combined PR #21 candidate, inspect [bootstrap and recovery](candidate-bootstrap.md),
[upgrades and reversal](candidate-upgrades.md), and
[complete/historical verification](candidate-verification.md). Their source
headers and open review gates distinguish candidate commands from main.
For the separate workflow stack, inspect the
[candidate prerequisite and unsigned intake guide](candidate-workflow-prerequisites.md).
Follow [candidate producer evidence](candidate-producer-evidence.md) for complete
same-attempt build/policy collection, advisory freshness and private proof limits.
Read [producer startup](candidate-producer-startup.md) for the isolated Python
boundary, retired Node-first APIs and private-proof versus audit-JSON distinction.
Then inspect [final payload assembly](candidate-final-payloads.md) for measured
transformations, fixed subject slots and detached inventory authentication.
Its native read-token interface and limits on the complete handoff remain separate
from protected signing or publication authority.

## Know the boundaries

| Document | What it establishes |
| --- | --- |
| [Configuration, lock and plan contracts](contracts-v1.md) | Local intent, pin syntax, preview identity and ownership |
| [Release trust contracts](trust-contracts-v1.md) | Versioned inventory, policy, evidence and publication interfaces; runtime limitations |
| [Trust examples](../examples/trust-v1/README.md) | Synthetic examples for contract validation |
| [Architecture](../ARCHITECTURE.md) | Accepted design direction and proposed future interfaces |
| [Implementation plan](../IMPLEMENTATION_PLAN.md) | Delivery slices, operational gates and pilot work |
| [Documentation delivery plan](documentation-plan.md) | Current guides, remaining operational documentation and their implementation gates |
| [Release readiness](release-readiness.md) | Required evidence and the operator's path through incomplete release gates |
| [Platform prerequisites](platform-prerequisites.md) | Source-checked GitHub eligibility, manual settings and observation limits |
| [Evidence maintenance](evidence-maintenance.md) | Online root preparation, offline authority, expiry/freshness and outage decisions |
| [Release recovery](release-recovery.md), [incident template](release-incident-template.md) | Draft/conflict/alias and new-version decisions; receipt checks versus pending backend execution |
| [Credential boundaries](credential-boundaries.md) | Current CI/build and candidate read-token interfaces, planned protected approvals and rotation decisions |
| [Candidate workflow prerequisites](candidate-workflow-prerequisites.md) | Native configuration states, ephemeral read-token isolation and complete unsigned Apple intake; no signing/publication grant |
| [Candidate producer evidence](candidate-producer-evidence.md) | Complete build/policy handoffs, final freshness checks and transport versus original private identity proofs |
| [Candidate producer startup](candidate-producer-startup.md) | Byte-qualified fresh Node, exact child credentials, retired APIs and non-authorizing audit output |
| [Candidate final payloads](candidate-final-payloads.md) | Fixed Linux/library assembly, exact retained evidence and acyclic inventory order; layout completion leaves authority false |
| [Dogfooding](dogfooding.md), [own intent example](../examples/armorer-dogfood/README.md) | Read-only inspection of Armorer itself and remaining complete rehearsal gates |
| [Documentation coverage](documentation-coverage.md) | Every planned documentation deliverable, present evidence and outstanding acceptance |
| [Research](../RESEARCH.md) | Dated primary-source research |
| [Supply-chain assessment](supply-chain-assessment.md) | Optional defenses and publishing-adapter boundaries |

The architecture includes planned commands. The CLI reference is the guide to
commands available on main. Research dates and validation receipts describe the
evidence recorded at that time; they are not live release-readiness reports.
Synthetic fixtures establish no real signature, Apple acceptance or publication.

## Keep the record

The [v0.1 acceptance ledger](v01-acceptance.md) preserves its dated tracker
snapshot; its pending labels are not current source or ticket-status claims.
Use [the source map](development-status.md) and [documentation coverage](documentation-coverage.md)
for the current guide boundary. The [support-policy draft](support-policy-draft.md)
prepares the missing maintenance/deprecation decision without promising a window.

Design decisions live in [the ADR directory](adr/). The retained
[ticket #2 validation record](trust-contract-validation.md) describes its original
delivery and remediation checks. Current open-PR review evidence is recorded in
[the original review packet](../reviews/2026-10-02-open-prs.md),
[continuation](../reviews/2026-10-02-pr-review-continuation.md) and
[combined-source review](../reviews/2026-10-02-integration-pr21.md) and
[successor review](../reviews/2026-10-02-integration-pr21-successor.md), followed
by the [context-ordering continuation](../reviews/2026-10-02-pr21-context-ordering.md)
and [resolved-feature review](../reviews/2026-10-02-pr21-resolved-features.md), then
the [evidence and lock continuation](../reviews/2026-10-03-pr21-evidence-and-locks.md).
The [contributor repair record](../reviews/2026-10-03-pr23-schema-guide.md)
retains the failing literal command and #23's validated 24-schema correction;
the draft fix remains separate from #21's current source.
The separate [workflow prerequisite review](../reviews/2026-10-02-workflow-prerequisites.md)
and [integration continuation](../reviews/2026-10-03-workflow-integration.md)
retain the source and qualification boundary behind the operational preparation.
The [identity-interface continuation](../reviews/2026-10-03-identity-prerequisites.md)
checks implemented credential names and original-proof export lifetimes.
The [cleanup continuation](../reviews/2026-10-03-workflow-cleanup.md) retains the
latest process/schema regressions and failed Linux collection receipt.
The [transport continuation](../reviews/2026-10-03-workflow-transport-cleanup.md)
records the successor's terminal joined rehearsal, shared-reader fault and tested successor repair.

The [producer startup continuation](../reviews/2026-10-03-workflow-producer-startup.md)
retains the initial native-test skip and its wiring repair with renewed hosted
startup and combined-handoff qualification.

Contributors follow [the contributor guide](../CONTRIBUTING.md) and
[the writing creed](writing-guide.md). Update behavior guides when behavior
changes. Preserve old receipts and label new evidence with its exact source.

**This is the way.**
