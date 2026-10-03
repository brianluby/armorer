# Version-two bootstrap examples

`library`, `cli` and `service` are dependency-free fixtures, with explicit config,
committed Cargo.lock, source, license and owner-decision policy. Their saved
`bootstrap-plan.json` files show create previews; they are structural/read-only
planning examples, not hosted build, attestation or release receipts. The MIT
allowlist belongs only to these examples; author your own reviewed project policy.

From a built Armorer checkout, inspect an example without changing its files:

```sh
target/debug/armorer --repository examples/bootstrap-v2/library bootstrap plan --policy examples/bootstrap-v2/library/ci-policy.reviewed.toml
```

Use a temporary copy to try `bootstrap apply`; review the exact plan and supply its
SHA-256 separately. Never apply directly to committed example fixtures, which the
independent schema and Rust tests expect to remain in the pre-bootstrap state.
See [bootstrap and recovery](../../docs/bootstrap-v2.md) for the full sequence,
limits, compatibility boundaries and required later gates.
