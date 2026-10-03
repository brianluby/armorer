# Name what the workspace must deliver

One workspace can forge several deliverables. Name each one. Select its package,
target and features. Keep its evidence separate. **This is the way.**

This guide covers local inspection at main source
`5e8e2b7fd84a4b6a39650278ae14dbc7cae0afce`. Full adoption, packaging and release
acceptance remain separate gates in [the development map](development-status.md).

## Choose a profile

| Profile | Required selection | Scope |
| --- | --- | --- |
| `library` | Package with a library target; omit `binary` | Library source-package intent; no Developer ID executable requirement |
| `cli` | Package and explicit binary target | CLI executable intent |
| `service` | Package and explicit binary target | Service executable intent; no deployment, container or service-manager setup |

Cargo package names identify `package`; a filesystem directory name is not a
substitute. `binary` names the Cargo binary target, which can differ from the
package name. Profiles do not select packages automatically. Use the workspace
root `Cargo.toml`; nested members may inherit version, edition, license and MSRV.
The configured compiler must meet each selected package's declared MSRV.

Every deliverable needs a unique `id`, at least one supported target, and one
named feature set. Repeating the same package/binary/target/feature-set selection
under another ID is rejected. Separate IDs can select distinct feature cases.

## Inspect all three

Build Armorer as described in [onboarding](onboarding.md), then run this block
from the Armorer checkout in a POSIX shell:

```sh
armorer_binary="$PWD/target/debug/armorer"
armorer_profile_root="$(mktemp -d /tmp/armorer-profiles.XXXXXX)" || exit 1
cp -R examples/profile-workspace/. "$armorer_profile_root/"
"$armorer_binary" --repository "$armorer_profile_root" plan \
  > "$armorer_profile_root-plan.json"
cat "$armorer_profile_root-plan.json"
"$armorer_binary" --repository "$armorer_profile_root" check
```

On the owner's workstation, feed the block to `rtk proxy sh`. Expect two packages,
three deliverables and four target selections. The library uses Linux x64; the
CLI names Linux x64 and macOS ARM64; the service uses Linux ARM64. These are
metadata selections even when the host has a different architecture.

`plan` exits 0 with `state: configuration-valid` and proposes the toolchain file.
`check` exits 2. Findings are `cargo-lock-missing`, `reviewed-lock-missing`,
`license-policy-unreviewed`, `github-capabilities-unchecked` and
`release-runtime-unavailable`. License presence alone cannot validate dependency
licenses. Discovery creates no consumer Cargo.lock, target output or build.

Read the declared selection. Keep the missing proof visible.
**This is the way.**

## Make features explicit

Read [the example config](../examples/profile-workspace/armorer.toml) alongside
[the tools manifest](../examples/profile-workspace/crates/tools/Cargo.toml).
The standard case enables package defaults and lists no additional features.
The daemon case disables defaults and explicitly selects `service`. Cargo's local
feature edge `service = ["store"]` is considered while checking the binary's
`required-features = ["service"]`.

Selected features must be declared by the selected package. Shared feature-set
names do not cause all workspace packages to acquire those features. Discovery
checks local feature/default edges, including cycles, but does not resolve
dependency feature unions or target-specific dependency closure. Complete Cargo
graph and SBOM checks belong to the separate consumer implementation.

To see a rejected selection, change only the copied fixture's daemon feature
list from `["service"]` to `[]`, keeping defaults false, and rerun `plan`.
Expect exit 1 with `invalid-config`: the selected binary's required features are
disabled. Restore the fixture selection and inspect again. Do not weaken required
features to turn a missing runtime dependency into a passing gate.

## Bring an existing project

Keep an isolated adoption branch. Inventory bespoke workflows, Cargo configuration,
toolchain files, license policy and uncommitted work before adding `armorer.toml`.
Choose the actual deliverables and review [platform prerequisites](platform-prerequisites.md).
Run read-only `plan` and `preview`; compare the exact proposed bytes with your
existing toolchain. Discovery excludes repository `.cargo` configuration and
reports that exclusion. A successful metadata snapshot does not prove those
overrides are safe or that the real build works.

On current main, differing unowned toolchain files and legacy `rust-toolchain`
produce conflicts. Identical files remain unowned. There is no force/adopt flag.
Preserve the custom bytes, resolve the desired toolchain deliberately, then review
a fresh plan. Follow [apply and recovery](apply.md) for the toolchain-only change.
Use [bootstrap](bootstrap-v2.md) for five-file provisioning and
[reviewed upgrades](upgrades.md) for explicit managed-file imports and migration.
Two-accepted-catalog migration and full release adoption remain operational gates.

Preserve the craft already in the repository. **This is the way.**
