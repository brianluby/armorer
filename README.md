# Armorer

This branch is the [combined v0.1 source candidate](docs/source-integration-v1.md),
retaining the bootstrap, upgrade and strict verification interfaces. Operational
release acceptance remains incomplete.

Repeatable secure CI and verifiable releases for Rust repositories.

**Early development:** read-only `check`/`plan` validate explicit Cargo workspace selections. Digest-approved v1 `apply` configures only the pinned toolchain. The explicit version-two `bootstrap` path previews and transactionally provisions all five local CI/build files using reviewed catalog pins and owner-selected policy, with exact recovery. The separate reviewed `upgrade` path migrates stored generated bases, preserves supported customizations, imports individually reviewed targets and supports exact historical rollback. Migration between two accepted catalogs, final-byte verification, protected signing and publication require later integration. No SLSA level is claimed by this slice.

The public MIT repositories are [armorer](https://github.com/brianluby/armorer) for the Rust CLI and [armorer-workflows](https://github.com/brianluby/armorer-workflows) for separately versioned trusted reusable workflows.

Install Rust 1.95.0 through rustup and build with `cargo build --locked`. Author `armorer.toml` in the consuming repository using [the CLI example](examples/armorer.toml), replacing repository, package and binary identities. Library deliverables omit `binary`; CLI/service deliverables require it. Targets and feature sets are explicit.

```sh
armorer --repository /path/to/workspace plan
armorer --repository /path/to/workspace check
armorer schema config
```

From this checkout use `cargo run --locked -- --repository /path/to/workspace plan`. JSON goes to stdout. `plan` exits 0 for a valid preview, even with unmet requirements. `check` exits 2 when setup requirements remain; this development slice always reports missing release runtime and unverified capabilities. Invalid input/discovery exits 1 with a JSON error. Neither command writes consuming files or contacts GitHub. CLI argument errors follow Clap's conventional stderr output.

The explicit [complete release verifier](docs/verify-release-cli.md) authenticates
downloaded inventories, every required bundle, exact assets and paired Cargo
SBOM/graph evidence against an independently approved context. It uses pinned
offline signature/SBOM adapters. Apple executables additionally require macOS
[native signature and notarization checks](docs/apple-native-verification-v1.md),
whose ticket lookup can use the system cache or network. Verification grants no
publication or release-level claim.

See [onboarding and boundaries](docs/onboarding.md), [reviewed apply and recovery](docs/apply.md), [version-one contracts](docs/contracts-v1.md), and [contributing](CONTRIBUTING.md).

- [Architecture](ARCHITECTURE.md): Rust CLI, separately versioned trusted workflows, TOML configuration, preservation of customization, final-byte evidence and independent verification.
- [Implementation plan](IMPLEMENTATION_PLAN.md): reviewable slices, owners, estimates, failure tests, pilot adoption and decisions.
- [Research](RESEARCH.md): primary-source capabilities and additional supply-chain tooling.
- [Supply-chain assessment](docs/supply-chain-assessment.md): prioritized optional defenses, eligibility/maintenance policy, bounded dist and crates.io adapters, and inputs to the release contracts ([ADR 0005](docs/adr/0005-additional-defenses-and-adapters.md), proposed for review).
- [Accepted design ADRs](docs/adr/0001-repositories-language-and-configuration.md): repository/language/configuration, [attestations and claims](docs/adr/0002-platform-attestations-and-level-claims.md), and [publication/migration](docs/adr/0003-fail-closed-publication-and-migration.md).
- [Veans epic #1](https://kanban.luby.us/tasks/1304): scoped baseline and optional follow-up tickets on [project 16](https://kanban.luby.us/projects/16/61).

Confirmed: MIT license, fail-closed secure releases, and Momus/Rusty Brain pilots. v0.1 targets SLSA Build L2. Linux release assets will be authenticated through signed GitHub/Sigstore attestations; macOS additionally requires Developer ID signing and notarization before final-byte attestation. L3 assessment and gap closure are deferred to an unscheduled future backlog item. Additional security controls complement provenance.

Inspired by the Armorer's craft: forge protection into the tools projects begin with.

`armorer catalog` exposes the reviewed bootstrap tool/workflow authority without
network access or consuming files. See [catalog source and limits](docs/bootstrap-catalog.md).
`armorer plan --preview` (also `armorer preview`) shows exact current/proposed bytes and complete diffs while
retaining the approval-bound v1 plan. See [review and apply](docs/apply.md) and
the [v0.1 acceptance ledger](docs/v01-acceptance.md) for delivered scope and gates.

Use [version-two bootstrap and recovery](docs/bootstrap-v2.md) with an explicit reviewed policy to provision the catalog-backed files.

See [reviewed upgrades, imports and rollback](docs/upgrades.md) for the separately versioned migration contracts and their operational limits.
