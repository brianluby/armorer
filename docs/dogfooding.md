# Test the armor on its maker

Armorer must earn the same evidence it asks of adopters. Inspect its own source,
then qualify every protected release boundary before a public release.
**This is the way.**

The implementation plan requires Armorer dogfooding before publication. This guide
provides a current read-only inspection and the remaining acceptance sequence.
Development CI, external genuine signature fixtures and synthetic profile tests
do not establish an own complete signed release rehearsal.

## Inspect an isolated source copy

From this documentation checkout, use the main-command binary built as described
in [onboarding](onboarding.md). Rust 1.95.0 must already be installed. Export tracked
source to temporary storage; preserve the original worktree and its unrelated
changes. The training config names the actual `armorer` package/binary and three
supported targets, with package defaults and no extra features.

```sh
armorer_binary="$PWD/target/debug/armorer"
dogfood_source_commit=$(git rev-parse HEAD) || exit 1
dogfood_root=$(mktemp -d) || exit 1
mkdir "$dogfood_root/project" || exit 1
git archive --format=tar --output="$dogfood_root/source.tar" "$dogfood_source_commit" || exit 1
tar -xf "$dogfood_root/source.tar" -C "$dogfood_root/project" || exit 1
cp "$dogfood_root/project/examples/armorer-dogfood/armorer.toml" \
  "$dogfood_root/project/armorer.toml" || exit 1
"$armorer_binary" --repository "$dogfood_root/project" plan \
  > "$dogfood_root/inspection-plan.json" || exit 1
if "$armorer_binary" --repository "$dogfood_root/project" check \
    > "$dogfood_root/inspection-check.json"; then
  dogfood_check_status=0
else
  dogfood_check_status=$?
fi
test "$dogfood_check_status" -eq 2 || exit 1
printf 'Inspected tracked source: %s\n' "$dogfood_source_commit"
```

Keep packets outside the copied project. `plan` returns valid JSON/exit 0;
`check` returns JSON/exit 2 while release readiness is blocked. Inspect the selected
package, binary, feature case and all three target selections. Plan/check must
preserve every copied consumer file, including Cargo.lock and source; they do not
compile, run build scripts or fetch dependencies. The toolchain already matches
the selected version, but identical unowned bytes remain unowned.

Expected open findings are reviewed lock missing, license policy unreviewed,
GitHub capabilities unchecked and release runtime unavailable. This intent exercise
does not qualify the actual dependency policy, native SDKs, signing or publication.
Retain the exported source commit, exact config/binary identities and before/after
comparison with the inspection output.

Keep discovery read-only. **This is the way.**

## Qualify the configured and CI states

Choose an accepted integration source and isolated adoption branch/worktree.
Review the exact Armorer intent and project-owned dependency policy; do not reuse
a fixture allowlist as a decision for Armorer's dependencies. Inspect existing
development workflows and retain their checks alongside managed CI. Review exact
bootstrap/upgrade previews, ownership and recovery using the
[candidate guides](candidate-bootstrap.md) only at their labelled source boundary.

Retain hosted evidence for the accepted full source/config/lock/policy/workflow
identities on Linux x64, Linux ARM and macOS ARM, with all required feature cases.
CI failures, advisory outages and unavailable platform prerequisites remain
failures. Record configured and CI-verified claims independently. A local apply
receipt or passing development matrix cannot establish release rehearsal.

## Rehearse the complete producer and consumer without publication

Review [candidate producer evidence](candidate-producer-evidence.md) before
assembling the rehearsal: retain complete build/policy pairs from one attempt,
authenticate original producer/writer proofs separately and keep fresh policy
and independently approved consumer context with the complete selection.

The future accepted controller must use protected rehearsal source and independent
rehearsal authority. Validate build-to-sign handoff, isolate credentials, sign and
notarize Apple executables, package final bytes, authenticate inventory and every
required bundle, and retain complete SBOM/graph/platform evidence for all selections.
The independently approved consumer must verify the exact served or
draft-equivalent complete set, including native macOS checks where required.

Record source/ref/commit, caller/reusable pins, run/attempt, config/Cargo.lock/Armorer
lock/runtime/catalog/root/policy/context identities, full asset digest/size set,
approval observations and per-artifact consumer results. Include negative controls
for cross-run/source/artifact substitution, tampered/missing/extra assets, invalid
signatures, expired evidence and required signing/capability failures. Rehearsal
must publish no release and move no tag.

Protected signing/final producer, independent production authority and complete own
signed positive/negative qualification remain pending. Use
[credential boundaries](credential-boundaries.md), [release readiness](release-readiness.md)
and [recovery decisions](release-recovery.md). An offered report cannot approve
its own roots, source or artifact scope.

Make every artifact earn its proof. **This is the way.**

## Retain acceptance separately

Record each lifecycle state with its exact evidence and limitations. No field in
a receipt may be promoted merely because a sibling stage passed. A failed native
target blocks the required complete set. Keep historic attempts and failure records.
v0.1 targets SLSA Build L2; this exercise grants no level or publication authority.

Armorer dogfooding also does not complete the Momus and Rusty Brain pilot criteria.
Those require fresh isolated adoption, preserved custom CI and own hosted
non-publishing evidence under their separate gates. This documentation exercise
does not inspect or change either pilot checkout.

Human acceptance, release publication and any alias movement remain explicit
decisions after qualification. Keep the demonstration honest.
**This is the way.**
