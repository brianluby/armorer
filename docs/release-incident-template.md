# Record the breach in the armor

Copy this template outside the original release record. Fill it with observed
facts and retained evidence. Mark unknown fields `unknown`; leave unperformed
checks `not performed`. A completed form is an incident record, not a verification
proof, publication approval or message sent to anyone.

Keep the evidence close. **This is the way.**

## Identity and scope

| Field | Observed value / evidence |
| --- | --- |
| Incident owner and reviewer | |
| Detection time, UTC | |
| Repository, stable tag, full source commit and ref | |
| Release ID, URL and observed draft/published/immutable state | |
| Config, Cargo.lock, Armorer lock and runtime identities | |
| Caller/reusable workflow pins, run and attempt | |
| Original inventory, policy, context and review identities | |
| Affected artifact names, targets, feature cases, SHA-256 and sizes | |
| Root/verifier identities and observation times | |
| Original publication, membership and per-artifact verification receipts | |
| Current failed stage, stable error and retained diagnostic identity | |
| Affected aliases, consumers and exact observed downstream references | |
| Known limitations and unverified scope | |

## Timeline and evidence

| UTC time | Observation/action | Evidence location and identity | Owner |
| --- | --- | --- | --- |
| | | | |

Preserve original files byte-for-byte. Keep failed and later successful attempts
separate. Record exporter/verifier versions, exact commands without secret values,
exit status and source identity. Sanitize a separate copy for sharing; retain
restricted evidence under owner control. Never copy credentials into this record.

## Containment decision

- Verified failure and consequence:
- Remaining uncertainty:
- Promotion held and exact observed state:
- Proposed remote action, if any:
- Required explicit owner authorization and its review record:
- Authorized reporting channel and audience:
- Readback/evidence after any authorized action:

Holding promotion or recording a proposal does not prove a remote setting changed.
Never infer notification, revocation or tag movement from intent.
**This is the way.**

## Corrected version and closure evidence

- Validated cause and source change:
- New stable version/source identity selected by owner:
- Fresh independent catalog/root/policy/context approvals:
- Required source, producer, native consumer and rehearsal results:
- New immutable publication/membership and per-artifact provenance results:
- Separately authorized alias operation and exact readback:
- Remaining risks, support/security follow-up and owner acceptance:

Follow [release recovery](release-recovery.md). A corrected release keeps its own
receipts. Preserve the failed version's original status and limitations.

Keep the old record. Earn the new one. **This is the way.**
