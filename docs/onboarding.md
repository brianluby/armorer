# Your first inspection

Bring a Rust workspace. Name what it must deliver. Inspect it before you change it.

This guide covers current main. Start with the installed rustup toolchain and
Armorer binary described in [the README](../README.md). Open PR features are
marked in [the development map](development-status.md).

**This is the way.**

## Train on a small workspace

Run this from the Armorer checkout in a POSIX shell. It creates a new temporary
training workspace and saves a plan outside it. The package has no external
dependencies. Inspection does not build or execute it.

```sh
armorer_binary="$PWD/target/debug/armorer"
armorer_training_root="$(mktemp -d /tmp/armorer-training.XXXXXX)" || exit 1
mkdir "$armorer_training_root/src"
cat > "$armorer_training_root/Cargo.toml" <<'TOML'
[package]
name = "training-cli"
version = "0.1.0"
edition = "2024"
TOML
cat > "$armorer_training_root/src/main.rs" <<'RUST'
fn main() { println!("This is the way."); }
RUST
cat > "$armorer_training_root/armorer.toml" <<'TOML'
schema_version = 1
repository = "example/training-cli"
toolchain = "1.95.0"
manifest = "Cargo.toml"

[[deliverables]]
id = "application"
profile = "cli"
package = "training-cli"
binary = "training-cli"
targets = ["x86_64-unknown-linux-gnu"]
feature_set = "release"

[feature_sets.release]
default_features = true
features = []

[policy]
license_file = "LICENSE"
attestations = "required"
TOML
"$armorer_binary" --repository "$armorer_training_root" plan \
  > "$armorer_training_root-plan.json"
cat "$armorer_training_root-plan.json"
"$armorer_binary" --repository "$armorer_training_root" check
```

The owner's workstation prefixes shell commands with `rtk`. For this multi-line
exercise, use `rtk proxy sh` with the block as its input.

The Linux target is an explicit metadata selection, including on a macOS host.
Expect `state: configuration-valid`, `capability_state: unknown`, and a proposed
`create` change for `rust-toolchain.toml`. Expect missing Cargo.lock, license and
reviewed-pin findings, plus unreviewed license policy, unchecked GitHub capabilities
and unavailable release runtime. `plan` exits 0; `check` exits 2. These are the
intended results.

Inspect `changes`, `findings`, input digests and `plan_sha256`. The digest identifies
intent. Approval still requires your review. The training source stays inert.

**This is the way.**

## Bring your own repository

1. Install rustup and the exact toolchain declared in `armorer.toml`. Armorer never installs consuming tools during check/plan. The CLI builds with Rust 1.95.0.
2. Author configuration using [the example](../examples/armorer.toml) and [contracts](contracts-v1.md). Select packages, binaries, targets and features explicitly; omit binary for libraries.
3. Run `armorer --repository /path/to/workspace plan` and review findings and proposed content. Run `check` for a failing setup gate; it exits 2 while requirements remain.
4. Review and retain a plan digest, then use [v1 transactional apply](apply.md) for toolchain-only setup or [version-two bootstrap](bootstrap-v2.md) to provision callers, catalog-backed lock and explicit policy. Check/plan stay read-only. Track missing settings, credentials and reviewed pins as setup work; this slice does not contact GitHub, request secrets or configure GitHub settings.

For all three profiles, follow [the complete workspace exercise](profiles.md).
Keep [compatibility](compatibility.md), [platform prerequisites](platform-prerequisites.md)
and [troubleshooting](troubleshooting.md) close when bringing an existing project.

## Know what discovery sees

Armorer makes a temporary snapshot of Cargo manifests/locks and empty Rust source paths. It uses installed pinned rustup Cargo/compiler with `cargo metadata --no-deps --offline --locked --format-version 1`, isolated Cargo home and cleared environment. Unix subprocess PATH is limited to the installed toolchain and standard system utility directories. Repository `.cargo`, compiler wrappers and credential providers are excluded. Discovery does not run build scripts, compile source or resolve dependency graphs. Escaping manifest paths and symlinks are rejected. Native `links` and build-script declarations are reported.

This inspects metadata and target discovery, not builds, dependencies, license compliance, platform compatibility or SBOM completeness. Cargo.lock is preserved or reported missing, never generated in the consumer. Missing packages/binaries, unknown features, disabled required features, unsupported targets and selected package MSRV above the compiler fail validation.

The local rustup installation and OS are trusted. This is not an OS sandbox against concurrent adversarial filesystem changes; inspect a stable checkout. Traversal is bounded during enumeration to 20,000 entries and 128 directory levels; regular manifest/lock inputs to 1 MiB each and 8 MiB combined; metadata to 1 MiB; and the metadata process to 15 seconds. Special input files such as FIFOs are rejected before opening. Skipped trees: `.git`, `.worktrees`, `target`, `node_modules`, `.armorer`, `.cargo`. Projects depending on packages there, external workspace/path dependencies or symlinks need migration or a future adapter. Cargo package include/exclude globs and opaque package/workspace metadata are preserved as data; actual Cargo filesystem fields retain containment validation.

## Hold the boundary

Check/plan leave no consumer changes to recover. Apply uses a durable journal and explicit recovery; follow [the recovery procedure](apply.md) after interrupted writes. Review JSON error codes; TOML errors omit input values. The operating-system temporary directory must resolve outside the consuming workspace and have no ancestor `.cargo/config` or `.cargo/config.toml`; unsafe placement is rejected before snapshot creation. Use an external temporary directory with no inherited Cargo configuration when rerunning. Ancestor configuration is checked for presence without opening its contents. For other discovery failures, inspect manifests using trusted installed Cargo in a context you control, correct unsupported selections/paths/toolchains and rerun. Armorer prints no credentials or arbitrary Cargo stderr.

Use [reviewed upgrades](upgrades.md), [complete release verification](verify-release-cli.md) and [owned-draft inspection/recovery](owned-draft-controller-v1.md) for the accepted implementation paths. Protected signing, effective publication prerequisites and complete operational rehearsals remain incomplete. Follow [the readiness path](readiness.md) and [current acceptance ledger](v01-acceptance.md) for their exact gates. Toolchain apply cannot authorize releases or claim SLSA compliance.

Read [error and finding codes](cli-reference.md) when inspection stops. Name the
limitation. Hold the gate. **This is the way.**

Primary sources: [cargo metadata](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html), [Cargo configuration](https://doc.rust-lang.org/cargo/reference/config.html), [features](https://doc.rust-lang.org/cargo/reference/features.html).
