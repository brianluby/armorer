# Rusty Brain pilot adoption — discovery report (plan step 20, ticket #13)

Date: 2026-11-12. Status: **DISCOVERY + DRAFTS ONLY — PROPOSED, UNAPPROVED.**
No file in the Rusty Brain primary checkout (`/Users/bluby/repos/rusty-brain`)
was modified; no branch was touched; nothing committed or pushed. All work ran
in the disposable clone `/tmp/armorer-pilot-rusty-brain` (cloned from the local
primary with `--no-hardlinks`, origin reset to
`https://github.com/brianluby/rusty-brain.git`; no fetch from GitHub).

- Snapshot: `main` @ `1694a38` ("Merge pull request #107 from brianluby/chore/82-coverage-baseline")
- Armorer binary: `/Users/bluby/repos/armorer/target/debug/armorer`, `runtime_version` 0.1.0
- Toolchain: repo uses `channel = "stable"` (mutable). `rustup check` (2026-11-12): local stable 1.95.0, **current published stable 1.100.0 (0ef1bc386 2026-11-09)**. Installed `1.100.0` (`rustup toolchain install 1.100.0 --profile minimal`, exit 0; `rustc 1.100.0 (0ef1bc386 2026-11-09)`). Discovery used 1.100.0. **The exact pin is PROPOSED; owner must approve replacing mutable stable (R1).**

## 1. Exact commands run and observed results

| # | Command (cwd) | Exit | Output |
|---|---|---|---|
| 1 | `rustup check` | 0 | stable update available 1.95.0 → 1.100.0 (0ef1bc386 2026-11-09) |
| 2 | `rustup toolchain install 1.100.0 --profile minimal` | 0 | installed; `rustup run 1.100.0 rustc --version` = 1.100.0 |
| 3 | `git clone --no-hardlinks /Users/bluby/repos/rusty-brain /tmp/armorer-pilot-rusty-brain` + origin reset | 0 | snapshot 1694a38 |
| 4 | `env RUSTC_WRAPPER= rustup run 1.100.0 cargo metadata --no-deps --offline --locked --format-version 1` (snapshot) | 0 | `/tmp/rb-metadata.json` — 18 workspace members |
| 5 | `armorer --repository . plan` (config only in `adoption/`, first attempt) | **1** | `{"error":{"code":"io","message":"cannot read local input: No such file or directory (os error 2)"}}` — armorer.toml must be at repo root |
| 6 | `cp adoption/armorer.toml armorer.toml` (snapshot root only) then `armorer --repository . plan` | **0** | `adoption/plan.json` (38,838 B) |
| 7 | `armorer --repository . check` | **2** (expected until ready) | `adoption/check.json` (38,839 B) |
| 8 | `armorer --repository . bootstrap check --policy adoption/ci-policy.draft.toml` | **2** | `adoption/bootstrap-check.json` — draft policy parsed/validated |
| 9 | `armorer --repository . bootstrap plan --policy adoption/ci-policy.draft.toml` | **0** | `adoption/bootstrap-plan.json` (55,285 B) |
| 10 | `env RUSTC_WRAPPER= rustup run 1.100.0 cargo deny --locked --offline check licenses bans` (cargo-deny 0.19.7, repo deny.toml) | **0** | `bans ok, licenses ok`; warnings: `Unicode-DFS-2016` allowance unmatched; 3 duplicate `base64` versions (warn policy) |
| 11 | `env RUSTC_WRAPPER= rustup run 1.100.0 cargo deny --locked --offline list --format json` | 0 | `/tmp/rb-deny-list.json` — license closure |

`cargo deny check advisories` was **not** run (requires advisory-db network fetch; out of scope for offline discovery). No `cargo build`/`cargo test` in the pilot. Armorer plan/check/bootstrap do no builds, no network, no consumer writes.

## 2. Workspace inventory (cargo metadata, exit 0) — 18 crates, all v0.0.1, edition 2021, MIT, no declared rust-version (MSRV)

| Crate | Kind | Binaries | Features |
|---|---|---|---|
| rb-types | lib | — | — |
| rb-config | lib | — | — |
| rb-store | lib | — | bench-utils |
| rb-redact | lib | — | — |
| rb-proto | lib | — | — |
| rb-embed | lib | — | **local** = dep:fastembed, dep:hf-hub, dep:tokio |
| rb-search | lib | — | — |
| rb-engine | lib | — | — |
| rb-enrich | lib | — | — |
| rb-daemon | lib | — | — |
| rb-mcp | lib | — | — |
| rb-tokens | lib | — | — |
| **rusty-brain** | lib + bin | **rusty-brain** | **local** = rb-embed/local |
| rb-agents | lib | — | — |
| **rb-hooks** | bin | **rusty-brain-hooks** | — |
| **rb-install** | bin | **rusty-brain-install** | — |
| rb-eval | lib + bins | assertion-precision, coding-memory, external-bench | record-local = rb-embed/local |
| rb-contract-guard | bin | rb-contract-guard | — |

Ticket #13 deliverables = `rusty-brain` / `rb-hooks` / `rb-install` — exactly what `release.yml` builds: `cargo build --release --locked --target <t> -p rusty-brain -p rb-hooks -p rb-install`, packaged as one tarball `rusty-brain-<version>-<target>.tar.gz` containing all three binaries. Binary names differ from package names for the two agent crates (verified via metadata). `rusty-brain` is daemon + CLI (`rusty-brain serve` runs the memory daemon in foreground) → draft profile `service`; hooks/install are CLIs.

Feature sets (ticket: default/local): `default` = default_features true, no features (release ships this); `local` = `["local"]` on rusty-brain → ONNX backend via fastembed/ort. Existing CI `build-local` job builds `-p rusty-brain --features local` on **ubuntu-latest only**; release ships default features only. `rb-eval`'s `record-local` is the same backend for evaluation tooling (not a release deliverable).

## 3. Existing release pipeline and the unsupported-target gap

`release.yml` (tags `v*` + workflow_dispatch): preflight (`scripts/release-preflight.sh` — tag/version/changelog/ancestry) → build matrix:

| Target | Runner |
|---|---|
| aarch64-apple-darwin | macos-latest |
| **x86_64-apple-darwin** | **macos-15-intel** |
| x86_64-unknown-linux-gnu | ubuntu-latest |

Build jobs hold `id-token: write` + `attestations: write` and run `actions/attest-build-provenance@4d101475` (v4.2.2) over `dist/*.tar.gz`; smoke job runs `scripts/verify-fresh-install.sh` per native runner; publish (tag push only) via `softprops/action-gh-release@3bb1273` + `SHA256SUMS`. **No Apple Developer ID signing/notarization, no SBOMs, no release manifest** (contrast Momus).

**Armorer v0.1 target enum** (`src/config.rs:12-16`, confirmed): `aarch64-apple-darwin`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-gnu`. **`x86_64-apple-darwin` is genuinely unsupported.** Owner decision **R2** with two options:

- **(a) RECOMMENDED for the rehearsal:** adopt Armorer for the three supported targets only; preserve the existing Intel-mac release path **unowned** (ownership map §5). Draft `armorer.toml` implements (a). Gap documented here; Intel-mac tarballs keep flowing through the existing release.yml unchanged.
- **(b)** Defer Rusty Brain adoption until a **reviewed target-enum extension**. That is a contract change — config schema, evidence runner mapping (`src/trust/evidence.rs:247-251`), native catalog targets, CI matrix — **not a quick fix; not attempted in this discovery.**

Runner-label note: even for the supported `aarch64-apple-darwin`, Armorer's evidence binding is fixed to **`macos-15`** (`src/trust/evidence.rs:250`), while the repo's Intel path uses the distinct **`macos-15-intel`** label and the ARM path uses `macos-latest`. Any future evidence adoption must reconcile labels explicitly; `macos-15-intel` has no Armorer mapping at all. Also note the draft **adds `aarch64-unknown-linux-gnu`**, which the current release matrix does not ship (owner decision R6).

## 4. Draft configs (this directory; armorer.toml also copied to snapshot root)

- `armorer.toml` — toolchain `1.100.0` (PROPOSED exact pin, R1); 4 deliverables: `rusty-brain` (service, 3 supported targets, default), `rusty-brain-local` (service, x86_64-unknown-linux-gnu only — mirrors CI build-local; R3), `rb-hooks` (cli, binary rusty-brain-hooks, 3 targets, default), `rb-install` (cli, binary rusty-brain-install, 3 targets, default). Feature sets `default` + `local`. Policy LICENSE + attestations required.
- `ci-policy.draft.toml` — reconciled from `deny.toml` (§6). Validated by bootstrap check/plan (exit 2/0, no parse errors; exception expiry 2027-02-10 = today+90 within the enforced 0–90-day window).

## 5. Ownership conflict map

| Existing asset | Armorer v0.1 disposition | Conflict / required decision |
|---|---|---|
| `rust-toolchain.toml` (`channel="stable"`, components rustfmt+clippy) | **OWNED** | **conflict**: proposed `channel="1.100.0" profile="minimal" components=["clippy","rustfmt"]`. Mutable→exact pin is owner decision R1. **Caveat:** every CI job passes `toolchain: stable` explicitly to `dtolnay/rust-toolchain@29eef33`, which overrides the file — pinning the file alone does NOT pin CI; workflows must be updated in the same reviewed change. |
| `release.yml` Intel-mac path (`x86_64-apple-darwin`, macos-15-intel) | **CANNOT be owned** (target unsupported) | R2 option (a): keep unowned, unchanged. Ownership split: Armorer owns the 3 supported targets' intent; existing release.yml keeps publishing all 3 current targets incl. Intel until a reviewed cutover. |
| `release.yml` preflight/build/attest/smoke/publish | **NOT owned — v0.1 ships no release runtime** (`release-runtime-unavailable`) | Preserve entirely. Armorer bootstrap adds only `armorer-ci.yml` (PR/push CI caller) + `armorer-build.yml` (manual **unsigned** build caller), both pinned to `brianluby/armorer-workflows@772ca83e`. No duplication of attest-build-provenance today; future release runtime must not regress it. |
| `deny.toml` + `cargo-deny` CI job (EmbarkStudios/cargo-deny-action@3c63498) | Not owned; bootstrap CI runs catalog cargo-deny 0.20.2 with `.armorer/ci-policy.toml` | **Two deny configs would coexist.** ci-policy-v1 cannot express crate-scoped license exceptions (option-ext MPL-2.0), wildcards/highlight/skip, or all-features graph (§6) → existing deny.toml job MUST stay (R4/R7). |
| `cargo-audit` CI job (rustsec/audit-check@69366f3, checks:write) | Not owned; not in Armorer catalog | Preserve (R7). |
| `ci.yml` fmt / clippy-test (ubuntu+macos, --all-features -D warnings) / build-local / build-agents / contract-drift | Not owned | Preserve — bespoke semantic/contract CI (R7). contract-drift runs `cargo run -p rb-contract-guard -- check` against contract-snapshot.toml + `scripts/release-preflight.test.sh`. |
| `semantic-quality.yml` (weekly offline replay, rb-eval semantic_gate goldens) | Not owned | Preserve (bespoke). |
| `memory-scorecard.yml`, `nightly-claude-smoke.yml` (scheduled, external LLM/CLI) | Not owned | Preserve (bespoke; external downloads §7). |
| `momus.yml` (PR review via `brianluby/momus-review@v0.9.0`, `verify-attestations: required`) | Not owned | Preserve; **live cross-pilot dependency on Momus ticket #12** — Momus attestation contract changes hit this job (R9). |
| `armorer.lock`, `.armorer/ci-policy.toml`, `armorer-ci.yml`, `armorer-build.yml` | **Would be created** by bootstrap apply (not done) | New files only; sole existing-file overwrite is rust-toolchain.toml. |

## 6. deny.toml → ci-policy-v1 reconciliation (read in full)

| deny.toml | ci-policy-v1 draft | Notes |
|---|---|---|
| `[licenses] allow` (12 tokens incl. Zlib, NCSA for fastembed closure; confidence-threshold 0.9; permissive-only) | `licenses.allow` — all 12 carried over | Syntax-checked by Armorer; SPDX recognition/coverage stays with enforced cargo-deny CI. |
| `[[licenses.exceptions]]` option-ext `=0.2.0` → MPL-2.0 (weak copyleft, only via local feature: dirs-sys→dirs→hf-hub) | **NOT REPRESENTABLE** — `Licenses` has only `allow`, `additionalProperties=false` | deny.toml + its CI job must remain (R4). Do NOT add MPL-2.0 to global allow. |
| `[advisories] version=2, yanked="deny", ignore=["RUSTSEC-2024-0436"]` (paste unmaintained; build-time proc-macro via local feature only) | `advisories.exceptions` structured entry: id RUSTSEC-2024-0436, owner PROPOSED/unapproved, reason carried from deny.toml, **expires "2027-02-10"** (today+90; ci_policy.rs enforces 0–90 days) | R5: approve time-boxed exception or upgrade fastembed off paste. yanked=deny has no ci-policy field; stays in deny.toml. |
| `[bans] multiple-versions="warn", deny=[]` | `bans.multiple_versions="warn"`, `bans.deny=[]` | Observed: 3 base64 duplicates (warn). |
| `[bans] wildcards="warn", highlight="all", skip=[], skip-tree=[]` | **NOT REPRESENTABLE** | Stay in deny.toml CI. |
| `[sources] unknown-registry="deny", unknown-git="deny", allow-registry=[crates.io]` | `sources.allow_git=[]` | No git deps in Cargo.lock (verified). |
| `[graph] all-features=true` | **NO FIELD** | Existing cargo-deny job keeps all-features coverage. |

Observed license closure (cargo-deny 0.19.7 list, all-features, offline): MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-3-Clause, ISC, Unicode-3.0, CDLA-Permissive-2.0, Zlib, BSL-1.0 (ryu — `Apache-2.0 OR BSL-1.0`), Unlicense (aho-corasick/memchr — `Unlicense OR MIT`), LGPL-2.1-or-later (r-efi 6.0.0 — `MIT OR Apache-2.0 OR LGPL-2.1-or-later`, via getrandom 0.4.2 UEFI-only target dep). All disjunctive cases satisfy the permissive allowlist; `check licenses bans` exit 0 confirms. `Unicode-DFS-2016` allowance currently unmatched (warning only). Local cargo-deny is **0.19.7**; Armorer catalog pins **0.20.2** — version delta to qualify (R8).

## 7. External model / native downloads (ticket #13 requirement)

No `build.rs` exists in any crate (verified by find). Downloads come from dependency build scripts and runtime code:

1. **Native ONNX Runtime (build-time, `local` feature only).** fastembed 5.15.0 default features include `ort-download-binaries-native-tls` → ort 2.0.0-rc.12 `download-binaries` → ort-sys build script downloads prebuilt ONNX Runtime **1.24.2** from `https://cdn.pyke.io/0/pyke:ort-rs/ms@1.24.2/<target>.tar.lzma2`, hash-verified against `ort-sys/build/download/dist.txt` (17 entries). Relevant entries: `none/x86_64-unknown-linux-gnu` sha256 `acc1cba79c337594ead1d88ca72516147aa60054c84217b53399a31caa5ba671`; `none/aarch64-unknown-linux-gnu` `7e4f5fec4494cbf578c4e28082b0229c42f735523f584259028dde96acf3b092`; `none/aarch64-apple-darwin` `612739f75438dc0a075461e1fb454226b4a1eb175e60a7271ba966bbbb972cd4`. **There is NO `x86_64-apple-darwin` entry (0 of 17)** — the local feature cannot fetch a prebuilt runtime for Intel macOS, consistent with CI building `local` on ubuntu only. This download is outside Armorer's catalog trust path; adopting a `local` deliverable requires independent review of this native distribution (R3/R8).
2. **Embedding model (runtime, `local` feature only).** `rb-embed/src/local.rs` uses hf-hub 0.5 against `HF_ENDPOINT` (default `https://huggingface.co`), repo **`Qdrant/all-MiniLM-L6-v2-onnx`** pinned at revision **`5f1b8cd78bc4fb444dd171e59b18f3a3af89a079`**, 5 files each SHA-256-pinned and fail-closed on mismatch: model.onnx `bbd7b466f6d58e646fdc2bd5fd67b2f5e93c0b687011bd4548c420f7bd46f0c5`; tokenizer.json `da0e79933b9ed51798a3ae27893d3c5fa4a201126cef75586296df9b4d2c62a0`; config.json `1b4d8e2a3988377ed8b519a31d8d31025a25f1c5f8606998e8014111438efcd7`; special_tokens_map.json `5d5b662e421ea9fac075174bb0688ee0d9431699900b90662acd44b2a350503a`; tokenizer_config.json `bd2e06a5b20fd1b13ca988bedc8763d332d242381b4fbc98f8fead4524158f79`. Cache: `HF_HOME`/fastembed cache dir; optional `HF_TOKEN`; offline after first load. Pin exists because upstream fastembed commit 944c7f6 silently changed model_max_length 512→256 (Vikunja #100).
3. **Remote embedding API (runtime, default path).** Voyage: `https://api.voyageai.com/v1` (`rb-embed/src/voyage.rs`), API-key authenticated — network service, not a file download.
4. **CI-only externals:** `npm install -g @anthropic-ai/claude-code` (nightly-claude-smoke.yml, memory-scorecard.yml); LiteLLM endpoint probe `curl ${LITELLM_BASE_URL%/}/models` + model `litellm/deepseek-v4.1-flash` (memory-scorecard.yml); `gh release download` in `scripts/verify-fresh-install.sh` (release smoke).
5. **Vendored native, no download:** rusqlite `bundled` compiles SQLite from vendored C; sqlite-vec likewise. Default closure is fastembed-free by design (rb-embed feature comment).

## 8. Toolchain / catalog qualification gaps

- Armorer CLI + bootstrap catalog qualified under **Rust 1.95.0** (docs/bootstrap-catalog.md line 89; docs/readiness.md). Catalog: actionlint 1.7.12, cargo-cyclonedx 0.5.9, cargo-deny 0.20.2, cyclonedx 0.33.1, gitleaks 8.30.1, zizmor 1.30.1; workflow commit `772ca83e386c883c88cc3b936d69f8cb3216c91e`.
- **1.100.0 is unqualified** with that catalog (1.95-era). Discovery proved metadata + plan/check/bootstrap work on 1.100.0; catalog-tool operation (esp. cargo-deny 0.20.2 vs locally verified 0.19.7) needs a qualification run before adoption (R8).
- Mutable `stable` in CI means today's CI already floats (local stable 1.95.0 vs published 1.100.0) — the pin decision R1 also fixes this drift; all `dtolnay/rust-toolchain` `toolchain: stable` inputs must move with it (§5).

## 9. Every unmet prerequisite from check output (exit 2)

`state: "configuration-valid"`, `capability_state: "unknown"`, findings (verbatim codes):
1. `github-capabilities-unchecked` — visibility, plan eligibility, immutable releases, rulesets, environments need authenticated preflight.
2. `license-policy-unreviewed` — LICENSE presence inspected; dependency license policy needs owner review (draft reconciles deny.toml, §6).
3. `release-runtime-unavailable` — no reviewed reusable workflow/runtime catalog in this slice; secure release readiness blocked.
4. `reviewed-lock-missing` — armorer.lock needs reviewed workflow + tool pins (bootstrap plan renders it; apply not performed).
5. `toolchain-conflict` — existing rust-toolchain.toml customization preserved; stable→1.100.0 normalization conflict must be resolved before apply (R1).

Bootstrap check/plan findings (policy accepted): `github-capabilities-unchecked`, `release-runtime-unavailable` ("Callers provide CI and manual unsigned builds; secure publication, final-byte attestations and Apple finalization remain required").

## 10. Owner decisions (all unapproved)

- **R1** — Approve exact toolchain pin **1.100.0** replacing mutable `stable` (current published stable as of 2026-11-12), AND update every CI `dtolnay/rust-toolchain` `toolchain: stable` input in the same reviewed change; accept normalization (`components=["clippy","rustfmt"]`, profile minimal).
- **R2** — Unsupported `x86_64-apple-darwin`: **(a) RECOMMENDED** — adopt Armorer for the three supported targets only, preserve the Intel-mac release path unowned (gap documented, §3/§5); or **(b)** defer adoption pending a reviewed target-enum contract extension (config schema + evidence runner mapping + native catalog + CI matrix; not a quick fix, not attempted). Note `macos-15-intel` label vs Armorer's fixed `macos-15` evidence binding.
- **R3** — `local` feature deliverable: draft includes `rusty-brain-local` on x86_64-unknown-linux-gnu only (mirrors CI build-local). Approve, or keep `local` CI-only and drop the deliverable. Adopting it pulls the cdn.pyke.io native ONNX Runtime download + HF model runtime download (§7) into the release trust discussion — both outside the Armorer catalog.
- **R4** — Keep `deny.toml` + its cargo-deny CI job unowned: ci-policy-v1 cannot express the option-ext `=0.2.0` MPL-2.0 crate-scoped exception, wildcards/highlight/skip bans, yanked=deny, or all-features graph. Two deny configs coexist by design until the schema grows.
- **R5** — Approve the time-boxed RUSTSEC-2024-0436 exception (owner/reason carried from deny.toml; expires 2027-02-10, re-review ≤90 days) or upgrade fastembed off `paste` first.
- **R6** — Draft adds `aarch64-unknown-linux-gnu` (not in today's release matrix). Approve the new target or remove it from the draft.
- **R7** — Preserve all bespoke CI unowned: ci.yml fmt/clippy-test/build-local/build-agents/contract-drift (rb-contract-guard + contract-snapshot.toml), deny + audit jobs, semantic-quality.yml weekly replay, memory-scorecard.yml, nightly-claude-smoke.yml, release.yml preflight/attest-build-provenance/smoke/publish. Armorer CI/build callers are additive only.
- **R8** — Qualify catalog tools under 1.100.0 (1.95-era gap), including cargo-deny 0.20.2 vs verified-local 0.19.7 behavior on this deny.toml.
- **R9** — Coordinate with Momus ticket #12: `momus.yml` consumes `brianluby/momus-review@v0.9.0` with `verify-attestations: required`; Momus adoption must not break that contract.
- **R10** — Authorize (separately) any bootstrap apply, hosted rehearsal, or publication. Nothing in this discovery authorizes writes to the pilot.

## 11. Evidence files (this directory)

`armorer.toml` (draft, also copied to snapshot root), `ci-policy.draft.toml`, `plan.json` (exit 0), `check.json` (exit 2), `bootstrap-plan.json` (exit 0), `bootstrap-check.json` (exit 2), `*.err` (empty). Plan digest `0f248c6a2cbf25cf85eb904579479c0e4e4c6c1a27b11d02df621767efec1bab`; config digest `a1f94928c8291e03bda344d5de3715b3da6b439034c2b6250e2d7000c7e2867a`; bootstrap plan digest `94a260aedd4ad9c230f836cba4e61fc7ddb8f4eb6cf955cbe04c7be575213bd5`. External: `/tmp/rb-metadata.json`, `/tmp/rb-deny-list.json`, `/tmp/rb-deny-licenses-bans.json` (stderr log of the exit-0 check), `/tmp/armorer-catalog.json`.

## 12. Final non-interference verification (2026-11-12)

- `git -C /Users/bluby/repos/rusty-brain status --porcelain` shows only **pre-existing user work** (active `beam-eval` feature: modified `.gitignore`, `Cargo.lock`, `crates/rb-eval/*`; untracked beam benches/docs/scripts) — none created or touched by this discovery; zero paths overlap `armorer`/`adoption`.
- No `armorer.toml` or `adoption/` exists in the primary checkout (verified absent).
- `git status --porcelain -- rust-toolchain.toml Cargo.toml deny.toml .github` → empty (toolchain/deny/CI inputs untouched).
- HEAD unchanged: `1694a389f1383b6a031005f7f4a5137a279b3359` on `main`; no feature branch disturbed. No commits, no pushes.
- Snapshot `/tmp/armorer-pilot-rusty-brain` diff vs clone: only untracked `armorer.toml` (draft copy) + `adoption/` outputs.
- Note: the primary's in-flight `Cargo.lock` edit means the clone's lock (used for all discovery) is the committed main lock, not the user's working-copy lock — correct isolation.