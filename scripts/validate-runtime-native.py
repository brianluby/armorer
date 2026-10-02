"""Independently validate explicit native-v2 schema shapes; never authenticate or execute them."""
import copy
import json
from pathlib import Path
from jsonschema import Draft202012Validator, ValidationError

root=Path(__file__).resolve().parent.parent
cases=[('native-catalog-v2','catalog.json'),('runtime-distribution-v1','runtime-distribution.json'),('verification-context-v2','context.json')]
negative=0
for kind,name in cases:
    schema=json.loads((root/'schemas'/(kind+'.json')).read_bytes())
    Draft202012Validator.check_schema(schema)
    validator=Draft202012Validator(schema)
    example=json.loads((root/'examples/runtime-native-v2'/name).read_bytes())
    validator.validate(example)
    for mutation in [lambda v:v.update(schema_version=999), lambda v:v.update(command='arbitrary caller shell')]:
        value=copy.deepcopy(example); mutation(value)
        try: validator.validate(value)
        except ValidationError: negative+=1
        else: raise AssertionError('unsupported version or caller input accepted')
print(f'3 synthetic native schema examples; {negative} rejections; no runtime approval or provenance claim')
