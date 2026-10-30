//! Shared Browse catalog and query projections for Apple Health originals.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Deserialize, Serialize)]
pub struct Definition {
    pub kind: String,
    pub identifier: String,
    pub family: String,
    pub title: String,
    pub unit: String,
    pub description: String,
    pub browse: bool,
}
pub fn definitions() -> &'static [Definition] {
    static CATALOG: OnceLock<Vec<Definition>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("apple_catalog.json"))
            .expect("validated Apple Health catalog")
    })
}
pub fn definition(kind: &str) -> Option<&'static Definition> {
    definitions().iter().find(|d| d.kind == kind)
}
pub fn unit(kind: &str) -> Option<&'static str> {
    definition(kind)
        .filter(|d| d.family == "quantity")
        .map(|d| d.unit.as_str())
}
pub fn browse_catalog(sources: &[Value]) -> Vec<Value> {
    let mut counts = BTreeMap::<&str, i64>::new();
    for source in sources {
        if source["platform"] == "apple_health"
            && let (Some(kind), Some(count)) = (
                source["record_type"].as_str(),
                source["record_count"].as_i64(),
            )
        {
            *counts.entry(kind).or_default() += count;
        }
    }
    let mut result: Vec<_> = definitions()
        .iter()
        .filter(|d| d.browse && counts.get(d.kind.as_str()).is_some_and(|n| *n > 0))
        .map(|d| {
            let mut value = serde_json::to_value(d).expect("serializable catalog definition");
            value["metric_id"] = json!(d.kind);
            value["record_count"] = json!(counts[d.kind.as_str()]);
            value
        })
        .collect();
    result.sort_by(|a, b| a["title"].as_str().cmp(&b["title"].as_str()));
    result
}

/// HKQuantity descriptions already express percent as 0–100, unlike doubleValue(.percent()).
/// Only dimensionally compatible units are converted; invalid records remain in history.
pub fn normalize(value: f64, from: &str, to: &str) -> Option<f64> {
    if !value.is_finite() {
        return None;
    }
    fn alias(u: &str) -> &str {
        match u {
            "kcal/hr·kg" => "kcal/(kg*hr)",
            "ml/kg*min" | "ml/min·kg" | "mL/kg/min" => "ml/(kg*min)",
            "bpm" => "count/min",
            "ml" => "mL",
            _ => u,
        }
    }
    let from = alias(from);
    let to = alias(to);
    let linear = |u: &str| match u {
        "kg" => Some(("mass", 1000.0)),
        "g" => Some(("mass", 1.0)),
        "mg" => Some(("mass", 0.001)),
        "mcg" | "ug" | "µg" => Some(("mass", 0.000001)),
        "lb" => Some(("mass", 453.59237)),
        "km" => Some(("length", 1000.0)),
        "m" => Some(("length", 1.0)),
        "cm" => Some(("length", 0.01)),
        "mm" => Some(("length", 0.001)),
        "ft" => Some(("length", 0.3048)),
        "in" => Some(("length", 0.0254)),
        "mi" => Some(("length", 1609.344)),
        "hr" | "h" => Some(("time", 3600.0)),
        "min" => Some(("time", 60.0)),
        "s" => Some(("time", 1.0)),
        "ms" => Some(("time", 0.001)),
        "L" => Some(("volume", 1000.0)),
        "mL" => Some(("volume", 1.0)),
        "m/s" => Some(("speed", 1.0)),
        "km/hr" | "km/h" => Some(("speed", 1.0 / 3.6)),
        "mi/hr" | "mph" => Some(("speed", 0.44704)),
        "count/min" => Some(("rate", 1.0)),
        "count/s" => Some(("rate", 60.0)),
        "kcal" => Some(("energy", 1.0)),
        "kJ" => Some(("energy", 1.0 / 4.184)),
        "J" => Some(("energy", 1.0 / 4184.0)),
        _ => None,
    };
    let result = if from == to {
        value
    } else if from == "degF" && to == "degC" {
        (value - 32.0) / 1.8
    } else if let (Some((a, x)), Some((b, y))) = (linear(from), linear(to)) {
        if a != b {
            return None;
        }
        value * x / y
    } else {
        return None;
    };
    (result.is_finite() && (result >= 0.0 || to == "degC") && (to != "%" || result <= 100.0))
        .then_some(result)
}

pub fn projection(kind: &str, payload: &Value) -> Option<Value> {
    let target = unit(kind)?;
    let parse = |text: &str| -> Option<f64> {
        let mut parts = text.split_whitespace();
        let value = parts.next()?.parse::<f64>().ok()?;
        let raw_unit = parts.next()?;
        if parts.next().is_some() {
            return None;
        }
        let value = normalize(value, raw_unit, target)?;
        // Existing named percentage types store HKUnit.percent fractions.
        Some(if ["body_fat", "oxygen_saturation"].contains(&kind) {
            value / 100.0
        } else {
            value
        })
    };
    let mut fields = serde_json::Map::new();
    if kind.starts_with("HKQuantityTypeIdentifier")
        && let Some(value) = payload["raw_quantity"].as_str().and_then(parse)
    {
        fields.insert("value".into(), json!(value));
        fields.insert("unit".into(), json!(target));
    }
    if let Some(series) = payload["series"].as_array().filter(|s| !s.is_empty()) {
        let samples: Option<Vec<_>> = series
            .iter()
            .map(|p| {
                let at = p["start_at"].as_f64().filter(|v| v.is_finite())?;
                Some(json!({"at":at,"value":p["quantity"].as_str().and_then(parse),"unit":target}))
            })
            .collect();
        if let Some(samples) = samples {
            fields.insert("quantity_samples".into(), json!(samples));
        }
    }
    (!fields.is_empty()).then_some(Value::Object(fields))
}

/// Display only health fields; binary archives, hashes and device internals are not metrics.
pub fn details(kind: &str, payload: &Value) -> Value {
    let mut result = serde_json::Map::new();
    for (key, value) in payload.as_object().into_iter().flatten() {
        if ![
            "raw_archive",
            "raw_medications",
            "date_components_archive",
            "metadata",
            "device",
            "platform_identifier",
            "source_name",
            "source_version",
            "series",
            "voltage_measurements",
            "fhir",
            "related_sample_ids",
            "start_at_precise",
            "end_at_precise",
            "quantity_samples",
            "original_record",
        ]
        .contains(&key.as_str())
        {
            result.insert(key.clone(), value.clone());
        }
    }
    if let Some(code) = payload["category"].as_i64() {
        let label = match (kind, code) {
            ("HKCategoryTypeIdentifierAppleStandHour", 0) => Some("Stood"),
            ("HKCategoryTypeIdentifierAppleStandHour", 1) => Some("Idle"),
            (_, 0) => Some("Recorded event"),
            _ => None,
        };
        if let Some(label) = label {
            result.insert("category".into(), json!(label));
        }
    }
    if kind == "HKCharacteristicTypeIdentifierBloodType"
        && let Some(label) = payload["value"].as_u64().and_then(|n| {
            ["Not set", "A+", "A−", "B+", "B−", "AB+", "AB−", "O+", "O−"].get(n as usize)
        })
    {
        result.insert("value".into(), json!(label));
    }
    if kind == "HKCharacteristicTypeIdentifierBiologicalSex"
        && let Some(label) = payload["value"]
            .as_u64()
            .and_then(|n| ["Not set", "Female", "Male", "Other"].get(n as usize))
    {
        result.insert("value".into(), json!(label));
    }
    if let Some(encoded) = payload["date_components_archive"].as_str() {
        result.insert(
            "date_of_birth".into(),
            crate::apple_archive::decode(encoded).unwrap_or(json!("Original could not be decoded")),
        );
    }
    if !kind.starts_with("HKQuantity")
        && !["heart_rate", "electrocardiogram", "workout", "sleep"].contains(&kind)
    {
        if let Some(encoded) = payload["raw_archive"]["data"].as_str()
            && let Some(decoded) = crate::apple_archive::decode(encoded)
        {
            result.insert("health_record".into(), decoded);
        }
        if let Some(items) = payload["raw_medications"].as_array() {
            result.insert(
                "medications".into(),
                Value::Array(
                    items
                        .iter()
                        .map(|v| {
                            v["data"]
                                .as_str()
                                .and_then(crate::apple_archive::decode)
                                .unwrap_or(json!("Original could not be decoded"))
                        })
                        .collect(),
                ),
            );
        }
    }
    Value::Object(result)
}
