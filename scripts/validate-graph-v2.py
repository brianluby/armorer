"""Independently check v2 graph schema/fixtures; establish no authenticity."""
import copy
import json
from pathlib import Path
from jsonschema import Draft202012Validator, ValidationError

root = Path(__file__).resolve().parent.parent
schema = json.loads((root / "schemas/cargo-graph-v2.json").read_bytes())
Draft202012Validator.check_schema(schema)
validator = Draft202012Validator(schema)
positive = 0
negative = 0
for path in sorted((root / "tests/fixtures/cargo-graph-v2").glob("*.graph.json")):
    graph = json.loads(path.read_bytes())
    validator.validate(graph)
    positive += 1
    for mutate in (
        lambda r: r.update(schema_version=1),
        lambda r: r.update(caller_command="build"),
        lambda r: r.pop("nodes"),
        lambda r: r["nodes"][0].update(unknown="unrecognized"),
    ):
        record = copy.deepcopy(graph)
        mutate(record)
        try:
            validator.validate(record)
        except ValidationError:
            negative += 1
        else:
            raise AssertionError("invalid v2 graph structure accepted")
assert positive == 3 and negative == 12
print(f"v2 graph schema checked; {positive} positive fixtures; {negative} structural rejections")
