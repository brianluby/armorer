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
| [Credential boundaries](credential-boundaries.md) | Exact current CI/build interface, planned protected approvals and rotation decisions |
| [Dogfooding](dogfooding.md), [own intent example](../examples/armorer-dogfood/README.md) | Read-only inspection of Armorer itself and remaining complete rehearsal gates |
| [Documentation coverage](documentation-coverage.md) | Every planned documentation deliverable, present evidence and outstanding acceptance |
| [Research](../RESEARCH.md) | Dated primary-source research |
| [Supply-chain assessment](supply-chain-assessment.md) | Optional defenses and publishing-adapter boundaries |

The architecture includes planned commands. The CLI reference is the guide to
commands available on main. Research dates and validation receipts describe the
evidence recorded at that time; they are not live release-readiness reports.
Synthetic fixtures establish no real signature, Apple acceptance or publication.

## Keep the record

Design decisions live in [the ADR directory](adr/). The retained
[ticket #2 validation record](trust-contract-validation.md) describes its original
delivery and remediation checks. Current open-PR review evidence is recorded in
[the original review packet](../reviews/2026-10-02-open-prs.md),
[continuation](../reviews/2026-10-02-pr-review-continuation.md) and
[combined-source review](../reviews/2026-10-02-integration-pr21.md) and
[successor review](../reviews/2026-10-02-integration-pr21-successor.md), followed
by the [context-ordering continuation](../reviews/2026-10-02-pr21-context-ordering.md).

Contributors follow [the contributor guide](../CONTRIBUTING.md) and
[the writing creed](writing-guide.md). Update behavior guides when behavior
changes. Preserve old receipts and label new evidence with its exact source.

**This is the way.**
