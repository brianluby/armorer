# Bootstrap caller templates

These exact unprivileged templates match `AuthenticatedCatalog::caller_workflows`.
They pin the reviewed CI and unsigned builder baseline; they do not implement
release/signing/publication. Required `armorer.toml`, reviewed `armorer.lock`,
tracked Cargo.lock and explicit `.armorer/ci-policy.toml` must exist before hosted
use. The next #4 provisioning slice will preview/apply them transactionally.

Existing workflows and policy stay unowned; do not replace them with these examples.
See [catalog authority and limits](../../docs/bootstrap-catalog.md).
