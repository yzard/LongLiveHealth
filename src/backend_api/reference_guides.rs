//! Versioned, evidence-linked guidance. Unknown context is never treated as normal.
use crate::{app::AppState, error::AppError};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::LazyLock};

static CATALOG: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("reference_guides.json"))
        .expect("validated reference guide catalog")
});

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub birth_date: Option<String>,
    pub reference_sex: Option<String>,
}
impl Profile {
    pub fn validate(&self) -> Result<(), AppError> {
        if let Some(date) = &self.birth_date {
            let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map_err(|_| AppError::Invalid("Birth date must be YYYY-MM-DD"))?;
            if date.year() < 1850 || date > chrono::Utc::now().date_naive() {
                return Err(AppError::Invalid(
                    "Birth date is outside the supported range",
                ));
            }
        }
        if self
            .reference_sex
            .as_deref()
            .is_some_and(|s| !["male", "female"].contains(&s))
        {
            return Err(AppError::Invalid(
                "Reference sex must be male, female, or null",
            ));
        }
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationRequest {
    pub metric_id: String,
    pub value: f64,
    pub unit: String,
    pub measured_at: i64,
    pub context: BTreeMap<String, String>,
}

pub fn catalog() -> &'static Value {
    &CATALOG
}
pub fn guide(metric: &str) -> Value {
    let mut result = CATALOG["metrics"][metric].clone();
    if result.is_null() {
        result = json!({"metric_id":metric,"rules":[],"reason":"No verified guide for this metric identity; use its original laboratory reference interval.","sources":[]});
    }
    result["version"] = CATALOG["version"].clone();
    result["citations"] = Value::Array(
        result["sources"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|id| CATALOG["sources"].get(id.as_str()?))
            .cloned()
            .collect(),
    );
    result
}
fn unknown(reason: &str) -> Value {
    json!({"status":"not_assessed","label":"Not assessed","reason":reason})
}
fn age(profile: &Profile, at: i64) -> Option<i32> {
    let born = NaiveDate::parse_from_str(profile.birth_date.as_deref()?, "%Y-%m-%d").ok()?;
    let date = chrono::DateTime::from_timestamp(at, 0)?.date_naive();
    if date < born {
        return None;
    }
    Some(
        date.year()
            - born.year()
            - i32::from((date.month(), date.day()) < (born.month(), born.day())),
    )
}
fn numeric(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.parse().ok())
}
fn interval(value: f64, lo: Option<f64>, hi: Option<f64>, li: bool, ui: bool) -> Value {
    let (status, label) = if lo.is_some_and(|n| value < n || (!li && value == n)) {
        ("low", "Below range")
    } else if hi.is_some_and(|n| value > n || (!ui && value == n)) {
        ("high", "Above range")
    } else {
        ("in_range", "In range")
    };
    json!({"status":status,"label":label,"lower":lo,"upper":hi,"lower_inclusive":li,"upper_inclusive":ui})
}

pub fn evaluate(request: &EvaluationRequest, profile: &Profile) -> Value {
    if !request.value.is_finite() || request.value < 0.0 {
        return unknown("A finite, nonnegative numeric measurement is required");
    }
    let entry = &CATALOG["metrics"][&request.metric_id];
    if entry["rules"].as_array().is_none_or(Vec::is_empty) {
        return unknown(entry["reason"].as_str().unwrap_or("No verified interval"));
    }
    // All chart producers already normalize analyte-specific units. Never guess a conversion here.
    let expected = entry["unit"].as_str().unwrap_or("");
    let same_unit = request.unit == expected
        || crate::laboratory::canonical_unit(&request.unit)
            .zip(crate::laboratory::canonical_unit(expected))
            .is_some_and(|(a, b)| a == b);
    if !same_unit {
        return unknown(
            "Unit does not match this guide; analyte-specific normalization is required",
        );
    }
    let Some(age) = age(profile, request.measured_at) else {
        return unknown(
            "Set date of birth in Settings → Reference Profile to determine age at measurement",
        );
    };
    let pressure = matches!(
        request.metric_id.as_str(),
        "blood_pressure_systolic" | "blood_pressure_diastolic"
    );
    let pregnancy_unknown = !pressure
        && profile.reference_sex.as_deref() == Some("female")
        && entry["rules"]
            .as_array()
            .is_some_and(|rules| rules.iter().any(|r| r["kind"] != "fitness_percentile"))
        && request.context.get("pregnancy").map(String::as_str) != Some("not_pregnant");
    let candidates: Vec<_> = entry["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| {
            age as i64 >= r["min_age"].as_i64().unwrap_or(0)
                && r["max_age_exclusive"]
                    .as_i64()
                    .is_none_or(|max| (age as i64) < max)
                && (r["sex"].is_null() || r["sex"].as_str() == profile.reference_sex.as_deref())
        })
        .collect();
    let exact: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|r| {
            r["context"]
                .as_object()
                .into_iter()
                .flatten()
                .all(|(k, v)| request.context.get(k).map(String::as_str) == v.as_str())
        })
        .collect();
    let candidates = if exact.is_empty() { candidates } else { exact };
    if candidates.len() != 1 {
        return unknown(
            "No unique applicable rule: age, reference sex, specimen, fasting status, or measurement context is missing or unsupported",
        );
    }
    let missing_context: Vec<_> = candidates[0]["context"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(k, v)| request.context.get(*k).map(String::as_str) != v.as_str())
        .map(|(k, v)| format!("{}: {}", k.replace('_', " "), v.as_str().unwrap_or("")))
        .collect();
    if pregnancy_unknown || !missing_context.is_empty() {
        let reason = if pregnancy_unknown {
            "Pregnancy context at the measurement date is needed for this general-population guide"
                .to_owned()
        } else {
            format!(
                "Measurement context required: {}",
                missing_context.join(", ")
            )
        };
        let mut result = unknown(&reason);
        result["label"] = json!("Context required");
        // Show a conditional guide without assigning a normal/abnormal category.
        // Formula-derived bounds require the height transformation below and cannot be reused here.
        if candidates[0]["formula"].is_null() {
            result["applicable_rule"] = candidates[0].clone();
            result["unit"] = json!(expected);
            result["kind"] = json!("conditional_guide");
        }
        return result;
    }
    let mut selected_rule = candidates[0].clone();
    if selected_rule["formula"] == "adult_bmi_to_weight" {
        let height = request
            .context
            .get("height_m")
            .and_then(|v| v.parse::<f64>().ok());
        let Some(height) = height.filter(|v| v.is_finite() && *v > 0.0 && *v <= 3.0) else {
            return unknown(
                "A valid height recorded on or before this measurement is required for the BMI-based weight guide",
            );
        };
        for band in selected_rule["bands"].as_array_mut().into_iter().flatten() {
            for key in ["lower", "upper"] {
                if let Some(value) = numeric(&band[key]) {
                    band[key] = json!(value * height * height);
                }
            }
        }
        selected_rule["height_m"] = json!(height);
        selected_rule["height_measured_at"] = request
            .context
            .get("height_measured_at")
            .map_or(Value::Null, |v| json!(v));
    }
    let r = &selected_rule;
    let mut answer = if let Some(b) = r["bands"].as_array().into_iter().flatten().find(|b| {
        numeric(&b["lower"]).is_none_or(|v| {
            request.value > v || (request.value == v && b["lower_inclusive"] != false)
        }) && numeric(&b["upper"]).is_none_or(|v| {
            request.value < v || (request.value == v && b["upper_inclusive"] == true)
        })
    }) {
        json!({"status":b["status"],"label":b["label"],"lower":b["lower"],"upper":b["upper"],"lower_inclusive":b["lower_inclusive"],"upper_inclusive":b["upper_inclusive"]})
    } else if r["bands"].as_array().is_some_and(|bands| !bands.is_empty()) {
        return unknown("Measurement is outside the defined classification bands");
    } else {
        interval(
            request.value,
            numeric(&r["lower"]),
            numeric(&r["upper"]),
            r["lower_inclusive"] == true,
            r["upper_inclusive"] == true,
        )
    };
    // Return the exact population rule used, so clients never choose a different
    // age/sex interval from the complete research catalog.
    answer["applicable_rule"] = r.clone();
    answer["rule_id"] = r["id"].clone();
    answer["kind"] = r["kind"].clone();
    answer["source"] = r["source"].clone();
    answer["unit"] = json!(expected);
    answer["value"] = json!(request.value);
    answer["age_at_measurement"] = json!(age);
    answer["version"] = CATALOG["version"].clone();
    answer
}

/// The printed interval belongs to one observation only; never transfer it across sources.
pub(crate) fn report_assessment(point: &Value, value: f64, unit: &str) -> Option<Value> {
    let reference = &point["reference"];
    if reference["unit"].as_str()? != unit {
        return None;
    }
    let lo = numeric(&reference["lower"]);
    let hi = numeric(&reference["upper"]);
    if lo.is_none() && hi.is_none() || lo.zip(hi).is_some_and(|(a, b)| a > b) {
        return None;
    }
    let mut result = interval(
        value,
        lo,
        hi,
        reference["lower_inclusive"].as_bool().unwrap_or(true),
        reference["upper_inclusive"].as_bool().unwrap_or(true),
    );
    result["kind"] = json!("report_reference");
    result["source"] = json!("Original report reference interval");
    result["unit"] = json!(unit);
    Some(result)
}

/// Only a single unqualified FHIR interval with matching UCUM units is eligible.
/// Multiple sex/age/method alternatives require their explicit context, never first-match selection.
pub(crate) fn clinical_assessment(point: &Value, value: f64, unit: &str) -> Option<Value> {
    let ranges = point["original"]["clinical_observation"]["referenceRange"].as_array()?;
    if ranges.len() != 1 {
        return None;
    }
    let reference = &ranges[0];
    if !reference["age"].is_null()
        || reference["appliesTo"]
            .as_array()
            .is_some_and(|a| !a.is_empty())
    {
        return None;
    }
    let mut bounds = [None, None];
    for (index, key) in ["low", "high"].iter().enumerate() {
        let q = &reference[key];
        if q.is_null() {
            continue;
        }
        if !q["comparator"].is_null() {
            return None;
        }
        let raw_unit = q["code"]
            .as_str()
            .or_else(|| q["unit"].as_str())
            .or_else(|| point["original"]["valueQuantity"]["code"].as_str())
            .or_else(|| point["original"]["valueQuantity"]["unit"].as_str());
        // FHIR may omit the unit on both the result and its own interval. Comparison
        // remains valid on that unchanged observation; never transfer this to another unit.
        let unreported = unit == "Not reported"
            && raw_unit.is_none()
            && numeric(&point["original"]["valueQuantity"]["value"]) == Some(value);
        if unreported {
            bounds[index] = numeric(&q["value"]);
            bounds[index]?;
            continue;
        }
        let raw_unit = raw_unit?;
        if raw_unit != unit
            && crate::laboratory::canonical_unit(raw_unit)
                .zip(crate::laboratory::canonical_unit(unit))
                .is_none_or(|(a, b)| a != b)
        {
            return None;
        }
        bounds[index] = numeric(&q["value"]);
        bounds[index]?;
    }
    if bounds.iter().all(Option::is_none) || bounds[0].zip(bounds[1]).is_some_and(|(a, b)| a > b) {
        return None;
    }
    let mut answer = interval(value, bounds[0], bounds[1], true, true);
    answer["kind"] = json!("report_reference");
    answer["source"] = json!("Original clinical record reference interval");
    answer["unit"] = json!(unit);
    Some(answer)
}

pub fn annotate(result: &mut Value, metric: &str, profile: &Profile) {
    let points: Vec<Value> = result["points"].as_array().into_iter().flatten().map(|p| json!({
        "source_key":p["source_key"], "source_name":p["source_name"], "report_id":p["report_id"], "timestamp":p["timestamp"], "value":p["value"],
        "reference":p["reference"], "original":{"valueQuantity":if p["original"]["valueQuantity"].is_null() { &p["original"]["clinical_observation"]["valueQuantity"] } else { &p["original"]["valueQuantity"] },"clinical_observation":{"referenceRange":p["original"]["clinical_observation"]["referenceRange"]}}
    })).collect();
    let mut point_index: BTreeMap<(String, u64), Vec<&Value>> = BTreeMap::new();
    for point in &points {
        if let (Some(source), Some(at)) =
            (point["source_key"].as_str(), numeric(&point["timestamp"]))
        {
            point_index
                .entry((source.to_owned(), at.to_bits()))
                .or_default()
                .push(point);
        }
    }
    result["reference_guidance"] = guide(metric);
    if let Some(periods) = result["chart_ranges"].as_object_mut() {
        for (period, rows) in periods.iter_mut() {
            for row in rows.as_array_mut().into_iter().flatten() {
                let unit = row["unit"].as_str().unwrap_or("").to_owned();
                let field = if period == "reference_days" && metric == "vo2_max" {
                    "mean"
                } else {
                    "latest_value"
                };
                let Some(value) = numeric(&row[field]) else {
                    continue;
                };
                let Some(timestamp) = numeric(&row["last_at"]).filter(|n| n.is_finite()) else {
                    continue;
                };
                let at = timestamp.floor() as i64;
                let matching: Vec<_> = point_index
                    .get(&(
                        row["source"].as_str().unwrap_or("").to_owned(),
                        timestamp.to_bits(),
                    ))
                    .into_iter()
                    .flatten()
                    .filter(|p| numeric(&p["value"]) == Some(value))
                    .collect();
                let printed = if matching.len() == 1 {
                    report_assessment(matching[0], value, &unit)
                        .or_else(|| clinical_assessment(matching[0], value, &unit))
                } else {
                    None
                };
                let mut context: BTreeMap<String, String> = row["reference_context"]
                    .as_object()
                    .into_iter()
                    .flatten()
                    .filter_map(|(key, value)| value.as_str().map(|v| (key.clone(), v.into())))
                    .collect();
                if metric == "sleep" {
                    context.insert("duration".into(), "daily_total".into());
                }
                let mut assessment = printed.unwrap_or_else(|| {
                    evaluate(
                        &EvaluationRequest {
                            metric_id: metric.into(),
                            value,
                            unit,
                            measured_at: at,
                            context,
                        },
                        profile,
                    )
                });
                if assessment["kind"] == "report_reference" {
                    if let Some(point) = matching.first() {
                        assessment["reference_origin"] = json!({"source_key":point["source_key"],"name":point["source_name"],"report_id":point["report_id"]});
                    }
                    assessment["applicable_rule"] = json!({"id":format!("report:{}:{}:{}", assessment["unit"],assessment["lower"],assessment["upper"]),"kind":"report_reference","source":assessment["source"],"lower":assessment["lower"],"upper":assessment["upper"],"lower_inclusive":assessment["lower_inclusive"],"upper_inclusive":assessment["upper_inclusive"],"bands":[]});
                }
                assessment["measured_at"] = json!(at);
                row["reference_assessment"] = assessment;
            }
        }
    }
}

pub async fn profile(state: &AppState, user: &str) -> Result<Profile, AppError> {
    let preferences = state.database.wellness_preferences(user).await?;
    serde_json::from_value(preferences["preferences"]["reference_profile"].clone())
        .map_err(|_| AppError::Internal)
}

/// Keep history annotation off the async request executor, as with chart aggregation.
pub async fn enrich(
    state: &AppState,
    user: &str,
    metric: String,
    mut result: Value,
) -> Result<Value, AppError> {
    let profile = profile(state, user).await?;
    if metric == "body_mass" {
        add_height_context(&state.database, user, &profile, &mut result).await?;
    }
    state
        .database
        .cpu
        .run(move || {
            annotate(&mut result, &metric, &profile);
            Ok(result)
        })
        .await
}

/// Read height history once per chart request; never apply a future height to an older weight.
pub(crate) async fn add_height_context(
    db: &crate::database::Database,
    user: &str,
    profile: &Profile,
    result: &mut Value,
) -> Result<(), AppError> {
    use sqlx::Row;
    let end = result["chart_ranges"]
        .as_object()
        .into_iter()
        .flat_map(|p| p.values())
        .flat_map(|v| v.as_array().into_iter().flatten())
        .filter_map(|r| numeric(&r["last_at"]))
        .filter(|v| v.is_finite())
        .max_by(f64::total_cmp);
    let Some(end) = end else {
        return Ok(());
    };
    let query = crate::database::queries::reference_context_history_query();
    let records = sqlx::query(&query)
        .bind(user)
        .bind("height")
        .bind(-3_786_825_600_i64)
        .bind(end.floor() as i64 + 1)
        .fetch_all(&db.pool)
        .await?;
    let mut heights = Vec::new();
    for row in records {
        let at: f64 = row.try_get("at")?;
        if age(profile, at.floor() as i64).is_none_or(|v| v < 20) {
            continue;
        }
        let platform: String = row.try_get("platform")?;
        let unit: Option<String> = row.try_get("unit")?;
        let value: Option<f64> = row.try_get("value")?;
        if let Some(height) =
            crate::wellness::series::convert(&platform, "height", unit.as_deref(), value)
                .filter(|v| *v <= 3.0)
        {
            heights.push((at, height));
        }
    }
    for rows in result["chart_ranges"]
        .as_object_mut()
        .into_iter()
        .flat_map(|p| p.values_mut())
    {
        for row in rows.as_array_mut().into_iter().flatten() {
            let Some(at) = numeric(&row["last_at"]) else {
                continue;
            };
            let index = heights.partition_point(|(time, _)| *time <= at);
            if let Some((measured, height)) = index.checked_sub(1).and_then(|i| heights.get(i)) {
                row["reference_context"] = json!({"height_m":height.to_string(),"height_measured_at":measured.to_string()});
            }
        }
    }
    Ok(())
}
