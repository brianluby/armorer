# Armorer

Forge the setup. Keep the evidence. Earn the release.

Armorer is building repeatable secure CI and verifiable releases for Rust repositories. Start with explicit intent. Preserve customizations. Hold the gate when required evidence is missing. A release must stand on its own proof.

**This is the way.**

## What stands today

Read-only `check` and `plan` validate explicit Cargo workspace selections; `preview` shows exact before/after text and complete diffs. Version-one `apply` manages the toolchain. The accepted version-two `bootstrap` path provisions all five local CI/build files from reviewed catalog pins and owner-selected policy. `upgrade` preserves stored generated bases, exposes conflicts and supports explicit imports and rollback. The strict complete-release consumer verifies independent context, authenticated inventories, exact assets and paired SBOM/graph evidence. Native Apple executable verification requires macOS. Follow [the source map](docs/development-status.md) and [acceptance ledger](docs/v01-acceptance.md) for accepted heads and operational gates.

**v0.1 targets SLSA Build L2. Current main claims no SLSA level.** L3 belongs to a future-version backlog item. Configuration, hosted CI, release rehearsal, publication and verified provenance each need separate evidence.

## Enter the forge

Start from a reviewed checkout with Rust 1.95.0 installed through rustup:

```sh
rustup run 1.95.0 cargo build --locked
./target/debug/armorer --help
./target/debug/armorer schema config
```

The owner's workstation prefixes shell commands with `rtk`; use `rtk proxy rustup run 1.95.0 cargo build --locked` there. If an unavailable compiler wrapper blocks the build, follow [the contributor setup](CONTRIBUTING.md).

Bring a consuming workspace. Author `armorer.toml` using [the CLI example](examples/armorer.toml). Name the repository, packages, binaries, targets and feature sets explicitly. Libraries omit `binary`; CLI and service deliverables require it.

```sh
./target/debug/armorer --repository /path/to/workspace plan
./target/debug/armorer --repository /path/to/workspace plan --preview
./target/debug/armorer --repository /path/to/workspace check
```

Read the JSON. `plan` exits 0 for valid inspection, including unmet setup requirements. `check` exits 2 for every valid inspection in this development version because release readiness remains blocked. Invalid input/discovery exits 1 with a JSON error. Argument errors use Clap's stderr diagnostics. Check and plan preserve consuming files, contact no GitHub service, and run no repository build scripts.

Follow [your first inspection](docs/onboarding.md) for a complete local exercise. Before changing a real project, read [reviewed apply and recovery](docs/apply.md). Inspect the plan. Retain the approved digest. Apply the exact intent.

**This is the way.**

## Choose your path

| Your task | Your guide |
| --- | --- |
| Inspect a Rust workspace | [Onboarding](docs/onboarding.md) |
| Select a library, CLI or service | [Profiles and workspace walkthrough](docs/profiles.md) |
| Find commands, exit statuses and finding codes | [CLI reference](docs/cli-reference.md) |
| Resolve a stopped inspection or transaction | [Troubleshooting](docs/troubleshooting.md) |
| Check contract and platform support | [Compatibility](docs/compatibility.md), [platform prerequisites](docs/platform-prerequisites.md) |
| Apply a plan or recover a transaction | [Apply and recovery](docs/apply.md) |
| Inspect the bootstrap authority | [Catalog source and limits](docs/bootstrap-catalog.md) |
| Verify an individual attestation through the library | [Offline verifier](docs/offline-verifier.md) |
| Understand configuration, locks and plan digests | [Version-one contracts](docs/contracts-v1.md) |
| Understand release trust and evidence | [Trust contracts](docs/trust-contracts-v1.md) |
| Provision all five local files | [Bootstrap and recovery](docs/bootstrap-v2.md) |
| Review upgrades, imports and rollback | [Upgrades](docs/upgrades.md) |
| Verify complete or historical release bytes | [Complete verifier](docs/verify-release-cli.md), [explicit historical policy](docs/historical-verification.md) |
| Inspect owned-draft publication gates | [Controller](docs/owned-draft-controller-v1.md) |
| Review accepted source and remaining work | [Development map](docs/development-status.md) |
| Contribute code or documentation | [Contributing](CONTRIBUTING.md), [writing creed](docs/writing-guide.md) |

The [documentation index](docs/README.md) also leads to architecture, ADRs, research, the [v0.1 acceptance ledger](docs/v01-acceptance.md) and retained validation records.

## The clan's work

The public MIT repositories are [armorer](https://github.com/brianluby/armorer) for the Rust CLI and [armorer-workflows](https://github.com/brianluby/armorer-workflows) for separately versioned trusted reusable workflows. Momus and Rusty Brain are planned pilots.

Work is tracked in [Veans project 16](https://kanban.luby.us/projects/16/61) under [epic #1](https://kanban.luby.us/tasks/1304). Merges and release publication require explicit human authorization. Keep credential values with their owner. Build protection into the tools. Make every claim answer to evidence.

**This is the way.**
