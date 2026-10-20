//! Data-driven clinical items. Identity comes from codes, never a guessed analyte name.
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

pub(crate) struct Group {
    pub definition: Value,
    pub records: Vec<Value>,
    pub points: Vec<Value>,
}

pub(crate) fn groups(records: &[Value]) -> BTreeMap<String, Group> {
    let mut groups = BTreeMap::<String, Group>::new();
    for record in records {
        let Some(encoded) = record["payload"]["fhir"]["data_base64"]
            .as_str()
            .filter(|s| s.len() <= 8 * 1024 * 1024)
        else {
            continue;
        };
        let Some(resource) = STANDARD
            .decode(encoded)
            .ok()
            .and_then(|v| serde_json::from_slice::<Value>(&v).ok())
        else {
            continue;
        };
        if resource["resourceType"] != "Observation" {
            continue;
        }
        let candidates = std::iter::once((&resource, "root".to_owned())).chain(
            resource["component"]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
                .map(|(i, c)| (c, format!("component:{i}"))),
        );
        for (candidate, suffix) in candidates {
            // Panel headings without their own result are not duplicate measurements.
            if suffix == "root" && resource["component"].is_array() && !has_value(candidate) {
                continue;
            }
            let (metric, name) = identity(candidate, record, &suffix);
            let known = crate::laboratory::metrics()
                .into_iter()
                .find(|m| m.metric_id == metric);
            let definition = known.map(|m| json!(m)).unwrap_or_else(|| json!({"metric_id":metric,"name":name.clone(),"standard_unit":"","accepted_units":[],"clinical":true}));
            let group = groups.entry(metric.clone()).or_insert_with(|| Group {
                definition,
                records: vec![],
                points: vec![],
            });
            let quantity = &candidate["valueQuantity"];
            let unit = quantity["code"]
                .as_str()
                .filter(|u| !u.trim().is_empty())
                .or_else(|| quantity["unit"].as_str())
                .unwrap_or("");
            let date = resource["effectiveDateTime"]
                .as_str()
                .or_else(|| resource["effectivePeriod"]["start"].as_str());
            let timestamp = date.and_then(|d| crate::reports::parse_time(d).ok());
            let provider = record["payload"]["source_name"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or("Apple Health");
            let point_id = format!(
                "apple_health:{}:{}:{suffix}",
                record["source_id"].as_str().unwrap_or(""),
                record["record_id"].as_str().unwrap_or("")
            );
            let mut entry = json!({"point_id":point_id,"metric_id":metric,"source_name":provider,"source_id":record["source_id"],"source_key":format!("apple_health:{}", record["source_id"].as_str().unwrap_or("")),"method":resource["method"],"specimen":resource["specimen"],"channel":"Apple Health","sampled_at":date,"timestamp":timestamp,"display_value":display_value(candidate),"unit":unit,"status":resource["status"],"original":candidate});
            let final_result = matches!(
                resource["status"].as_str(),
                Some("final" | "amended" | "corrected")
            );
            let valid_system = quantity["system"]
                .as_str()
                .is_none_or(|s| s == "http://unitsofmeasure.org");
            if valid_system && !unit.is_empty() {
                entry["unit"] = json!(crate::laboratory::canonical_unit(unit).unwrap_or(unit));
            }
            if final_result
                && valid_system
                && quantity["comparator"].is_null()
                && timestamp.is_some()
                && !unit.is_empty()
                && let Some(value) = quantity["value"].as_number()
            {
                let normalized = if metric.starts_with("clinical:") {
                    rust_decimal::Decimal::from_str(&value.to_string())
                        .ok()
                        .map(|value| {
                            (
                                value.normalize().to_string(),
                                entry["unit"].as_str().unwrap_or(unit).to_owned(),
                            )
                        })
                } else {
                    crate::laboratory::normalized(&crate::reports::ObservationPayload {
                        raw_name: name.clone(),
                        raw_result: value.to_string(),
                        raw_unit: Some(unit.into()),
                        reference_range: None,
                        report_flag: None,
                        sampled_at: date.map(str::to_owned),
                        metric_id: Some(metric.clone()),
                        source: crate::reports::Source {
                            page: 1,
                            quote: String::new(),
                            bounding_box: None,
                        },
                        notes: None,
                    })
                    .filter(|_| {
                        metric != "hba1c" || crate::laboratory::canonical_unit(unit) == Some("%")
                    })
                };
                if let Some((value, unit)) = normalized {
                    entry["value"] = json!(value);
                    entry["display_value"] = json!(value);
                    entry["unit"] = json!(unit);
                    entry["plotted"] = json!(true);
                }
            }
            group.records.push(entry);
        }
    }
    for group in groups.values_mut() {
        group.records.sort_by_key(|r| {
            (
                r["timestamp"].as_i64(),
                r["point_id"].as_str().unwrap_or("").to_owned(),
            )
        });
        let raw_count = group.records.len();
        let mut seen = BTreeMap::new();
        let mut unique: Vec<Value> = Vec::new();
        for mut record in group.records.drain(..) {
            // Display-only coalescing. Preserve source, status, method and specimen boundaries.
            let key = if record["plotted"] == true {
                Some(
                    json!([
                        record["source_id"],
                        record["source_name"],
                        record["timestamp"],
                        record["sampled_at"],
                        record["value"],
                        record["unit"],
                        record["status"],
                        record["method"],
                        record["specimen"],
                        record["original"]["method"],
                        record["original"]["code"]["coding"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|coding| Some((
                                coding["system"].as_str()?,
                                coding["code"].as_str()?
                            )))
                            .collect::<BTreeSet<_>>()
                    ])
                    .to_string(),
                )
            } else {
                None
            };
            let original = json!({"point_id":record["point_id"],"observation":record["original"]});
            if let Some(index) = key.as_ref().and_then(|k| seen.get(k)).copied() {
                let previous: &mut Value = &mut unique[index];
                previous["original_records"]
                    .as_array_mut()
                    .expect("originals array")
                    .push(original);
                continue;
            }
            if let Some(key) = key {
                seen.insert(key, unique.len());
            }
            record["original_records"] = json!([original]);
            unique.push(record);
        }
        group.records = unique;
        group.points = group.records.iter().filter(|r| r["plotted"] == true).cloned().map(|mut point| {
            point["original"] = json!({"provider":point["source_name"],"source_id":point["source_id"],"clinical_observation":point["original"]});
            point
        }).collect();
        group.definition["raw_record_count"] = json!(raw_count);
        group.definition["display_units"] = json!(
            group
                .records
                .iter()
                .filter_map(|r| r["unit"].as_str())
                .filter(|u| !u.is_empty())
                .collect::<BTreeSet<_>>()
        );
        group.definition["record_count"] = json!(group.records.len());
    }
    groups
}

fn has_value(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.keys().any(|key| key.starts_with("value")))
}

fn identity(candidate: &Value, record: &Value, suffix: &str) -> (String, String) {
    let coding = candidate["code"]["coding"].as_array();
    let loinc: BTreeSet<&str> = coding
        .into_iter()
        .flatten()
        .filter(|c| {
            matches!(
                c["system"].as_str(),
                Some("http://loinc.org" | "https://loinc.org")
            )
        })
        .filter_map(|c| c["code"].as_str())
        .collect();
    let name = candidate["code"]["text"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            coding
                .into_iter()
                .flatten()
                .find_map(|c| c["display"].as_str().filter(|s| !s.trim().is_empty()))
        })
        .unwrap_or("Unnamed lab result")
        .to_owned();
    if loinc.len() == 1
        && let Some(metric) = loinc
            .iter()
            .next()
            .and_then(|c| crate::metric_history::loinc_metric(c))
    {
        return (metric.into(), name);
    }
    let key = if !loinc.is_empty() {
        json!(["loinc", loinc])
    } else {
        let codes: BTreeSet<(&str, &str)> = coding
            .into_iter()
            .flatten()
            .filter_map(|c| Some((c["system"].as_str()?, c["code"].as_str()?)))
            .collect();
        if codes.is_empty() {
            json!(["uncoded", record["source_id"], record["record_id"], suffix])
        } else {
            json!(["local", record["source_id"], codes])
        }
    };
    let metric = format!("clinical:{:x}", Sha256::digest(key.to_string().as_bytes()));
    let name = if name == "Unnamed lab result" && !loinc.is_empty() {
        format!(
            "Lab result · {}",
            loinc.into_iter().collect::<Vec<_>>().join(", ")
        )
    } else {
        name
    };
    (metric, name)
}

fn display_value(candidate: &Value) -> String {
    let q = &candidate["valueQuantity"];
    if let Some(value) = q["value"].as_number() {
        return format!("{}{}", q["comparator"].as_str().unwrap_or(""), value);
    }
    for key in [
        "valueString",
        "valueBoolean",
        "valueInteger",
        "valueDateTime",
        "valueTime",
    ] {
        let value = &candidate[key];
        if !value.is_null() {
            return value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
        }
    }
    let coded = &candidate["valueCodeableConcept"];
    if let Some(text) = coded["text"].as_str().or_else(|| {
        coded["coding"]
            .as_array()
            .and_then(|v| v.first())
            .and_then(|c| c["display"].as_str().or_else(|| c["code"].as_str()))
    }) {
        return text.into();
    }
    candidate
        .as_object()
        .and_then(|o| o.iter().find(|(key, _)| key.starts_with("value")))
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "No result value supplied".into())
}
