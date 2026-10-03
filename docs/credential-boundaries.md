# Keep the keys with their owner

Know which job needs authority. Give it only that authority. Keep the key out of
the build. **This is the way.**

This guide records current setup and prepares the future protected release
adapters. It names no unimplemented secret interface. Main's local discovery,
catalog, preview and transaction commands collect no credentials. The offline
verifier takes independently approved public trust inputs and qualified native
tools, not signing keys. See [platform prerequisites](platform-prerequisites.md)
for manual settings reads and [candidate verification](candidate-verification.md)
for the proposed complete consumer.

## Current CI and unsigned build need no provisioned secrets

The embedded catalog pins `brianluby/armorer-workflows` commit
`772ca83e386c883c88cc3b936d69f8cb3216c91e`. Its
[Rust CI](https://github.com/brianluby/armorer-workflows/blob/772ca83e386c883c88cc3b936d69f8cb3216c91e/.github/workflows/rust-ci.yml)
and [unsigned builder](https://github.com/brianluby/armorer-workflows/blob/772ca83e386c883c88cc3b936d69f8cb3216c91e/.github/workflows/rust-build.yml)
declare `contents: read`, no caller inputs/secrets, no signing environment and no
OIDC/publication grant. Checkout uses `persist-credentials: false`. Generated
callers also grant only contents read and pass no secrets.

The builder's nested CI call is separately pinned to
[`d54407809590`](https://github.com/brianluby/armorer-workflows/blob/d544078095902bb9d29d9770959bad706de8e8bd/.github/workflows/rust-ci.yml).
That workflow retains the same no-input/no-secret, contents-read interface.
Inspect every nested pin; the outer commit does not move the nested reference.
Both workflows build the trusted intent validator from Armorer commit
`1615339b6415afa9bb9d65e2c37304e977158c4e` outside caller source.

| Current operation | Credential setup |
| --- | --- |
| Local check/plan/preview/catalog/apply/recovery | No token or signing secret required by the interface |
| Generated CI and manual unsigned build | No user-provisioned secret names; GitHub supplies its scoped workflow token |
| Offline signature/SBOM consumer | Approved verifier/root/policy/context bytes; no signing credential collection |
| Manual immutable-setting inspection | Owner's separately configured Administration-read identity; no accepted Armorer credential-name adapter yet |

Do not add `secrets: inherit`, Apple credentials, OIDC or write permissions to
these build calls. Caller Cargo/tests/build scripts execute in their unprivileged
jobs. The [workflow source guide](https://github.com/brianluby/armorer-workflows/blob/772ca83e386c883c88cc3b936d69f8cb3216c91e/docs/ci.md)
describes fixed commands, credential-cleared child environments and scanner
limits. Passing CI does not establish signing or publication readiness.

Keep the build unprivileged. **This is the way.**

## Prepare each future protected boundary

The [credential architecture](../ARCHITECTURE.md#trusted-release-controller-and-credentials)
defines these duties. Exact secret/App-binding names, provider setup and rotation
commands must come from each reviewed adapter before production adoption.

| Boundary | Planned minimum authority | Remaining setup definition |
| --- | --- | --- |
| Build-evidence/final attestation | Contents read, attestations write, id-token write; fixed trusted helpers | Accepted controller and exact protected identity; no caller provenance input |
| Apple transformation | Consuming repository's protected `release-signing` environment; named signing/notarization credentials; no publication rights | Exact adapter secret names, owner-controlled provider setup and handoff/rotation qualification |
| Capability preflight | Narrow Administration/metadata read identity isolated from build/sign jobs | Reviewed App or equivalent binding and authenticated current observations |
| Draft/publication | Contents write in protected `release-publish` jobs; no unnecessary OIDC | Owned-receipt backend, effective approvals and remote transition/race tests |
| Optional alias movement | Separately approved identity and exact stable-release/ref decision | Qualified alias adapter and exact readback |

`release-signing` and `release-publish` are design names, not current generated
jobs or evidence that an environment exists. Record the actual consuming
repository, effective ref/tag restrictions, reviewer policy, self-review and
administrator bypass behavior, and exact workflow/runtime pins. Confirm account
eligibility before relying on a protected environment.

GitHub documents different behavior for explicitly passed secrets and job-level
environment secrets. Nested permissions cannot increase beyond the caller grant.
Prove the actual cross-repository environment/approval behavior in a hosted
rehearsal; a workflow-host environment does not supply that proof.
[Reusable workflow rules](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows),
[environment protection](https://docs.github.com/en/actions/reference/workflows-and-actions/deployments-and-environments).

Record names, presence, owner and scope. Presence does not establish validity.
The owner provisions values through local/provider-controlled interfaces. Never
request, paste, echo, upload or store their values in a guide, issue, plan or
review packet. Public team/certificate identities and root digests are evidence
fields; they do not replace secret handling or independent policy approval.

Give each protected job its own purpose. **This is the way.**

## Review signing approval before releasing credentials

The future signing adapter must validate the exact source/run/attempt and unsigned
handoff before accessing credentials. Its fixed trusted code signs/notarizes and
packages without running caller code. The final archive must bind the signed
executable, expected team, certificate and platform evidence; final attestations
come after every mutation. Library source packages have no executable Developer
ID signing requirement.

Keep source approval, environment approval, signing result and publication
authorization distinct. An environment label or a certificate installed in a
runner proves none of those outcomes. Rehearsal evidence must show the required
approval/ref boundary and complete final-byte consumer checks on macOS, while
publication and tag movement remain disabled. A native reference fixture does
not exercise the consuming repository's production credentials.

Hold credentials until the handoff is verified. **This is the way.**

## Prepare a rotation record

This is a decision sequence for the future qualified adapter, not a current
credential-rotation CLI. Retain only names and public identities:

1. Inventory the accepted adapter commit, exact binding names, owner, scopes,
   protected environment and affected jobs. Record pending runs and public
   team/certificate identities where applicable.
2. Review the proposed replacement, affected approvals and any containment action.
   Obtain owner authorization before changing a provider, environment or remote
   workflow setting. Keep the old release/evidence record intact.
3. The owner provisions replacement values locally. Reinspect name/presence and
   effective scope without exposing values; test through the fixed protected
   adapter rather than a credential-bearing diagnostic command.
4. Independently approve changed certificate/root/catalog/policy/context identities
   where required. Rehearse the complete new handoff, signing/notarization and
   final-byte consumer chain without publishing or moving tags.
5. Record exact source/run/attempt, adapter/public identities, required approval
   observations and consumer results. Retire the old credential through an
   explicitly authorized provider action, then read back the resulting state.

Failed qualification keeps rotation incomplete. Review retry/containment with the
owner; do not silently restore broader credentials or bypass a trust failure.
Use [evidence maintenance](evidence-maintenance.md) for independently approved
root changes and [release recovery](release-recovery.md) for incidents involving
published artifacts. A changed key cannot grant original-build provenance to an
older release or rewrite its receipts.

Keep every approval. Protect every key. **This is the way.**
