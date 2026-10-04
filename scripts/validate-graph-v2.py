"""Independently check v2 graph schema/fixtures; establish no authenticity."""
from schema_checks import ROOT, coverage, document, positive, rejection, validator

check = validator("cargo-graph-v2.json")
paths = sorted((ROOT / "tests/fixtures/cargo-graph-v2").glob("*.graph.json"))
if {path.name for path in paths} != {"minimal.graph.json", "optional.graph.json", "zero-library.graph.json"}:
    raise ValueError("v2 graph fixture set changed; explicitly review fixture coverage")
count = negative = 0
for path in paths:
    graph = document(path)
    positive(check, graph, str(path))
    count += 1
    for label, mutate, keyword, location in [
        ("unsupported version", lambda r: r.update(schema_version=1), "minimum", ["schema_version"]),
        ("caller command", lambda r: r.update(caller_command="build"), "additionalProperties", []),
        ("missing nodes", lambda r: r.pop("nodes"), "required", []),
        ("unknown node field", lambda r: r["nodes"][0].update(unknown="unrecognized"), "additionalProperties", ["nodes", 0]),
    ]:
        rejection(check, graph, f"{path.name}: {label}", mutate, keyword, location)
        negative += 1
coverage("v2 graph", count, negative, 3, 12)
print(f"v2 graph schema checked; {count} positive fixtures; {negative} structural rejections")
