# Offline evidence-slot verification

Know the subject. Bring independent policy. Trust only the proof that survives
every comparison. **This is the way.**

The `verification::sigstore::OfflineVerifier` library authenticates one exact
artifact/evidence slot using GitHub CLI **2.102.0**, source commit
`fc4b137cdef0a6bd28fd461b7cf9c84a5812a8cd`. It produces a private-constructor
`VerifiedAttestation` only after real signature verification and all required
identity comparisons succeed. That proof covers the named subject and evidence
scope. It does not establish complete-release acceptance or SLSA Build L2.

## Independent authority

Supply the policy file and its approved SHA-256 through an independent reviewed
channel. Do not derive that SHA, source revision, workflow pin, roots, verifier
identity or expected asset list from the release download. `open` compares the
exact policy bytes before parsing, validates its current review/expiry records,
using the observed system clock before and after verification, and copies only the executable and root bytes whose SHA-256 and size match that
policy. The verifier identity must also match one of the three compiled official
native distribution pins; a different approved executable cannot override them.
An unsupported version, script, non-native executable, symlink, expired
review or hash mismatch fails before executing the verifier.

`ExpectedAttestation` binds the exact source repository, commit and ref; SHA-pinned
reusable signer; source caller workflow; invocation run and attempt; trigger;
predicate; evidence scope; and exact subject basename. v0.1's supported invoking
triggers are `push` for a release policy and `workflow_dispatch` for a rehearsal
policy. The existing policy requires stable release tags or named rehearsal
branches respectively. Direct signers whose certificate SAN ends in a moving
branch ref are outside this adapter's supported identity format: the reusable
signer SAN must contain its full immutable commit.

The caller supplies the independently expected subject digest/size. The complete
consumer must authenticate the inventory first and obtain each subsequent slot's
byte identity and fixed relationship from that authenticated inventory and its
independent configuration. The inventory-first file consumer is now implemented
as a library, with complete signed positive qualification and the final-byte
producer still required for #8 acceptance. This slice does not supply an
`armorer verify` command or automatically approve a downloaded policy/catalog.
The [selected Cargo graph v2 reader](cargo-graph-v2.md) adds independently bound
SBOM graph reconciliation; it is a semantic foundation for the complete consumer,
not a substitute for authenticating graph and SBOM bytes. The
[qualified complete CycloneDX validator](complete-sbom-validation.md) adds the
whole-document schema check, including fields outside the graph. The
[independently approved inventory-first consumer](authenticated-inventory.md)
now connects these checks with inventory authentication and verified-predicate
comparison. Its own complete signed positive rehearsal, qualified platform
report semantics and final-byte producer remain required acceptance gates.

One slot earns one scoped proof. The whole release needs every required gate.
**This is the way.**

## Execution and result checks

Each request snapshots regular artifact and bundle files into a private temporary
directory. The approved native executable and
roots remain separate read-only snapshots. The child receives an empty environment
with an isolated `HOME` and `GH_CONFIG_DIR`, disabled prompts/update notifications,
null stdin, and fixed `gh attestation verify` arguments. No caller shell,
provenance input, artifact extraction, downloaded program execution, credential
inheritance, user gh extension/configuration or repository build occurs.

The command always supplies local `--bundle`, `--custom-trusted-root`, exact
source/ref/digest, exact signer workflow/digest, GitHub OIDC issuer, SHA-256,
required predicate and `--deny-self-hosted-runners`. Its output and execution time
are bounded. Failed verification returns a static stage error; raw child stderr,
signed predicate content and downloaded metadata are not included in that error.
See the [pinned CLI source](https://github.com/cli/cli/tree/fc4b137cdef0a6bd28fd461b7cf9c84a5812a8cd/pkg/cmd/attestation)
and [CLI verification flags](https://cli.github.com/manual/gh_attestation_verify).

A successful exit must yield exactly one verified statement with a verified
timestamp. The adapter compares certificate issuer, SAN, signer digest, hosted
runner, source/ref/commit, caller workflow/digest, trigger, invocation and backend
visibility. Legacy certificate extensions, when present, must agree. The returned
statement must equal the parsed original signed payload, with one exact subject
name and SHA-256 digest. SLSA predicates must additionally agree on build type,
caller workflow, event, repository IDs, resolved source commit, hosted builder and
run invocation. Signed predicates remain workflow-produced assertions; accepting
the signer still requires independently reviewed workflow/catalog expectations.

The reader accepts the exact historical build type
`https://slsa-framework.github.io/github-actions-buildtypes/workflow/v1` and the
current producer's `https://actions.github.io/buildtypes/workflow/v1`. Similar URLs,
different versions, absent fields and wrong JSON types fail. The current build
type requires the exact reusable signer builder URI and internal hosted runner
claim; it cannot use the older generic runner builder. Historical build-type
verification keeps its existing exact builder and certificate checks.

The reader recognizes two source-qualified `runDetails.builder.id` forms. The
genuine 2024 fixture uses `https://github.com/actions/runner/github-hosted`.
The pinned [actions/attest v4.2.0 source](https://github.com/actions/attest/tree/f7c74d28b9d84cb8768d0b8ca14a4bac6ef463e6)
locks `@actions/attest` 3.2.0; its
[SSRI-pinned official distribution](https://registry.npmjs.org/@actions/attest/-/attest-3.2.0.tgz)
derives the builder URI from OIDC `job_workflow_ref`. That modern URI must equal
the independently approved reusable signer repository, path and full commit.
Modern provenance must also contain the OIDC-derived
`internalParameters.github.runner_environment: "github-hosted"`. The older form
may omit that internal field, but any supplied value must agree. Both forms
always require the verified certificate's hosted runner, exact signer URI/digest,
source/caller/run and all other existing checks. Moving refs, caller identities,
wrong workflow pins, missing modern runner claims and self-hosted runners fail.

`tests/fixtures/sigstore/attest-v4-producer-source.json` records actual Git blob
identities and the lock-matched npm SHA-512 distribution identity. The official
release tag is v4.2.0 while its package manifest reports 4.1.0; qualification uses
the immutable action commit and dependency distribution, not that version string.
The npm version metadata has no `gitHead`; the receipt does not invent one.
The additive `attest-v4-predicate-source-v1.json` receipt binds the build-type
constant to that same qualified library source, preserves the previous receipt
byte for byte and records that no producer execution or catalog approval occurred.
New-format tests use this source-derived URI and cover post-crypto semantics only;
they cannot manufacture an
authenticated proof. An own-repository current-producer signed positive remains
required before complete producer/consumer or Build L2 acceptance.

Public-good fixtures pass real offline verification. The private backend supplies
`--no-public-good` and requires a `private` visibility certificate; this slice
only tests rejection of public evidence in that mode. Genuine private-backend
positive qualification remains incomplete. Internal-repository visibility,
GitHub Enterprise, OCI and Windows are unsupported. Do not silently substitute
another backend or relaxed identity policy.

## Transport, bounds and offline roots

Published slots use one bare Sigstore bundle v0.2 or v0.3 with one DSSE signature.
The explicit `verify_download` API accepts at most 32 gh-download JSONL records,
either bare bundles or known wrappers containing `bundle`, optional `bundle_url`
and optional `initiator`. Metadata URLs are never followed or accepted as
authority. Download proofs identify the serialized extracted bare bundle bytes;
they do not authenticate wrapper metadata or its original serialization. It
verifies **every** supplied record separately against the same
independent expectations; one invalid record fails the entire operation. Exact
duplicate records, arrays, unknown wrapper fields, concatenated JSON, duplicate
JSON keys (including inside the base64 signed payload) and unsupported bundle
versions fail. This closes the native CLI's verified-subset behavior, demonstrated
with a genuine valid record plus an invalid signature.

Current explicit bounds are 64 MiB total bundle/download bytes, 17 MiB decoded
signed payload, 4 MiB roots, 128 MiB native verifier, 1 GiB per subject, 128 MiB
captured stdout, 1 MiB captured stderr and 30 seconds per native invocation. JSON
is also limited to 262,144 value nodes and 64 nesting levels before allocation. Both
streams are bounded while being read. Frozen v1 contract loaders retain their
1 MiB limit. Inputs must be on a stable filesystem; this adapter does not protect
against a malicious process with the same OS account modifying private snapshots
or directory ancestry. It checks regular-file identity during open and rehashes
all verification snapshots after the child exits.

Keep the limitations with the result. **This is the way.**

Root transport accepts one bounded JSON object or at most 32 JSONL root objects;
duplicate keys, arrays, missing or malformed records fail. Root updates require
a new independently approved policy digest and current root
review. The offline call does not fetch or silently refresh roots. Offline key
material has no built-in expiry and cannot reveal revocations since it was
exported; Armorer's policy review expiry is a separate gate. See
[GitHub's offline root guidance](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/verify-attestations-offline).
Historical byte compatibility uses the separate explicit
[`verify-historical-bytes` operation](historical-verification.md), never a fallback
from an authentication error in this adapter.

## Authentic integration evidence

`tests/fixtures/sigstore/source-receipt.json` binds the inert upstream artifact,
signed bundle and test roots to Git blobs and SHA-256 at the pinned CLI commit.
The artifact is never executed. `verified-result.json` is retained output from the
real pinned native verifier and supports post-crypto consistency tests only.
The fixture is an external project from 2024, not an Armorer release, production
root approval, independent source review or Build L2 acceptance. Its original MIT
license is retained as `LICENSE.cli`.

`verifier-pins.json` binds official archives and exact executable leaves for Linux
x86_64, Linux ARM64 and macOS ARM64. Each archive matched the publisher release
digest and the exact SHA-pinned checksum manifest. The CI qualification helper
reads only a named regular archive leaf, checks executable bytes and never
executes an unqualified download. Run the real test explicitly:

```sh
python3 -I scripts/qualify-test-verifier.py --output-directory /tmp/armorer-fixture-gh
ARMORER_TEST_GH=/tmp/armorer-fixture-gh/gh cargo test --locked --test real_sigstore -- --ignored --test-threads=1
```

Ordinary `cargo test` reports this test as ignored because it requires the pinned
native binary. Hosted development CI has a separate required explicit step on
Linux/macOS; absence or wrong bytes fail that step. Local qualification validates
real signatures on the workstation's native platform; fetching the other archive
alone does not validate that platform's execution.

The exact `base64 = 0.22.1` dependency is added only to decode signed payload bytes
before duplicate-key checking; Cargo.lock retains all previous dependency versions
and records its registry checksum. Cryptography remains delegated to pinned gh.
