# Keep the roots. Watch the clock.

Use this guide when trusted roots, policy reviews, advisory databases or capability
observations need renewal. It covers current contract checks and preparation for
the implemented release controller, whose native writes remain gated. Main
verifies individual evidence slots through the [offline library API](offline-verifier.md)
and complete sets through the [complete verifier](verify-release-cli.md). Neither interface provides a root-update or advisory-fetch command.

Keep authority outside the offered claim. **This is the way.**

## Prepare roots online; approve them independently

On an online preparation machine, use the reviewed qualified GitHub CLI to export
candidate roots into a new directory. Set `qualified_gh` to that executable's
path. Preserve earlier exports and their approvals.

```sh
qualified_gh=/path/to/qualified/gh
root_packet=$(mktemp -d) || exit 1
"$qualified_gh" attestation trusted-root \
  > "$root_packet/trusted-root.candidate.jsonl"
root_export_status=$?
printf 'Root export exit status: %s\n' "$root_export_status"
```

Only exit 0 supplies a candidate export to review. A failed or partial export is
failure evidence, never approved input. This is an operator preparation command,
outside Armorer's fixed offline verification invocation. It fetches material for
both public-good and GitHub Sigstore instances; that does not qualify Armorer's
private backend. Do not substitute an unreviewed custom TUF mirror or root.
See [the CLI root-export interface](https://cli.github.com/manual/gh_attestation_trusted-root).

Record exporter version/byte identity, origin, observation time, root byte digest
and size, selected backend and the independent review record. Review the source
and trust policy through their approved channel. A computed hash identifies bytes;
it does not approve them. The policy must bind those exact bytes and a current
root review. Replacing roots needs a newly approved policy identity; a complete
candidate context must also bind the updated policy/input identities.

Main's root transport accepts one bounded JSON object or at most 32 JSONL root
objects, within 4 MiB. Duplicate keys, arrays and malformed records fail. Transport
acceptance does not establish upstream authenticity. Keep catalog and native-tool
approval separate from root approval.

Bring exact approved root, policy, verifier and bundle/subject bytes into the
offline environment. Include independent source/ref/commit/workflow expectations;
the downloaded release cannot select them. Armorer never silently refreshes roots.
GitHub's offline guidance explains that exported keys have no built-in expiry and
cannot reveal later revocations. Renew the preparation record when importing new
signed material; policy review expiry remains a separate gate.
[Offline root guidance](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/verify-attestations-offline).

Retain old roots and receipts. Approve the replacement. **This is the way.**

## Distinguish the clocks

| Evidence | Current check | Renewal decision |
| --- | --- | --- |
| Policy/root/exception review | Review time is positive and no later than actual time; expiry is strictly later than actual time | New genuine review record and approved policy/context identity |
| Tool/database observation | Positive observation no later than record time; supplied age ceiling is positive and not exceeded | Re-observe exact approved input bytes and their authentication record |
| Database evidence | `max_age_seconds` is mandatory | Enforce the independently required freshness ceiling |
| Complete artifact evidence | Selection agrees with independent requirements and `max_age_seconds` is not exceeded | Obtain complete fresh evidence under the reviewed producer contract |
| Required capability | Correct capability, retained evidence, available/enforced state and observation within caller-supplied maximum age | Fresh authenticated platform observation near the protected operation |
| Candidate bootstrap/upgrade policy | UTC-date exception expiry; upgrade also binds UTC review day | Fresh exact plan and approval when the day/input boundary changes |

The [evidence contract](trust-contracts-v1.md#per-artifact-evidence-and-apple-limits)
and source types define these checks. A timestamp in a producer JSON record does
not prove that observation occurred. Review-record validation does not prove a
human performed the review. Future adapters must authenticate those records.

At [candidate #21](candidate-verification.md#keep-approved-observations-and-exception-use-exact),
tool/database `observed_at` must exactly match the independently approved
requirement, and required coverage must retain the exact approved `exception_ids`.
Renewing an observation or changing exception use therefore needs renewed
independent requirements/context approval. A producer cannot freshen its label
or attach a merely allowed exception to an exception-free coverage requirement.
This tightening is included in accepted main through #21. The dated candidate
guide retains its original source and validation boundary.

Keep units, source and uncertainty with every observation.
**This is the way.**

## Hold the gate during an outage

An unavailable advisory service, failed root export, stale required database,
expired review or uncertain capability cannot satisfy a required release gate.
Preserve the last approved bytes and the new failure separately. Do not update
timestamps, widen freshness ceilings, remove required tools or convert an error
into an available observation to make a check pass.

The pinned reusable CI already freshly fetches advisory/registry/replacement
feeds, stops on fetch failure and runs offline cargo-deny with a fixed one-day
RustSec fetch-age ceiling. Its
[exact CI guide](https://github.com/brianluby/armorer-workflows/blob/772ca83e386c883c88cc3b936d69f8cb3216c91e/docs/ci.md)
and [runtime](https://github.com/brianluby/armorer-workflows/blob/772ca83e386c883c88cc3b936d69f8cb3216c91e/armorer_runtime/ci.py#L161)
describe that implemented gate. An outage there is a failed CI attempt; fix the
feed/prerequisite and rerun in a fresh attempt. A policy exception cannot turn
unavailable data into a successful fresh observation.

The planned release controller must retain that fail-closed advisory requirement
and authenticate its release evidence. Armorer's local semantic tests enforce
evidence consistency/freshness; they do not exercise the reusable workflow's
live fetcher or establish production release readiness. Optional reporting
capabilities remain governed by explicit independent policy. Reporting success
cannot satisfy a required capability.

The separate [candidate producer reader](candidate-producer-evidence.md#watch-policy-time-and-advisory-outages)
already checks complete build/policy handoffs, retained advisory bytes, age from
observation start and UTC exception expiry again at the final boundary. Its
unsigned consistency and native storage observations do not authenticate producer
claims or grant protected finalization authority.

For resumption, obtain a successful authenticated observation or independently
approved replacement input. Inspect source/run/attempt, policy/catalog/tool/root
identities, scope, age and all required evidence together. Rerun the applicable
verification against exact expectations. A new producer attempt is a new identity;
it cannot inherit permission to resume an old owned draft. Follow
[publication recovery](release-recovery.md) for that decision.

Root expiry or authentication failure never selects historical comparison. The
candidate's explicit historical route has its own source allowlist and weaker
result. Native Apple ticket checks also have their
[documented online/cache limits](candidate-verification.md); offline Sigstore
verification alone cannot satisfy required Apple executable checks.

A corrected input may require new production authority or a new release version.
Keep that owner decision visible. **This is the way.**
