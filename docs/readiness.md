# Onboarding and release readiness

The accepted source can inspect Rust workspaces, provision unprivileged CI/build
callers, preview reviewed upgrades and verify complete release evidence. Protected
Apple finalization, native publication enforcement, production trust catalogs and
both adoption rehearsals remain incomplete. Use the
[acceptance ledger](v01-acceptance.md) to identify exact candidate heads and receipts.
No SLSA level or release authority follows from installing the CLI.

## Inspect and configure a project

Build the reviewed CLI checkout with Rust 1.95.0 using `cargo build --locked`.
Run the commands below from that checkout using `./target/debug/armorer`;
Cargo does not add the built executable to `PATH`. Author `armorer.toml` in a stable
consuming checkout with explicit repository, packages, binaries, targets and feature
sets. Start with the [library, CLI or service examples](../examples/bootstrap-v2/README.md)
and review their [discovery limits](onboarding.md). Armorer does not install tools
or execute consuming build scripts during check/plan.

Create a separate reviewed policy file. Its license, advisory exception, source and
dependency-ban decisions belong to the owner; an example is not approval.
Place the plan output outside the consuming checkout:

```sh
./target/debug/armorer --repository /path/to/project bootstrap check --policy /path/to/reviewed-policy.toml
./target/debug/armorer --repository /path/to/project bootstrap plan --policy /path/to/reviewed-policy.toml > /path/outside-project/bootstrap-plan.json
```

`check` exits 2 while secure-release prerequisites remain. A valid `plan` exits 0
even if its exact before/proposed views contain conflicts. Review every managed
file, catalog pin, policy byte, ownership disposition and complete diff. An unowned
customization requires an explicit conflict decision. Neither command writes the
project or contacts GitHub.

After independently approving and retaining `plan_sha256`, apply that exact intent:

```sh
./target/debug/armorer --repository /path/to/project bootstrap apply --plan /path/outside-project/bootstrap-plan.json --expect-plan-sha256 APPROVED_SHA256
```

Follow [bootstrap ownership, repeatability and recovery](bootstrap-v2.md) for the
five fixed files, stale-preimage rejection, incomplete journals and exact rollback.
Retain the approved digest. Reading a replacement plan's own digest is not approval.
Follow [reviewed upgrades/imports/reversal](upgrades.md) for later changes; migration
between two accepted catalog revisions still needs operational qualification.

## Verify release evidence separately

Local configuration and successful unsigned CI do not authenticate a release.
[Complete verification](verify-release-cli.md) needs independently approved expected
source/workflow/run context, exact inventory and asset set, native verifier/catalog
identities, roots and all required bundles. Compare paired SBOM/graph evidence and
rehash the actual downloaded bytes. Authentication failure blocks verification;
it never selects a weaker context or historical fallback.

Use [offline roots and bundles](offline-verifier.md) only within their documented
trust and freshness boundaries. An independently reviewed
[exact historical policy](historical-verification.md) has its own weaker evidence
state. Required Apple executables additionally need the supported macOS
[native signature/team/hardened-runtime/timestamp/notarization checks](apple-native-verification-v1.md).
Linux cannot confer those Apple checks. Public reference fixtures establish scoped
verifier behavior; a complete signed rehearsal of Armorer's own producer/consumer flow remains required.

## Before signing or publication

The [owned-draft controller](owned-draft-controller-v1.md) provides reviewable intent,
separate staging/publication approvals, exact owned release/asset identities,
served-byte re-download, durable recovery and signed postpublication checks.
Its native writes remain blocked by unsupported current-attempt environment
enforcement and serialization across runners. A capability setting, local lock or
syntactically valid approval cannot establish those missing guarantees.

Prepare reviewed workflow pins, production tool/root/catalog digests, exact stable
ref restrictions and approved reviewers distinct from the initiating actor before
requesting protected setup. Keep credential values under the owner's control.
Do not expose caller commands or untrusted build execution in credentialed jobs.
No settings, credentials, tags, drafts or published bytes are changed by this guide.

Ambiguous controller writes require the documented recovery operation; do not
delete a journal, adopt an unowned draft, replace an asset or move a stable tag.
Expired/revoked/missing trust evidence, unsupported platforms and provider outages
remain explicit failures or incomplete states. Retain failed receipts and use a
new reviewed candidate when source or authority changes.

## Record the achieved state

| State | Required evidence |
| --- | --- |
| Configured | Approved plan, exact local files/ownership and policy, selected immutable pins |
| CI verified | Exact source head and all required hosted platform jobs; required/reporting/disabled controls remain distinct |
| Release rehearsed | Non-publishing complete signed producer output and independent final-byte consumer receipt on every supported target |
| Published | Separately authorized provider transition and verified immutable release/served asset identities |
| Provenance verified | Independently approved context/roots and real signed evidence matching the exact final bytes |

The accepted implementation has scoped local and hosted CI receipts. Full release rehearsal,
publication validation, own signed Armorer dogfooding and the dependent Momus and
rusty-brain pilots remain open. Complete the ledger's required review and human
acceptance gates before promoting a candidate to an accepted release baseline.
