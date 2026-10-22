//! Source-preserving laboratory history. Raw archives are never rewritten.
use crate::{
    database::{Database, queries},
    error::AppError,
    reports::{ObservationPayload, Source, TrendRequest, parse_time},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use futures_util::TryStreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::Row;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryRequest {
    pub metric_id: String,
}

pub async fn history(
    database: &Database,
    user_id: &str,
    request: HistoryRequest,
) -> Result<Value, AppError> {
    if request.metric_id.starts_with("clinical:") {
        let records = load_health(database, user_id, "").await?;
        return database.cpu.run(move || {
            let mut groups = crate::clinical_labs::groups(&records);
            let group = groups.remove(&request.metric_id).ok_or(AppError::NotFound)?;
            Ok(json!({"metric":group.definition,"points":group.points,"records":group.records,"excluded":[],"excluded_count":0,"algorithm_version":"clinical-history-v1"}))
        }).await;
    }
    let definition = crate::laboratory::metrics()
        .into_iter()
        .find(|metric| metric.metric_id == request.metric_id)
        .ok_or(AppError::Invalid("Unknown metric_id"))?;
    let report_history = crate::reports::trends(
        database,
        user_id,
        TrendRequest {
            metric_ids: vec![request.metric_id.clone()],
            start_at: None,
            end_at: None,
        },
    )
    .await?;
    let mut points = report_history["points"]
        .as_array()
        .cloned()
        .ok_or(AppError::Internal)?;
    let mut sources = BTreeMap::new();
    for point in &mut points {
        let report_id = point["report_id"]
            .as_str()
            .ok_or(AppError::Internal)?
            .to_owned();
        if !sources.contains_key(&report_id) {
            let report = database.report_summary(user_id, &report_id).await?;
            let context: Value = serde_json::from_str(&report.context_json)?;
            let name = context["source_name"]
                .as_str()
                .map(str::trim)
                .filter(|name| !name.is_empty());
            let source_id = crate::sources::report_key(name, &report_id);
            let source_name = name.unwrap_or(&report.original_name).to_owned();
            sources.insert(
                report_id.clone(),
                (source_id, source_name, report.original_name),
            );
        }
        let (source_id, source_name, filename) = &sources[&report_id];
        point["point_id"] = json!(format!(
            "report:{}",
            point["observation_id"].as_str().ok_or(AppError::Internal)?
        ));
        point["source_id"] = json!(source_id);
        point["source_key"] = json!(source_id);
        point["source_name"] = json!(source_name);
        point["filename"] = json!(filename);
    }
    let health = load_health(
        database,
        user_id,
        if request.metric_id == "glucose" {
            "blood_glucose"
        } else {
            ""
        },
    )
    .await?;
    let mut excluded = report_history["excluded"]
        .as_array()
        .cloned()
        .ok_or(AppError::Internal)?;
    database.cpu.run(move || {
        let groups = crate::clinical_labs::groups(&health);
        let records = groups.get(&request.metric_id).map(|g| g.records.clone()).unwrap_or_default();
        let mut unassigned = Vec::new();
        for record in health { append_health(&record, &request.metric_id, &mut points, &mut excluded, &mut unassigned); }
        // Clinical charts and historical rows share normalization and duplicate grouping.
        points.retain(|point| point["original"]["clinical_observation"].is_null());
        if let Some(group) = groups.get(&request.metric_id) { points.extend(group.points.clone()); }
        points.sort_by(|a,b| a["timestamp"].as_i64().cmp(&b["timestamp"].as_i64()).then_with(|| a["point_id"].as_str().cmp(&b["point_id"].as_str())));
        let chart_ranges = crate::chart::ranges(points.iter().filter_map(|p| {
            Some((p["source_key"].as_str().or_else(|| p["source_id"].as_str())?.to_owned(), p["unit"].as_str()?.to_owned(), p["timestamp"].as_f64()?, p["value"].as_f64()?))
        }), chrono_tz::UTC);
        Ok(json!({"chart_ranges":chart_ranges,"metric":definition,"records":records,"points":points,"excluded":excluded,"excluded_count":excluded.len(),
            "unassigned_clinical_records":unassigned,"unassigned_clinical_count":unassigned.len(),
            "algorithm_version":"metric-history-v2","conversion_version":crate::laboratory::CONVERSION_VERSION}))
    }).await
}

fn append_health(
    record: &Value,
    metric: &str,
    points: &mut Vec<Value>,
    excluded: &mut Vec<Value>,
    unassigned: &mut Vec<Value>,
) {
    let id = format!(
        "apple_health:{}:{}",
        record["source_id"].as_str().unwrap_or(""),
        record["record_id"].as_str().unwrap_or("")
    );
    if record["record_type"] == "blood_glucose" {
        let payload = &record["payload"];
        let date = record["start_at"]
            .as_i64()
            .and_then(|time| chrono::DateTime::from_timestamp(time, 0))
            .map(|time| time.to_rfc3339());
        append_value(
            metric,
            &id,
            payload["value"].clone(),
            payload["unit"].as_str(),
            date.as_deref(),
            record,
            points,
            excluded,
        );
        return;
    }
    let Some(encoded) = record["payload"]["fhir"]["data_base64"]
        .as_str()
        .filter(|data| data.len() <= 8 * 1024 * 1024)
    else {
        unassigned.push(json!({"point_id":id,"reason":"unreadable_clinical_record"}));
        return;
    };
    let Some(resource) = STANDARD
        .decode(encoded)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
    else {
        unassigned.push(json!({"point_id":id,"reason":"unreadable_clinical_record"}));
        return;
    };
    if resource["resourceType"] != "Observation" {
        unassigned.push(json!({"point_id":id,"reason":"unsupported_clinical_resource"}));
        return;
    }
    let date = resource["effectiveDateTime"]
        .as_str()
        .or_else(|| resource["effectivePeriod"]["start"].as_str());
    let mut candidates = vec![(&resource, id.clone())];
    if let Some(components) = resource["component"].as_array() {
        candidates.extend(
            components
                .iter()
                .enumerate()
                .map(|(index, item)| (item, format!("{id}:component:{index}"))),
        );
    }
    for (candidate, point_id) in candidates {
        let codes = candidate["code"]["coding"].as_array();
        let matches: Vec<&str> = codes
            .into_iter()
            .flatten()
            .filter(|coding| {
                matches!(
                    coding["system"].as_str(),
                    Some("http://loinc.org" | "https://loinc.org")
                )
            })
            .filter_map(|coding| coding["code"].as_str().and_then(loinc_metric))
            .collect();
        if matches.is_empty() && !candidate["valueQuantity"].is_null() {
            unassigned.push(json!({"point_id":point_id,"reason":"unmapped_clinical_metric"}));
            continue;
        }
        if !matches.contains(&metric) {
            continue;
        }
        if matches.iter().any(|found| *found != metric) {
            excluded.push(json!({"point_id":point_id,"reason":"ambiguous_metric_code"}));
            continue;
        }
        if !matches!(
            resource["status"].as_str(),
            Some("final" | "amended" | "corrected")
        ) {
            excluded.push(json!({"point_id":point_id,"reason":"clinical_result_not_final"}));
            continue;
        }
        let quantity = &candidate["valueQuantity"];
        if !quantity["comparator"].is_null() {
            excluded.push(json!({"point_id":point_id,"reason":"bounded_result"}));
            continue;
        }
        if quantity["system"]
            .as_str()
            .is_some_and(|system| system != "http://unitsofmeasure.org")
        {
            excluded.push(json!({"point_id":point_id,"reason":"unsupported_unit_system"}));
            continue;
        }
        let unit = quantity["code"]
            .as_str()
            .or_else(|| quantity["unit"].as_str());
        if metric == "hba1c" && unit.and_then(crate::laboratory::canonical_unit) != Some("%") {
            excluded
                .push(json!({"point_id":point_id,"reason":"incompatible_hba1c_method_and_unit"}));
            continue;
        }
        let original = json!({"record_id":record["record_id"],"source_id":record["source_id"],"provider":record["payload"]["source_name"],"clinical_observation":candidate});
        append_value(
            metric,
            &point_id,
            quantity["value"].clone(),
            unit,
            date,
            &original,
            points,
            excluded,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn append_value(
    metric: &str,
    id: &str,
    value: Value,
    unit: Option<&str>,
    date: Option<&str>,
    original: &Value,
    points: &mut Vec<Value>,
    excluded: &mut Vec<Value>,
) {
    let Some(timestamp) = date.and_then(|date| parse_time(date).ok()) else {
        excluded.push(json!({"point_id":id,"reason":"missing_or_invalid_collection_date"}));
        return;
    };
    let raw_result = if let Some(number) = value.as_number() {
        number.to_string()
    } else {
        excluded.push(json!({"point_id":id,"reason":"non_numeric_result"}));
        return;
    };
    let payload = ObservationPayload {
        raw_name: metric.into(),
        raw_result,
        raw_unit: unit.map(str::to_owned),
        reference_range: None,
        report_flag: None,
        sampled_at: date.map(str::to_owned),
        metric_id: Some(metric.into()),
        source: Source {
            page: 1,
            quote: String::new(),
            bounding_box: None,
        },
        notes: None,
    };
    let Some((value, unit)) = crate::laboratory::normalized(&payload) else {
        excluded.push(json!({"point_id":id,"reason":"incompatible_unit_or_value"}));
        return;
    };
    points.push(json!({"point_id":id,"source_key":format!("apple_health:{}", original["source_id"].as_str().unwrap_or("")),"source_id":original["source_id"],"source_name":"Apple Health", "metric_id":metric,
        "sampled_at":date,"timestamp":timestamp,"value":value,"unit":unit,"original":original}));
}

// Closed LOINC mapping, verified against https://loinc.org/<code>/.
// Unknown codes remain in the raw archive; labels are never guessed from display text.
pub(crate) fn loinc_metric(code: &str) -> Option<&'static str> {
    Some(match code {
        "2089-1" | "13457-7" | "18262-6" => "ldl_cholesterol",
        "2093-3" => "total_cholesterol",
        "2085-9" => "hdl_cholesterol",
        "2571-8" => "triglycerides",
        "1884-6" => "apob",
        "4548-4" => "hba1c",
        "2345-7" => "glucose",
        "718-7" => "hemoglobin",
        "2160-0" => "creatinine",
        "2276-4" => "ferritin",
        "1751-7" => "albumin",
        "1988-5" => "crp",
        "736-9" => "lymphocyte_percent",
        "787-2" => "mcv",
        "788-0" => "rdw",
        "6768-6" => "alp",
        "6690-2" => "wbc",
        _ => return None,
    })
}

async fn load_health(
    database: &Database,
    user_id: &str,
    quantity_kind: &str,
) -> Result<Vec<Value>, AppError> {
    let mut stream = sqlx::query(queries::METRIC_HEALTH_HISTORY)
        .bind(user_id)
        .bind(quantity_kind)
        .fetch(&database.pool);
    let mut rows = Vec::new();
    let mut bytes = 0;
    while let Some(row) = stream.try_next().await? {
        bytes += row.try_get::<String, _>("chart_payload")?.len();
        if rows.len() >= 10000 || bytes > 32 * 1024 * 1024 {
            return Err(AppError::TooLarge);
        }
        rows.push(row);
    }
    drop(stream);
    rows.into_iter().map(|row| {
        Ok(json!({"source_id":row.try_get::<String,_>("source_id")?, "record_id":row.try_get::<String,_>("record_id")?,
            "record_type":row.try_get::<String,_>("record_type")?, "start_at":row.try_get::<i64,_>("start_at")?,
            "payload":serde_json::from_str::<Value>(&row.try_get::<String,_>("chart_payload")?)?}))
    }).collect()
}

pub async fn browse(database: &Database, user_id: &str) -> Result<Value, AppError> {
    let records = load_health(database, user_id, "").await?;
    let reports = database.confirmed(user_id).await?;
    database
        .cpu
        .run(move || {
            let mut entries: BTreeMap<String, Value> = crate::clinical_labs::groups(&records)
                .into_iter()
                .map(|(key, group)| (key, group.definition))
                .collect();
            for observation in reports {
                if let Some(definition) = crate::laboratory::metrics()
                    .into_iter()
                    .find(|m| Some(m.metric_id) == observation.payload.metric_id.as_deref())
                {
                    entries
                        .entry(definition.metric_id.into())
                        .or_insert(serde_json::to_value(definition)?);
                }
            }
            let mut metrics: Vec<Value> = entries.into_values().collect();
            metrics.sort_by_key(|metric| metric["name"].as_str().unwrap_or("").to_lowercase());
            Ok(json!({"metrics":metrics}))
        })
        .await
}
