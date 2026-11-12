# Gate execution evidence — ticket #26 (partial), 2026-11-12

Workstation: macOS arm64 (darwin 27.2.0). Source: armorer main `ab526d7a0d3b547cc39041b52262e60173223b49`, clean.
This is local execution evidence only; no tracker write, no ticket completion claim, no release claim.

## Gate 1 — real_pinned_gh_release_attestation_matches_frozen_baseline: PASS

```sh
q=$(mktemp -d /tmp/armorer-gh-qual.XXXXXX)
python3 -I scripts/qualify-test-verifier.py --output-directory "$q"
# Qualified native GitHub CLI 2.102.0: /private/tmp/armorer-gh-qual.RMVoxS/gh
ARMORER_TEST_GH="$q/gh" RUSTC_WRAPPER= rustup run 1.95.0 cargo test --locked --lib \
  verification::publication::github::tests::real_pinned_gh_release_attestation_matches_frozen_baseline \
  -- --ignored --exact
# test result: ok. 1 passed; finished in 5.05s
```

Verifier identity: gh 2.102.0 darwin-arm64, archive/manifest/leaf digests matched
`tests/fixtures/sigstore/verifier-pins.json`; authentication: existing keyring gh session (read-only GETs and `release verify` only).

## Gate 2 — real_native_publication_observations_never_grant_write_authority: PASS

Same qualified binary and environment:

```sh
ARMORER_TEST_GH="$q/gh" RUSTC_WRAPPER= rustup run 1.95.0 cargo test --locked --lib \
  verification::publication::github::tests::real_native_publication_observations_never_grant_write_authority \
  -- --ignored --exact
# test result: ok. 1 passed; finished in 3.63s
```

Fixed GET routes for repository/settings/branches/rulesets/environments/actions; write-method refusal verified by the test's own negative assertions.

## Gate 3 — native_python_graphs_match_independent_rust_reader: PASS

Producer: workflows repo `brianluby/armorer-workflows` at `b94969c7030b8c1105882a1374ed4d7a3eab653a`
(current origin/main), cloned to `/tmp/armorer-wf-gate` (the stale `.worktrees/armorer-workflows`
checkout is 65 commits behind and lacks `tests/build_cases_v2.py`; do not use it).

Environment note: system python3 is 3.15.2, outside the hash-locked `requirements-schema.txt`
support range (CPython 3.11–3.14); pip correctly refused the unhashed cp315 rpds-py wheel.
Used `/opt/homebrew/bin/python3.14` venv at `/tmp/armorer-gate-venv`; all hashes matched.

```sh
git clone /Users/bluby/repos/armorer/.worktrees/armorer-workflows /tmp/armorer-wf-gate
git -C /tmp/armorer-wf-gate checkout b94969c7030b8c1105882a1374ed4d7a3eab653a
PYTHONPATH=/tmp/armorer-wf-gate:/tmp/armorer-wf-gate/tests /tmp/armorer-gate-venv/bin/python \
  tests/build_cases_v2.py --armorer /Users/bluby/repos/armorer/target/debug/armorer \
  --tool-directory "$t" --receipt-directory /tmp/armorer-gate-receipts.BdO4pM
# BUILD_OK (four native cases; schema/graph/SBOM mutation rejections inside harness)
ARMORER_TEST_GRAPH_RECEIPTS=/tmp/armorer-gate-receipts.BdO4pM RUSTC_WRAPPER= rustup run 1.95.0 \
  cargo test --locked --test cargo_graph_v2 native_python_graphs_match_independent_rust_reader \
  -- --ignored --exact --test-threads=1
# test result: ok. 1 passed
```

Receipt identity: `attested=false`, `run_identity="synthetic fixture-only 17/2"`, target
aarch64-apple-darwin, runtime_commit b94969c…; interoperability only, no authentication claim.

## Gate 4 — real_native_runtime_distribution_matches_independent_provider_qualification: BLOCKED

Required material does not exist locally:
- All three exact-head native CI artifacts (`runtime-candidate-{os}-run-<id>-attempt-<n>`) from one
  successful Development run at the reviewed head. GitHub reports 0 retained artifacts repo-wide
  (14-day retention expired; latest run 37218646841 at ab526d7 succeeded 2026-10-04).
- An independently approved runtime-distribution spec + digest + source commit/ref
  (ARMORER_TEST_RUNTIME_SPEC, _SPEC_SHA256, _SOURCE_COMMIT, _SOURCE_REF). No approved spec exists;
  owner approval is a human gate.

Path forward: re-run Development CI at the accepted head, download the three artifacts inside the
retention window, obtain independent spec approval, then execute the ignored test. This is also a
prerequisite for the production runtime decision in #4/#8.

## Cross-check

Development run 37218646841 (head ab526d7, all three platforms) already executed gates 1–2 plus the
Apple/Sigstore/CyclonedX ignored gates as named CI steps, all success. Gate 3 is not a CI step
anywhere; gate 4 is an explicit operator gate. This receipt adds current-workstation execution of
gates 1–3 at the same head.
