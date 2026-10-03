# Know which armor fits

Keep the runtime, contracts and workflow pins together. Reject a mismatch before
changing the repository. **This is the way.**

This is the compatibility boundary of main source
`96457ee418439dde097339cfcd374c08f2cc98ad`, not a promise of long-term support.
Contracts remain experimental before the v0.1 release. Cargo's package metadata
currently names Armorer `0.1.0`; that version string establishes no published
release, platform qualification or SLSA achievement.

## Current local contracts

| Input | Accepted boundary |
| --- | --- |
| `armorer.toml` | `schema_version = 1`; unknown fields rejected; workspace-root `Cargo.toml` |
| `armorer.lock` | Version 1, exact config-byte SHA-256, exact runtime-version equality, full workflow SHA and exact tool versions/digests |
| Saved plan | Version 1 and exact runtime; canonical typed digest and all current preimages must match |
| Ownership state/journal | Current schema/runtime and approved plan bindings; incompatible or malformed state blocks mutation |
| Trust documents | Version 1 structural and semantic validation; unknown fields/versions fail; independent expected context required |

Editing a TOML comment changes the config digest. The lock must bind the reviewed
new bytes. A saved plan must be regenerated and reviewed after relevant input,
ownership or intent changes. Reformatting plan JSON does not alter its typed
digest, but changes to fields do. Duplicate JSON keys are rejected. Editing the
stored digest is not a migration or approval.

All 14 CLI schema names are in [the command reference](cli-reference.md).
Schemas establish structure. Semantic validation, upstream authentication and
hosted execution have separate gates.

## Targets and hosts

| Target selection | Current config support | Native evidence-slot verifier host |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | Library/CLI/service metadata selection | Linux x86_64 qualified `gh` identity |
| `aarch64-unknown-linux-gnu` | Library/CLI/service metadata selection | Linux ARM64 qualified `gh` identity |
| `aarch64-apple-darwin` | Library/CLI/service metadata selection | macOS ARM64 qualified `gh` identity |

These columns describe different checks. Discovery can inspect a supported target
on another host without cross-compiling. The verifier chooses its native binary
identity from its **host**, not the artifact target. Unsupported hosts are rejected
by that adapter. Initial local transaction validation is on Linux and macOS;
Windows, musl and other release targets have no accepted adapter in this baseline.
Installed target components and a passing native build remain separate evidence.

Selected package MSRV must not exceed `toolchain`. The CLI's own development
toolchain is Rust 1.95.0. Check/plan use the consuming config's exact installed
stable `x.y.z` toolchain and do not install a missing one.

Select the target. Prove the build on its own host. **This is the way.**

## Keep both version streams visible

Armorer and `armorer-workflows` are versioned independently. The embedded
[bootstrap catalog](bootstrap-catalog.md) binds one reviewed workflow commit and
target-qualified tool distributions. Its renderer supplies locks and fixed
unprivileged caller text; current main does not transactionally adopt those files.
A syntactically valid lock does not prove that every supplied pin belongs to the
catalog or that its upstream distribution was authenticated.

The release trust catalog is a different document. Independent policy controls
roots, verifier bytes and accepted identities. A downloaded release cannot choose
its own catalog, version or historical exception. The
[offline verifier](offline-verifier.md) accepts only its compiled native `gh`
2.102.0 pins in addition to independent policy binding.

## Review an upgrade deliberately

Current main has no upgrade or downgrade CLI. The proposed bootstrap/upgrade and
native-context v2 interfaces are linked at exact commits in
[the development map](development-status.md). They do not reinterpret a v1
toolchain plan as permission for multi-file mutation.

Before adopting a future version, review its compatibility/catalog changes,
owned generated bases, exact previews and recovery path. Preserve the original
lock, state, plan and evidence. Do not bypass version rejection by manually
changing `schema_version` or `runtime_version`. Incompatible trust meanings need
explicit versioned reader migration; an optional field cannot silently disable
a required gate. Rehearsal policy cannot authorize publication.

No supported-version/deprecation window is promised yet. A future released
support policy needs maintainer acceptance and actual platform evidence.
Keep the experimental boundary explicit. **This is the way.**
