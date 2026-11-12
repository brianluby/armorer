# Rehearsal vehicle provider settings — 2026-11-12

Repository: `brianluby/armorer-test` (public inert fixture; single-owner
rehearsal exception documented at `d114999`). Settings applied under owner
execution authorization; every change verified by GET readback. No releases,
tags, workflows, or secrets were created or touched. `release-signing`
environment unchanged.

## Applied settings

| Setting | State | Readback identity |
| --- | --- | --- |
| Immutable releases | `enabled: true`, `enforced_by_owner: false` | PUT empty body → 204 (`{"enabled":true}` rejected 422: key not permitted) |
| Environment `release-publish` | required reviewer brianluby (3779002); `can_admins_bypass: false`; `prevent_self_review: true`; custom branch policy main-only | id 27112588232; rules 72284467 (reviewers) / 72284468 (branch); policy 66495435 |
| Default-branch ruleset | active; `~DEFAULT_BRANCH`; deletion + non_fast_forward; no bypass actors; main `protected: true` | ruleset 27148317 |
| Tag ruleset | active; target `tag`; include `~ALL`, exclude `[]`; deletion + update; `bypass_actors: []` | ruleset 27148316 |

The tag ruleset shape matches the controller gate exactly
(`src/verification/publication/github.rs:463-486`): target tag, active,
`~ALL` include, empty exclude, no bypass actors, both rules present.
`prevent_self_review: true` is required by the publish-environment gate
(`github.rs:318`); `release-signing` keeps `false` under the documented
single-owner signing exception.

## Consequence: publish requires a second reviewer

The controller rejects self-approval twice over: the environment's
`prevent_self_review: true` and the Rust preflight
(`src/verification/publication/mod.rs:505-514`, actor IDs must differ from
the approver). A single owner therefore **cannot** complete a
`release-publish` deployment approval alone. The single-owner rehearsal
exception covers signing only. Before Phase 4 provider trials, the owner
must either add an independent second reviewer to `release-publish` or
explicitly accept that publish rehearsal stays blocked. This is an
independent-approval requirement, not a defect; no gate may be weakened to
accommodate it.

Follow-up verification (read-only): `release-publish` has exactly one
deployment branch policy (`main`, id 66495435). Tag **creation** is not
blocked by the ruleset: GitHub governs creation with a separate `creation`
rule type, which ruleset 27148316 does not include; `update` blocks moving
existing tags and `deletion` blocks removal, with no bypass actors. This
matches the controller flow (owner pushes a new tag; the tag-push run is
authenticated; the controller resolves the existing tag and never creates
tags). Doc/schema-based verification, not a live push test — a test tag
would be permanent under the deletion rule.

## Remaining owner gates for this vehicle

- Apple Developer ID + notarization credentials provisioned into
  `release-signing` (names only: APPLE_CERTIFICATE_P12_BASE64,
  APPLE_CERTIFICATE_PASSWORD, APPLE_ID, APPLE_APP_SPECIFIC_PASSWORD).
- Independent second reviewer for `release-publish` (above).
- Production catalog/roots/publication-policy approval (separate channel).

Settings alone grant no authority: the controller's
`current-attempt-environment-enforcement` and `runner-wide-serialization`
gates remain Unsupported until the matched workflow/adapter work lands.
