# Armorer review: architecture, security, and roadmap — 2026-11-16

Reviewed at `caf7b89` (branch `claude/rust-setup-architecture-review-c722d8`).
This is guidance for continued development, not a release qualification.

## Follow-up status (same branch, uncommitted as of 2026-11-16)

- **S1 addressed locally:** `rustix` 1.1.6; Armorer bootstrapped onto itself
  (`armorer.toml`, `armorer.lock`, `.armorer/`, `armorer-ci.yml`,
  `armorer-build.yml`). The hosted workflow has not run yet.
- **S2 addressed locally:** `SECURITY.md`, `.github/CODEOWNERS`,
  `.github/dependabot.yml`. Private vulnerability reporting and Dependabot
  security updates still need enabling in repository settings.
- **S3 addressed locally:** locks and ownership state follow a patch-line rule
  (`src/runtime.rs`); catalogs record the frozen contract `0.1.0`. Release
  contexts and the native catalog accept an approved runtime that is a later
  patch of the locked contract. Plans and
  journals stay exact. Committed example packets must be regenerated at each
  version bump.
- **S7 addressed locally for `bootstrap check`:** exit 0 when provisioning is in
  sync, exit 2 with reasons on stderr for drift, and `--release-ready` for the
  strict release gate. Top-level version-one `check` still always exits 2; it
  is scheduled for replacement under ADR 0012.
- **A4 decided:** the owner accepted removal of superseded contracts before
  0.1.0; see [ADR 0012](../docs/adr/0012-pre-release-contract-consolidation.md).
  The removals themselves are not implemented.

## Scope and confidence

| Area | Depth |
| --- | --- |
| `lib.rs`, `main.rs`, `config.rs`, `discovery.rs`, `catalog.rs`, `plan.rs`, `ci_policy.rs`, `filesystem.rs`, `bootstrap/plan.rs`, `.github/workflows/development.yml` | Read in full |
| `apply.rs`, `bootstrap/transaction.rs`, `upgrade/*` | Outline, lock/replace/rollback paths, constants |
| `verification/*`, `trust/*` (about 9,000 lines) | Subprocess call sites and credential handling only |
| `armorer-workflows` repository | Not reviewed (separate repository) |

The verification and trust code was line-reviewed two days ago (`docs/ocr-20261114/`),
with residuals tracked as Veans #27. I did not repeat that work, so this report
makes no new claim about the cryptographic verification paths.

Checks run locally (macOS arm64, Rust 1.95.0):

- `cargo test --locked`: 282 passed, 0 failed, 10 ignored.
- `cargo clippy --locked --all-targets`: no warnings.
- `cargo audit`: one advisory, RUSTSEC-2026-0486 (`rustix` 1.1.5, unsound xattr functions).
- `cargo deny check`: advisories and licenses fail. The licenses failure is because the repository has no `deny.toml`.

## Summary

The security engineering inside the tool is strong. Inputs are bounded and
hash-bound, unknown fields are rejected, subprocesses run with a cleared
environment and fixed arguments, writes are journaled, and nothing claims more
than its evidence supports.

The main risks are elsewhere:

1. **The product does not yet do what its purpose statement says.** There is no
   `init`. A new user must hand-write `armorer.toml` and a CI policy file before
   anything works, and the generated file set stops at CI callers, a lock, and a
   toolchain file.
2. **The codebase is frozen around Rust and around a fixed five-file set.** Go
   and Python cannot be added without first introducing an ecosystem boundary
   and a general file-set transaction engine.
3. **Contract surface is far ahead of adoption.** There are 12 commands, 27
   schemas, three verification-context versions, two bootstrap catalogs, and
   three transaction engines, with no external consumer and no tagged release.
4. **Armorer does not run its own gates.** Its CI has no dependency audit,
   license policy, or secret scan, and a live advisory is sitting in `Cargo.lock`.

## Security findings

Ordered by priority. None is a confirmed exploitable bypass.

### S1. Armorer's own CI omits the gates it provisions for consumers (high)

`.github/workflows/development.yml` runs fmt, clippy, tests, actionlint, and
zizmor. It does not run cargo-deny, an advisory check, or gitleaks, and the
repository has no `deny.toml`. The result is visible today: `rustix` 1.1.5
carries RUSTSEC-2026-0486 and nothing flagged it.

The advisory itself is not reachable. Armorer calls only `rustix::fs::open`
(`src/lib.rs`) and `rustix::process::getuid`. Bump the pin to 1.1.6 anyway.

Recommendation: adopt Armorer's own `armorer-ci.yml` caller and
`.armorer/ci-policy.toml` on this repository. That closes the gap and is the
first real dogfooding evidence.

### S2. Missing repository security baseline (medium)

There is no `SECURITY.md` (a draft sits in `docs/security-reporting-draft.md`),
no `CODEOWNERS` covering `.github/` and `catalogs/`, and no Dependabot or
Renovate configuration. All dependencies and action SHAs are exact-pinned, so
without an update bot they only move when someone remembers.

### S3. Exact runtime-version equality blocks security updates (medium)

`armorer.lock`, plans, and state all require `runtime_version ==
CARGO_PKG_VERSION` (`src/config.rs:354`, `src/apply.rs:237`,
`src/bootstrap/plan.rs:287`). `upgrade` refuses to run at all unless the crate
version is exactly `0.1.0` (`src/upgrade/authority.rs`,
`BOOTSTRAP_RUNTIME_VERSION`).

A 0.1.1 security fix would therefore reject every consumer's existing lock and
state, and the upgrade command that should migrate them would refuse. That makes
patching the tool expensive, which is a security problem in itself. Decide a
compatibility rule (for example, same minor version) before the first tag.

### S4. Pinned tool catalog has no freshness signal (medium)

The catalog is compiled into the binary and hash-authenticated, which is a good
trust design. But six tools (cargo-deny 0.20.2, zizmor 1.30.1, actionlint
1.7.12, gitleaks 8.30.1, cargo-cyclonedx 0.5.9, cyclonedx 0.33.1) can only
change through a CLI release, and S3 makes CLI releases costly. Nothing reports
a pinned tool as old or vulnerable.

Recommendation: record a review date per catalog and add a finding when the
catalog is older than a set window. Script the catalog refresh so it is a
routine reviewed PR.

### S5. Hand-written parsers have no fuzz or property tests (medium)

The duplicate-key-rejecting JSON walker (`src/verification/io.rs`), the HTTP
response parser (`src/verification/publication/github.rs`), the unified-diff
renderer (`src/bootstrap/plan.rs`), `safe_path`, and the date parser
(`src/ci_policy.rs`) all handle untrusted bytes and are tested only by examples.
Add `cargo-fuzz` targets for the first two and `proptest` for the three-way
merge decision and diff round-trip.

### S6. Write paths check for symlinks, then act by path (low)

`safe_path` walks components with `symlink_metadata`, and the write follows
later by path. `lock()` (`src/apply.rs:306`) opens `.armorer/apply.lock` with
`create(true)` and no `O_NOFOLLOW`. `replace()` creates its temporary file in a
parent validated earlier. `read_small` already does this correctly with
`O_NOFOLLOW` plus an inode comparison.

Exploiting this needs a concurrent local process with write access to the
repository, so severity is low. The durable fix is to open the repository root
once and do every managed read and write relative to that directory handle
(`openat` with `O_NOFOLLOW`, which `rustix` already provides).

### S7. `check` always exits 2 (low, becomes medium once adopted)

`src/main.rs` returns status 2 for every successful `check`. A consumer cannot
use it as a CI gate or drift detector, and the likely workaround is `|| true`,
which hides real failures. Split the meaning: 0 when managed files match
recorded state, 2 for drift or unmet requirements, 1 for errors.

### S8. Smaller items

- **Generated CI caller cancels runs on `main`.** `cancel-in-progress: true` in
  `armorer-ci.yml` (`src/catalog.rs`) applies to pushes as well as pull
  requests. Two quick merges leave the first commit with no completed CI run.
  Armorer's own workflow already restricts cancellation to pull requests.
- **`trusted_rustup` takes the first `rustup` on `PATH` outside the repository**
  (`src/discovery.rs:110`). It does not check that the directory is not
  world-writable. Acceptable for a developer tool; worth a finding in output.
- **Non-Unix hosts.** Reads fall back to a plain `File::open` without symlink
  protection, while writes refuse to run. Fail at startup with one clear
  message instead.

### What is done well

Keep these as the house rules for any new ecosystem:

- Discovery never executes repository code: manifests are copied to a private
  snapshot, sources become empty placeholders, `.cargo` config is excluded, and
  Cargo runs offline with a cleared environment and a 15-second limit.
- Every mutation requires a plan plus a digest supplied separately.
- Consumer input cannot choose paths, templates, URLs, or commands.
- Error messages never echo configuration values.
- `unsafe_code = "forbid"`, and type-state wrappers such as
  `AuthenticatedCatalog` cannot be constructed without passing verification.

## Architecture findings

### A1. Three transaction engines

`src/apply.rs` (1,025 lines), `src/bootstrap/transaction.rs` (858), and
`src/upgrade/transaction.rs` (1,069) each implement their own journal,
validation, rollback, and recovery. They share only `lock`, `replace`, and a
few helpers. Every fix to recovery logic has to be made and tested three times.

Extract one engine: given a list of `(path, expected-before, after)` writes and
an ownership record, it journals, applies, verifies, and recovers. Bootstrap,
upgrade, and rollback become planners that produce that list. This is the
single most valuable refactor and it is a prerequisite for A2.

### A2. The managed file set is a fixed array

`bootstrap::TARGETS` is `[&str; 5]` (`src/bootstrap/mod.rs:17`) and is referenced
from plan validation, state validation, journal bounds, and upgrade imports.
Plans are rejected unless `changes.len() == TARGETS.len()`. Adding one file
(for example `SECURITY.md`) or a second language touches all of those sites and
changes the plan contract.

Make the file set a property of the catalog entry, validated as "exactly the
set this catalog renders", not a compile-time constant.

### A3. Templates are inline `format!` strings

Workflow YAML is built in `caller_workflows` and the toolchain file is rendered
in two places (`src/catalog.rs` `bootstrap_files`, `src/plan.rs`
`toolchain_change`). Move templates to files under `catalogs/` loaded with
`include_str!`, with a small fixed-placeholder renderer. Reviewers then diff
YAML, not Rust string escapes.

### A4. Too much frozen contract before the first release

The repository carries v1 `apply` (toolchain only) beside v2 `bootstrap`,
`legacy-v1` / `native-v2` / `native-v3` verification contexts, and
`bootstrap-v1` / `bootstrap-v2` catalogs. No consumer depends on the old ones.
Each retained version is permanent test and review cost.

Recommendation: before tagging 0.1.0, remove superseded versions (v1 `apply`,
`recover`, `preview --plan`, and the older contexts) and start the public
contract at one version of each document. This contradicts the "stable IDs are
immutable" rule in `upgrade/authority.rs`, so it needs an explicit ADR. The
rule is right after release and costly before it.

### A5. One crate, one binary, 17,000 lines

Planning, transactions, Cargo discovery, Sigstore, Apple, CycloneDX, and GitHub
publication are all in one library with a 790-line `main.rs` match. Suggested
workspace:

| Crate | Contents |
| --- | --- |
| `armorer-core` | Paths, bounded I/O, digests, transaction engine, plan and ownership model |
| `armorer-rust` | Cargo discovery, Rust templates, Cargo graph checks |
| `armorer-verify` | Sigstore, SBOM, Apple, and historical verification |
| `armorer-publish` | Owned-draft controller and GitHub adapter |
| `armorer` (binary) | CLI wiring only |

A separate `armorer-verify` matters most. Someone verifying a release needs a
small, auditable tool, not the provisioning and publication code.

Also decide whether the library is public API. Exact `=` dependency pins are
right for a binary but make a published library hard to depend on.

### A6. CLI ergonomics

- Output is JSON only. Add a human-readable default with `--format json` for
  automation; the reviewer of a plan is a person.
- `verify-release` needs seven path and digest flags, `publication` needs ten.
  Accept one reviewed manifest file that names them.
- `plan` and `bootstrap plan` are different commands with different contracts.
  After A4 there should be one.

### A7. Repository hygiene

`reviews/` holds 146 tracked files, mostly exit codes and checksums from
rehearsals, and `docs/ocr-20261114/` is a raw review dump. Move rehearsal
evidence to a dedicated branch, repository, or release asset and keep a short
index in `docs/`. The project's own completion plan already notes status drift
across the 72 documents; fewer, generated-where-possible documents would help.

## Functionality gaps against the stated goal

The goal is to set up a project securely from the beginning. Today Armorer
requires an existing Cargo workspace, a hand-written `armorer.toml`, and a
hand-written CI policy, and then writes five files.

| # | Capability | Notes |
| --- | --- | --- |
| F1 | `armorer init` | Run discovery, propose `armorer.toml` and a starter CI policy, and route them through the normal plan and digest approval. The largest adoption blocker. |
| F2 | Wider managed file set | `SECURITY.md`, `CODEOWNERS` for `.github/` and Armorer files, Dependabot or Renovate config for Actions and Cargo, `[lints]` defaults. Depends on A2. |
| F3 | GitHub settings preflight | Read-only `check --online`: rulesets, secret scanning and push protection, default token permissions, immutable releases. Already recorded as the `github-capabilities-unchecked` finding. |
| F4 | Drift detection | Meaningful `check` exit codes (S7) plus a scheduled caller that fails when managed files or pins drift. |
| F5 | Catalog freshness | `armorer outdated` style report (S4). |
| F6 | Local parity | A way to run the same gates locally that CI runs, so policy failures are found before push. |
| F7 | Linux release path | Finish attest, draft, verify, publish for Linux targets first. Apple signing is the longest dependency chain and should not block a first verifiable release. |
| F8 | `armorer new` | Scaffold a new project with the hardened defaults. Lower priority than `init`. |

The optional defenses in Veans #17–#24 (CodeQL, cargo-auditable, cargo-vet,
Scorecard) are correctly deferred.

## Adding Go and Python

### Where Rust is baked in

| Location | Coupling |
| --- | --- |
| `config::Config` | `toolchain` is a Rust version, `manifest` must equal `Cargo.toml`, `TARGETS` are Rust triples, deliverables are package / binary / feature set |
| `plan::Plan`, `bootstrap::Plan` | Embed `discovery::Workspace` (Cargo packages), so the plan digest depends on Cargo's shape |
| `discovery.rs` | Entirely Cargo and rustup |
| `catalog.rs` | Renders `rust-ci.yml`, `rust-build.yml`, `rust-toolchain.toml`; one tool list |
| `bootstrap::TARGETS` | Includes `rust-toolchain.toml` |
| `ci_policy.rs` | Shaped like cargo-deny (licenses, advisories, sources, bans); concepts transfer, the enforcement tool does not |
| `verification/graph.rs`, parts of `trust/evidence.rs` | Cargo dependency graph against CycloneDX |

Already language-neutral: path and I/O guards, transaction mechanics, three-way
merge, Sigstore bundle verification, CycloneDX validation, Apple verification,
the publication controller, and historical byte matching.

### Proposed boundary

Introduce an ecosystem adapter with a narrow interface:

1. `discover(root, intent) -> ProjectModel`: static inspection only, returning a
   neutral model (units, versions, buildable outputs, input digests) plus
   ecosystem-specific detail in a tagged field.
2. `managed_files(intent, catalog) -> file set`: toolchain pin, CI callers,
   policy files.
3. `tools(catalog) -> pins`: the subset of the catalog this ecosystem needs.
4. `validate_sbom(model, sbom)`: the dependency-graph check.

Config becomes schema version 2 with an explicit `ecosystem` and tagged
deliverables; version 1 maps onto it as `ecosystem = "rust"`. Do A1, A2, and A3
first. Without them every adapter would need its own transaction engine and
constants.

Carry the discovery rule over unchanged: **never execute project code or let
the toolchain reach the network during inspection.** That rule decides most of
the design below.

### Go (do this first)

Go maps closely onto the existing CLI and service profiles: static binaries,
cross-compilation, checksummed modules.

- **Discovery.** Parse `go.mod` and `go.work` directly. Avoid `go list` where
  possible: it can download modules and even a different toolchain. If it must
  run, set `GOTOOLCHAIN=local`, `GOFLAGS=-mod=readonly`, `GOPROXY=off`, and a
  private `GOPATH` and `GOCACHE`.
- **Toolchain pin.** The `go` directive is only a minimum and `toolchain` is only
  a suggestion, so neither pins the build. Record an exact Go version in Armorer
  intent, have CI install exactly that version, run with `GOTOOLCHAIN=local` so
  Go cannot switch or download another one, and fail unless `go version`
  reports the recorded version. Require the `go.mod` directives to be no newer
  than the pin.
- **Targets.** `GOOS/GOARCH` pairs, not triples; the target list must become
  per-ecosystem.
- **Build defaults.** `-trimpath`, `-mod=readonly`, `CGO_ENABLED=0` unless
  declared, `GOFLAGS=-buildvcs=true`. Flag `replace` directives pointing outside
  the module and any `GONOSUMDB` / `GOPRIVATE` settings.
- **Gates.** `go vet`, staticcheck, govulncheck, `go mod verify`, a license
  check, and gitleaks / actionlint / zizmor as today.
- **SBOM.** cyclonedx-gomod. Check it against the selected build list, not
  `go.sum`: `go.sum` keeps stale versions, has no dependency edges and omits
  local replacements. Under the offline settings above:
  - **Components:** compare with `go list -m -json all`, which gives each
    selected module version and any `replace` target.
  - **Edges:** derive them from the import graph of the built targets,
    `go list -deps -json` for the same packages, tags, `GOOS` and `GOARCH` as
    the build, mapping each package to its module. Do not use `go mod graph`
    for this: it lists module requirements at the required versions, not
    imports, and an edge disappears when Go selects a newer version.
  - **Cross-check:** `go version -m` on the built binary must list a subset of
    the SBOM components with the same versions and replacements. It carries no
    edges, so it cannot validate the graph by itself.

### Python (second, and scope it narrowly)

Python differs in ways that matter for the security model:

- **Discovery must refuse dynamic metadata.** `setup.py` is arbitrary code.
  Support only static `pyproject.toml` (PEP 621) and report anything dynamic as
  an unsupported capability, the same way native toolchains are handled now.
- **Lockfile is mandatory.** Require a hash-pinned lock (`uv.lock`, or
  requirements with `--require-hashes`). Armorer's own
  `requirements-schema.txt` already follows this pattern.
- **Deliverables.** Wheels and sdists, not executables. The library profile
  fits; CLI and service need a decision (container image, zipapp, or out of
  scope).
- **Publication.** PyPI Trusted Publishing and PEP 740 attestations are a
  different publication adapter from GitHub releases.
- **Gates.** ruff, a type checker, pip-audit, bandit, a license check.
- **SBOM.** cyclonedx-py against the lock.

Start Python with library CI and hardening only, and add release support once
the Go adapter has proven the boundary.

## Suggested sequence

**Now (before tagging 0.1.0)**

1. S1, S2: run Armorer's own gates on Armorer; bump `rustix`; add `SECURITY.md`, `CODEOWNERS`, update bot.
2. S3: define and implement the runtime compatibility rule.
3. A4: remove superseded pre-release contracts as decided in [ADR 0012](../docs/adr/0012-pre-release-contract-consolidation.md).
4. S7: meaningful `check` exit codes.

**Next (makes the product match its purpose)**

5. A1: single transaction engine.
6. A2, A3: catalog-defined file set and file-based templates.
7. F1: `armorer init`.
8. F2, F4, F5: wider file set, drift detection, catalog freshness.
9. S5: fuzz and property tests.
10. F7: Linux-only verifiable release, then the two pilots.

**Later (expansion)**

11. A5: workspace split, standalone `armorer-verify`.
12. Ecosystem adapter boundary and config schema 2.
13. Go adapter; then Python (CI and hardening first).
14. F3 online preflight, Apple signing, S6 directory-handle I/O, A6 CLI ergonomics.

## Open decisions for the owner

- Is the Rust library a supported public API, or is only the binary supported?
- Should Python CLI and service deliverables be in scope at all?
- Is GitHub the only forge for the foreseeable future? The publication and
  capability code assumes it throughout.
