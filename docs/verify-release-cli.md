# Complete release verification from the CLI

`verify-release` authenticates the complete release set with the inventory-first
strict consumer. Signature/SBOM adapters operate offline; required Apple
executable checks use macOS Security.framework and request an online ticket
check through fixed system codesign. The final native requirement may use the
system ticket store; no fresh service-response or offline availability guarantee
is claimed.
It authenticates the independent context before opening executable adapters or
reading a release. It then verifies the inventory before parsing producer claims,
rehashes every exact declared asset, verifies every required Sigstore bundle,
compares signed SBOM predicates with retained JSON and selected Cargo graphs, and
validates complete evidence chains against the independently reviewed requirements.
A failed required member produces no successful subset.

```sh
armorer verify-release \
  --directory /path/to/downloaded-release \
  --trusted-inputs /path/to/independently-reviewed-inputs \
  --expect-context-sha256 '<independently approved 64-character SHA-256>' \
  --context-kind native-v3 \
  --gh /path/to/qualified/gh \
  --trusted-root /path/to/approved/trusted-root.json \
  --cyclonedx /path/to/qualified/cyclonedx
```

The placeholder digest must be replaced by an independently approved digest;
hashing an offered release's context and treating that hash as approval defeats
the trust boundary. Keep the trusted input directory under separate ownership
from the downloaded release. It contains `armorer.toml`, `armorer.lock`,
`Cargo.lock`, `catalog.json`, `verification-policy.json` and the explicitly
selected context:

| `--context-kind` | Context file | Catalog semantics |
| --- | --- | --- |
| `native-v3` | `armorer-verification-context-v3.json` | Reviewed runtime distribution, target-specific native members and exact resolved root features |

The removed `legacy-v1`/`native-v2` kinds are argument errors (ADR 0012); no fallback.

The context digest binds exact config/lock/catalog/policy identities, source and
ref, caller and reusable signer pins, run/attempt, native validator, selections
and evidence requirements. Required reviews and time-sensitive evidence are
rechecked using the actual clock. Catalog/schema agreement alone does not prove
upstream authenticity or acceptance; reviewed native distribution/root/tool
qualification remains an independent prerequisite. Context failures never retry
another version. Legacy v1 here still requires genuine signatures; the separate
`verify-historical-bytes` command retains its explicit unauthenticated comparison
semantics and is never a verification fallback.

Executable paths are offered locations, not pins or generic command inputs. The
consumer snapshots the executables and root privately, compares their exact
bytes with independent context/policy and compiled native approvals, checks
native executable format and invokes the signature/SBOM adapters only through
their fixed offline commands with a sterile environment. After whole-file
authentication, required Apple executables also undergo the native macOS
signature, timestamp, team, runtime and ticket checks described below. It does not search PATH for replacements, learn source or
signer expectations from bundles, collect tokens, download tools, contact release
APIs, run Cargo, execute/extract payloads, mutate tags or publish releases. Stable
filesystem ancestry and no hostile process sharing the consumer's OS account
remain required. See [offline verifier details](offline-verifier.md) for native
version/root support, bounds and limitations.

## One manifest instead of seven flags

The seven path and digest flags can instead be named by one JSON manifest, pinned by
a single digest typed on the command line:

```sh
armorer verify-release \
  --manifest /path/to/release-inputs.json \
  --expect-manifest-sha256 '<independently approved 64-character SHA-256>'
```

The manifest has `schema_version` 1, `contract: "armorer-release-inputs"` and one
field per flag: `directory`, `trusted_inputs`, `expect_context_sha256`,
`context_kind` (`native-v3`), `gh`, `trusted_root` and `cyclonedx`. An optional
`publication` block is ignored here and required by `publication`. See the
[example](../examples/release-inputs/README.md) and
`armorer schema release-inputs-manifest`.

* **Its digest is the approval.** The manifest holds the expected context digest, so
  it must not be trusted on its own. Armorer reads the file (at most 64 KiB, regular
  file, never a symlink), hashes its exact bytes and compares them with
  `--expect-manifest-sha256` before parsing it or opening any path it names. A
  mismatch reports only `release-inputs-manifest-digest-mismatch`. Reviewing the
  manifest means reviewing the digests inside it, and the digest you pass must come
  from somewhere other than the manifest's own location.
* **Strict parsing.** Unknown fields, duplicate keys, another `schema_version` or
  `contract`, missing fields, empty paths and paths with control characters are
  refused. Other path content is not restricted, exactly like the flags.
* **Relative paths** resolve against the directory that contains the manifest, never
  the working directory, so a manifest and its inputs can be moved together. Absolute
  paths are used as written.
* **Exclusive with the flags.** `--manifest` and `--expect-manifest-sha256` require
  each other, and `--manifest` combined with any of the seven input flags is an
  argument error (exit 2). The flag form is unchanged and still requires every flag.
* **No change to verification.** The manifest does not make verification weaker or
  stronger: it moves seven values into one reviewed file, then runs the same
  sequence in the same order (context first, then adapters and release files). On
  success the output is the same plus `inputs_manifest` with the manifest's `sha256`
  and `size`; the flag form's output is unchanged.

On complete success, stdout is JSON with `status: authenticated-release-files`,
`cryptographic_release_authenticated: true`, `provenance_verified: true`, the
independent context/catalog/input identities, inventory and detached bundle
identities, approved native verifier/root/policy identities, all inventory assets
and counts of verified bundles/SBOMs. The command exits 0 only after every required
consumer check succeeds. This report is an audit record, not a reconstructable
private authorization proof. `publication_authorized` remains false and
`slsa_build_level` remains null: file verification alone does not establish hosted
release acceptance, protection, Apple operational qualification or publication.

Operational errors use the existing JSON error format and exit 1; argument errors
use Clap's stderr format and exit 2. Native verifier diagnostics are redacted by
the existing adapters. Fix the explicit failed prerequisite and rerun the same
independent expectations. There is no partial, unsigned or historical downgrade
switch, source/signer override or custom provenance input.

## Evidence and remaining gates

CLI failure cases cover unapproved context bytes before release/tool reads,
explicit version rejection, no caller-selected fake executable use and argument
claim injection. The required genuine native inventory integration also invokes
the CLI with a real retained Sigstore bundle and qualified gh/CycloneDX; unrelated
bundles cannot authenticate malformed inventories or activate historical fallback.
These tests do not substitute for a complete own signed producer positive.

Production catalog/root acceptance, qualified final producer integration, the own
complete signed non-publishing rehearsal, protected Apple finalization, immutable
publication, both pilots and human acceptance remain incomplete. This command
makes the consumer usable without weakening or completing those external gates.

Apple CLI/service selections also require the [native Apple verification v1](apple-native-verification-v1.md) gate on macOS. Required native failures yield no successful complete-release result; library/Linux sets report `apple_verification: not-required`.

## Explicit resolved features

`--context-kind native-v3` and an independently approved
`armorer-verification-context-v3.json` carry complete default/implied feature sets.
[The guide](resolved-root-features-v3.md) describes exact expectations.
No version fallback is attempted.
