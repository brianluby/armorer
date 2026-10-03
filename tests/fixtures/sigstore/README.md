# Authentic external Sigstore fixtures

Copied byte-for-byte from MIT-licensed cli/cli commit
`fc4b137cdef0a6bd28fd461b7cf9c84a5812a8cd` (GitHub CLI 2.102.0).
`source-receipt.json` records exact source paths, Git blobs, sizes and SHA-256.
The artifact is inert test input and must never be executed.

These fixtures are genuinely signed external examples. They are not Armorer
artifacts, approved Armorer signers or evidence of release readiness/SLSA.
The root is fixture-specific pinned test material, not a production root policy.
Independent expected identity comes from the pinned upstream integration tests
and is checked against the successfully verified certificate in the real harness.
Production input policy/root/verifier approval and full asset/SBOM graph validation
remain #8 implementation work.

`verified-result.json` is the successful output of independently hash-qualified
native gh 2.102.0 with the exact source/signer/hosted/predicate/offline-root flags.
It is a post-cryptographic consistency fixture, not independently signed evidence.
`verifier-pins.json` records exact official native distributions for hosted real
verification. Neither document qualifies these test roots as production policy.
See `docs/offline-verifier.md` for bounds, test invocation and remaining gates.
