# Genuine release-attestation output

`verified-cli-release.json` is the complete output of the hash-qualified official gh 2.102.0 command `gh release verify v2.102.0 --repo cli/cli --format json`, observed on 2026-10-03. Exact SHA-256: `2535499492919edc378f0be2fb05b3e784b9538b9306b9bf3dafcfb0c8764840`; 10,192 bytes. It contains the real signed bundle and native verification result, including the GitHub release certificate and verified timestamp.

Independent fixed identities: repository `cli/cli`, repository/package ID `212613049`, owner ID `59704711`, release ID `399674740`, tag `v2.102.0`, tag-object SHA `fc4b137cdef0a6bd28fd461b7cf9c84a5812a8cd`. The statement uses `https://in-toto.io/attestation/release/v0.2`.

Ordinary tests exercise post-cryptographic consistency and tamper checks. Their synthetic asset sizes do not authenticate downloaded asset bytes. The ignored native test repeats genuine public-release verification and compares its complete subject set with this frozen baseline. That qualification uses the CLI's public live root routing, **not** Armorer's independently approved production root. It grants no publication or ownership authority and is not an Armorer signed-release rehearsal. No release asset is executed or extracted.

Primary implementation: [pinned verification command](https://github.com/cli/cli/blob/v2.102.0/pkg/cmd/release/verify/verify.go), [pinned release verifier](https://github.com/cli/cli/blob/v2.102.0/pkg/cmd/release/shared/attestation.go). Source blob IDs retained locally: `c1a9ae4a20a019133f1bac6ef4d5039eda00e5cc`, `f26d8eb8b05a5b78787bcdcc7b6885face1b7699`.
