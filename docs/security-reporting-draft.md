# Keep the clan's defenses intact

Bring evidence of a broken boundary. Protect the people who depend on it.
**This is the way.**

**Draft for maintainer review.** Private reporting for `brianluby/armorer` returned
`enabled: false` on 2026-10-02 America/Los_Angeles. A working private reporting
channel must be selected before this becomes the repository's published security
policy. No GitHub setting was changed by this documentation work.

## Report through the chosen private channel

Channel selection is pending: GitHub private vulnerability reporting after owner
enablement, or an explicitly designated security email address. This draft does
not invent a contact or promise an unavailable form.

Use ordinary [GitHub issues](https://github.com/brianluby/armorer/issues) for
non-sensitive defects and documentation problems. Keep vulnerability details,
unpatched exploit instructions and private affected-repository information out
of public issue/PR discussions until coordinated disclosure is agreed.

## Bring a useful record

Provide the affected repository/component and exact commit or version; relevant
host/toolchain; expected trust boundary; observed failure and impact; and a minimal
credential-free reproducer. For verification findings, identify subject bytes,
source/signer/predicate expectations and the policy/root identities that matter.
For local mutations, identify the approved plan, preimages and conflicting state.
Share only the necessary evidence through the private channel.

Keep credential values and signing keys with their owner. Redact sensitive data
from the report and describe how a maintainer can reproduce the issue using inert
fixtures. Retain original evidence privately. Testing must preserve unrelated
projects and respect the affected system owner's authorization.

Bind the finding to a concrete boundary and consequence.
**This is the way.**

## Scope and evidence

Relevant boundaries include read-only discovery unexpectedly executing source or
writing consumer files; path/symlink escape; replacement of customized/owned files;
approval, preimage or journal bypass; caller-supplied shell/provenance; weakened
independent source/signer/root/subject expectations; and omitted required evidence
accepted as a complete proof.

Report workflow/runtime findings with their separately pinned source identities.
Distinguish a contract consistency issue from actual cryptographic, platform or
publication behavior. Keep synthetic/mock evidence labeled. A documented
unimplemented capability or rejected unsupported input does not establish that a
runtime security boundary was crossed.

## Version and response expectations

Armorer remains experimental before v0.1. Current compatibility limits are in
[the version guide](compatibility.md), and implementation boundaries are in
[the source map](development-status.md). No supported-version window, response SLA,
bounty or completion deadline is promised by this draft.

The maintainer must accept the reporting channel and policy, triage scope and
disclosure process before publication. An accepted report still needs reproduced
evidence, a reviewed fix and appropriate independent validation. Release and
publication authorization remain separate from triage or local test success.

Preserve evidence. Coordinate the fix. Earn the claim.
**This is the way.**
