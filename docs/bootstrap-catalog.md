# Reviewed bootstrap catalog foundation

Inspect the authority before you forge a lock. Keep each native target bound to
its own bytes. **This is the way.**

This #4 slice supplies an independently anchored, immutable bootstrap catalog,
v1-compatible lock rendering and fixed CI/unsigned-build caller templates. It
performs no adoption or consumer mutation itself. [Version-two bootstrap](bootstrap-v2.md)
combines this authority with explicit project policy and transactional provisioning.
The release trust Catalog is a separate contract; no bootstrap snapshot qualifies
attestations, Apple signing, release publication or SLSA Build L2.

## Authority and source evidence

`armorer catalog` authenticates the embedded exact tool-catalog bytes before
parsing them and reports the current `bootstrap-v2` authority. The expected SHA-256 and workflow repository/full commit live in
reviewed CLI source; they are never accepted from a consuming lock, downloaded
inventory, schema, API response or caller-supplied expected hash. A modified or
reformatted catalog fails authentication even if its JSON schema is valid.
The operator must already trust the CLI source/distribution. This is trust in
membership in that reviewed source, not Sigstore verification of the CLI or tools.

The production registry has one entry: `bootstrap-v2`, the default for new
bootstrap, pinning workflows main
`e5e9c0d31baff87e357046d81068b646a39a9f23`. Its tool catalog was read from the
authenticated GitHub HTTPS API at the predecessor commit
`772ca83e386c883c88cc3b936d69f8cb3216c91e`, compared byte-for-byte with its clean
checkout and checked against Git blob `6f001a407768dcd716324624faff60a1633362f6`.
The `bootstrap-v1` predecessor was removed from production builds before 0.1.0
(ADR 0012); it survives only behind the `test-predecessor` feature for
catalog-to-catalog migration tests. `pins/tools.json` is byte-identical
at both commits (and at `b94969c`), and the bootstrap callers rust-ci.yml /
rust-build.yml are unchanged between them, so v2 inherits the PR #23 download
hardening. The entry authenticates the embedded tool-catalog
SHA-256 `e21e6cbd2cdd1125dbb9817116b1f46d043807b4d92128abc9dc07256dbb4b13`
(5,970 bytes); only the workflow pin moves. The
[source/asset receipt](../catalogs/bootstrap-tools-v1-source.json) retains all 18
native distribution identities from six publishers' exact release assets,
rechecked against publisher GitHub asset digests on 2026-10-02. These HTTPS
metadata observations are review evidence, never a replacement root at runtime.
The accepted workflow installer rehashes actual downloaded bytes before
extraction/execution. No upstream download is initiated by `catalog` or rendering.

Review freshness: `CATALOG_REVIEWED_ON` in `src/catalog.rs` records the UTC date the
bootstrap-v2 pins were last reviewed (2026-10-02, from the source receipt's `verified_on`),
and `REVIEW_WINDOW_DAYS` is 180. Once the catalog is older than the window, `catalog`,
`check` and `bootstrap check` warn on stderr and `bootstrap check --release-ready` reports
the `catalog-review-stale` blocker. Updating the catalog requires updating the date in the
same reviewed change. The date never enters a plan, lock or digest.

Stable IDs have immutable meanings: a future source update retains these entries
and adds a separately reviewed successor rather than moving an ID. Unknown IDs
never fall back. The tool versions remain cargo-deny 0.20.2, actionlint 1.7.12,
Zizmor 1.30.1, Gitleaks 8.30.1, cargo-cyclonedx 0.5.9 and CycloneDX CLI 0.33.1.
This deliberately uses the accepted workflow baseline rather than selecting new
moving versions. Upgrades require reviewed source changes and #5
compatibility/migration previews.

## Compatible lock and caller rendering

`AuthenticatedCatalog::render_lock` reads and validates exact `armorer.toml`
bytes without executing Cargo or source. It renders the existing v1 lock schema,
binds its config SHA-256 and names each distribution `tool--target`. Every tool
has three explicit native distribution records, so multi-target adopters never
interpret a single platform's SHA-256 as every platform's bytes. The lock contains
18 entries even when the selected project uses fewer targets. No schema is changed.
The old v1 reader permits these identifier keys; existing imported unqualified
locks are preserved and require an explicit #5 migration rather than silent rewrite.
The workflow's pinned runtime tool catalog remains authoritative during download.

Review the pins. Preserve existing locks until a migration is approved.
**This is the way.**

`caller_workflows` renders two fixed files shown in [examples](../examples/bootstrap-catalog/README.md).
The file text lives in `catalogs/templates/*.tmpl` (compiled in with `include_str!`),
so reviewers read real YAML and TOML. Templates use only the fixed placeholders
`%%WORKFLOW_REPOSITORY%%`, `%%WORKFLOW_COMMIT%%` and `%%TOOLCHAIN%%`. The renderer
substitutes in a single pass, rejects unknown or unused placeholders, and rejects values
containing `%%`, newlines or control characters. The rendered bytes are frozen by
golden SHA-256 tests in `src/catalog.rs`.
CI supports ordinary PR/push/manual events; the build caller is explicitly manual
and unsigned. Both jobs pin the reviewed full workflow commit and grant only
contents read. Neither exposes shell, inputs, custom provenance, OIDC, signing
credentials, environments or inherited secrets. Existing custom workflows/jobs
stay separate; this renderer does not write or adopt files.

Both callers key their concurrency group by `github.ref` for pull requests and by
`github.sha` for every other event. GitHub keeps at most one pending run per group
and cancels the older pending run when a newer one queues, even with
`cancel-in-progress: false`; a per-ref group therefore dropped queued push runs
during bursts of merges. With one group per commit, a queued run for one commit is
never replaced by another commit's run. Pull requests keep one group per ref and
cancel superseded runs. The build caller also keeps `cancel-in-progress: false`.

## Managed file set

Each compiled catalog entry declares its own managed file set as one ordered table
in `src/catalog.rs`. An entry names a repository-relative path and how its bytes
are produced: a caller template, the toolchain template, the rendered lock, the
owner's approved policy bytes, or a fixed template. Rendering walks that table,
and `AuthenticatedCatalog::managed_paths` derives the sorted path list from the
same table, so the declared set and the rendered set cannot drift apart. Plans,
ownership states, journals, import scope and upgrade bounds ask the catalog that
governs the document; nothing else lists the files ([ADR 0013](adr/0013-catalog-defined-managed-file-set.md)).
`bootstrap-v2` declares five unconditional files: the project CI policy, the two
callers, the lock and the toolchain file.

An entry is either unconditional or optional. The three optional entries,
`SECURITY.md`, `.github/dependabot.yml` and `.github/CODEOWNERS`, are rendered only
when `[repository_files]` in `armorer.toml` selects them, and every switch is off
by default ([ADR 0015](adr/0015-optional-managed-repository-files.md)). Two sets
come from the one table. `managed_paths` is the maximal set, every path the catalog
can ever render (eight for `bootstrap-v2`). `selected_paths` is the set a given
configuration selects (five with every switch off). A bootstrap plan lists exactly
the selected set for its intent. An owned path in the maximal set but outside the
selected set is released, never deleted. Documents that carry no configuration, such as
bootstrap ownership and journal count bounds, are bounded by the maximal set.

`SECURITY.md.tmpl` takes `%%REPOSITORY%%` and `dependabot.yml.tmpl` takes
`%%WORKFLOW_REPOSITORY%%`, through the same single-pass renderer and its rules.
`.github/CODEOWNERS` has a variable number of owners, so it is rendered in code
from the validated owner list, with the same control-character refusal. Golden
SHA-256 tests in `src/catalog.rs` freeze both optional templates as well.

Adding an unconditional managed file means adding a table entry and its template
under `catalogs/templates/`, then regenerating the example packets. It changes
every consumer's plan, so after 0.1.0 it ships as a reviewed catalog successor,
never as an edit to an accepted entry. An optional entry that is off by default
changes no existing plan; before 0.1.0, ADR 0012 lets it join the pre-release
`bootstrap-v2` entry in place. Guards that run before a catalog is known, such as
the license-path overlap check, use the union of every compiled maximal set, so a
`license_file` of `SECURITY.md` is refused.

Count the files the catalog declares, not the files you expect.
**This is the way.**

[GitHub's reusable-workflow documentation](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows)
confirms job-level full-SHA calls and that nested permissions cannot be elevated.
[GitHub release asset API](https://docs.github.com/en/rest/releases/assets)
documents publisher asset identities/digests. These platform capabilities are
not a hosted-adoption or credential-boundary rehearsal for generated callers.

## Validation and next gate

Tests reject schema-valid substituted digests/URLs and reformatted/oversized
catalog bytes; verify exact config binding and all 18 target-qualified lock pins;
preserve existing locks/custom jobs; and print the catalog with no consumer
files, PATH tools or build execution. Independent actionlint/Zizmor validation
covers the example callers. The complete existing schema/fixture and Rust suite
must pass before committing/pushing this slice; hosted results and review findings
are retained at its exact PR head.

The additive [version-two provisioning plan](bootstrap-v2.md) provides explicit
policy, complete before/proposed views, generated bases and multi-file recovery.
A v1 toolchain plan still cannot authorize writes to these callers or policy.
Reviewed three-way migration/upgrades, hosted adoption and release verification
remain separate gates.

## Preserved catalog-foundation local receipt — 2026-10-02

This original receipt records the catalog slice before integration with other
main changes. Preserve its counts and source scope; use
[the development map](development-status.md) for the current merged boundary.

Pinned Rust 1.95.0 locked build, fmt, strict all-target/all-feature Clippy and
**75 tests** pass (9 library, 10 apply, 6 catalog, 24 inspection,
25 trust-contract and 1 doctest). Existing **14** v1 schemas remain byte-for-byte
compatible; independent hash-locked validation passes **54 positive examples /
7 structural rejections**. Catalog-authenticated actionlint/Zizmor checks pass
for development CI and both exact caller templates. Hosted Linux/macOS will run
the same native workflow audits and record results at the PR's immutable head.
