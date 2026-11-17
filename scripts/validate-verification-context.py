"""Check context structure; never approve policy or execute consuming code."""
from schema_checks import ROOT, coverage, document, positive, rejection, validator

check = validator("verification-context-v3.json")
context = document(ROOT / "examples/runtime-native-v2/context-v3.json")
positive(check, context, "synthetic v3 context")
negative = 0
for label, mutate, keyword, path in [
    ("unsupported version", lambda v: v.update(schema_version=999), "maximum", ["schema_version"]),
    ("PR trigger", lambda v: v["release"].update(trigger="pull_request"), "enum", ["release", "trigger"]),
    ("caller shell", lambda v: v.update(command="caller shell input"), "additionalProperties", []),
    ("zero attempt", lambda v: v["release"]["inputs"]["run"].update(attempt=0), "minimum", ["release", "inputs", "run", "attempt"]),
    ("moving verifier", lambda v: v["release"]["native_sbom_validator"].update(sha256="moving-ref"), "pattern", ["release", "native_sbom_validator", "sha256"]),
    ("caller URL", lambda v: v["release"]["selections"][0].update(download_url="https://attacker.invalid/context"), "additionalProperties", ["release", "selections", 0]),
    ("empty signers", lambda v: v["release"].update(signers={}), "minProperties", ["release", "signers"]),
    ("zero freshness", lambda v: v["release"]["selections"][0]["evidence_requirements"].update(max_age_seconds=0), "minimum", ["release", "selections", 0, "evidence_requirements", "max_age_seconds"]),
]:
    rejection(check, context, label, mutate, keyword, path)
    negative += 1
coverage("verification context", 1, negative, 1, 8)
print(f"1 synthetic context schema example; {negative} structural rejections; no trust or signature claims")
