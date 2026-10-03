# Explicit historical byte compatibility

`armorer verify-historical-bytes` compares the complete local download directory
with an independently reviewed historical allowlist in a v1 verification policy.
It succeeds only for the exact requested repository, stable `v` tag, full source
commit, asset names, SHA-256 digests and sizes. This operation establishes a
**historical byte match**. It does not authenticate source, provenance, signer,
build environment or SLSA level.

## Independent selection

Obtain the policy bytes, approved policy SHA-256, source repository, full commit
and tag through an independent reviewed channel. A downloaded policy or checksum
file cannot approve itself. Review must bind the actual historic asset bytes and
record their limitations; the example policy uses synthetic fixtures and is not
approval for a real release. Store the policy outside the asset-only directory.

```sh
armorer verify-historical-bytes \
  --directory /path/to/historical-assets \
  --policy /path/to/independently-reviewed-policy.json \
  --expect-policy-sha256 "$APPROVED_POLICY_SHA256" \
  --source-repository owner/project \
  --source-commit "$APPROVED_SOURCE_COMMIT" \
  --source-tag v0.1.0
```

The command never reads repository manifests, invokes Cargo, downloads anything,
executes payloads or extracts archives. The global `--repository` option is
irrelevant to this operation. It checks the exact policy digest before parsing;
rejects ambiguous JSON; validates policy, root and historical review expiry at the
actual system clock before and after comparison; and snapshots only bounded
regular asset leaves into a private temporary directory. Root review is required
by the frozen policy contract, but this operation does not use those roots or run
the signature verifier.

## No authentication fallback

Call this command explicitly for a separately reviewed historical source.
Authenticated verification never calls this API, including after missing,
invalid or unsupported signatures. A source ref cannot appear in both modern and
historical policy lists. Rehearsal policies, moving branches, unstable tags,
wrong commits/repositories and unlisted releases fail.

Every downloaded filename must match the complete allowlist. Missing, extra,
unsafe, non-UTF-8, nested, symlinked or special-file entries fail. Known modern
inventory and attestation filenames are forbidden even if their hashes were
mistakenly approved as legacy assets: `armorer-release-inventory.json`, its
detached bundle, and case-insensitive `.sigstore.json`, `.sigstore.jsonl`,
`.intoto.jsonl`, `.attestation.json` and `.attestations.json` suffixes. This name
guard does not claim to detect arbitrarily renamed signature formats inside
archives; no archive contents are interpreted. Independent historical review
and the explicit source/mode separation remain required.

Successful JSON has `status: "historical-byte-match"`,
`authenticity: "not-established"`, `provenance_verified: false` and a null
`slsa_build_level`, alongside the requested source, exact approved policy identity,
matched asset identities and reviewed limitations. Exit status is zero only after
every comparison succeeds. Errors produce an error object and nonzero exit;
there is no partial result or automatically selected alternate policy.

## Bounds and recovery

The policy limit is 1 MiB with duplicate-key, 262,144-node and 64-depth JSON
limits. Assets are at most 1 GiB each, 4 GiB combined and 8,192 regular leaves.
Limitations are at most 128 nonempty entries of 4,096 bytes without control
characters. A 20-minute elapsed-time check runs between bounded filesystem
operations; it does not interrupt an individually stalled filesystem read.
Snapshots are read-only and rehashed before success. Use stable local inputs;
hostile mutation by the same OS account or directory-ancestor substitution is
outside this snapshot guarantee.

If a digest, set or review-expiry check fails, retain the failed evidence and
obtain corrected files or a newly independently approved policy. Retry the
explicit command against the same reviewed identities. Do not discard offered
attestation files to turn an authenticated release into a historical exception.
This route does not establish production root approval, a complete signed
Armorer rehearsal, publication readiness or full ticket #8 acceptance.
