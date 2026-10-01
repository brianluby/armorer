# Armorer

Repeatable secure CI and verifiable releases for Rust repositories.

This workspace currently contains an architecture proposal and scoped implementation plan. There is no implemented CLI or provisioned upstream repository yet.

- [Architecture](ARCHITECTURE.md): Rust CLI, separately versioned trusted workflows, TOML configuration, preservation of customization, final-byte evidence and independent verification.
- [Implementation plan](IMPLEMENTATION_PLAN.md): reviewable slices, owners, estimates, failure tests, pilot adoption and decisions.
- [Research](RESEARCH.md): primary-source capabilities and additional supply-chain tooling.
- [Proposed ADRs](docs/adr/0001-repositories-language-and-configuration.md): repository/language/configuration, [attestations and claims](docs/adr/0002-platform-attestations-and-level-claims.md), and [publication/migration](docs/adr/0003-fail-closed-publication-and-migration.md).
- [Veans epic #1](https://kanban.luby.us/tasks/1304): fifteen scoped child tickets on [project 16](https://kanban.luby.us/projects/16/61).

Confirmed: brianluby/armorer and brianluby/armorer-workflows are the proposed public upstreams; MIT license; fail-closed secure releases; Momus and Rusty Brain pilots. v0.1 targets SLSA Build L2. L3 assessment and gap closure are deferred to the backlog for an unscheduled future version and do not block initial delivery. Additional security controls complement provenance.

Inspired by the Armorer's craft: forge protection into the tools projects begin with.
