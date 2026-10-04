"""Check context structure; never approve policy or execute consuming code."""
from schema_checks import ROOT, coverage, document, positive, rejection, validator

check = validator("verification-context-v1.json")
context = document(ROOT / "examples/verification-context-v1/synthetic-linux.json")
positive(check, context, "synthetic-linux context")
negative = 0
for label, mutate, keyword, path in [
    ("unsupported version", lambda v: v.update(schema_version=2), "maximum", ["schema_version"]),
    ("PR trigger", lambda v: v.update(trigger="pull_request"), "enum", ["trigger"]),
    ("caller shell", lambda v: v.update(command="caller shell input"), "additionalProperties", []),
    ("zero attempt", lambda v: v["inputs"]["run"].update(attempt=0), "minimum", ["inputs", "run", "attempt"]),
    ("moving verifier", lambda v: v["native_sbom_validator"].update(sha256="moving-ref"), "pattern", ["native_sbom_validator", "sha256"]),
    ("caller URL", lambda v: v["selections"][0].update(download_url="https://attacker.invalid/context"), "additionalProperties", ["selections", 0]),
    ("empty signers", lambda v: v.update(signers={}), "minProperties", ["signers"]),
    ("zero freshness", lambda v: v["selections"][0]["evidence_requirements"].update(max_age_seconds=0), "minimum", ["selections", 0, "evidence_requirements", "max_age_seconds"]),
]:
    rejection(check, context, label, mutate, keyword, path)
    negative += 1
coverage("verification context", 1, negative, 1, 8)
print(f"1 synthetic context schema example; {negative} structural rejections; no trust or signature claims")
