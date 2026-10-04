"""Validate native schema shapes; never authenticate or execute the examples."""
from schema_checks import ROOT, coverage, document, positive, rejection, validator

cases = [("native-catalog-v2", "catalog.json"), ("runtime-distribution-v1", "runtime-distribution.json"), ("verification-context-v2", "context.json"), ("verification-context-v3", "context-v3.json")]
negative = 0
for kind, filename in cases:
    check = validator(kind + ".json")
    example = document(ROOT / "examples/runtime-native-v2" / filename)
    positive(check, example, filename)
    rejection(check, example, filename + " unsupported version", lambda v: v.update(schema_version=999), "maximum", ["schema_version"])
    rejection(check, example, filename + " caller shell", lambda v: v.update(command="arbitrary caller shell"), "additionalProperties", [])
    negative += 2
members = document(ROOT / "examples/runtime-native-v2/runtime-distribution.json")
rejection(validator("runtime-distribution-v1.json"), members, "empty runtime members", lambda v: v.update(members={}), "minProperties", ["members"])
negative += 1
coverage("native examples", len(cases), negative, 4, 9)
print(f"4 synthetic native schema examples; {negative} rejections; no runtime approval or provenance claim")
