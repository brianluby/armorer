# The Armorer's field guide

Know your task. Take the guide that serves it. Keep the evidence close.

**This is the way.**

## Learn the craft

Read [onboarding](onboarding.md) for your first inspection, then
[the CLI reference](cli-reference.md) for exact commands and status codes.
Use [apply and recovery](apply.md) when you are ready to approve a local change.
These guides describe the implementation on `main`; the
[development map](development-status.md) marks work still under PR review.

## Know the boundaries

| Document | What it establishes |
| --- | --- |
| [Configuration, lock and plan contracts](contracts-v1.md) | Local intent, pin syntax, preview identity and ownership |
| [Release trust contracts](trust-contracts-v1.md) | Versioned inventory, policy, evidence and publication interfaces; runtime limitations |
| [Trust examples](../examples/trust-v1/README.md) | Synthetic examples for contract validation |
| [Architecture](../ARCHITECTURE.md) | Accepted design direction and proposed future interfaces |
| [Implementation plan](../IMPLEMENTATION_PLAN.md) | Delivery slices, operational gates and pilot work |
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
[the review packet](../reviews/2026-10-02-open-prs.md).

Contributors follow [the contributor guide](../CONTRIBUTING.md) and
[the writing creed](writing-guide.md). Update behavior guides when behavior
changes. Preserve old receipts and label new evidence with its exact source.

**This is the way.**
