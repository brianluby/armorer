# A workspace for every profile

Inspect the library. Name the CLI. Declare the service's features.
**This is the way.**

This training workspace contains two packages and three deliverables. Its only
dependency is a local workspace member. The service executable prints a greeting
and exits; it is a profile selection fixture, not a deployed service.
`repository = "example/profile-workspace"` is an example identity, not a live
GitHub repository or release source. No production trust pins are supplied.

Copy it to a temporary directory before inspection. Follow the
[profile walkthrough](../../docs/profiles.md). `plan` should report valid
configuration with two packages and three deliverables. `check` exits 2 because
reviewed pins, dependency policy, platform capabilities and release runtime remain
unproven. No Cargo.lock is shipped; inspection must preserve its absence.

The Linux and macOS targets are metadata selections. Discovery neither installs
targets nor cross-compiles. The daemon case explicitly enables `service`, which
enables the local `store` feature and satisfies the binary's `required-features`.
The standard case enables no service feature.

The source can be built separately for the local host as a contributor check.
That build is not part of check/plan or an Armorer release rehearsal. No package,
container, installation, startup unit or publication is generated here.

Keep the selection's scope with its evidence. **This is the way.**
