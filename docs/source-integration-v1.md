# Combined v0.1 source candidate

This branch integrates the bootstrap and verification stacks with accepted main
`96457ee418439dde097339cfcd374c08f2cc98ad`. It is a review candidate. Full ticket
acceptance, production trust configuration and release rehearsals remain open.

The bootstrap/upgrade input is `e184062d61a73a72e7d7fefdedb65523bc138820`.
The strict consumer and Apple input is
`6c0c3d192360b9dc983533f427babe323eb0baeb`. The preview documentation input is
`29c8e6fadf7f2c4e98eb67cbae42938e4e8b2b16`. Existing immutable catalog pins,
trusted roots, schema bytes and historical receipts keep their original meanings.

The combined CLI exposes these independently approved paths:

| Command | Contract and runbook |
| --- | --- |
| `check`, `plan`, `plan --preview`, `preview`, `apply`, `recover` | [Version-one preview and toolchain transaction](apply.md) |
| `catalog` | [Embedded bootstrap authority](bootstrap-catalog.md) |
| `bootstrap check/plan/apply/recover` | [Explicit five-file provisioning](bootstrap-v2.md) |
| `upgrade plan/apply/recover/rollback-plan/rollback-apply` | [Reviewed migration and reversal](upgrades.md) |
| `verify-release` | [Context-first complete-release verification](verify-release-cli.md) |
| `verify-historical-bytes` | [Explicit historical byte comparison](historical-verification.md) |
| `schema` | [All current schema generation](../CONTRIBUTING.md) |

Plans and verification are separate commands. A saved plan digest authorizes only
the fixed local transaction it describes. A historical byte match establishes no
signature authentication. Complete-release verification still requires an
independently approved context, native verifier and roots, and supported macOS
verification for required Apple executables. Authentication errors never select a
different context version or historical fallback.

The combined Development workflow retains Linux x64, Linux ARM64 and macOS,
strict Rust checks, all 23 byte-matched schemas, every profile/schema example,
authenticated workflow audits, genuine native Sigstore and CycloneDX integrations,
Apple native checks and unprivileged runtime candidate retention. The auxiliary
workflow-tools checkout is moved outside the candidate before runtime staging.
No signing, attestation or publication permission is introduced.

The integrated CLI regression uses all three profile examples with build-script
traps. It checks readonly previews, wrong-digest rejection, five-file bootstrap,
identical replay, upgrade ownership migration and exact approved reversal. Bespoke
workflow bytes survive, and no consuming build, target directory or Cargo.lock is
created. Context-first verification failures also leave consuming bytes unchanged.
This same-catalog migration is not evidence for a second accepted catalog upgrade.

Native runtime now has [ADR 0011](adr/0011-common-runtime-and-native-tool-members.md).
Bootstrap retains ADR 0009 and upgrades ADR 0010. Their proposal status is preserved.

Protected Apple transformation, own complete signed producer/consumer evidence,
effective publication approvals and immutable draft transitions, race/retry
qualification, both gated pilots and operational acceptance remain required.
Configured, CI verified, release rehearsed, published and provenance verified
remain distinct. This integration grants no SLSA level or release authority.
