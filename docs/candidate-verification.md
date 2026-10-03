# Verify the cargo. Hold the trust boundary.

This is a retained implementation walkthrough at the immutable source below.
The source/workflow integrations were subsequently merged. Use the
[current source map](development-status.md) and [acceptance ledger](v01-acceptance.md)
for command availability and remaining gates. The candidate label identifies
this guide's original qualification boundary.

Candidate guide: [PR #21 at `56d7380a5534`](https://github.com/brianluby/armorer/tree/56d7380a5534583b7973b5eedc55a1223b46b60e).
Accepted main provides `verify-release` and `verify-historical-bytes`. Use a binary
built from a reviewed accepted checkout for current verification, or the exact
source above to reproduce its retained qualification. That successor repaired the earlier
artifact-scope defect and added independently approved resolved root features; the
[feature-authority review](../reviews/2026-10-02-pr21-resolved-features.md) records
that source boundary. Complete own signed producer/consumer qualification and
production acceptance remain open. The latest
[evidence and lock review](../reviews/2026-10-03-pr21-evidence-and-locks.md)
records exact observation/exception binding and transaction lifetime repairs.

Inspect the evidence. Earn the verdict. **This is the way.**

## Supply authority through an independent path

Set `armorer_candidate` to the reviewed accepted binary. Obtain the expected context
digest through independent review. Keep trusted inputs outside the downloaded
asset directory and under separate ownership. Hashing a context offered with a
release does not approve it.

The trusted directory contains `armorer.toml`, `armorer.lock`, `Cargo.lock`,
`catalog.json`, `verification-policy.json` and the explicitly selected context:

| Mode | Context file | Meaning |
| --- | --- | --- |
| `native-v3` | `armorer-verification-context-v3.json` | Native v2 runtime authority plus an independently approved exact resolved root-feature set for every selection |
| `native-v2` | `armorer-verification-context-v2.json` | Native runtime authority; defaults disabled and root features exactly equal to literal requests |
| `legacy-v1` | `armorer-verification-context.json` | Frozen catalog v1 and exact distributions; the same literal feature restriction and genuine signatures still required |

The digest binds configuration/lock/catalog/policy, source/ref, caller and reusable
signer pins, run/attempt, selected deliverables, native validator and evidence
requirements. Reviews and time-sensitive evidence use the actual clock. A context
failure never retries another mode. Context/catalog schema agreement alone does
not establish accepted production roots or upstream tool authenticity.

```sh
"$armorer_candidate" verify-release \
  --directory /path/to/downloaded-release \
  --trusted-inputs /path/to/independently-reviewed-inputs \
  --expect-context-sha256 "$approved_context_sha256" \
  --context-kind native-v3 \
  --gh /path/to/qualified/gh \
  --trusted-root /path/to/approved/trusted-root.json \
  --cyclonedx /path/to/qualified/cyclonedx
```

The candidate authenticates context before release/tool reads, then inventory
before producer claims. It snapshots exact retained assets, authenticates required
bundles, validates whole SBOM documents, compares signed predicates with retained
JSON, and reconciles selected Cargo graphs against independent context. Each
retained platform/log reference must match allowed-role bytes scoped to the
current final artifact. Runtime
approval/extraction is not a CLI operation; complete verification accepts an
already approved explicit context.

Offered executable paths are locations, not trust pins. Fixed adapters use private
snapshots, compiled/native approvals and sterile environments. Signature and SBOM
adapters run offline. The consumer never runs Cargo, executes payloads, downloads
tools, collects tokens or publishes. Stable filesystem ancestry and a trusted
local account remain assumptions.

Apple CLI/service selections additionally require native macOS Developer ID/team,
certificate, hardened runtime, timestamp and ticket checks. Linux cannot satisfy
that gate. Library source packages do not require executable signing. Fixed
codesign requests an online ticket check, but the system ticket store may supply
the result; this establishes no fresh-service-response guarantee or authenticated
producer submission UUID. See the
[exact native Apple contract](https://github.com/brianluby/armorer/blob/56d7380a5534583b7973b5eedc55a1223b46b60e/docs/apple-native-verification-v1.md).

The implemented success report uses `status: authenticated-release-files`,
`cryptographic_release_authenticated: true` and `provenance_verified: true` with
bound identities and verified counts. The source implements the required scope
check, but its ordinary regressions do not supply the missing genuine own complete
signed producer positive. In every case `publication_authorized` stays false and
`slsa_build_level` stays null.
Operational errors use JSON/exit 1; argument errors use Clap stderr/exit 2.

Preserve failed evidence. Correct the explicit prerequisite. Retry against the
same independently approved expectations. There is no partial, unsigned,
source/signer override or automatic historical fallback.

Keep the evidence bound to the artifact. Hold the remaining acceptance gates.
**This is the way.**

## Keep approved observations and exception use exact

The independent evidence requirements bind each tool/database observation's
`observed_at`, along with its kind, version, byte identity, authentication record
and age ceiling. A producer-supplied fresh timestamp cannot renew an approved
older observation. Any different time fails with `evidence-tool-pin-mismatch`,
even if both observations would otherwise be fresh.

Required coverage also binds the exact `exception_ids` vector. A reviewed allowed
exception record does not authorize using it for coverage approved without that
exception. Adding or removing an expected ID fails with
`required-coverage-not-enforced`. Obtain a new independent requirement/context
approval when observation or exception use changes; all other scope, expiry and
nonwaivable requirements still apply.

These are semantic checks, with unchanged schema shapes. Neither a matching
timestamp nor a structural exception record proves authentic observation or human
approval. The [producer guide](candidate-producer-evidence.md) keeps native
transport, source/job/writer proofs and final cryptographic evidence separate.

Bring renewed proof. Never renew its label alone. **This is the way.**

## Approve every activated root feature

Requested flags can activate other package features, and defaults can activate
additional members. Review the complete set against separately reviewed immutable
source, package, target, default-feature decision and requested flags.
[Cargo's feature reference](https://doc.rust-lang.org/cargo/reference/features.html)
explains that expansion. The downloaded graph cannot approve its own expectation.

Native v3 retains the native catalog v2 and Cargo graph v2 formats. Its outer
context adds `root_features`, keyed by `deliverable_id--target--feature_set`.
Each selection needs one sorted, unique list. For a separately reviewed `extra`
feature that activates `implied`, the relevant context fragment could be:

```json
{
  "root_features": {
    "app--x86_64-unknown-linux-gnu--optional": ["extra", "implied"]
  }
}
```

This fragment illustrates shape; it is neither a complete context nor an approval.
Include every activated root feature, including default members when enabled.
An independently approved empty set can cover a package without a default
feature. Do not infer that absence from offered release bytes.

The reader rejects missing/extra selection keys, duplicate or unsorted features,
control characters, lists above 1,024 entries and omitted requested features.
The graph's actual root set must equal the approved set; additions and omissions
fail with `cargo-graph-resolved-feature-mismatch`. Approval digest checking still
precedes JSON decoding and offered release/tool reads.

Older contexts have no independent resolved-feature field. Default-enabled
selections fail with `cargo-graph-resolved-feature-expectation-required`; expanded
root sets fail with `cargo-graph-unapproved-root-feature`. Review a separate v3
context for such selections. Keep the intended flags and defaults; an error does
not authorize reducing the selection or trying another mode.

The retained producer graph can aggregate host and target observations. V3 adds
independent root-set authority; it does not remove that producer limitation or
derive an approval from Cargo on the consumer. Review the limitation with the
source and exact selection. See the
[candidate contract](https://github.com/brianluby/armorer/blob/56d7380a5534583b7973b5eedc55a1223b46b60e/docs/resolved-root-features-v3.md).

Approve the complete set. Accept no extra armor. **This is the way.**

## Compare historical bytes only by explicit decision

This separate route accepts an independently approved historical allowlist. Obtain
its exact policy digest, repository, full source commit and stable `v` tag through
review. Keep the policy outside the asset-only directory.

```sh
"$armorer_candidate" verify-historical-bytes \
  --directory /path/to/historical-assets \
  --policy /path/to/independently-reviewed-policy.json \
  --expect-policy-sha256 "$approved_policy_sha256" \
  --source-repository owner/project \
  --source-commit "$approved_source_commit" \
  --source-tag v0.1.0
```

The command compares the complete exact asset set, names, digests and sizes at the
approved source. Missing/extra, nested, unsafe, symlinked or special-file entries
fail. Known modern inventory/attestation filenames are forbidden, even if listed
in the historical policy. Do not remove failed modern evidence to manufacture a
legacy exception. Modern and historical policy source lists cannot overlap.

Success is `status: historical-byte-match`, `authenticity: not-established`,
`provenance_verified: false` and a null SLSA level. It authenticates no source,
signer, provenance or build environment. Actual-clock policy/root/historical
review validity is checked before and after comparison; the frozen policy's root
review requirement does not mean this operation uses a signature verifier.

The [source limits and recovery guide](https://github.com/brianluby/armorer/blob/56d7380a5534583b7973b5eedc55a1223b46b60e/docs/historical-verification.md)
define byte/count/time bounds. Keep failed evidence and obtain corrected files or
a newly independently approved policy. Explicit historical matching never becomes
a fallback after authenticated verification fails.

The [feature-authority review](../reviews/2026-10-02-pr21-resolved-features.md) retains
the preceding head's focused tests and failed ARM receipt. The
[latest review](../reviews/2026-10-03-pr21-evidence-and-locks.md) records the evidence
and lock repairs, affected-scope validation and remaining contributor-guide typo
separately from these open gates: a complete signed rehearsal of Armorer's own
producer/consumer flow, protected Apple production, immutable publication and pilots.
v0.1 targets SLSA Build L2. No candidate file report earns that level by itself.

Keep the source. Keep the limits. **This is the way.**
