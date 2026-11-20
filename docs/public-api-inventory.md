# Public API inventory of the `armorer` library crate

Taken at `origin/main` `9244948` (2026-11-19) for the owner's pre-0.1.0 decision
that the library is a supported public API (see
[the architecture review](architecture-review-2026-11-16.md#owner-decisions-2026-11-19)
and [ADR 0016](adr/0016-library-is-a-supported-public-api.md)). This is a
snapshot, not a contract: it lists what is exported today and recommends what to
keep. It changes no code.

## Summary

- **29 modules are exported**, 10 at the top (`bootstrap`, `catalog`, `ci_policy`, `config`, `discovery`, `init`, `plan`, `trust`, `upgrade`, `verification`) and 19 below them, plus 3 items at the crate root (`Error`, `Result`, `safe_path`).
- **217 exported items** in the default build: 110 structs, 38 enums, 46 functions, 22 constants, 1 type alias, 0 traits. Under them sit 118 public inherent methods on 47 types, 548 public fields on 93 structs and 167 enum variants.
- Features add 10 more: `test-predecessor` exposes 3 catalog items; `fuzzing` exposes a `#[doc(hidden)]` module `fuzzing` with 7 functions.
- Outside users, by item name: 16 only the binary, 50 binary and tests, 71 only tests, 80 nobody by name. Nothing under `fuzz/` uses anything but `armorer::fuzzing`.
- Main finding: the surface follows the binary and the tests, not a design. Only 66 of 217 items are named by `src/main.rs`; 71 exist for tests alone.
- All 38 public enums and all 93 structs with public fields are exhaustive; the crate has no `#[non_exhaustive]`. Any later field or variant is an incompatible change (section 3).
- `Error` is an exhaustive 11-variant enum with free-text `String` payloads (section 4).
- Pinned third-party versions and digests are exported as constants (section 5). Changing a constant's value does not break compilation, so it is not an incompatible change under Cargo's rules; whether callers may rely on the values is a contract decision for the owner.
- A binary and an integration test are separate crates, so `restrict` below cannot be done with `pub(crate)` alone for the 71 + 16 items they use; see "Reading the tables".

## Method

Nightly was installed (`nightly-aarch64-apple-darwin`, rustc 1.101.0-nightly 2026-09-30), so the surface was read from rustdoc JSON, not from source alone. `rust-toolchain.toml` pins 1.95.0, so the nightly `cargo` had to come first on `PATH`:

```
cargo rustdoc --locked --lib -- -Z unstable-options --output-format json
cargo rustdoc --locked --lib --features fuzzing,test-predecessor -- -Z unstable-options --output-format json
```

A script walked the JSON from the crate root through `pub mod` and `pub use` (including inlined re-exports and glob re-exports), kept only `public` items, and counted fields, variants and inherent methods. Cross-checks:

- `RUSTC_WRAPPER= rustup run 1.95.0 cargo doc --locked --no-deps --lib` generates 120 struct, 40 enum, 59 function, 22 constant and 1 type page. Subtracting the redirect stubs rustdoc writes for items re-exported from private modules (`bootstrap/plan`, `upgrade/transaction` and similar: 10 structs, 2 enums, 13 functions) gives 110, 38, 46, 22 and 1, the same as the JSON walk.
- The default and the feature builds differ by exactly the 3 + 7 items named above. The `fuzzing` module is hidden from rustdoc, so it was read from `src/fuzzing.rs`.

Users were found by searching `src/main.rs`, `src/render.rs`, `tests/**/*.rs` (including `tests/support`) and `fuzz/fuzz_targets/*.rs` for each item's identifier, then resolving name clashes (`Plan`, `Change`, `Receipt`, `Exception`, `Trigger`, `Source`) by hand. "None" means no file names the item. An item can still be reached without being named: a returned value, a field type, or `schemars::schema_for!` (counted as a binary use). In-crate unit tests are not external users. The one doc test that calls an item is the crate-level example in `src/lib.rs` (`config::repository_name`); four `compile_fail` doc tests in `verification` check that proof types cannot be forged.

## Reading the tables

Users: `bin` is `src/main.rs`; `tests` is any file under `tests/`; `fuzz` is `fuzz/fuzz_targets/`; `none` is nobody by name.

Recommendations:

- `keep`: a sensible part of a library someone would call, or a type that a kept item names.
- `restrict`: public only because the binary or tests reach it. Because those are separate crates, narrowing needs a mechanism: move the code into the binary, put it behind a feature, mark it `#[doc(hidden)]` (which still is not a compatibility exemption in Cargo's rules), or move the tests into the crate. The owner chooses; the A5 workspace split in the architecture review is the structural answer.
- `decide`: needs the owner; the question is stated.

Data types that mirror a versioned document (`schemas/*.json`) are `keep` here, because ADR 0012 already freezes the document. Whether their fields should stay public is one question, raised in section 3, not repeated per row.

## Crate root

| Item | Kind | Users | Recommendation |
| --- | --- | --- | --- |
| `Error` | enum (11 variants, exhaustive) | bin, tests | decide: freeze the variant list, or make it `#[non_exhaustive]`; see section 4 |
| `Result` | type | bin, tests | keep |
| `safe_path` | function | tests | decide: is the lexical-plus-symlink path gate a supported primitive, or an implementation detail? |

## `armorer::bootstrap` (8 items; `plan` and `transaction` are `pub(crate)`)

| Item | Kind | Users | Recommendation |
| --- | --- | --- | --- |
| `Plan` (18 public fields; methods `drift`, `release_blockers`, `digest`), `Change` (8), `Receipt` (8) | struct x3 | bin, tests; `Receipt` none by name (returned by `apply`) | keep |
| `inspect`, `inspect_managed`, `load_plan`, `apply`, `recover` | function x5 | bin, tests | keep |

## `armorer::catalog` (19 items; 3 more under `test-predecessor`)

| Item | Kind | Users | Recommendation |
| --- | --- | --- | --- |
| `WORKFLOW_REPOSITORY`, `WORKFLOW_COMMIT_V2`, `TOOLS_SHA256` | const x3 | tests | restrict: a repository name, a pinned commit and a digest that change with each catalog revision |
| `RUNTIME_VERSION` | const | none | restrict: the runtime contract the catalogs freeze (`"0.1.0"`). It is not the crate version: patch releases keep it, so a catalog renders the same lock bytes across a patch line (`src/runtime.rs`) |
| `CATALOG_REVIEWED_ON`, `REVIEW_WINDOW_DAYS` | const x2 | bin, tests | restrict: freshness policy for the CLI warning |
| `Freshness` (4 public fields), `freshness_at`, `freshness_now`, `stale_warning`, `with_review_blocker` | struct, function x4 | bin (not `freshness_at`, `Freshness`), tests | restrict: CLI reporting |
| `AuthenticatedCatalog` (opaque; methods `managed_paths`, `selected_paths`, `tools`, `render_lock`, `caller_workflows`), `reviewed_v2` | struct, function | bin | keep |
| `Tool`, `Distribution` (3 public fields each) | struct x2 | none by name; reachable through `AuthenticatedCatalog::tools` | keep |
| `authenticate` | function | tests | decide: may a caller authenticate its own catalog bytes, or only the compiled one? |
| `compiled_ids`, `reviewed_for`, `all_managed_paths` | function x3 | tests | restrict |
| `WORKFLOW_COMMIT`, `WORKFLOW_COMMIT_TEST_WIDE`, `reviewed` (feature `test-predecessor` only) | const x2, function | tests | restrict: exist only for migration tests |

## `armorer::ci_policy` (10 items)

| Item | Kind | Users | Recommendation |
| --- | --- | --- | --- |
| `CiPolicy`, `Licenses`, `Advisories`, `Exception`, `Sources`, `Bans` (1 to 5 public fields each) | struct x6 | `CiPolicy`: bin, tests; the rest none (fields of `CiPolicy`) | keep |
| `DuplicatePolicy` | enum (2 variants) | none | keep |
| `load` | function | tests | keep |
| `utc_day`, `parse_at` | function x2 | tests | restrict: `parse_at` takes the current day as a parameter for tests |

## `armorer::config` (19 items)

| Item | Kind | Users | Recommendation |
| --- | --- | --- | --- |
| `Config` (8 public fields, method `validate`), `Lock` (5), `Deliverable` (6), `FeatureSet` (2), `Policy` (2), `WorkflowPin` (2), `ToolPin` (2), `RepositoryFiles` (3; methods `is_default`, `validate`) | struct x8 | `Config`, `Lock`: bin, tests; `RepositoryFiles`, `ToolPin`: tests; others none | keep |
| `Profile` (3), `Attestations` (1) | enum x2 | `Profile`: tests; `Attestations`: none | keep |
| `load_config`, `load_lock` | function x2 | tests | keep |
| `repository_name` | function | doc test in `src/lib.rs` only | decide: the crate-level example uses it; keep it as the advertised example, or drop the example |
| `code_owner`, `identifier`, `hex_digest` | function x3 | none | restrict |
| `VERSION`, `MAX_CODE_OWNERS`, `TARGETS` | const x3 | `TARGETS`: tests; others none | restrict |

## `armorer::discovery` (5 items)

| Item | Kind | Users | Recommendation |
| --- | --- | --- | --- |
| `Workspace` (4 public fields), `Package` (7), `Target` (3) | struct x3 | none | decide: their shape follows `cargo metadata`; are they a supported model of a workspace? |
| `discover`, `discover_workspace` | function x2 | tests (`tests/init.rs`) | decide: the same question; both run the installed Cargo |

## `armorer::init` (10 items)

| Item | Kind | Users | Recommendation |
| --- | --- | --- | --- |
| `Proposal` (9 public fields; method `digest`), `ProposedFile` (3), `Receipt` (11) | struct x3 | `Proposal`: bin, tests; others none | keep |
| `run`, `write`, `propose` | function x3 | `run`, `write`: bin, tests; `propose`: tests | keep |
| `CONTRACT`, `SCHEMA_VERSION`, `CONFIG_PATH`, `POLICY_DRAFT_PATH` | const x4 | `CONFIG_PATH`, `POLICY_DRAFT_PATH`: tests; others none | restrict: file names and document identifiers |

## `armorer::plan` (2 items)

| Item | Kind | Users | Recommendation |
| --- | --- | --- | --- |
| `Disposition` (4 variants), `Finding` (2 public fields) | enum, struct | tests | keep |

## `armorer::trust` (67 items: 7 at the root, 60 in 6 modules)

Root of `trust`:

| Item | Kind | Users | Recommendation |
| --- | --- | --- | --- |
| `ByteIdentity` (methods `validate`, `from_bytes`, `matches`), `Source`, `WorkflowIdentity`, `RunIdentity`, `InputIdentity`, `Review` | struct x6 | bin (`ByteIdentity`, `Source`, `Review`), tests | keep |
| `load_json` | function | tests | keep |

`trust::capability` (10 items), `trust::evidence` (12), `trust::inventory` (7), `trust::native` (9), `trust::policy` (8), `trust::publication` (14). These are the Rust form of the capability, evidence, verification-policy, release-inventory, native-catalog, publish-set and receipt documents:

| Items | Kind | Users | Recommendation |
| --- | --- | --- | --- |
| `CapabilityConfig`, `CapabilityObservation`, `Catalog`; `ArtifactEvidence`, `EvidenceRequirements`; `VerificationPolicy`; `ReleaseInventory`; `NativeCatalogV2`, `RuntimeDistributionV1`; `LifecycleRecord`, `GithubReceipt`, `PublishSet`, `RegistryReceipt` (13 structs) | struct | bin (schema export and validation), tests | keep |
| The other 19 structs (`CatalogPin`, `Adapter`, `Selection`, `BuildStep`, `ToolEvidence`, `Exception`, `Coverage`, `AppleAssertions`, `Asset`, `AssetRequirement`, `RuntimeMember`, `NativePin`, `NativeAdapter`, `TrustRoots`, `SignerRule`, `HistoricalRelease`, `TriggerContext`, `CrateIntent`, `CrateReceipt`) and the 23 enums (`CapabilityId` 17 variants, `AdapterId` 13, `AssetRole` 11 and similar) | struct, enum | tests name 6 structs and 17 enums; none name the other 13 structs and 6 enums (`AdapterId`, `AppleTicketMode`, `NativeTarget`, `DistributionFormat`, `PinMaterial`, `Registry`); all are field types of the 13 above | keep |
| `expected_assets`, `validate_trigger` | function x2 | tests | keep |
| `INVENTORY_NAME`, `INVENTORY_BUNDLE_NAME` | const x2 | tests | decide: release-asset file names; are they a published contract? |
| `MAX_NATIVE_MEMBER` | const | none | restrict |

## `armorer::upgrade` (18 items; `authority` and `merge` are `pub mod`)

| Item | Kind | Users | Recommendation |
| --- | --- | --- | --- |
| `Plan` (23 public fields; method `digest`), `Change` (13), `Restore` (6), `RollbackPlan` (8), `Receipt` (8), `SourceFormat` (3 variants) | struct x5, enum | `Plan`, `Change`: bin, tests; `Receipt`, `Restore`: none | keep |
| `inspect`, `load_plan`, `validate_fresh`, `inspect_rollback`, `load_rollback_plan`, `apply`, `apply_rollback`, `recover` | function x8 | bin, tests (`validate_fresh`: tests) | keep |
| `merge::Decision` | enum (7 variants) | tests | keep: a field type of `Change` |
| `merge::decide` | function | none | restrict: the three-way merge rule |
| `authority::Authority` (8 public fields), `authority::select` | struct, function | `select`: bin, tests; `Authority` none | decide: is the catalog-authority record something callers build on? |

## `armorer::verification` (56 items in 11 modules)

| Module and items | Kind | Users | Recommendation |
| --- | --- | --- | --- |
| `publication`: `PublicationPolicy`, `TrustedPublicationPolicy`, `PublicationPlan`, `FrozenRelease`, `PublicationApproval`, `ApprovedPublication`, `NativeGithub`, `OwnedDraftController` (methods `open`, `inspect`, `stage`, `publish`, `recover_published`) | struct x8 | bin | decide: should the controller that stages and publishes a GitHub release be a supported library API, or stay with the binary? |
| `publication`: `CapabilityReport`, `GateState` (6 variants), `ApprovalKind`, `DraftVerification`, `ServedAsset` | struct x3, enum x2 | none by name | keep: return and field types of the row above; follows its decision |
| `context`: `TrustedReleaseContext` (10 methods), `ReleaseExpectationsV3`, `ReleaseSelection`, `ReleaseExpectations`, `ContextTrigger` | struct x4, enum | `TrustedReleaseContext`: bin, tests; `ReleaseExpectationsV3`: bin; others none | keep |
| `context::FEATURE_CONTEXT_NAME` | const | tests | decide: the same question as the inventory file names |
| `release::AuthenticatedReleaseFiles`, `runtime::ApprovedRuntimeFiles`, `apple::{VerifiedAppleRelease, NativeApplePayload}`, `historical::HistoricalByteMatch`, `bundle::UnverifiedBundle`, `cyclonedx::{ValidatedSbom, OfflineSbomValidator}`, `sigstore::{OfflineVerifier, VerifiedAttestation, ExpectedAttestation}` | struct x11 (10 opaque) | bin 5 (4 also tests), tests only 2 (`ApprovedRuntimeFiles`, `ExpectedAttestation`), none 4 (`NativeApplePayload`, `UnverifiedBundle`, `ValidatedSbom`, `VerifiedAttestation`) | keep: the proof types of the verifier; their private fields are the guarantee (see section 5) |
| `sigstore::Trigger` | enum | tests | keep |
| `bundle::load_one`, `bundle::load_download` | function x2 | tests (`tests/sigstore_bundle.rs`) | restrict: the fuzz hook `fuzzing::sigstore_bundle` reaches the parser separately |
| `cyclonedx::qualified_native_validator`, `sigstore::qualified_native_verifier` | function x2 | tests | restrict: return the pinned tool identity |
| `cyclonedx::VALIDATOR_VERSION`, `cyclonedx::VALIDATOR_SOURCE`, `sigstore::VERIFIER_VERSION` | const x3 | none | restrict: pinned third-party tool versions |
| `graph`: `CargoGraphV2` (14 public fields), `GraphSource`, `GraphInputs`, `CargoPackage`, `DependencyContext`, `CargoDependency`, `CargoNode`, `NativeLinkage`, `GraphScope`, `GraphGap`, `DependencyKind` | struct x8, enum x3 | `CargoGraphV2`: bin, tests; `DependencyContext`: tests; others none | keep |
| `manifest`: `ReleaseInputs`, `ReleaseInputsManifest`, `PublicationManifest`, `PublicationFiles`, `ReleaseContextKind` | struct x4, enum | bin; `ReleaseInputs` also tests | keep |
| `manifest::CONTRACT`, `manifest::MAX_MANIFEST` | const x2 | none | restrict |

## 3. Public fields and exhaustive enums

None of these has `#[non_exhaustive]`. For each, adding a field breaks every caller that builds the struct with a literal, and adding a variant breaks every exhaustive `match`. Tests already build these structs by literal (for example `trust::evidence::Selection { .. }` in `tests/cargo_graph_v2.rs`), so callers will follow.

**Structs with public fields: 93 of 110, 548 fields.** By module: `trust` 38, `verification` 22, `config` 8, `ci_policy` 6, `upgrade` 6, `bootstrap` 3, `catalog` 3, `discovery` 3, `init` 3, `plan` 1. The widest are `upgrade::Plan` (23), `bootstrap::Plan` (18), `verification::graph::CargoGraphV2` (14), `trust::publication::GithubReceipt` (14), `verification::publication::PublicationPolicy` (14) and `upgrade::Change` (13). Most are `Serialize + Deserialize + JsonSchema` document types, where a field change is also a schema change that ADR 0012 already controls. The 17 structs with no public field are the proof and handle types. Exceptions that are not documents: `catalog::Freshness`, `discovery::{Workspace, Package, Target}`, `trust::inventory::AssetRequirement`, `verification::sigstore::ExpectedAttestation`.

**Enums, all exhaustive: 38, 167 variants.** `ci_policy::DuplicatePolicy` (2), `config::{Profile (3), Attestations (1)}`, `plan::Disposition` (4), `Error` (11), `upgrade::{SourceFormat (3), merge::Decision (7)}`, `trust::capability::{CapabilityId (17), CapabilityPolicy (3), Availability (4), Enforcement (5), AdapterId (13)}`, `trust::evidence::{Outcome (7), StepKind (4), InputKind (8), AppleTicketMode (2)}`, `trust::inventory::AssetRole` (11), `trust::native::{NativeTarget (3), DistributionFormat (3), PinMaterial (2)}`, `trust::policy::{Predicate (2), TrustBackend (2), EvidenceScope (5), VerificationMode (2)}`, `trust::publication::{Trigger (2), LifecycleStage (5), GithubState (8), ConflictRecovery (3), Registry (1), RegistryState (6)}`, `verification::context::ContextTrigger` (2), `verification::graph::{GraphScope (1), GraphGap (2), DependencyKind (2)}`, `verification::manifest::ReleaseContextKind` (1), `verification::publication::{GateState (6), ApprovalKind (2)}`, `verification::sigstore::Trigger` (2). Several have one variant (`config::Attestations`, `Registry`, `GraphScope`, `ReleaseContextKind`) and plainly expect to grow. Their serde names are identity-bearing (`src/plan.rs` says variants must never be renamed), so the wire form is already frozen; the Rust form is not.

Public trait implementations are also part of the contract. Of the 148 structs and enums, 136 implement `Debug`, 126 `Clone`, 127 `Serialize`, 118 `Deserialize`, 115 `JsonSchema` (from `schemars`, a public dependency of the API) and 63 `PartialEq` and `Eq`.

## 4. The public `Error` type

`armorer::Error` (`src/lib.rs`) is a `thiserror` enum with 11 variants: `Io(std::io::Error)` (through `From`), `Toml`, `TomlAt { offset }`, `Invalid(String)`, `Path(String)`, `Cargo(&'static str)`, `Metadata`, `Json`, `Transaction(&'static str)`, `TransactionCause { message, cause: Box<Error> }` and `TransactionRollback { message, cause, rollback }`. `Error::code()` maps them to 8 category strings: `io`, `invalid-toml`, `invalid-config`, `unsafe-path`, `cargo-discovery`, `cargo-metadata`, `json`, `transaction`. `docs/cli-reference.md` lists these as "stable error categories" and adds `unsupported-host`, which only the binary emits (`src/main.rs`), not the library.

- The code is stable by documentation. The enum is not: it is exhaustive, so a new variant breaks downstream `match`. `code()` is the safer contract; `src/main.rs` matches no variant and uses only `code()` and `to_string()`. Tests construct and match `Invalid` and `Path`, and compare `Path` message text such as `symlink input or parent` (`tests/directory_handles.rs`, `tests/upgrade.rs`), so the message strings are de facto contract in this repository's own tests.
- `Invalid(String)` and `Path(String)` carry free text. A caller cannot tell why a configuration was invalid except by comparing strings. `Transaction(&'static str)` and `TransactionCause::message` are static strings used as sub-codes; they are not documented and not covered as stable.
- `Display` text is printed in the CLI's JSON `message` field. Nothing says whether it is stable.
- `Error` exposes `std::io::Error` (a public type of std, stable) and recursion through `Box<Error>`. It is not `Clone` or `PartialEq`.
- `Result<T>` aliases the crate error, so changing `Error` changes every signature that returns `armorer::Result`.

## 5. Unintended or risky to freeze

- **Pinned third-party versions and digests as constants.** `catalog::TOOLS_SHA256`, `catalog::WORKFLOW_COMMIT_V2`, `catalog::WORKFLOW_REPOSITORY`, `catalog::RUNTIME_VERSION` (`"0.1.0"`, the frozen runtime contract, which patch releases keep and which is not the crate version), `catalog::CATALOG_REVIEWED_ON` and `REVIEW_WINDOW_DAYS`, `verification::cyclonedx::{VALIDATOR_VERSION, VALIDATOR_SOURCE}` and `verification::sigstore::VERIFIER_VERSION`. Most of these change with a reviewed bump. Changing a value keeps the `&str` signature and compiles for every caller, so it is not an incompatible change under Cargo's rules; it is a behavioral change that needs the maintainer's judgment. The decision for the owner is what the values promise: exported as "the current reviewed pin", they may move in any release; a caller must not treat one as fixed. `RUNTIME_VERSION` and the document identifiers are different: their values are already governed by the runtime rule and ADR 0012.
- **Test-only items behind a feature.** `catalog::{reviewed, WORKFLOW_COMMIT, WORKFLOW_COMMIT_TEST_WIDE}` exist for migration tests (ADR 0012 rule 3). The dev-dependency `armorer = { path = ".", features = ["test-predecessor"] }` turns the feature on for `cargo test` only. A downstream user who enables `test-predecessor` gets a surface the docs call test-only, so the feature name itself is exposed.
- **The `fuzzing` feature.** `armorer::fuzzing` is `#[doc(hidden)]` and compiled only with `--features fuzzing`; its 7 functions (`ci_policy`, `config_toml`, `http_response`, `lock_toml`, `sigstore_bundle`, `trust_json`, `verification_json`) exist for `fuzz/`. `doc(hidden)` does not remove them from Cargo's compatibility rules. The `fuzz/` crate depends on the feature; no one else should.
- **Internal file names and contract identifiers.** `init::{CONFIG_PATH, POLICY_DRAFT_PATH, CONTRACT, SCHEMA_VERSION}`, `manifest::{CONTRACT, MAX_MANIFEST}`, `inventory::{INVENTORY_NAME, INVENTORY_BUNDLE_NAME}`, `context::FEATURE_CONTEXT_NAME`, `native::MAX_NATIVE_MEMBER`, `config::{VERSION, MAX_CODE_OWNERS, TARGETS}`. The `.armorer/` state, journal and ownership paths are `pub(crate)` and not exported.
- **Command-line shaped helpers.** `catalog::{freshness_at, freshness_now, stale_warning, with_review_blocker}` and `Freshness` build the CLI's staleness warning and release blocker; `ci_policy::parse_at` takes the current day for tests.
- **Security guarantees that depend on private fields.** The 17 opaque types (`TrustedReleaseContext`, `AuthenticatedReleaseFiles`, `ApprovedRuntimeFiles`, `FrozenRelease`, `ApprovedPublication`, `TrustedPublicationPolicy`, `OwnedDraftController` and others) keep their fields private so a caller cannot forge an authenticated or approved value. Four `compile_fail` doc tests (`src/verification/{runtime,historical,release}.rs`, `src/verification/publication/mod.rs`) guard this. Keep them in any compatibility check, and treat making one of these fields public as a security change, not only an API change.
- **A library entry point that shells out and can publish.** `NativeGithub` drives the installed `gh` CLI; `OwnedDraftController::{stage, publish, recover_published}` perform release actions. Supporting this as library API commits the project to its misuse surface and to `gh` behavior.
- **Names that carry versions.** `TrustedReleaseContext::open_native_v3`, `ReleaseExpectationsV3`, `NativeCatalogV2`, `CargoGraphV2`, `RuntimeDistributionV1`. ADR 0012 keeps these numbers on purpose; a successor adds a new name beside them, so each successor widens the exported surface.
- **A public dependency in signatures.** `schemars::JsonSchema` is derived on 115 exported types, and `serde` traits on most. Their major versions are pinned with `=` in `Cargo.toml`, but upgrading `serde` or `schemars` across a major version would be visible to callers.
- **Crate packaging.** `Cargo.toml` has no `publish = false`, an `include` list and a `readme`; `rust-version = "1.95"` is also part of the contract. Whether the crate is published is undecided (ADR 0016).

## 6. What could not be determined

- Users outside this repository. No external adopter is known, `examples/` and `.github/` contain no Rust that imports the crate, and the two pilots (Momus, Rusty Brain) have not started; this inventory cannot see them.
- Items that are used without being named (a returned value, an inferred type, a field) show as "none". The "none" count of 80 is an upper bound on dead exports: many of them are field or return types of items that are used (for example the 19 `trust` types above).
- Whether a method signature or trait bound would pass a semver check. Only names, kinds, field counts and variant counts were recorded, not full signatures; a signature-level baseline needs the rustdoc JSON itself or `cargo-semver-checks`.
- Auto-trait and layout properties (`Send`, `Sync`, `Unpin`, size) of exported types. They are also part of Cargo's compatibility rules and were not examined.
- Whether `docs.rs` or any consumer reads the `fuzzing` and `test-predecessor` features. Nothing in the repository documents them as public.
- The meaning of "supported" for the binary's JSON output relative to the library types that produce it (`schemas/*.json` are generated from the exported types through `schemars::schema_for!`). The two share types, so a Rust-level change can change a schema; `tests/` check the committed schemas but this inventory did not check every one.
