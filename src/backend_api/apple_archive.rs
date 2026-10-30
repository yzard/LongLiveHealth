//! Bounded read-only decoding of archived HealthKit property fields, never object instantiation.
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};

pub fn decode(encoded: &str) -> Option<Value> {
    if encoded.len() > 8 * 1024 * 1024 {
        return Some(json!({"detail":"Original is too large for a record preview."}));
    }
    let bytes = STANDARD.decode(encoded).ok()?;
    let root = plist::Value::from_reader(std::io::Cursor::new(bytes)).ok()?;
    let dictionary = root.as_dictionary()?;
    let objects = dictionary.get("$objects")?.as_array()?;
    let first = dictionary.get("$top")?.as_dictionary()?.get("root")?;
    let mut remaining = 4000;
    Some(resolve(first, objects, 0, &mut remaining))
}
fn resolve(
    value: &plist::Value,
    objects: &[plist::Value],
    depth: usize,
    remaining: &mut usize,
) -> Value {
    if depth > 12 || *remaining == 0 {
        return json!("Additional details retained in original");
    }
    *remaining -= 1;
    match value {
        plist::Value::Uid(uid) => objects
            .get(uid.get() as usize)
            .map(|v| resolve(v, objects, depth + 1, remaining))
            .unwrap_or(Value::Null),
        plist::Value::Dictionary(fields) => {
            if let (Some(keys), Some(values)) = (
                fields.get("NS.keys").and_then(|v| v.as_array()),
                fields.get("NS.objects").and_then(|v| v.as_array()),
            ) {
                let mut result = serde_json::Map::new();
                for (key, value) in keys.iter().zip(values).take(100) {
                    let key = resolve(key, objects, depth + 1, remaining);
                    if let Some(key) = key.as_str() {
                        result.insert(key.into(), resolve(value, objects, depth + 1, remaining));
                    }
                }
                return Value::Object(result);
            }
            if let Some(array) = fields.get("NS.objects").and_then(|v| v.as_array()) {
                return Value::Array(
                    array
                        .iter()
                        .take(100)
                        .map(|v| resolve(v, objects, depth + 1, remaining))
                        .collect(),
                );
            }
            Value::Object(
                fields
                    .iter()
                    .filter(|(k, _)| {
                        ![
                            "$class",
                            "UUID",
                            "uuid",
                            "device",
                            "sourceRevision",
                            "metadata",
                            "_metadata",
                            "_sourceRevision",
                            "_device",
                        ]
                        .contains(&k.as_str())
                    })
                    .take(100)
                    .map(|(k, v)| (k.clone(), resolve(v, objects, depth + 1, remaining)))
                    .collect(),
            )
        }
        plist::Value::Array(a) => Value::Array(
            a.iter()
                .take(100)
                .map(|v| resolve(v, objects, depth + 1, remaining))
                .collect(),
        ),
        plist::Value::Boolean(v) => json!(v),
        plist::Value::Real(v) => json!(v),
        plist::Value::Integer(v) => v
            .as_signed()
            .map(|v| json!(v))
            .or_else(|| v.as_unsigned().map(|v| json!(v)))
            .unwrap_or(Value::Null),
        plist::Value::String(v) => {
            if v == "$null" {
                Value::Null
            } else {
                json!(v.chars().take(2000).collect::<String>())
            }
        }
        plist::Value::Date(v) => json!(v.to_xml_format()),
        plist::Value::Data(_) => json!("Binary attachment retained in original"),
        _ => Value::Null,
    }
}
