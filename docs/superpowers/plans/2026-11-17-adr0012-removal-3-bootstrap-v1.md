# ADR 0012 Removal 3: retire bootstrap-v1 production catalog — Plan

Spec: ADR 0012 row 4 + rule 3. Veans #33 (1498). Branch adr0012/remove-bootstrap-v1-catalog.

**Mechanism (ruling):** Cargo feature `test-predecessor` gates WORKFLOW_COMMIT, reviewed(), and the v1 arms of reviewed_for/id_for_commit/select. `[dev-dependencies] armorer = { path = ".", features = ["test-predecessor"] }` enables it for all test builds (lib+bin unify), so `cargo test --locked` stays the gate. Production `cargo build --locked` has no feature → bootstrap-v1 is an unknown-catalog error; a v1-pinned bootstrap state fails closed ("not a compiled catalog authority"). CI gains one step proving the production binary rejects `--catalog bootstrap-v1`.

**Changes:**
- Cargo.toml: feature + self dev-dep (no new external deps).
- src/catalog.rs: `authenticate()` binds V2 pin (public byte-auth stays); `reviewed()` + WORKFLOW_COMMIT → cfg(feature); v1 registry arms → cfg(feature); v1 unit-test halves gated.
- src/upgrade/authority.rs: v1 select arm cfg(feature); v1 unit tests gated.
- tests/catalog.rs: reviewed()/WORKFLOW_COMMIT → v2 equivalents; predecessor round-trip assertions in one gated test.
- tests/upgrade.rs: render helper → reviewed_v2; version-error tests target bootstrap-v2; cross-revision block + install_v1_bootstrap gated (rule 3 coverage preserved under feature).
- tests/inspection.rs: check_uses_the_upgrade_owners_catalog gated (v1-specific).
- development.yml: after schema build, assert production binary rejects bootstrap-v1 (exit 1, unknown-catalog error).
- Docs: bootstrap-catalog.md, upgrades.md, candidate-upgrades.md, credential-boundaries.md — bootstrap-v2 only authority; historical ocr receipts untouched. ADR status row 4.
- Gates: test --locked, build --locked + manual negative, clippy --all-features AND no-features, validators.

## Ledger
- Ruling: feature + self dev-dep instead of cfg(test) — integration tests use the public API; cfg(test) cannot reach them — cost: unusual but standard Cargo pattern.
- Ruling: authenticate() rebinds to V2 pre-release (ADR permits; it authenticates the same tool bytes, only the pin moves) — keeps byte-auth tests unchanged — cost: none, digest identical.
