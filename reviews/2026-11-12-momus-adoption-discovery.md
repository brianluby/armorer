# Momus pilot adoption — discovery report (plan step 19, ticket #12)

Date: 2026-11-12. Status: **DISCOVERY + DRAFTS ONLY — PROPOSED, UNAPPROVED.**
No file in the Momus primary checkout (`/Users/bluby/repos/momus-review`) was
modified; nothing was committed or pushed. All work ran in the disposable clone
`/tmp/armorer-pilot-momus` (cloned from the local primary checkout with
`--no-hardlinks`, origin reset to `https://github.com/brianluby/momus-review.git`;
no fetch from GitHub except read-only release-asset metadata via `gh`).

- Snapshot: `main` @ `bb64d2f` ("Merge pull request #45 from brianluby/codex/release-source-install")
- Armorer binary: `/Users/bluby/repos/armorer/target/debug/armorer`, `runtime_version` 0.1.0 (built at `ab526d7` per assignment)
- Toolchain: **1.98.1 already installed** (`rustup toolchain list`); no install needed. `cargo 1.98.1 (797e8a9bc 2026-08-05)`.

## 1. Exact commands run and observed results

| # | Command (cwd) | Exit | Output |
|---|---|---|---|
| 1 | `rustup toolchain list` | 0 | `1.98.1-aarch64-apple-darwin` present |
| 2 | `git clone --no-hardlinks /Users/bluby/repos/momus-review /tmp/armorer-pilot-momus` + `git remote set-url origin https://github.com/brianluby/momus-review.git` | 0 | snapshot bb64d2f |
| 3 | `env RUSTC_WRAPPER= rustup run 1.98.1 cargo metadata --no-deps --offline --locked --format-version 1` (snapshot) | 0 | `/tmp/momus-metadata.json` |
| 4 | `armorer --repository . plan` (config only in `adoption/`, first attempt) | **1** | `{"error":{"code":"io","message":"cannot read local input: No such file or directory (os error 2)"}}` — armorer.toml must be at the repo root |
| 5 | `cp adoption/armorer.toml armorer.toml` (snapshot root only) then `armorer --repository . plan` | **0** | `adoption/plan.json` (12,626 B) |
| 6 | `armorer --repository . check` | **2** (expected until ready) | `adoption/check.json` (12,627 B) |
| 7 | `armorer --repository . bootstrap check --policy adoption/ci-policy.draft.toml` | **2** | `adoption/bootstrap-check.json` |
| 8 | `armorer --repository . bootstrap plan --policy adoption/ci-policy.draft.toml` | **0** | `adoption/bootstrap-plan.json` — draft ci-policy parsed/validated |
| 9 | `env RUSTC_WRAPPER= rustup run 1.98.1 cargo fetch --locked` (snapshot; registry cache only) | 0 | needed so read-only cargo-deny could list licenses offline |
| 10 | `env RUSTC_WRAPPER= rustup run 1.98.1 cargo deny --locked --offline list --config /tmp/momus-deny-discovery.toml --format json` (cargo-deny 0.19.7) | 0 | `/tmp/momus-deny-list.json` — license closure |
| 11 | `gh release view v0.2.0 -R brianluby/momus-review --json tagName,assets` (read-only) | 0 | `/tmp/momus-v020-assets.json` — real asset names/sizes/digests |
| 12 | `git show v0.2.0:.github/workflows/release.yml` (clone) | 0 | pre-attestation pipeline confirmed |

No `cargo build`/`cargo test` was run in the pilot. Armorer plan/check/bootstrap
perform no builds, no network, no consumer writes (verified: only the draft
`armorer.toml` copy and `adoption/` outputs exist beyond the clone).

## 2. Discovered facts (cargo metadata, exit 0)

Single package `momus-review` v0.9.0, edition 2024, `rust-version = "1.88"` (MSRV ≤ pin 1.98.1), MIT, `Cargo.lock` present (v4).
Targets: lib `momus_review`, bin **`momus`** (src/main.rs), bin **`momus-eval`** (src/bin/momus-eval.rs — golden-set evaluation tool, **not** a release asset). **No package features declared.** No git dependencies.

## 3. Existing three-target pipeline (main @ bb64d2f)

`release.yml` (tags `v*.*.*` + `workflow_dispatch` rehearsal) orchestrates SHA-pinned reusables; the pin `290feb94f125b7525302a46e1a37ff5eaefd3465` appears in six places and moves together:

1. **audit** — `cargo-audit 0.22.2` via `scripts/audit.sh --advisory-db-outage fail`; exception file `scripts/audit-exceptions.txt` currently **empty** (format `RUSTSEC-ID|YYYY-MM-DD|reason`, hard expiry).
2. **build** → `builder.yml@290feb9` — fixed-command, no caller inputs; matrix `x86_64-unknown-linux-gnu` (ubuntu-22.04), `aarch64-unknown-linux-gnu` (ubuntu-22.04-arm), `aarch64-apple-darwin` (macos-14); `cargo test --locked`; `cargo build --release --locked --bin momus`; target SBOM via pinned `cargo-cyclonedx 0.5.9` + `scripts/finalize-sbom.py` + CycloneDX 1.5 JSON-schema validation (jsonschema 4.25.0, schema commit `55343ba`); `momus-<target>.tar.gz` + `.sha256`; per-target release metadata (toolchain rustc/cargo versions, runner image, Cargo.lock digest, generator versions).
3. **sign-macos** → `brianluby/apple-signing/.github/workflows/sign-macos.yml@bac7bb3123e4a1b1d6f89fc928914a20f21386d6` (Developer ID + notarization; replaces the macOS archive).
4. **attest** → `attest.yml@290feb9` — repo's only `id-token/attestations` grants; `actions/attest@1e69f48` v4.2.2; SLSA build-provenance over FINAL archive + SBOM, SBOM attestation (subject archive, predicate CycloneDX); checksums recomputed from final bytes; manifest records unsigned→signed transformation + notarization evidence.
5. **verify** — `scripts/verify-release.sh` (checksums, `gh attestation verify` with `--signer-workflow`/`--signer-digest`, SBOM predicate match, macOS codesign/notarization).
6. **publish** — tag push only; `workflow_dispatch` on main is a full non-publishing rehearsal.

Other workflows (product/CI, all unowned by Armorer): `momus.yml` (action self-test), `review.yml` (PR review via local action), `scan.yml` (sharded scan, builds `--bin momus`), `scripts.yml` (shellcheck, script tests, audit with `--advisory-db-outage warn`, product fixtures), `consumer-smoke.yml`, `rust-docs.yml`. Consumer contract: `action.yml` + `scripts/verify-release.sh` + `docs/slsa.md`.

## 4. Draft configs (in this directory; copies also at snapshot root for plan/check)

- `armorer.toml` — 1 deliverable: `momus` (cli, package `momus-review`, binary `momus`, all 3 targets, feature_set `default` = default_features true, no features). Toolchain `1.98.1` (matches existing pin). Policy: `license_file = "LICENSE"`, `attestations = "required"`.
- `ci-policy.draft.toml` — **new protection** (Momus has no cargo-deny today). Permissive-only allowlist derived from the observed closure (§6). Validated by `bootstrap check/plan` (exit 2/0; no parse errors).

## 5. Ownership conflict map

| Existing asset | Armorer v0.1 disposition | Conflict / required decision |
|---|---|---|
| `rust-toolchain.toml` (channel 1.98.1, minimal, comment header) | **OWNED** (fixed target of plan/apply + bootstrap) | **conflict** in plan/check: Armorer normalizes to `channel="1.98.1" profile="minimal" components=["clippy","rustfmt"]` — adds components, strips the audit comment header. Semantics preserved; owner must accept normalization or hand-merge comments (M3). |
| `release.yml` + `builder.yml` + `attest.yml` (three-target build/SBOM/sign/attest/verify/publish) | **NOT owned — v0.1 ships no release runtime** (`release-runtime-unavailable`) | Must remain fully unowned and unchanged. Armorer bootstrap only adds `armorer-ci.yml` (PR/push CI) + `armorer-build.yml` (manual **unsigned** build, workflow_dispatch). No duplication today; future release-runtime adoption must not regress: final-byte-only attestation order, signer-digest pinning, SBOM predicate verification, notarization evidence, rehearsal semantics (M4). |
| `scripts/audit.sh` + `audit-exceptions.txt` (cargo-audit 0.22.2) | Not owned; bootstrap CI runs catalog **cargo-deny 0.20.2** instead | Different tool, overlapping advisory scope. Keep audit.sh on the release path until owner decides consolidation (M5). Draft ci-policy exceptions list is empty, matching the empty exception file. |
| `scripts/verify-release.sh`, `docs/slsa.md`, `action.yml`, consumer-smoke | Not owned | Preserve; consumer verification contract unchanged. |
| `momus.yml`, `review.yml`, `scan.yml`, `scripts.yml`, `rust-docs.yml` | Not owned | Preserve (product CI). Armorer CI caller is additive. |
| `momus-eval` binary | Not selected | Owner decision M2: release asset or internal tool (draft: internal). |
| `armorer.lock`, `.armorer/ci-policy.toml`, `armorer-ci.yml`, `armorer-build.yml` | **Would be created** by bootstrap apply (not done) | New files only; no existing-file overwrite except rust-toolchain.toml. |

**Runner-label / glibc regression risk (cross-cutting).** Armorer's evidence runner
bindings are fixed (`src/trust/evidence.rs:247-251`): `x86_64-unknown-linux-gnu →
ubuntu-24.04`, `aarch64-unknown-linux-gnu → ubuntu-24.04-arm`, `aarch64-apple-darwin
→ macos-15`. Momus's builder deliberately uses **ubuntu-22.04 / ubuntu-22.04-arm**
("22.04 builds link an older glibc, so they run on more hosts") and **macos-14**.
Adopting Armorer's build/evidence path would move Momus to newer glibc + macos-15,
**regressing the documented older-glibc compatibility guarantee**. This is an
ownership conflict that must be resolved before any Armorer build owns Momus
releases (owner decision M9); it is a reason to keep the existing builder unowned.

**Live cross-pilot consumer.** Rusty Brain's `.github/workflows/momus.yml` runs
`brianluby/momus-review@v0.9.0` with `verify-attestations: required` on every PR.
Any change to Momus's attestation signer/identity or release contract directly
affects Rusty Brain CI. Adoption must preserve the v0.9.0 attestation contract
that this consumer pins (coordinated with ticket #13).

## 6. License closure (cargo-deny 0.19.7 list, offline, all-features graph)

MIT (141 crates), Apache-2.0 (115), Unicode-3.0 (19), ISC (5), BSD-3-Clause (2), Apache-2.0 WITH LLVM-exception (1), CDLA-Permissive-2.0 (1), Unlicense (3: aho-corasick/memchr/bstr — all `Unlicense OR MIT`), BSL-1.0 (1: ryu — `Apache-2.0 OR BSL-1.0`). No copyleft. Draft allowlist carries exactly these 9 tokens; owner approval required.

## 7. v0.2.0 historical compatibility (verify-historical-bytes allowlist shape)

`git show v0.2.0:.github/workflows/release.yml` confirms the pre-attestation pipeline: build matrix (same 3 targets) → `momus-<target>.tar.gz` + `.sha256` → sign-macos → publish + moving `v0` major tag. **No builder.yml/attest.yml, no SBOMs, no provenance bundles, no release manifest.** Tag `v0.2.0` = commit `3b99f6434cf4f9df70028de0188d6992471d2a56`.

Real published assets (`gh release view v0.2.0`, read-only; digests are GitHub-reported, owner must independently re-hash downloads before approving):

| Asset | Size (bytes) | SHA-256 |
|---|---|---|
| momus-aarch64-apple-darwin.tar.gz | 4,192,741 | 715d46889a326cceed52a31aa7e5e5a956537413a19a528160eb9f3abb41a567 |
| momus-aarch64-apple-darwin.tar.gz.sha256 | 100 | cb6c954ca7299edebaa756d73dc93f975bf589619401b234fe51cbd7ccbb0be1 |
| momus-aarch64-unknown-linux-gnu.tar.gz | 4,621,263 | 0084f581a303701f903aada4947823d5295e90957d138f68dff937da6e88c9c4 |
| momus-aarch64-unknown-linux-gnu.tar.gz.sha256 | 105 | 5213f6de356159ae8129cd25e4ee342f69074ecc0b4a47d169f218caa7d04c1e |
| momus-x86_64-unknown-linux-gnu.tar.gz | 4,706,043 | 7e9f1170cc58d0ca88df160e3ce7d557c89d88aba12b4d648d56aa1471b69d26 |
| momus-x86_64-unknown-linux-gnu.tar.gz.sha256 | 104 | b5ed91a2cde6bd8a4d239bb747c48fa2594505468a943a133bd65cf16ec1908d |

Allowlist shape (v1 verification policy, per `docs/historical-verification.md` + `examples/trust-v1/historical-verification-policy.json`): one `historical[]` entry with `source {repository: "brianluby/momus-review", commit: "3b99f6434cf4f9df70028de0188d6992471d2a56", git_ref: "refs/tags/v0.2.0"}`, `assets {<name>: {sha256, size}}` for **exactly** the 6 assets above (the command requires a complete directory match — no extra/missing files), per-entry `review {owner, rationale, reviewed_at, expires_at, record{sha256,size}}`, `limitations: ["Exact reviewed bytes only; no provenance authentication or SLSA level"]`. The policy also requires root `review` + sigstore `roots` fields by contract even though this operation does not use them. Invocation: `armorer verify-historical-bytes --directory <assets-only-dir> --policy <independently-reviewed-policy.json> --expect-policy-sha256 <approved> --source-repository brianluby/momus-review --source-commit 3b99f64… --source-tag v0.2.0`. Policy bytes + digest must travel through an independent reviewed channel; a downloaded policy cannot approve itself. **Historical byte match establishes no provenance** — v0.2.0 consumers keep using checksums + Apple notarization as before; Armorer authenticated verification must never silently fall back to this path.

## 8. Toolchain / catalog qualification gaps

- Armorer CLI itself is built/qualified with **Rust 1.95.0** (`rust-toolchain.toml`, `docs/readiness.md`); bootstrap catalog qualification (`docs/bootstrap-catalog.md` line 89) is "Pinned Rust 1.95.0 locked build, fmt, strict all-target/all-feature Clippy…". Catalog tools: actionlint 1.7.12, cargo-cyclonedx 0.5.9, cargo-deny 0.20.2, cyclonedx 0.33.1, gitleaks 8.30.1, zizmor 1.30.1 (18 target-qualified pins, workflow commit `772ca83e386c883c88cc3b936d69f8cb3216c91e`).
- **Neither 1.98.1 (Momus) nor 1.100.0 (Rusty Brain) is qualified** with that catalog. Discovery proved metadata + plan/check work on 1.98.1, but catalog-tool operation under 1.98.1 (cargo-deny 0.20.2 vs local 0.19.7, cargo-cyclonedx 0.5.9, SBOM schema validation) needs a qualification run before adoption (M6).
- Momus builder installs cargo-cyclonedx via `cargo install --locked` under the repo pin; Armorer's catalog uses prebuilt native distributions — different trust paths, both pinned; do not mix silently.
- cargo-audit 0.22.2 (Momus release gate) is **not in the Armorer catalog**; keeping it is an unowned dependency (M5).

## 9. Every unmet prerequisite from check output (exit 2)

`state: "configuration-valid"`, `capability_state: "unknown"`, findings (verbatim codes):
1. `github-capabilities-unchecked` — visibility, plan eligibility, immutable releases, rulesets, environments need authenticated preflight (public repo: attestations + environment reviewers available per docs/platform-prerequisites.md).
2. `license-policy-unreviewed` — LICENSE presence inspected; dependency license policy (draft ci-policy) needs owner review.
3. `release-runtime-unavailable` — no reviewed reusable workflow/runtime catalog in this slice; secure release readiness blocked.
4. `reviewed-lock-missing` — armorer.lock needs reviewed workflow + tool pins (bootstrap plan renders it; apply not performed).
5. `toolchain-conflict` — existing rust-toolchain.toml customization preserved; normalization conflict (§5) must be resolved before apply.

Bootstrap check/plan findings (policy accepted): `github-capabilities-unchecked`, `release-runtime-unavailable` ("Callers provide CI and manual unsigned builds; secure publication, final-byte attestations and Apple finalization remain required"). `recovery_required: false`.

## 10. Owner decisions (all unapproved)

- **M1** — Approve draft `armorer.toml` (single `momus` cli deliverable, 3 targets, 1.98.1) and `ci-policy.draft.toml` (9-token permissive allowlist, empty exceptions).
- **M2** — Confirm `momus-eval` stays an internal tool (not an Armorer deliverable / release asset).
- **M3** — Accept rust-toolchain.toml normalization (adds `components=["clippy","rustfmt"]`, drops comment header) or hand-merge; resolve `toolchain-conflict` before any apply.
- **M4** — Keep release.yml/builder.yml/attest.yml + apple-signing + verify-release.sh fully unowned; define future coexistence so Armorer never duplicates/regresses final-byte attestation, signer pinning, SBOM predicate checks, notarization evidence, rehearsal semantics. **Cross-pilot consumer contract:** Rusty Brain's `momus.yml` consumes `brianluby/momus-review@v0.9.0` with `verify-attestations: required` on every PR — any Momus release-process change under Armorer adoption MUST keep that consumer's attestation contract intact (coordinate with ticket #13).
- **M5** — Decide advisory-tooling coexistence: keep cargo-audit 0.22.2 release gate alongside catalog cargo-deny 0.20.2 CI, or consolidate (with exception-format migration `ID|date|reason` → structured expiring exceptions).
- **M6** — Qualify catalog tools under Rust 1.98.1 before adoption (1.95-era qualification gap).
- **M7** — Independently download + re-hash the six v0.2.0 assets, then review/approve a historical-verification policy (§7 shape) through an independent channel; record limitations; keep authenticated verification separate.
- **M8** — Authorize (separately) any bootstrap apply, hosted rehearsal, or publication. Nothing in this discovery authorizes writes to the pilot.
- **M9** — Runner-label/glibc conflict (§5): Armorer's evidence contract (`src/trust/evidence.rs:247-251`) hard-binds ubuntu-24.04 / ubuntu-24.04-arm / macos-15, so Momus's deliberate ubuntu-22.04 older-glibc builds CANNOT be represented in Armorer evidence without a contract change. Either/or: **(a)** adopt Armorer CI for the supported labels while preserving the ubuntu-22.04 release path unowned, OR **(b)** defer until a reviewed runner-mapping extension. **Do not silently rebind** the evidence runner labels.

## 11. Evidence files (this directory)

`armorer.toml` (draft, also copied to snapshot root), `ci-policy.draft.toml`, `plan.json` (exit 0), `check.json` (exit 2), `bootstrap-plan.json` (exit 0), `bootstrap-check.json` (exit 2), `plan.err`/`check.err`/`bootstrap-*.err` (empty). Plan digest `86a86e1fb1991f89b7f77c44b3b4650643c0221fcf0df4091ae9a0004ce5d52f`; config digest `2760f511911b2fbaf5f720c401454cec963dfd8219232d98df199427997a941b`. External: `/tmp/momus-metadata.json`, `/tmp/momus-deny-list.json`, `/tmp/momus-v020-assets.json`, `/tmp/armorer-catalog.json`.

## 12. Final non-interference verification (2026-11-12)

- `git -C /Users/bluby/repos/momus-review status --porcelain` shows only **pre-existing user work** (untracked `.claude/`, `.opencode/`, `.serena/`, `.veans.yml`, `docs/research/`, `tests/jeeves_research.rs`) — none created or touched by this discovery.
- No `armorer.toml` or `adoption/` exists in the primary checkout (verified absent).
- `git status --porcelain -- rust-toolchain.toml Cargo.toml Cargo.lock .github` → empty (release inputs untouched).
- HEAD unchanged: `bb64d2f3b09c32cce75ced029fd1065f0e34ef61` on `main`. No commits, no pushes, no tag moves.
- Snapshot `/tmp/armorer-pilot-momus` diff vs clone: only untracked `armorer.toml` (draft copy) + `adoption/` outputs. Bootstrap plan digest `633389abb8dd85bd6ab17442cc50f8c5326ee1d1fe88219fefe87eda653186d6`.