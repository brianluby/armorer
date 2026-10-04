"""Named structural checks. These never establish authentication or runtime trust."""
import copy
import json
from pathlib import Path
import tomllib
from jsonschema import Draft202012Validator, ValidationError

ROOT = Path(__file__).resolve().parent.parent

def document(path):
    """Read one explicit UTF-8 document with its filename in any failure."""
    try:
        text = path.read_text(encoding="utf-8")
        return tomllib.loads(text) if path.suffix in (".toml", ".lock") else json.loads(text)
    except (OSError, ValueError) as error:
        raise ValueError(f"cannot read structural example {path}: {error}") from error

def validator(filename):
    schema = document(ROOT / "schemas" / filename)
    Draft202012Validator.check_schema(schema)
    return Draft202012Validator(schema)

def positive(check, value, label):
    try:
        check.validate(value)
    except ValidationError as error:
        raise ValueError(f"{label}: {error.validator} at {list(error.absolute_path)}") from error

def rejection(check, base, label, mutate, keyword, path):
    """A valid control must fail at the intended schema keyword and location."""
    positive(check, base, label + " control")
    changed = copy.deepcopy(base)
    try:
        mutate(changed)
    except (KeyError, IndexError, TypeError) as error:
        raise ValueError(f"{label}: mutation fixture shape changed") from error
    errors = list(check.iter_errors(changed))
    if not any(error.validator == keyword and list(error.absolute_path) == path for error in errors):
        observed = [(error.validator, list(error.absolute_path)) for error in errors]
        raise ValueError(f"{label}: expected {keyword} at {path}; observed {observed}")

def coverage(label, positive_count, negative_count, expected_positive, expected_negative):
    """Explicit coverage gate survives Python optimization and empty discovery."""
    if (positive_count, negative_count) != (expected_positive, expected_negative):
        raise ValueError(f"{label}: coverage {(positive_count, negative_count)}; expected {(expected_positive, expected_negative)}")
