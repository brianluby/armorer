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

The [candidate workflow guide](candidate-workflow-prerequisites.md#preserve-the-exact-credential-boundary)
records one exact new qualification interface: ephemeral `github.token` through
`ARMORER_READ_TOKEN`, isolated into the pinned native child as `GH_TOKEN`.
It is a read-process variable, not a repository secret to provision. The candidate
observer recognizes the two named environments without creating jobs or accessing
their secrets. Production administrative-read and Apple credential bindings,
effective current-attempt approvals and rotation remain unqualified.

## Match the candidate helper's exact read interface

These interfaces belong to workflow candidate
[`e88e25fc46c1bb5298579c5cd49f52d43215b9e1`](https://github.com/brianluby/armorer-workflows/tree/e88e25fc46c1bb5298579c5cd49f52d43215b9e1).
They are not generated main callers or production signing-secret definitions.
Use the fixed helper's declared name; a similarly named variable does not supply
the same interface.

| Name | Source and consumer | Boundary |
| --- | --- | --- |
| `ARMORER_READ_TOKEN` | Qualification steps explicitly bind `github.token` for Python source/controller/transport/capability readers | Read-process input; native child receives the selected token as `GH_TOKEN` in a sterile environment |
| `ARMORER_WORKFLOW_READ_TOKEN` | Qualification actions explicitly bind `github.token` for the fixed mapped-job worker and artifact-writer observer | A distinct Node/worker interface; mapped worker needs contents/actions read; writer observation also needs checks read |
| `GH_TOKEN` | Fixed native subprocess adapter constructs this binding from its explicit read token | Ambient credentials or debug/proxy settings cannot replace the independently selected token |
| `ACTIONS_RUNTIME_TOKEN` | Platform runtime supplies the artifact-service credential to the writer observer | Separate from REST read identity; used only for fixed read-only `ListArtifacts` POST, never create/finalize/delete/download-URL methods |
| `ACTIONS_RESULTS_URL` | Platform context identifies the artifact receiver | Location metadata, not a credential; observer accepts only its fixed GitHub.com receiver origin |
| `ACTIONS_ID_TOKEN_REQUEST_URL`, `ACTIONS_ID_TOKEN_REQUEST_TOKEN` | Platform supplies the request location and bearer credential to the fixed producer OIDC helper | Request service only; issuer/JWKS/audience are fixed separately and the bearer never goes to JWKS |

The current development transport and combined collection jobs grant
`contents: read`, `actions: read`, `checks: read` and bind the corresponding
ephemeral read variable explicitly. They request no live OIDC and grant no
signing/publication authority. The OIDC helper's real producer integration would
need separately authorized `id-token: write` and qualification of its fixed
service route and exact claims. Do not add that grant to consuming build jobs
to reproduce a synthetic fixture.

The artifact observer validates independent intent and fixed platform origin
before consuming its two credentials. It removes the REST/runtime variable
bindings from its own process environment; the credentials remain internal to
the bounded observation. Repeated use cannot assume those bindings still exist.
Deleting environment bindings is not memory zeroization or protection from
hostile code already in the same process.

Runtime-token decoding supplies routing IDs only. The artifact-service response
must join independent native job/check records and exact artifact identities.
OIDC needs native RS256 verification and exact independent source/caller/signer/
run/attempt/check-run claims. Neither a decoded token, configured environment nor
copied audit JSON can grant a protected operation. Keep original private proofs
within their [documented lifetimes](candidate-producer-evidence.md#use-original-identity-proofs-within-their-lifetimes).

No operator token-export command belongs here. The owner keeps credential values
within platform/local provisioning. The
[identity-interface review](../reviews/2026-10-03-identity-prerequisites.md)
records source checks and synthetic test scope. Actual protected signing,
administrative-read App bindings and publication setup still require their
accepted production adapters.

Name the interface. Keep the credential inside it. **This is the way.**

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

The [candidate unsigned intake](candidate-workflow-prerequisites.md#freeze-the-complete-unsigned-apple-handoff)
now snapshots and inspects a complete handoff before a future transformation.
It receives no credential, executes no payload and keeps all signing authority
false. Its byte/shape checks cannot replace the protected approval or Developer
ID/notarization/final-byte evidence required above.

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
