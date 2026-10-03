# Earn the release

An artifact carries its own burden of proof. Bind source, final bytes, policy
and evidence. Hold the gate when required proof is missing.
**This is the way.**

This guide explains planned v0.1 gates and current limitations. It is not an
executable publication runbook. Available commands are in
[the CLI reference](cli-reference.md); pending work is in
[the development map](development-status.md). v0.1 targets SLSA Build L2.
No current Armorer release achievement is asserted. L3 stays future work.

## Keep each state separate

| State | Evidence to retain |
| --- | --- |
| Configuration valid | Valid typed intent/local inspection; every unmet prerequisite |
| Configured | Reviewed pins/policy, exact managed files, resolved manual prerequisites |
| CI verified | Hosted required checks for exact source/config/pins and feature cases |
| Release rehearsed | Complete own build/sign/attest/consumer run; rehearsal identity; no publication |
| Published | Actual release, complete served assets, approval and immutability evidence |
| Provenance verified | Independent per-artifact result with reviewed policy/roots/verifier identity |

Use the [lifecycle contract](trust-contracts-v1.md#capabilities-prerequisites-and-lifecycle)
to bind source, config and policy. Changed config invalidates matching evidence.
An external genuine signature fixture qualifies a scoped verifier behavior; it
does not rehearse Armorer's own producer or close a pilot gate.

## Prepare independent authority

Review repository/ref/commit, caller/signer workflow pins, packages, binaries,
targets, features, expected assets, predicates and evidence requirements. Obtain
catalog, policy, roots and verifier identities through an independently approved
channel. Retain exact bytes and current review/expiry records. Synthetic fixtures
are training material.

Authenticate inventory against independent intent, then verify every required
asset and relationship. The [trust contracts](trust-contracts-v1.md) define
hash order, final-byte proof and historical limits. The
[offline verifier](offline-verifier.md) defines the current individual-slot API.
Downloaded inventory cannot select its own authority.

Keep authority outside the offered claim. **This is the way.**

## Prepare platform gates

Record repository/account eligibility, protected refs, effective required checks,
signing/publication environments, hosted runners and current immutable settings.
Unavailable observations and HTTP 403 cannot satisfy required capabilities.
Main check/plan do not run GitHub preflight or alter settings. Supported account
combinations need qualified preflight and current platform evidence before adoption.
Use [the platform preparation guide](platform-prerequisites.md) for the current
GitHub eligibility matrix and manual observation steps.

Build jobs execute Cargo/build scripts with read access and no signing, OIDC or
publication credentials. Trusted attestation jobs need `contents: read`,
`attestations: write` and `id-token: write`. Signing/publication have separate
protected boundaries. Immutable-setting inspection needs narrow Administration
read access. Follow [the credential architecture](../ARCHITECTURE.md#trusted-release-controller-and-credentials).

Exact credential names, approvals and rotation steps belong to reviewed adapters.
The owner provisions values locally. Never collect values or bypass explicit
bindings with inherited secret sets.

## Bind Apple finalization to the final archive

For macOS CLI/service deliverables, retain unsigned, signed, notarized and packaged
identities. Verify the exact archived executable, expected Developer ID team,
hardened runtime, timestamp and Apple acceptance. Attest after all mutations.
Library `.crate` source packages need no Developer ID executable chain.

Proposed #19 checks verify bounded standalone Mach-O bytes through Apple APIs
and request an online ticket check. Audit results allow cached or network ticket
evidence. They promise no fresh revocation evidence, offline acceptance, stapled
standalone executable or authenticated producer submission log. Protected
producer finalization remains separate work.

Let final bytes carry final proof. **This is the way.**

## Rehearse before publication

Use a protected rehearsal source and independently approved rehearsal policy.
Retain the entire chain, source/run/attempt, inventory and limitations. Rehearsals
must not publish or move tags. Hosted evidence must cover supported native
platforms and required feature cases. Pilots need fresh isolated adoption and
independent consumer results.

Draft creation, exact upload, served-byte verification, approval, immutable
publication and post-publication verification require the future release state
machine. Schemas, mocks and one successful target cannot establish those outcomes.
Merging and publication require explicit human authorization.

## Preserve evidence on failure

For local writes, retain the original digest/journal and follow
[apply recovery](apply.md#transaction-and-recovery). For trust failures, retain
the failed stage and source/policy/byte identities. Review independently approved
replacement inputs before retrying. Expired root policy or failed authenticity
cannot silently downgrade into historical mode.

Under planned policy, advisory outages or databases outside accepted freshness
block release. Owned identical drafts may resume only missing identical uploads
after fresh verification. Conflicting or published bytes require investigation;
replacement is forbidden. Post-publication failure is an incident requiring a
reviewed corrected version. Alias promotion waits for immutable-release verification.

Publication recovery rules remain design requirements until backend acceptance.
Keep failure visible. Preserve the original record. **This is the way.**
