# Review findings — group G_cli_ci (16 findings)

─── scripts/qualify-test-verifier.py:77-77 ───
[security · medium] `exist_ok=True` accepts a pre-existing destination without checking its owner or
permissions, which undermines the `O_EXCL`/0o700 protections below. The documented usage
(`docs/offline-verifier.md`) points at `/tmp/armorer-fixture-gh`; on a shared host another local
user can pre-create that directory world-writable. `O_EXCL` only protects file *creation* — with
write access to the directory the attacker can unlink/replace `gh` (and `qualification.json`) after
this script finishes, so the test would execute an unqualified binary despite all the digest pins.
Since the script's stated contract is "create a new owned output", verify ownership and mode of an
existing directory (or refuse it) before writing.

      destination.mkdir(mode=0o700, parents=True, exist_ok=True)
+     status = destination.stat()
+     if status.st_uid != os.getuid() or status.st_mode & 0o077:
+         raise ValueError("output directory must be owned by the current user and not group/world accessible")

─── src/ci_policy.rs:184-184 ───
[security · low] The control-character filter only rejects code points < 32, so DEL (U+007F) and C1
controls (U+0080–U+009F, e.g. CSI U+009B) pass through into `owner`/`reason`, which are presumably
rendered in CI output/review tooling. Since the clear intent here is to sanitize these fields, use
`char::is_control()`, which covers both ranges.

-                     && !value.chars().any(|c| u32::from(c) < 32),
+                     && !value.chars().any(char::is_control),

─── src/ci_policy.rs:172-179 ───
[maintainability · low] `seen.insert(&e.id)` is a side effect hidden as the last conjunct of a
short-circuiting `&&` chain: it only runs if every prior format check passes. It happens to be
correct today (any failure aborts the whole parse), but reordering the conjuncts or making `require`
non-eager would silently break duplicate detection. Split the mutation into its own statement so the
validation condition stays pure.

          require(
              b.len() == 17
                  && b.starts_with(b"RUSTSEC-")
                  && b[12] == b'-'
                  && b[8..12].iter().all(u8::is_ascii_digit)
-                 && b[13..].iter().all(u8::is_ascii_digit)
-                 && seen.insert(&e.id),
+                 && b[13..].iter().all(u8::is_ascii_digit),
          )?;
+         require(seen.insert(&e.id))?;

─── src/main.rs:390-393 ───
[bug · medium] A missing HOME (and no --owner-home) is an environment/config problem, but it is
surfaced as Error::Json, which renders as "JSON serialization failed" with code "json" (see lib.rs).
Operators get a wrong category and no hint about the real cause. Use a descriptive variant, e.g.
Error::Invalid("owner home not provided and HOME is unset".into()). The same misleading Error::Json
is used below for the missing-approval invariant (approval.as_ref().ok_or(Error::Json)?), which
likewise masks a logic error as a serialization failure.

─── src/main.rs:387-389 ───
[maintainability · medium] The Plan early-return invariant is enforced only by convention across
three separate matches (approval, state_directory/owner_home/read_only, and the final value match),
so both unreachable!() arms here and below will panic in a production CLI path if a variant is added
or the guard is refactored. Prefer handling Plan inside a single match (or returning a typed Error)
so the compiler enforces exhaustiveness rather than relying on a runtime panic with no message.

─── src/main.rs:530-530 ───
[maintainability · medium] This context/verifier/SBOM/AuthenticatedReleaseFiles/Apple verification
sequence duplicates freeze_publication_inputs almost verbatim. Because it is security-sensitive, a
fix or reordering applied to only one copy can silently weaken the other. Extract a single shared
helper that both VerifyRelease and publication freezing call.

─── src/main.rs:571-571 ───
[maintainability · medium] The +1 hard-codes the assumption that exactly one extra bundle (the
inventory_proof) is verified beyond attestations(). It is correct today because inventory_proof is
tracked separately, but if that model changes this security-audit count silently misreports
verification coverage. Derive the total from an explicit API (e.g. a verified_bundle_count() on
AuthenticatedReleaseFiles) instead of a magic constant.

─── src/main.rs:764-764 ───
[bug · medium] On serialization or stdout write failure this exits 1 with no diagnostic at all — the
operator sees empty output and cannot tell what went wrong (and a partial JSON document may already
be on stdout). Emit a short message to stderr before exiting so failures are observable. The
pervasive map_err(|_| Error::Json) similarly discards the underlying serde error detail.

─── examples/bootstrap-v2/library/Cargo.toml:5-5 ───
[maintainability · low] This library crate omits `rust-version` even though the repository has an
MSRV policy: the root `Cargo.toml` declares `rust-version = "1.95"` and the `profile-workspace`
example follows it. Edition 2024 requires Rust >= 1.85, so without an explicit MSRV an older
toolchain fails with a cryptic edition error instead of a clear "requires rustc X or newer" message.
Declare the repo MSRV here for consistency.

  edition = "2024"
+ rust-version = "1.95"

─── Cargo.toml:9-9 ───
[maintainability · medium] No `include`/`exclude` is set, so `cargo publish` packages everything not
git-ignored: the nested example projects under `examples/` (each with its own
`Cargo.toml`/`Cargo.lock`), all of `tests/fixtures/` (sigstore bundles, `trusted_root.json`,
attestation JSON), and `docs/`. This bloats the published crate and risks silently shipping future
fixtures/generated artifacts — a notable gap for a crate whose purpose is verifiable releases. Add
an explicit allowlist (or at minimum exclude `examples`, `tests`, `docs`) so the package contents
are intentional and reviewable via `cargo package --list`.

  readme = "README.md"
+ include = ["src/**/*", "Cargo.toml", "README.md", "LICENSE"]

─── .github/workflows/momus.yml:3-5 ───
[performance · low] No `concurrency` group is defined. This workflow triggers on every `synchronize`
event, so rapid successive pushes to a PR will queue overlapping review runs, wasting runner time
and potentially posting stale or out-of-order review comments. Add a concurrency group keyed on the
PR number with `cancel-in-progress: true` so only the latest commit is reviewed.

  on:
    pull_request:
      types: [opened, synchronize, reopened]
+
+ concurrency:
+   group: ${{ github.workflow }}-${{ github.event.pull_request.number }}
+   cancel-in-progress: true

─── .github/workflows/momus.yml:12-13 ───
[performance · low] The review job has no explicit `timeout-minutes`, so a hung or slow review can
occupy a runner for up to the default 360 minutes. Note that `timeout-minutes` cannot be added to
this calling job: because it invokes a reusable workflow via `uses:`, GitHub Actions only permits
`name`, `uses`, `with`, `secrets`, `needs`, `if`, `permissions`, `concurrency`, and `strategy` on
the calling job — adding `timeout-minutes: 15` here would make the workflow invalid. To bound
runtime (e.g., 15 minutes), `timeout-minutes` must be set on the jobs inside
`brianluby/momus-review/.github/workflows/review.yml` (pinned at v0.9.0); consider
verifying/requesting that upstream.

    review:
      if: github.event.pull_request.head.repo.full_name == github.repository
+     timeout-minutes: 15

─── .github/workflows/development.yml:145-145 ───
[bug · low] `archive` is not a recognized input for `actions/upload-artifact` (valid inputs: `name`,
`path`, `if-no-files-found`, `retention-days`, `compression-level`, `overwrite`,
`include-hidden-files`). Unknown inputs are silently ignored, so this line is dead config. Remove
it, or use `compression-level` if the intent was to control archiving/compression.

-           archive: true
+           compression-level: 6

─── .github/workflows/development.yml:31-31 ───
[performance · low] No dependency caching anywhere: every run on all 3 matrix legs reinstalls the
exact rustup toolchain and rebuilds all crates from scratch for clippy, test, debug build, and
release build (within a 25-minute timeout). Consider `actions/cache` (or Swatinem/rust-cache) keyed
on `rust-toolchain.toml` + `Cargo.lock` for `~/.cargo` and `target/`. If caching is deliberately
omitted to avoid cache-poisoning of the uploaded runtime candidate, restrict any cache to read-only
on PR runs (`restore-keys` without save) — but as written this is a large, repeated cost.

─── .github/workflows/development.yml:13-13 ───
[bug · medium] `cancel-in-progress: true` applies to `push` on `main` as well as PRs. A new merge to
`main` will cancel the still-running validation of the previous main commit, so that commit never
produces its `runtime-candidate-*` artifact (which appears intended for downstream consumption,
given `overwrite: false` and per-run unique names). Limit cancellation to PRs so every main commit
completes and uploads its candidate.

-   cancel-in-progress: true
+   cancel-in-progress: ${{ github.event_name == 'pull_request' }}

─── .gitignore:1-1 ───
[maintainability · low] The leading slash anchors this to the repo root only. This repo contains
standalone Cargo packages under `examples/` (e.g. `examples/bootstrap-v2/cli/Cargo.toml`,
`examples/profile-workspace/`), and building them creates nested `target/` directories that will
show up as untracked files. Drop the anchor so build output is ignored at any depth.

- /target/
+ target/



──────── Project Summary ────────

### Top Issues

1. **Fail-open acceptance gates defeat the tool's core security promises.** `src/verification/publication/github.rs` treats an empty deployment-status list (`200 []`) as "no active deployment", excluding a competing deployment from `active_count` (fetch failures/truncation are likewise downgraded). `src/verification/historical.rs` returns a successful `HistoricalByteMatch` for an empty asset map. `src/trust/publication.rs` accepts any single `ByteIdentity` as "per-stage" evidence, leaves optional evidence unvalidated outside its owning state, and omits the remote draft digest from receipt identity — a retry can resume against replaced draft bytes. `src/trust/policy.rs` exposes acceptance helpers on a freely constructible policy with no enforcement that `validate(now)` ran. The schemas compound this: `schemas/publish-set-v1.json` accepts empty/duplicate crate sets and unconstrained identity strings; `schemas/native-catalog-v2.json` accepts pin-less adapters the runtime rejects.

2. **Check-then-act symlink races at every filesystem trust boundary.** The stat-then-open pattern repeats in `src/apply.rs` (`apply.lock`), `src/lib.rs` (read guard), `src/config.rs` (`try_exists` treats a dangling lock symlink as "no lock"), `src/bootstrap/transaction.rs` (`is_dir()` after `safe_path`), `src/verification/publication/mod.rs` (lock acquisition + directory trust), and `src/verification/io.rs` (path-based `set_permissions`; dev/ino binding is Unix-only). Worst case: `src/discovery.rs` gives `rustc` only an `is_file()` check while `cargo` is canonicalized and root-checked, so a workspace-pointing symlink is executed via `RUSTC`.

3. **Crash/failure windows permanently wedge publication; committed transactions are misreported as failures.** In `src/verification/publication/mod.rs`, `Creating`/`Uploading`/`Publishing` are persisted before the backend call with no retry or rollback path, and the journal is never rotated after `Complete` while `load` hard-fails on run/attempt-bound plan mismatch — one crash or 403 blocks all future releases for that repository. The same root cause appears in the apply trio: post-commit cleanup failures propagate as bare I/O errors indistinguishable from failed transactions (`src/apply.rs`, `src/bootstrap/transaction.rs`, `src/upgrade/transaction.rs`), and `src/apply.rs` ignores `unlock()` despite its own documented flock invariant.

4. **Non-injective identity keys and Debug-based dedup weaken pin/evidence binding.** `--`-joined composite keys collide because identifiers permit consecutive hyphens (`src/trust/evidence.rs`, `src/verification/context.rs`); `src/catalog.rs` silently overwrites duplicate pin keys in a `BTreeMap`; `src/trust/capability.rs` matches lock tools against pins across *all* adapters while uniqueness is enforced only per-adapter; `src/trust/native.rs` and `capability.rs` dedup adapters by `format!("{:?}")`; `src/verification/graph.rs` (`native_linkage`) and `src/trust/evidence.rs` (requirement lists) lack uniqueness checks.

5. **The security test suite frequently passes without exercising the guard it names.** Roughly 40% of all comments are in tests. Bare `is_err()` negatives accept unrelated failures (`tests/real_sigstore.rs`, `tests/historical.rs`, `tests/trust_contracts.rs`, `tests/release_context.rs`); mutations can be silent no-ops (`tests/real_sigstore.rs` base64 `'A'` flip, `tests/upgrade.rs` string replace yielding malformed JSON, `tests/inspection.rs` unmatched `replace`); assertions are vacuous (`tests/inspection.rs` compares stale bytes, `tests/historical.rs` size-cap checked against wrong content and markers written to the wrong cwd, `src/verification/publication/tests.rs` asserts `creates == 0` without ever attempting staging, fakes discard byte bounds and assign "foreign" authors inside the allow-list). Named contracts are untested: approval-required (`tests/apply.rs`), unknown-field rejection (`tests/trust_contracts.rs`), v2 catalog hardening (`tests/catalog.rs`).

6. **Published schemas drift from runtime validation, and the bounds generator corrupts schemas.** `src/config.rs` hardcodes `max = 3` against `TARGETS`, omits feature-name/`license_file` constraints, and pins `runtime_version` to `x.y.z` while requiring exact `CARGO_PKG_VERSION` equality (prerelease builds become unlockable). `src/trust/mod.rs` leaves `git_ref` unconstrained, mixes code-point and byte lengths, and allows unbounded version digits. `schemas/publish-set-v1.json` relies on annotation-only `format: uint64` with maxima beyond the 2^53−1 interop range and rejects legitimate `.yaml` workflows (repeated across sibling schemas). `src/schema_bounds.rs` overwrites stricter non-u64 `maximum`s with `u32::MAX`/`u64::MAX` and injects `maximum` into annotation payloads (`default`/`examples`/`const`/`enum`).

7. **Advertised time/memory bounds are not enforced end-to-end.** `src/verification/release.rs` never samples the deadline in its final rehash (up to 4 GiB / 8194 files) and does O(n²) asset lookups; `src/verification/runtime.rs` snapshots ~384 MiB outside the 20-minute budget; `src/verification/apple.rs` runs Security.framework validity checks with network access allowed and no deadline; `src/verification/sigstore.rs`'s "timeout" only breaks a poll loop (blocking `wait()` after `kill()`); `src/discovery.rs`'s `rustup which` is unbounded. Memory amplification: `github.rs` buffers 1 GiB assets at ~2× RSS, `apple.rs` holds the archive `Vec` through `native()`, `src/trust/inventory.rs` allocates all names before applying the 8192 cap.

8. **Error-category collapse hides security signals and misdirects operators.** `Error::Json` ("JSON serialization failed") covers parsing, duplicate-key anti-smuggling rejections, base64 faults, state-machine invariants, and even missing HOME (`src/trust/mod.rs`, `src/verification/io.rs`, `src/verification/bundle.rs`, `src/verification/publication/mod.rs`, `src/main.rs`). `Error::Io` renders write/delete failures as "cannot read local input" (`src/lib.rs`). `src/config.rs` discards TOML spans; `src/upgrade/state.rs` collapses six invariant failures into one message; `src/upgrade/transaction.rs`'s `.is_ok()` turns transient I/O into `configured: false`; `src/main.rs` exits 1 with no diagnostic at all on stdout/serialization failure.

9. **Security-critical logic is duplicated or compared via serialization.** `src/main.rs` re-implements the `freeze_publication_inputs` verification sequence verbatim; `src/verification/io.rs` duplicates the size-gated hashing loop; `src/verification/apple.rs` re-implements `ByteIdentity::from_bytes`; `src/upgrade/authority.rs` is a third copy of the catalog registry. Equality-by-`serde_json::to_value` in `src/trust/evidence.rs`/`inventory.rs` assumes serde injectivity for a security check; `src/verification/publication/github.rs` double-observation hashes entire provider bodies including volatile `pushed_at`/`updated_at`, guaranteeing spurious mismatches.

10. **Safety rests on undocumented cross-module coincidences; library paths can panic.** Unchecked `zip` cardinality (`src/bootstrap/plan.rs`, `src/upgrade/plan.rs`), direct indexing `files["rust-toolchain.toml"]` and `nodes[id]` (`src/upgrade/plan.rs`, `src/verification/graph.rs`), `unreachable!()` in CLI matches (`src/main.rs`), and Apple signing gates hardcoded to `aarch64-apple-darwin` (`src/trust/capability.rs`, `src/verification/context.rs`) — adding any Darwin triple to `TARGETS` silently bypasses notarization gating. `src/upgrade/plan.rs` also enforces overlap/sorting checks in `inspect_bytes` but not `validate()`, and recomputes `policy_review_day` so UTC rollover breaks freshness of byte-identical inputs.

### Module Hotspots

- **`src/verification/publication/`** — 16 comments, the highest severity concentration: fail-open gate (`github.rs`), three journal-wedging paths plus lock TOCTOU (`mod.rs`), and 7 comments in `tests.rs` about doubles that don't model the real backend.
- **`src/verification/` core** (`io.rs`, `release.rs`, `runtime.rs`, `sigstore.rs`, `apple.rs`, `context.rs`, `bundle.rs`, `graph.rs`, `historical.rs`) — ~25 comments: deadline gaps, symlink handling, duplicated hashing, vacuous historical success, byte-identity contract violations.
- **`src/trust/`** — 24 comments across `capability.rs`, `evidence.rs`, `publication.rs`, `mod.rs`, `inventory.rs`, `native.rs`, `policy.rs`: identity keys, serialization equality, per-state validation gaps.
- **Transaction core** — `src/apply.rs`, `src/bootstrap/transaction.rs`, `src/upgrade/transaction.rs` (plus `upgrade/state.rs`, `upgrade/plan.rs`): the identical post-commit error-handling defect in three places.
- **`tests/`** — ~78 comments (≈38% of the scan). Densest: `tests/release_context.rs` (9), `tests/historical.rs` (8), `tests/inspection.rs` (8), `tests/apply.rs` (7), `tests/integrated_cli.rs` (7).
- **Schema surface** — `schemas/`, `src/config.rs`, `src/trust/mod.rs`, `src/schema_bounds.rs` (~17 comments): contract drift plus active generator bugs.
- **`src/discovery.rs`** — 4 comments but includes the only direct code-execution boundary gap (`rustc` symlink).

### Cross-Cutting Concerns

- **Swallowed results.** `.is_ok()` / `.is_ok_and()` / `.ok()` / `unwrap_or_default()` / ignored `unlock()` erase the distinction between "absent", "denied", and "tampered": `src/apply.rs`, `src/upgrade/transaction.rs`, `tests/integrated_cli.rs`, `tests/preview.rs`.
- **Stringly-typed security invariants.** Magic disposition literals (`src/plan.rs`), `--` composite keys, Debug-format identity (`src/trust/native.rs`), raw TOML interpolation of `toolchain` (`src/catalog.rs`), weak `pin.version` charset (`src/trust/capability.rs`, `native.rs`), DEL/C1 controls passing the filter (`src/ci_policy.rs`).
- **Producer/consumer validation asymmetry.** `src/upgrade/plan.rs` (`validate()` vs `inspect_bytes`), `src/verification/bundle.rs` (`load_download` re-serializes "exact transport bytes"), `src/verification/context.rs` (Legacy pin-less adapter vacuously passes; authenticated `Cargo.lock` bytes dropped), `src/trust/publication.rs` (state-conditional evidence validation).
- **Test oracles that follow symlinks or ignore metadata.** `tests/bootstrap.rs`, `tests/integrated_cli.rs`, `tests/preview.rs` snapshots use `is_dir()`/`fs::read` (FIFO reads hang, symlink cycles escape the fixture) and never capture file types/permissions — the harnesses meant to detect escapes can themselves escape.
- **Environment-dependent tests.** Unconditional Unix imports (`tests/publication_cli.rs` breaks non-Unix compilation), external GNU `patch` (`tests/bootstrap.rs`, `tests/preview.rs`, `tests/upgrade.rs`), hardcoded `/usr/bin` paths (`tests/publication_cli.rs`, `tests/inspection.rs`), root/`0o500` assumptions (`tests/integrated_cli.rs`), rustup-on-PATH (`tests/inspection.rs`), coarse-mtime no-rewrite oracles (`tests/apply.rs`, `tests/bootstrap.rs`, `tests/upgrade.rs`).
- **Positional coupling to fixture ordering.** `changes[1]`/`files[0]` indexing (`tests/bootstrap.rs`, `tests/preview.rs`, `tests/integrated_cli.rs`), wildcard match arms with drifting ranges and fixed tar offsets (`tests/release_context.rs`), `remove(0)` (`tests/cargo_graph_v2.rs`), `checks + 2` (`src/verification/publication/tests.rs`).
- **Half-hearted non-Unix support.** `src/filesystem.rs` unused import fails `-D warnings`, `src/verification/io.rs` identity binding degrades silently, `tests/apply.rs` and `tests/cargo_graph_v2.rs` run Unix-only logic ungated.
- **Needless clones in hot paths.** `src/schema_bounds.rs` (deep schema clone + `unwrap`), `src/upgrade/state.rs` (preimage map), `src/upgrade/transaction.rs` (`proposed_content` on skipped writes), `src/trust/evidence.rs` (per-step/per-asset allocations), `src/plan.rs` (`sha256.clone()` for a comparison).

### Quick Wins

- **Workflows:** remove the dead `archive` input and scope `cancel-in-progress` to PRs only (main-commit artifacts are currently cancelled) in `.github/workflows/development.yml`; add a concurrency group to `.github/workflows/momus.yml`; add rustup/cargo caching.
- **Packaging:** add `include`/`exclude` to `Cargo.toml` (fixtures, docs, nested examples currently ship); unanchor `target/` in `.gitignore`; add `rust-version` to `examples/bootstrap-v2/library/Cargo.toml`.
- **Unbreak non-Unix builds:** cfg-gate the `OpenOptions` import in `src/filesystem.rs` and the `std::os::unix` import in `tests/publication_cli.rs`.
- **Close the exec gap:** apply cargo's canonicalize+root check to `rustc` in `src/discovery.rs`; skip broken PATH candidates with `let Ok(..) else { continue }`.
- **Sanitization:** reject U+007F and C1 controls in `src/ci_policy.rs`; refuse pre-existing destinations in `scripts/qualify-test-verifier.py` instead of `exist_ok=True`.
- **Schemas:** add `minItems`/`uniqueItems` and the shared crate-name pattern to `schemas/publish-set-v1.json`; accept `.yaml` in all sibling workflow patterns; make `previous_catalog` required (nullable) in `schemas/native-catalog-v2.json`.
- **Dedup:** reuse `ByteIdentity::from_bytes` and `drop(archive)` before `native()` in `src/verification/apple.rs`; derive `Ord` on `AdapterId` and dedup by typed id in `src/trust/capability.rs`/`native.rs`; destructure instead of clone+unwrap and skip annotation subtrees in `src/schema_bounds.rs`.
- **Diagnostics:** emit a stderr message before the silent exit 1 and use a config-error variant for missing HOME in `src/main.rs`; sample `deadline()` in the final rehash loop of `src/verification/release.rs`.
- **Diff correctness:** use `/dev/null` headers for empty sides in `src/preview.rs` and `src/upgrade/rollback.rs` (deletions currently render as truncations).
- **Mechanical test hardening (highest leverage per effort):** replace bare `is_err()` with expected error codes; `assert_ne!` after every fixture mutation; compare bytes instead of mtimes; include action/stderr in CLI failure messages; gate or PATH-resolve GNU `patch`/`mkfifo`/`touch` dependencies.
- **Allocation hygiene:** borrow instead of clone in `src/plan.rs` and `src/trust/evidence.rs` comparisons; hoist the expected asset name out of the `find` closure in `src/trust/evidence.rs`.

