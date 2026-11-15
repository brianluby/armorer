//! Standard unsigned numeric constraints; JSON Schema formats are annotations.
//! Consumers MUST parse integers losslessly (big-integer aware): u64 maxima exceed
//! IEEE-754's safe-integer range, and double-based parsers lose identity precision.
use schemars::Schema;
use serde_json::Value;

/// Bound primitive unsigned values even inside nullable or collection schemas.
pub(crate) fn unsigned(schema: &mut Schema) {
    fn visit(value: &mut Value) {
        match value {
            Value::Object(object) => {
                let maximum = match object.get("format").and_then(Value::as_str) {
                    Some("uint32") => Some(u32::MAX as u64),
                    Some("uint64") => Some(u64::MAX),
                    _ => None,
                };
                if let Some(maximum) = maximum {
                    // Only tighten: leave absent bounds alone unless missing, and never
                    // overwrite a present non-u64 (stricter or uninterpretable) bound.
                    let tighten = match object.get("maximum") {
                        None => true,
                        Some(Value::Number(n)) => n.as_u64().is_some_and(|old| old > maximum),
                        Some(_) => false,
                    };
                    if tighten {
                        object.insert("maximum".into(), maximum.into());
                    }
                }
                for (key, child) in object.iter_mut() {
                    // Annotation payloads are data, not subschemas; never inject bounds into them.
                    if !matches!(key.as_str(), "default" | "examples" | "const" | "enum") {
                        visit(child);
                    }
                }
            }
            Value::Array(array) => {
                for child in array {
                    visit(child);
                }
            }
            _ => {}
        }
    }
    if let Some(object) = schema.as_object_mut() {
        let mut value = Value::Object(std::mem::take(object));
        visit(&mut value);
        // visit never replaces the top-level value; move it back without cloning.
        if let Value::Object(updated) = value {
            *object = updated;
        }
    }
}
