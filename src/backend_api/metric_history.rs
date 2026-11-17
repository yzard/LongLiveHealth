//! Source-preserving laboratory history. Raw archives are never rewritten.
use crate::{
    database::{Database, queries},
    error::AppError,
    reports::{ObservationPayload, REPORTED_METRIC_PREFIX, Source, TrendRequest, parse_time},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use futures_util::TryStreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::Row;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryRequest {
    pub metric_id: String,
}

pub async fn history(
    database: &Database,
    user_id: &str,
    mut request: HistoryRequest,
) -> Result<Value, AppError> {
    request.metric_id = crate::laboratory::canonical_history_id(&request.metric_id).to_owned();
    let clinical = if crate::lab_metric_metadata::is_clinical_id(&request.metric_id) {
        Some(
            crate::lab_metric_metadata::clinical_groups(database, user_id)
                .await?
                .remove(&request.metric_id)
                .ok_or(AppError::NotFound)?,
        )
    } else {
        None
    };
    let reported = request.metric_id.starts_with(REPORTED_METRIC_PREFIX);
    let metadata = database.lab_metric_metadata(user_id).await?;
    let definition = if reported {
        metadata
            .named_report_metrics()
            .find(|metric| metric.metric_id == request.metric_id)
            .map(serde_json::to_value)
            .transpose()?
    } else if let Some(group) = &clinical {
        Some(group.definition.clone())
    } else if request.metric_id.starts_with("custom:") {
        let metric = database
            .custom_metrics(user_id)
            .await?
            .into_iter()
            .find(|metric| metric.metric_id == request.metric_id)
            .ok_or(AppError::NotFound)?;
        Some(serde_json::to_value(metric)?)
    } else {
        let metric = crate::laboratory::metrics()
            .into_iter()
            .find(|metric| metric.metric_id == request.metric_id)
            .ok_or(AppError::NotFound)?;
        Some(serde_json::to_value(metric)?)
    };
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
    let mut definition = definition
        .or_else(|| {
            report_history
                .get("reported_metric")
                .filter(|metric| !metric.is_null())
                .cloned()
        })
        .ok_or(AppError::NotFound)?;
    metadata.apply_name(&mut definition);
    let mut points = report_history["points"]
        .as_array()
        .cloned()
        .ok_or(AppError::Internal)?;
    let mut report_records = report_history["records"]
        .as_array()
        .cloned()
        .ok_or(AppError::Internal)?;
    let mut sources = BTreeMap::new();
    for point in points.iter_mut().chain(report_records.iter_mut()) {
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
    if request.metric_id.starts_with("custom:")
        || clinical.is_some()
        || metadata.named_reports.contains_key(&request.metric_id)
    {
        metadata.apply_loinc(
            &mut definition,
            clinical.as_ref().map(|group| &group.loinc_codes),
        );
    }
    if let Some(group) = clinical {
        points.extend(group.points);
        report_records.extend(group.records);
    }
    if reported
        || request.metric_id.starts_with("custom:")
        || crate::lab_metric_metadata::is_clinical_id(&request.metric_id)
    {
        points.sort_by_key(|point| point["timestamp"].as_i64());
        let (chart_ranges, chart_unit_conflict) =
            history_chart(&request.metric_id, &report_records, &points);
        return Ok(
            json!({"chart_ranges":chart_ranges,"chart_unit_conflict":chart_unit_conflict,"metric":definition,"records":report_records,"points":points,
            "excluded":report_history["excluded"],"excluded_count":report_history["incomparable_count"],
            "algorithm_version":"metric-history-v2","conversion_version":crate::laboratory::CONVERSION_VERSION}),
        );
    }
    let health = load_health(
        &database.pool,
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
        metadata.apply_loinc(&mut definition, groups.get(&request.metric_id).map(|group| &group.loinc_codes));
        let mut records = groups.get(&request.metric_id).map(|g| g.records.clone()).unwrap_or_default();
        records.extend(report_records);
        let mut unassigned = Vec::new();
        for record in health { append_health(&record, &request.metric_id, &mut records, &mut points, &mut excluded, &mut unassigned); }
        // Clinical charts and historical rows share normalization and duplicate grouping.
        points.retain(|point| point["original"]["clinical_observation"].is_null());
        if let Some(group) = groups.get(&request.metric_id) { points.extend(group.points.clone()); }
        points.sort_by(|a,b| a["timestamp"].as_i64().cmp(&b["timestamp"].as_i64()).then_with(|| a["point_id"].as_str().cmp(&b["point_id"].as_str())));
        let (chart_ranges, chart_unit_conflict) = history_chart(&request.metric_id, &records, &points);
        Ok(json!({"chart_ranges":chart_ranges,"chart_unit_conflict":chart_unit_conflict,"metric":definition,"records":records,"points":points,"excluded":excluded,"excluded_count":excluded.len(),
            "unassigned_clinical_records":unassigned,"unassigned_clinical_count":unassigned.len(),
            "algorithm_version":"metric-history-v2","conversion_version":crate::laboratory::CONVERSION_VERSION}))
    }).await
}

fn history_chart<'a>(
    metric: &str,
    records: &'a [Value],
    points: &[Value],
) -> (Value, Vec<&'a str>) {
    let mut reported = BTreeSet::new();
    let mut comparable = BTreeSet::new();
    for record in records {
        let originals = record["original_records"].as_array();
        if let Some(originals) = originals.filter(|items| !items.is_empty()) {
            for original in originals {
                let quantity = &original["observation"]["valueQuantity"];
                let code = reported_unit(&quantity["code"]);
                if let Some(label) = reported_unit(&quantity["unit"]).or(code) {
                    reported.insert(label);
                    comparable.insert(crate::laboratory::history_comparison_unit(
                        metric,
                        code.unwrap_or(label),
                    ));
                }
            }
        } else if let Some(unit) = reported_unit(&record["unit"]) {
            reported.insert(unit);
            comparable.insert(crate::laboratory::history_comparison_unit(metric, unit));
        }
    }
    let plotted: BTreeSet<_> = points
        .iter()
        .filter_map(|point| reported_unit(&point["unit"]))
        .collect();
    if reported.len() > 1 && (comparable.len() > 1 || plotted.len() > 1) {
        return (json!({}), reported.into_iter().collect());
    }
    (point_ranges(points), Vec::new())
}

fn reported_unit(value: &Value) -> Option<&str> {
    value
        .as_str()
        .map(str::trim)
        .filter(|unit| !unit.is_empty() && *unit != "Not reported")
}

fn point_ranges(points: &[Value]) -> Value {
    crate::chart::ranges(
        points.iter().filter_map(|p| {
            Some((
                p["source_key"]
                    .as_str()
                    .or_else(|| p["source_id"].as_str())?
                    .to_owned(),
                p["unit"].as_str()?.to_owned(),
                p["timestamp"].as_f64()?,
                p["value"]
                    .as_f64()
                    .or_else(|| p["value"].as_str()?.parse::<f64>().ok())?,
            ))
        }),
        chrono_tz::UTC,
    )
}

fn append_health(
    record: &Value,
    metric: &str,
    records: &mut Vec<Value>,
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
        records.push(json!({"point_id":id,"source_key":format!("apple_health:{}", record["source_id"].as_str().unwrap_or("")),"source_id":record["source_id"],"source_name":"Apple Health",
            "sampled_at":date,"timestamp":record["start_at"],"display_value":payload["value"],"unit":payload["unit"],"original":record}));
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
        let mut matches: Vec<&str> = codes
            .into_iter()
            .flatten()
            .filter(|coding| {
                matches!(
                    coding["system"].as_str(),
                    Some("http://loinc.org" | "https://loinc.org")
                )
            })
            .filter_map(|coding| {
                coding["code"]
                    .as_str()
                    .and_then(crate::laboratory::loinc_metric)
            })
            .collect();
        if crate::clinical_labs::quest_total_vitamin_d(candidate, record) {
            matches = vec!["vitamin_d_25_oh_total"];
        }
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
        ocr_mapping_note: None,
        ocr_metric_candidate: None,
        ocr_section_heading: None,
        unmapped_choice: None,
        ocr_new_metric: None,
        ocr_normalization: None,
    };
    let Some((value, unit)) = crate::laboratory::normalized(&payload) else {
        excluded.push(json!({"point_id":id,"reason":"incompatible_unit_or_value"}));
        return;
    };
    points.push(json!({"point_id":id,"source_key":format!("apple_health:{}", original["source_id"].as_str().unwrap_or("")),"source_id":original["source_id"],"source_name":"Apple Health", "metric_id":metric,
        "sampled_at":date,"timestamp":timestamp,"value":value,"unit":unit,"original":original}));
}

pub(crate) async fn load_health<'a>(
    executor: impl sqlx::SqliteExecutor<'a>,
    user_id: &str,
    quantity_kind: &str,
) -> Result<Vec<Value>, AppError> {
    let mut stream = sqlx::query(queries::METRIC_HEALTH_HISTORY)
        .bind(user_id)
        .bind(quantity_kind)
        .fetch(executor);
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
    let records = load_health(&database.pool, user_id, "").await?;
    let metadata = database.lab_metric_metadata(user_id).await?;
    let reports = database.confirmed(user_id).await?;
    let mut custom_metrics = database.custom_metrics(user_id).await?;
    custom_metrics.extend(metadata.named_report_metrics());
    database
        .cpu
        .run(move || {
            let mut entries: BTreeMap<String, Value> = crate::clinical_labs::groups(&records)
                .into_iter()
                .map(|(key, group)| {
                    let mut definition = group.definition;
                    definition["loinc_codes"] = json!(group.loinc_codes);
                    (key, definition)
                })
                .collect();
            let definitions: BTreeMap<_, _> = crate::laboratory::metrics()
                .into_iter()
                .map(|metric| (metric.metric_id, metric))
                .collect();
            for observation in reports {
                let Some(id) = observation.browse_metric_id() else {
                    continue;
                };
                if entries.contains_key(id) {
                    continue;
                }
                if let Some(definition) = definitions.get(id) {
                    entries.insert(id.into(), serde_json::to_value(definition)?);
                } else if let Some(metric) =
                    custom_metrics.iter().find(|metric| metric.metric_id == id)
                {
                    entries.insert(id.into(), serde_json::to_value(metric)?);
                } else if let Some(metric) = observation.reported_metric() {
                    entries.insert(id.into(), metric);
                }
            }
            let mut metrics: Vec<Value> = entries.into_values().collect();
            for definition in &mut metrics {
                metadata.apply_name(definition);
            }
            metrics.sort_by_key(|metric| metric["name"].as_str().unwrap_or("").to_lowercase());
            Ok(json!({"metrics":metrics}))
        })
        .await
}
