# ADR 0012 Removal 2: retire legacy-v1/native-v2 verification contexts — Plan

> Executor: superpowers:executing-plans, inline. Spec: docs/adr/0012-pre-release-contract-consolidation.md row 3 + "Kept"/"Rules". Veans #32 (id 1497). Branch adr0012/remove-legacy-contexts, stacked on removal 1.

**Goal:** only `native-v3` (`verification-context-v3`) survives. Removed kinds/readers/schemas are errors or absent — never fallback. Inner `ReleaseExpectations` keeps its shape (ADR "Kept").

## Global constraints
Same as removal 1 (rtk, fmt/clippy/test gates, hash-pinned schema venv at /tmp/schema-venv-path via python3.14, no merge/tag). `native-catalog-v2` and `runtime-distribution-v1` schemas SURVIVE (v3 embeds them). `CatalogAuthority::NativeV2` variant name survives (it names the catalog format, not the context).

## Task 1 — failing tests
- tests/integrated_cli.rs: removed kinds `legacy-v1`/`native-v2` must be rejected (clap exit 2); loop shrinks to native-v3.
- tests/release_context.rs: writing only `armorer-verification-context.json` (v1) or `-v2.json` with an approved digest must fail (file-not-found for v3 name), never fall back.
Run; watch fail.

## Task 2 — the cut (one commit)
- src/verification/context.rs: delete `open`, `open_native_v2`, `CONTEXT_NAME`, `NATIVE_CONTEXT_NAME`, `ReleaseExpectationsV2`, `CatalogAuthority::Legacy`; collapse `open_version` into `open_native_v3` (always FEATURE_CONTEXT_NAME, always ReleaseExpectationsV3, always Some(runtime_source)/Some(root_features)). `runtime_distribution()` keeps Result signature, always Ok. Keep every `require` check.
- src/main.rs: ContextKind keeps only NativeV3; open_trusted_context single arm; remove SchemaKind::VerificationContext + VerificationContextV2 and arms.
- schemas: delete verification-context-v1.json, verification-context-v2.json (v3 unchanged — verify byte-equal). CONTRIBUTING loop drops both entries; 26→24 counts.
- tests/release_context.rs: generic `fixture()` emits v3 outer (schema_version 3, runtime_source from lock workflows + synthetic source, root_features per selection key — copy pattern from native_v3 test :1241); `open()` helper → open_native_v3 + FEATURE_CONTEXT_NAME. native_fixture/open_native → v3 (add root_features, bump version). v2-fallback test → removed-kind rejection. legacy-context-runtime test: drop the legacy half (no legacy reader exists).
- tests/publication_cli.rs: kind → native-v3 (digest check precedes parsing; same behavior).
- tests/trust_contracts.rs: schema list drops removed kinds if present.
- examples: delete examples/verification-context-v1/, examples/runtime-native-v2/context.json. scripts/validate-verification-context.py → v3 schema + context-v3.json example (adjust mutation paths to nested release). scripts/validate-runtime-native.py drops v2 case. scripts/test-review-gates.py both-contexts test → v3 only.

## Task 3 — docs + gates
cli-reference (schema table, 24 names), candidate-verification.md (context-kind table), runtime-native-v2.md, verify-release-cli.md, authenticated-inventory.md, resolved-root-features-v3.md: remove legacy/v2 rows, note removal per ADR 0012. ADR status: row 3 implemented on this branch. Full gates + all validators + review-gates. Push; PR stacked on #46; update #32 with readback.

## Ledger
- Ruling: generic test fixture rebuilt on native_fixture (v3 outer + native catalog); old generic fixture renamed template_fixture and only feeds native_fixture — every semantic mutation test survives on the surviving contract — cost if wrong: v1-catalog-specific admission paths lose direct coverage; they no longer exist in the reader.
- Ruling: open() test helper rebuilds the v3 outer from the mutated inner release (write_v3); root_features always [] per selection key — matches the pre-existing v3 test pattern — cost if wrong: none, reader validates keys not values.
- Ruling: legacy-context runtime rejection test half deleted — no legacy reader exists to reject; cross-format rejection kept as v2-outer-under-v3-name — cost if wrong: none.
- Ruling: CatalogAuthority::NativeV2 variant name kept — it names the surviving native-catalog-v2 format, not the removed context (ADR "Kept") — cost if wrong: cosmetic.
- Verification: fmt/clippy clean; 259 tests pass; v3 schema byte-unchanged; all six validators + test-review-gates OK; removed kinds exit 2 at clap (watched fail→pass).
