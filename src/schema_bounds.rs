//! Standard unsigned numeric constraints; JSON Schema formats are annotations.
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
                if let Some(maximum) = maximum
                    && object
                        .get("maximum")
                        .and_then(Value::as_u64)
                        .is_none_or(|old| old > maximum)
                {
                    object.insert("maximum".into(), maximum.into());
                }
                for child in object.values_mut() {
                    visit(child);
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
        *object = value.as_object().unwrap().clone();
    }
}
