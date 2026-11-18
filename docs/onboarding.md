# Your first inspection

Bring a Rust workspace. Name what it must deliver. Inspect it before you change it.

This guide covers current main. Start with the installed rustup toolchain and
Armorer binary described in [the README](../README.md). Open PR features are
marked in [the development map](development-status.md).

**This is the way.**

## Initialize

A new workspace has no `armorer.toml`. Let `init` propose one, with an unreviewed
CI policy draft, from the same read-only discovery that inspection uses:

```sh
armorer --repository /path/to/workspace init --repo-name owner/name \
  --toolchain 1.95.0 > /path/outside-workspace/init-proposal.json
```

`--repo-name` is the GitHub `owner/name`. Omit `--toolchain` only when the
workspace's `rust-toolchain.toml` pins an exact `x.y.z` channel; Armorer never
guesses `stable` or its own toolchain. The proposal changes no file. It lists
`files[]` (`armorer-policy.draft.toml` and `armorer.toml`, exact content and
SHA-256), the discovered `workspace`, `findings` and `proposal_sha256`.

Read what it proposes. Each binary target that default features enable becomes a
`cli` deliverable named after the binary; a package with a library and no binary
becomes a `library` deliverable. Every supported target and one `default` feature
set are selected. Then create both files with the digest you reviewed. The write
recomputes the original proposal, so edit the created `armorer.toml` afterwards,
not the saved JSON:

```sh
armorer --repository /path/to/workspace init --repo-name owner/name \
  --toolchain 1.95.0 --write --expect-proposal-sha256 REVIEWED_PROPOSAL_SHA256
```

The write recomputes the proposal; a changed workspace needs a fresh review. It
creates the draft, then `armorer.toml`, and never overwrites either. Once
`armorer.toml` exists, `init` refuses to run. Now edit the created config where your
intent differs: prune targets you do not ship and change a long-running binary to
`service`. Complete the draft: list the dependency licenses you accept in
`licenses.allow` and review every other section. `bootstrap plan` rejects the
draft until `licenses.allow` names at least one license. The receipt keeps
`configured` and `policy_reviewed` false: these files are owner input, not
provisioning.

Your workspace, your decisions. **This is the way.**

## Train on a small workspace

Run this from the Armorer checkout in a POSIX shell. It creates a new temporary
training workspace and saves a plan outside it. The package has no external
dependencies. Inspection does not build or execute it.

```sh
armorer_binary="$PWD/target/debug/armorer"
armorer_training_root="$(mktemp -d /tmp/armorer-training.XXXXXX)" || exit 1
mkdir "$armorer_training_root/src" || exit 1
cat > "$armorer_training_root/Cargo.toml" <<'TOML' || exit 1
[package]
name = "training-cli"
version = "0.1.0"
edition = "2024"
TOML
cat > "$armorer_training_root/src/main.rs" <<'RUST' || exit 1
fn main() { println!("This is the way."); }
RUST
cat > "$armorer_training_root/armorer.toml" <<'TOML' || exit 1
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
"$armorer_binary" --repository "$armorer_training_root" bootstrap plan \
  --policy examples/bootstrap-v2/cli/ci-policy.reviewed.toml \
  > "$armorer_training_root-plan.json" || exit 1
cat "$armorer_training_root-plan.json" || exit 1
"$armorer_binary" --repository "$armorer_training_root" check
```

The owner's workstation prefixes shell commands with `rtk`. For this multi-line
exercise, use `rtk proxy sh` with the block as its input.

The Linux target is an explicit metadata selection, including on a macOS host.
Expect `capability_state: unknown` and proposed `create` changes for the five
managed files, including `rust-toolchain.toml`. Expect missing Cargo.lock and
license findings, unchecked GitHub capabilities and unavailable release runtime.
`bootstrap plan` exits 0; top-level `check` fails closed with a JSON error because
the training root has no managed `.armorer/ci-policy.toml` yet. These are the
intended results.

Inspect `changes`, `findings`, input digests and `plan_sha256`. The digest identifies
intent. Approval still requires your review. The training source stays inert.

**This is the way.**

## Bring your own repository

1. Install rustup and the exact toolchain declared in `armorer.toml`. Armorer never installs consuming tools during check/plan. The CLI builds with Rust 1.95.0.
2. [Initialize](#initialize) configuration and a policy draft, or author it using [the example](../examples/armorer.toml) and [contracts](contracts-v1.md). Either way, the selected packages, binaries, targets and features are your explicit decisions; omit binary for libraries.
3. Run `armorer --repository /path/to/workspace bootstrap plan --policy /path/to/reviewed-policy.toml` and review findings and proposed content. After provisioning, `check` reports drift over the managed policy; it exits 2 while drift remains.
4. Review and retain a plan digest, then use [version-two bootstrap](bootstrap-v2.md) to provision callers, catalog-backed lock and explicit policy. Check/plan stay read-only. Track missing settings, credentials and reviewed pins as setup work; this slice does not contact GitHub, request secrets or configure GitHub settings.

For all three profiles, follow [the complete workspace exercise](profiles.md).
Keep [compatibility](compatibility.md), [platform prerequisites](platform-prerequisites.md)
and [troubleshooting](troubleshooting.md) close when bringing an existing project.

## Know what discovery sees

Armorer makes a temporary snapshot of Cargo manifests/locks and empty Rust source paths. It uses installed pinned rustup Cargo/compiler with `cargo metadata --no-deps --offline --locked --format-version 1`, isolated Cargo home and cleared environment. Unix subprocess PATH is limited to the installed toolchain and standard system utility directories. Repository `.cargo`, compiler wrappers and credential providers are excluded. Discovery does not run build scripts, compile source or resolve dependency graphs. Escaping manifest paths and symlinks are rejected. Native `links` and build-script declarations are reported.

This inspects metadata and target discovery, not builds, dependencies, license compliance, platform compatibility or SBOM completeness. Cargo.lock is preserved or reported missing, never generated in the consumer. Missing packages/binaries, unknown features, disabled required features, unsupported targets and selected package MSRV above the compiler fail validation.

The local rustup installation and OS are trusted. This is not an OS sandbox against concurrent adversarial filesystem changes; inspect a stable checkout. Traversal is bounded during enumeration to 20,000 entries and 128 directory levels; regular manifest/lock inputs to 1 MiB each and 8 MiB combined; metadata to 1 MiB; and the metadata process to 15 seconds. Special input files such as FIFOs are rejected before opening. Skipped trees: `.git`, `.worktrees`, `target`, `node_modules`, `.armorer`, `.cargo`. Projects depending on packages there, external workspace/path dependencies or symlinks need migration or a future adapter. Cargo package include/exclude globs and opaque package/workspace metadata are preserved as data; actual Cargo filesystem fields retain containment validation.

## Hold the boundary

Check/plan leave no consumer changes to recover. Apply uses a durable journal and explicit recovery; follow [the recovery procedure](bootstrap-v2.md#ownership-repeatability-and-recovery) after interrupted writes. Review JSON error codes; TOML errors omit input values. The operating-system temporary directory must resolve outside the consuming workspace and have no ancestor `.cargo/config` or `.cargo/config.toml`; unsafe placement is rejected before snapshot creation. Use an external temporary directory with no inherited Cargo configuration when rerunning. Ancestor configuration is checked for presence without opening its contents. For other discovery failures, inspect manifests using trusted installed Cargo in a context you control, correct unsupported selections/paths/toolchains and rerun. Armorer prints no credentials or arbitrary Cargo stderr.

Use [reviewed upgrades](upgrades.md), [complete release verification](verify-release-cli.md) and [owned-draft inspection/recovery](owned-draft-controller-v1.md) for the accepted implementation paths. Protected signing, effective publication prerequisites and complete operational rehearsals remain incomplete. Follow [the readiness path](readiness.md) and [current acceptance ledger](v01-acceptance.md) for their exact gates. Toolchain apply cannot authorize releases or claim SLSA compliance.

Read [error and finding codes](cli-reference.md) when inspection stops. Name the
limitation. Hold the gate. **This is the way.**

Primary sources: [cargo metadata](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html), [Cargo configuration](https://doc.rust-lang.org/cargo/reference/config.html), [features](https://doc.rust-lang.org/cargo/reference/features.html).
