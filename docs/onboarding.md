# Onboarding and discovery

1. Install rustup and the exact toolchain declared in `armorer.toml`. Armorer never installs consuming tools during check/plan. The CLI builds with Rust 1.95.0.
2. Author configuration using [the example](../examples/armorer.toml) and [contracts](contracts-v1.md). Select packages, binaries, targets and features explicitly; omit binary for libraries.
3. Run `armorer --repository /path/to/workspace plan` and review findings and proposed content. Run `check` for a failing setup gate; it exits 2 while requirements remain.
4. Review and retain a plan digest, then use [v1 transactional apply](apply.md) for toolchain-only setup or [version-two bootstrap](bootstrap-v2.md) to provision callers, catalog-backed lock and explicit policy. Check/plan stay read-only. Track missing settings, credentials and reviewed pins as setup work; this slice does not contact GitHub, request secrets or configure GitHub settings.

## Discovery boundary

Armorer makes a temporary snapshot of Cargo manifests/locks and empty Rust source paths. It uses installed pinned rustup Cargo/compiler with `cargo metadata --no-deps --offline --locked --format-version 1`, isolated Cargo home and cleared environment. Unix subprocess PATH is limited to the installed toolchain and standard system utility directories. Repository `.cargo`, compiler wrappers and credential providers are excluded. Discovery does not run build scripts, compile source or resolve dependency graphs. Escaping manifest paths and symlinks are rejected. Native `links` and build-script declarations are reported.

This inspects metadata and target discovery, not builds, dependencies, license compliance, platform compatibility or SBOM completeness. Cargo.lock is preserved or reported missing, never generated in the consumer. Missing packages/binaries, unknown features, disabled required features, unsupported targets and selected package MSRV above the compiler fail validation.

The local rustup installation and OS are trusted. This is not an OS sandbox against concurrent adversarial filesystem changes; inspect a stable checkout. Traversal is bounded during enumeration to 20,000 entries and 128 directory levels; regular manifest/lock inputs to 1 MiB each and 8 MiB combined; metadata to 1 MiB; and the metadata process to 15 seconds. Special input files such as FIFOs are rejected before opening. Skipped trees: `.git`, `.worktrees`, `target`, `node_modules`, `.armorer`, `.cargo`. Projects depending on packages there, external workspace/path dependencies or symlinks need migration or a future adapter. Cargo package include/exclude globs and opaque package/workspace metadata are preserved as data; actual Cargo filesystem fields retain containment validation.

## Recovery and next gates

Check/plan leave no consumer changes to recover. Apply uses a durable journal and explicit recovery; follow [the recovery procedure](apply.md) after interrupted writes. Review JSON error codes; TOML errors omit input values. The operating-system temporary directory must resolve outside the consuming workspace and have no ancestor `.cargo/config` or `.cargo/config.toml`; unsafe placement is rejected before snapshot creation. Use an external temporary directory with no inherited Cargo configuration when rerunning. Ancestor configuration is checked for presence without opening its contents. For other discovery failures, inspect manifests using trusted installed Cargo in a context you control, correct unsupported selections/paths/toolchains and rerun. Armorer prints no credentials or arbitrary Cargo stderr.

Workflow/tool upgrades, authenticated GitHub checks, signing credential-name setup, hosted rehearsals, release verification and failed-draft recovery belong to later slices and require operational documentation before publication. Toolchain apply cannot authorize releases or claim SLSA compliance.

Primary sources: [cargo metadata](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html), [Cargo configuration](https://doc.rust-lang.org/cargo/reference/config.html), [features](https://doc.rust-lang.org/cargo/reference/features.html).
