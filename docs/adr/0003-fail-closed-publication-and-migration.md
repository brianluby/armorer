# ADR 0003: fail-closed publication and preservation of customization

Status: proposed; fail-closed secure releases confirmed 2026-09-30.

Context: partial draft uploads, unavailable settings APIs and upgrade drift can create false assurance or erase existing behavior.

Decision: use deterministic plans/file preimages, transactional apply and three-way upgrades. Report platform prerequisites; do not silently change settings. Secure releases require live capability evidence, exact draft inventory/served-byte verification and immutable publication. Identical owned retries may resume; conflicting bytes never overwrite. Separate lifecycle/evidence states.

Alternatives: overwrite generated files and rely on Git recovery; trust an old bootstrap settings check; publish immediately and attach assets later; permit implicit legacy checksum fallback. Each loses either preservation or enforceable artifact trust.

Consequences: administrators must provision restricted read access and protected environments. Unsupported private repositories can adopt explicitly labeled CI-only profiles. Published failures require incident handling and a corrected version, not replacement. Historical exceptions are exact-version/digest policies with weaker evidence status.
