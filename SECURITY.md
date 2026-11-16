# Security policy

Bring evidence of a broken boundary. Protect the people who depend on it.
**This is the way.**

## Report a vulnerability privately

Use GitHub private vulnerability reporting:
<https://github.com/brianluby/armorer/security/advisories/new>

If that form is unavailable, open a
[GitHub issue](https://github.com/brianluby/armorer/issues) that asks for a
private channel and contains no vulnerability details. Do not post exploit
steps, unpatched findings or private repository information in public issues,
pull requests or discussions.

Use ordinary issues for non-sensitive defects and documentation problems.

## What to include

- The affected component and the exact commit or version.
- Host platform and Rust toolchain.
- The trust boundary you expected to hold, what happened instead, and the impact.
- A minimal reproducer that uses inert fixtures and contains no credentials.
- For verification findings: the subject bytes, the source, signer and predicate
  expectations, and the policy and trusted-root identities involved.
- For local mutations: the approved plan, the file preimages and the
  conflicting state.

Keep credential values and signing keys with their owner. Redact sensitive data
and test only against systems you are authorized to test.

## In scope

- Read-only discovery executing repository code or writing consumer files.
- Path or symlink escape from the repository root.
- Replacement of customized or unowned files.
- Bypass of plan approval, preimage checks or the recovery journal.
- Caller-supplied commands, URLs or provenance reaching a trusted step.
- Weakened source, signer, trusted-root or subject expectations.
- Incomplete evidence accepted as a complete proof.

A documented unimplemented capability, or an unsupported input that Armorer
rejects, is not by itself a vulnerability. Report findings in the reusable
workflows to
[armorer-workflows](https://github.com/brianluby/armorer-workflows) with their
pinned source identities.

## Supported versions and response

Armorer is experimental and has no released version. Fixes land on `main`. No
supported-version window, response time or bounty is promised yet; see
[compatibility](docs/compatibility.md) and the
[development map](docs/development-status.md) for current limits.

An accepted report still needs reproduced evidence, a reviewed fix and
independent validation before disclosure is coordinated.
