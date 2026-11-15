# Review findings — group A_schemas (8 findings)

─── schemas/publish-set-v1.json:174-179 ───
[bug · medium] `crates` has no `minItems`/`uniqueItems`, so an empty publish set or the same crate
listed twice validates. A publish-set manifest is the approved input to publication;
duplicates/empties should fail closed here rather than relying on downstream code. Note
`uniqueItems` only rejects byte-identical entries — duplicate name+version pairs with different
archives still need a check in the consumer.

      "crates": {
        "items": {
          "$ref": "#/$defs/CrateIntent"
        },
-       "type": "array"
+       "minItems": 1,
+       "type": "array",
+       "uniqueItems": true
      },

─── schemas/publish-set-v1.json:32-35 ───
[bug · medium] Dependency names are unconstrained strings with no `uniqueItems`, while the "names
within this approved set" rule exists only in prose. Reuse the crate-name pattern used across the
other schemas (`^[A-Za-z0-9][A-Za-z0-9_-]{0,99}$`) so dangling/empty/injection-shaped names are
rejected at the boundary; referential integrity itself must still be asserted by the consumer.

            "items": {
+             "pattern": "^[A-Za-z0-9][A-Za-z0-9_-]{0,99}$(?![\\s\\S])",
              "type": "string"
            },
-           "type": "array"
+           "type": "array",
+           "uniqueItems": true

─── schemas/publish-set-v1.json:37-42 ───
[security · high] `name` and `version` are bare strings (empty, `../`, whitespace, arbitrary text
all pass), unlike every other identity field in this schema. These values are used for crate
identity matching and downstream path/URL construction. Every other schema in `schemas/` constrains
crate names with `^[A-Za-z0-9][A-Za-z0-9_-]{0,99}$`; reuse it here and pin `version` to the same
semver pattern already used by `runtime_version`.

          "name": {
+           "pattern": "^[A-Za-z0-9][A-Za-z0-9_-]{0,99}$(?![\\s\\S])",
            "type": "string"
          },
          "version": {
+           "pattern": "^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$(?![\\s\\S])",
            "type": "string"
          }

─── schemas/publish-set-v1.json:155-155 ───
[bug · medium] GitHub Actions accepts both `.yml` and `.yaml` workflow files, so a legitimate
`.yaml` workflow identity is rejected here. (The same restriction is repeated in the sibling schemas
— fix the pattern consistently rather than only in this file.)

-           "pattern": "^\\.github/workflows/[A-Za-z0-9][A-Za-z0-9_.-]*\\.yml$(?![\\s\\S])",
+           "pattern": "^\\.github/workflows/[A-Za-z0-9][A-Za-z0-9_.-]*\\.ya?ml$(?![\\s\\S])",

─── schemas/publish-set-v1.json:101-106 ───
[bug · low] `format: uint64` is annotation-only in 2020-12 (not an assertion), and the full u64
maximum 18446744073709551615 — on `attempt` and `id` here, and on the same fields in
`schemas/release-inventory-v1.json` — exceeds JSON's interoperable safe-integer range (2^53−1 =
9007199254740991). Validators/consumers that parse numbers as IEEE-754 doubles (e.g. JavaScript
`JSON.parse`) round the bound up to 2^64 and accept out-of-range ids/attempts, and silently lose
precision on any value above 2^53, breaking exact identity and byte-exact comparisons in
cross-language verifiers of this trust contract. Either cap the schema at 9007199254740991 or
explicitly require/document lossless big-integer-aware parsing in consumers. Note
`release-inventory-v1.json` is generated and the Rust producer/consumer (`RunIdentity`, u64 via
serde_json) is already exact: apply the cap in the schemars range / `schema_bounds::unsigned`
transform and regenerate, since a hand-edit to that file would be overwritten.

─── schemas/native-catalog-v2.json:403-412 ───
[bug · medium] `previous_catalog` is the catalog-chain link but is absent from the root `required`
list, so an omitted key and an explicit `null` are indistinguishable to any schema-only consumer: a
v2 catalog can silently drop its predecessor identity instead of asserting "no predecessor". The
Rust side (`Option<ByteIdentity>`, always serialized) treats both as `None`, so this only bites
external validators of the published contract — but for an explicitly-selected authority that is
exactly the case worth closing. Add `previous_catalog` to `required` (keeping the `null` branch) so
absence must be stated. Note this file is generated: apply `#[schemars(required)]` to
`NativeCatalogV2::previous_catalog` and regenerate, otherwise the schema-equality test will revert
the edit.

─── schemas/native-catalog-v2.json:397-402 ───
[bug · low] `adapters` (and `NativeAdapter.pins`) have no `minItems`, so an empty catalog / an
adapter with zero pins validates structurally while `NativeCatalogV2::validate` rejects it
(`!self.adapters.is_empty()`, `!adapter.pins.is_empty()`). Schema-only consumers therefore accept a
pin-lock authority that locks nothing. Add `minItems: 1` (and the enforced upper bounds 32 / 128 if
you want the schema to match) via the schemars derive on the Rust fields, then regenerate — do not
hand-edit this generated file. Skip `uniqueItems` here: uniqueness is enforced on adapter `id` and
pin `name`, which whole-object `uniqueItems` would not catch.

─── schemas/verification-context-v3.json:651-656 ───
[maintainability · low] The description promises "sorted unique features", but the schema enforces
neither. Consumer-side validation (src/verification/context.rs, resolved_root_features) does enforce
sorting/uniqueness/key linkage, so this is not a correctness hole in the pipeline — but any
schema-only validation of this file accepts duplicate entries that contradict the stated contract.
Uniqueness is cheaply expressible here; add `uniqueItems: true`. (Sortedness and linkage to
release.selections cannot be expressed in draft 2020-12 and correctly remain consumer-enforced.)

        "additionalProperties": {
          "items": {
            "type": "string"
          },
-         "type": "array"
+         "type": "array",
+         "uniqueItems": true
        },

