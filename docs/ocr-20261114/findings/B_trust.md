# Review findings — group B_trust (24 findings)

─── src/trust/capability.rs:164-164 ───
[security · medium] The Apple signing gate hard-codes the single triple `"aarch64-apple-darwin"`
instead of recognizing Apple targets structurally. Today `config::TARGETS` only contains this one
Apple triple, so there is no live bypass — but the gate's correctness silently depends on that
constant in another module never growing. Adding `x86_64-apple-darwin` (or any future macOS triple)
to `TARGETS` would let Apple deliverables pass `CapabilityConfig::validate` without
`AppleSigning`/`ProtectedSigningEnvironment` being Required. Match the platform suffix (or share a
constant/helper with `config::TARGETS`) so the gate can't drift out of sync.

-                 && d.targets.iter().any(|t| t == "aarch64-apple-darwin")
+                 && d.targets.iter().any(|t| t.ends_with("-apple-darwin"))

─── src/trust/capability.rs:275-279 ───
[security · medium] Lock-to-catalog pin binding is ambiguous across adapters: pin-name uniqueness
(`pins.insert(&pin.name)`) is enforced only within a single adapter, and this `.any()` matches a
lock tool against pins from all adapters. Two adapters can therefore pin the same tool name with
different versions/distributions; the lock tool binds to whichever happens to match, while the other
adapter's conflicting pin for the same tool goes unchecked against the lock. Hoist the `pins` name
set above the adapter loop so pin names are unique catalog-wide (each lock tool then maps to exactly
one pinned distribution). Note: `authentication_record` can't be compared here — `config::ToolPin`
carries only version/sha256 — so name uniqueness is the available guard.

─── src/trust/capability.rs:264-265 ───
[security · medium] `pin.version` is only checked for non-emptiness after trim plus a byte-length
cap, so values like `" 1.0 "`, `"../../x"`, or strings containing newlines/control bytes pass
validation — inconsistent with `pin.name` (identifier-checked) and `runtime_versions`
(`stable_version`). Since these sidecar strings feed downstream fixed-location URL/artifact
resolution, constrain them at this trust boundary. The crate already has `super::asset_name` (ASCII
alphanumeric start, then alnum/`_`/`.`/`-`), which covers semver, dates, and commit-style versions;
reuse it instead of the trim check (or `stable_version` if all pin versions are guaranteed SemVer).

-                         && !pin.version.trim().is_empty()
-                         && pin.version.len() <= 100,
+                         && pin.version.len() <= 100
+                         && super::asset_name(&pin.version),

─── src/trust/capability.rs:245-245 ───
[maintainability · low] Adapter de-duplication keys on `format!("{:?}", a.id)`: it allocates a
String per adapter and makes identity depend on Debug output stability (a future custom Debug impl
could silently break dedup). `AdapterId` is a plain Copy enum — add `PartialOrd, Ord` to its derive
and use `BTreeSet<AdapterId>` directly; no allocation, typed identity.

-             require(ids.insert(format!("{:?}", a.id)), "duplicate-adapter")?;
+             require(ids.insert(a.id), "duplicate-adapter")?;

─── src/trust/inventory.rs:64-65 ───
[performance · medium] The 8192-spec cap and name/uniqueness checks run only after every name string
has been allocated. This is a `pub` function that never calls `config.validate_shape()` or
`policy.validate()`, so an unvalidated config with huge `deliverables`/`targets` does unbounded
allocation work before rejection. Add a cheap incremental guard so work is bounded even for
unvalidated input (the final check still enforces exact uniqueness/naming).

          for target in &d.targets {
+             require(specs.len() <= 8192, "invalid-derived-asset-set")?;
              let key = format!("{}--{}--{}", d.id, target, d.feature_set);

─── src/trust/inventory.rs:252-255 ───
[security · medium] The `len == assets.len() + 2` equality only proves an exact published set if
asset names are unique AND exclude `INVENTORY_NAME`/`INVENTORY_BUNDLE_NAME`. Uniqueness is enforced
below, but the reserved-name exclusion lives only in `validate_against`. If this method is called on
an inventory that hasn't been through `validate_against` (the precondition is not documented here),
an asset named `INVENTORY_NAME` overlaps with the reserved key, so the count check passes while
`supplied` contains one extra, never-verified file — fail-open for a "complete published set" check.
Enforce the exclusion here (or reject up front) instead of relying on an unenforced caller
invariant.

+         for a in &self.assets {
+             require(
+                 a.name != INVENTORY_NAME && a.name != INVENTORY_BUNDLE_NAME,
+                 "inventory-hash-cycle",
+             )?;
+         }
          require(
              supplied.len() == self.assets.len() + 2,
              "published-asset-set-mismatch",
          )?;

─── src/trust/inventory.rs:265-269 ───
[performance · medium] Comparing via `serde_json::to_value` builds two full JSON trees, silently
assumes the serde representation stays injective for these types (any future
`skip_serializing_if`/lossy field would make distinct inventories compare equal in this security
check), and collapses failures into a context-free `Error::Json`. `Asset` and `InputIdentity`
already implement `PartialEq`; just add `PartialEq` to `ReleaseInventory`'s derive list and compare
directly — exact, allocation-free, representation-independent.

-         require(
-             serde_json::to_value(&parsed).map_err(|_| crate::Error::Json)?
-                 == serde_json::to_value(self).map_err(|_| crate::Error::Json)?,
-             "published-inventory-content-mismatch",
-         )?;
+         require(parsed == *self, "published-inventory-content-mismatch")?;

─── src/trust/policy.rs:206-208 ───
[security · medium] These public acceptance helpers perform only membership checks, but
`VerificationPolicy` is freely constructible (public fields + public `Deserialize`) and mutable, so
nothing enforces that `validate(now)` ran first. A caller that skips it silently loses repository
pinning, review expiry, duplicate rejection, the five-scope signer baseline, and the hosted-runner
guarantee — exactly the properties this trust policy exists to enforce. All current call sites do
validate first, but that is a convention, not a guarantee, and post-validation mutation of the
public fields also invalidates it. Minimal fix: document the precondition on both helpers (suggested
below). Stronger fix: have `validate()` consume `self` and return a `ValidatedPolicy` newtype that
alone exposes `accept_source`/`accept_signer`, making unvalidated acceptance unrepresentable.

+     /// Membership check only. Callers MUST run `validate(now)` on this policy
+     /// first and not mutate it afterwards; this helper does not re-check
+     /// repository pinning, review expiry, duplicates or the signer baseline.
      pub fn accept_source(&self, source: &Source) -> Result<()> {
          require(self.sources.contains(source), "source-not-allowed")
      }

─── src/trust/policy.rs:40-40 ───
[documentation · low] `workflow.repository` is deliberately not tied to `self.repository`
(reusable-workflow signers live in other repositories; `ExpectedAttestation` binds the *caller*
workflow to the source repo separately), while `sources` and `historical` entries are explicitly
pinned to `self.repository`. That asymmetry makes this look like a missing check to a future reader
or hardening pass. Document the intent here so it is not "fixed" by wrongly pinning signer repos —
or, if cross-repository signers are not actually intended, add `rule.workflow.repository ==
self.repository` to the signer validation instead.

+     /// May name a reusable workflow in another repository; authorized by exact
+     /// repository, path and commit, not by the policy repository.
      pub workflow: WorkflowIdentity,

─── src/trust/mod.rs:71-71 ───
[maintainability · medium] Schema/runtime contract drift: `git_ref` is emitted as an unconstrained
string, while `Source::validate` accepts only safe `refs/heads/` branches (≤255 bytes, no `..`, no
segment ending in `.` or `.lock`), canonical `refs/tags/vX.Y.Z` tags, or canonical
`refs/pull/<n>/merge` refs. Schema-driven consumers (editors, policy validators, other-language
tooling) see a materially weaker contract and will accept documents the runtime rejects — or, if
they trust the schema alone, accept arbitrary refs. Mirror the allowlist in the schema; the
per-segment branch rules can't be fully expressed in one regex, so keep `valid_source_ref`
authoritative and treat the pattern as a close approximation.

+     #[schemars(regex(
+         pattern = "^refs/(heads/[A-Za-z0-9][A-Za-z0-9_./-]{0,254}|tags/v(0|[1-9][0-9]{0,19})\\.(0|[1-9][0-9]{0,19})\\.(0|[1-9][0-9]{0,19})|pull/[1-9][0-9]{0,19}/merge)$(?![\\s\\S])"
+     ))]
      pub git_ref: String,

─── src/trust/mod.rs:200-201 ───
[maintainability · low] Byte/char bound mismatch: JSON Schema `minLength`/`maxLength` count Unicode
code points, but `validate_at` enforces `rationale.len() <= 4096` bytes and rejects whitespace-only
text. A schema-valid 4096-character multibyte rationale (up to 16 KiB of UTF-8) or a whitespace-only
string like `" "` passes schema validation yet fails at runtime. The direction is fail-closed, but
the published contract misleads consumers; exact byte bounds and non-whitespace aren't expressible
in JSON Schema, so document the runtime semantics on the field.

      #[schemars(length(min = 1, max = 4096))]
+     #[schemars(description = "Runtime bound is 1-4096 UTF-8 bytes (not code points) and must contain non-whitespace text")]
      pub rationale: String,

─── src/trust/mod.rs:152-155 ───
[maintainability · low] Unbounded version components in the schema: `[0-9]*` accepts arbitrarily
long digit strings, while `stable_version` (semver) rejects any component exceeding `u64`. Cap each
component at 20 digits so the schema stays close to the runtime contract and schema-only consumers
can't be fed unbounded numeric text.

      #[schemars(regex(
-         pattern = "^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$(?![\\s\\S])"
+         pattern = "^(0|[1-9][0-9]{0,19})\\.(0|[1-9][0-9]{0,19})\\.(0|[1-9][0-9]{0,19})$(?![\\s\\S])"
      ))]
      pub runtime_version: String,

─── src/trust/mod.rs:322-323 ───
[maintainability · medium] Both decode stages collapse every failure into `Error::Json` (whose
display text is even "JSON serialization failed", misleading for a parse path), so a duplicate-key
rejection — the anti-smuggling signal this visitor exists for — is indistinguishable from a syntax
error or a typed-shape mismatch when diagnosing rejected contracts. Without echoing untrusted
values: in stage 1, `Category::Data` errors can only originate from the duplicate-key visitor (the
parser never emits non-finite floats, and depth/number/trailing-input failures are `Syntax`), so
they can be mapped to a stable distinct code while everything else stays redacted.

-     let strict: UniqueJson = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
+     let strict: UniqueJson = serde_json::from_slice(bytes).map_err(|e| {
+         if e.classify() == serde_json::error::Category::Data {
+             Error::Invalid("duplicate-json-key".into())
+         } else {
+             Error::Json
+         }
+     })?;
      serde_json::from_value(strict.0).map_err(|_| Error::Json)

─── src/trust/native.rs:248-248 ───
[maintainability · low] Duplicate-adapter detection keys on `Debug` output rather than the typed id.
This is only correct as long as `AdapterId`'s derived `Debug` stays injective and stable — if a
variant ever gains fields or a manual `Debug` impl is added, distinct adapters could collide (or
duplicates escape detection) silently in this trust-validation path. It also allocates a `String`
per adapter. `AdapterId` already derives `Copy + Eq`; add `PartialOrd, Ord` to its derive list in
`trust/capability.rs` and use a typed `BTreeSet<AdapterId>` here (the same pattern in the v1 catalog
can then be fixed identically).

-             require(ids.insert(format!("{:?}", adapter.id)), "duplicate-adapter")?;
+             require(ids.insert(adapter.id), "duplicate-adapter")?;

─── src/trust/native.rs:176-176 ───
[bug · low] Pin version validation accepts whitespace-padded or otherwise non-canonical strings
(e.g. `" 1.2.3"`, `"1.2.3\n"`). Since `matches()` and the lock-coverage check compare versions by
exact equality against the canonically-formatted lock/evidence versions, such a pin passes
validation but can never apply — a silently inert approval that surfaces later as a confusing
`catalog-lock-pin-mismatch`/coverage failure instead of a clear error here. Requiring
`stable_version` for all pins would be too strict (pins also cover databases/schemas/policies with
non-semver versions), but at minimum reject leading/trailing whitespace so approved pins are never
dead on arrival.

-             identifier(&self.name) && !self.version.trim().is_empty() && self.version.len() <= 100,
+             identifier(&self.name)
+                 && !self.version.is_empty()
+                 && self.version == self.version.trim()
+                 && self.version.len() <= 100,

─── src/trust/evidence.rs:58-58 ───
[maintainability · medium] The key is not injective: `identifier()` (src/config.rs) permits hyphens
with no restriction on consecutive hyphens, so distinct selections can collide, e.g.
deliverable_id="a--x", feature_set="y" vs deliverable_id="a", feature_set="x--y" produce the same
key for the same target. This key determines the expected distributable asset name used to bind the
final step output below (and the same scheme is re-implemented in inventory/verification modules),
so a collision could bind the chain to the wrong artifact. Consider rejecting "--" inside identifier
components used in keys, or using an unambiguous encoding (length-prefixed/escaped separator).

─── src/trust/evidence.rs:527-528 ───
[security · medium] This trust gate decides exception allow-list membership by comparing serialized
JSON values instead of structural equality. `Exception` and all nested types (ByteIdentity, Review,
CapabilityId) already support `PartialEq`, so the serialization round-trip is unnecessary; any
future non-faithful serialization (skip/normalizing attributes) would silently equate distinct
exceptions and weaken the reviewed-exception policy. Also, `map_err(|_| crate::Error::Json)`
discards the serde error context. Derive `PartialEq, Eq` on `Exception` and compare directly.

-                 serde_json::to_value(e).map_err(|_| crate::Error::Json)?
-                     == serde_json::to_value(allowed).map_err(|_| crate::Error::Json)?,
+                 e == allowed,

─── src/trust/evidence.rs:485-485 ───
[maintainability · low] Unlike the producer-side lists (which reject duplicate tool names,
capabilities and exception ids), the requirements-side collections are not checked for uniqueness.
Duplicate names in `required.tools` (and duplicate ids in `allowed_exceptions` below) are silently
absorbed by this first-match lookup, leaving later entries unreachable and the policy ambiguous.
Reject duplicate names/ids in `EvidenceRequirements` before matching, mirroring the producer-side
checks.

─── src/trust/evidence.rs:361-361 ───
[performance · low] Unnecessary clone: `p.output` is cloned on every chained step solely to build a
one-element array for comparison. Compare by borrow instead, e.g. `s.inputs.as_slice() ==
std::slice::from_ref(&p.output)` (or `matches!(s.inputs.as_slice(), [x] if *x == p.output)`), which
avoids allocating a `ByteIdentity` copy.

-                     s.inputs == [p.output.clone()] && p.finished_at <= s.started_at,
+                     s.inputs.as_slice() == std::slice::from_ref(&p.output)
+                         && p.finished_at <= s.started_at,

─── src/trust/evidence.rs:377-378 ───
[performance · low] The expected asset name is rebuilt for every inventory asset inside the `find`
closure (a `Selection::key()` allocation plus a `format!` per iteration). Hoist it into a `let
expected_name = format!("{}{}", self.selection.key(), ...);` before the search and compare `a.name
== expected_name` in the closure.

─── src/trust/publication.rs:85-91 ───
[bug · medium] The doc comment promises per-stage evidence ("a scanner pass cannot advance a
stage"), but validate() only checks that evidence is non-empty: any stage — including
Published/ProvenanceVerified — can be asserted with a single arbitrary ByteIdentity, and
`limitations` is never validated. trust-contracts-v1.md lists "Lifecycle stage, evidence, and
limitations" as the minimum semantic coverage for this record. Either enforce stage-specific
evidence requirements and validate/bound `limitations` (as policy.rs does for its records), or
correct the doc/contract so callers don't assume stage advancement is gated here.

─── src/trust/publication.rs:213-216 ───
[bug · medium] Optional evidence is only validated in the states that require it, and is neither
rejected nor validated when present in other states — unlike `conflict` (exact iff gating) and
`draft_download_receipt` (validated whenever present). Concretely, all of these pass validate(): a
malformed/expired `approval` or `immutable_setting` on an OwnedDraft/Uploaded/Conflict receipt, a
malformed `release_attestation` on Published, a malformed `provenance_verification` on Approved, and
`published_at == 0` or in the future on a non-published receipt. Validate every optional field
whenever present (the pattern already used for draft_download_receipt and registry_bytes) and gate
presence per state. Also consider temporal ordering: for published states, `published_at` is not
ordered against approval.reviewed_at / setting.observed_at, and the immutable-setting freshness is
checked against `now` (validation time) rather than publication time.

─── src/trust/publication.rs:254-261 ───
[bug · high] The identity comparison omits the remote draft byte identity: two
DraftBytesVerified/Approved receipts for the same release_id with different `draft_download_receipt`
digests pass, so a retry can resume against replaced draft bytes — contradicting the documented
"remote draft identity" / "Never authorizes replacement" contract. Compare `draft_download_receipt`
(None == None keeps OwnedDraft/Uploaded retries working). Also decide whether
`approval`/`immutable_setting` substitution between Approved retries must be rejected or explicitly
documented as renewable; as written, an Approved draft can resume with swapped approval or setting
evidence.

          require(
              self.inputs == next.inputs
                  && self.tag == next.tag
                  && self.release_id == next.release_id
                  && self.inventory == next.inventory
-                 && self.policy == next.policy,
+                 && self.policy == next.policy
+                 && self.draft_download_receipt == next.draft_download_receipt,
              "conflicting-retry-identity",
          )

─── src/trust/publication.rs:398-406 ───
[bug · medium] `observation` is validated only for RegistryObserved/RegistryBytesVerified; a
malformed ByteIdentity attached to a Prepared/Uploaded/UploadResultUnknown/Conflict receipt passes
unchecked, unlike `registry_bytes` and `index_sha256`, which are validated whenever present.
Validate it whenever present and keep the presence requirement for the observing states.

+             if let Some(o) = &r.observation {
+                 o.validate()?;
+             }
              if matches!(
                  r.state,
                  RegistryState::RegistryObserved | RegistryState::RegistryBytesVerified
              ) {
-                 r.observation
-                     .as_ref()
-                     .ok_or_else(|| crate::Error::Invalid("registry-observation-missing".into()))?
-                     .validate()?;
+                 require(r.observation.is_some(), "registry-observation-missing")?;
              }

