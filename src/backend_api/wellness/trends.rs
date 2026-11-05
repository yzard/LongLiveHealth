//! Complete, source-preserving long-term histories for inline Trends cards.
use crate::{
    authentication::now,
    database::{Database, queries},
    error::AppError,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{Datelike, TimeZone};
use futures_util::TryStreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::collections::{BTreeMap, BTreeSet};

const BP: &str = "HKCorrelationTypeIdentifierBloodPressure";
const AHA: &str = "https://www.heart.org/en/health-topics/high-blood-pressure/understanding-blood-pressure-readings/monitoring-your-blood-pressure-at-home";
const APPLE: &str = "https://www.apple.com/healthcare/docs/site/Using_Apple_Watch_to_Estimate_Cardio_Fitness_with_VO2_max.pdf";
const EFLM: &str = "https://biologicalvariation.eu/";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub timezone: String,
}

#[derive(Default)]
struct Card {
    metric: String,
    title: String,
    source: String,
    source_name: String,
    unit: String,
    category: String,
    method: String,
    records: usize,
    missing: BTreeMap<String, usize>,
    samples: BTreeMap<String, Vec<(i64, f64)>>,
    reference_intervals: BTreeSet<String>,
    recent_start: Option<i64>,
}

impl Card {
    fn new(
        metric: &str,
        title: &str,
        source: &str,
        source_name: &str,
        unit: &str,
        category: &str,
        method: String,
    ) -> Self {
        Self {
            metric: metric.into(),
            title: title.into(),
            source: source.into(),
            source_name: source_name.into(),
            unit: unit.into(),
            category: category.into(),
            method,
            ..Self::default()
        }
    }

    fn finish(self, timezone: chrono_tz::Tz) -> Value {
        let mut dates = BTreeSet::new();
        let mut bounds = Vec::new();
        for samples in self.samples.values() {
            for &(at, _) in samples {
                if let Some(at) = timezone.timestamp_opt(at, 0).single() {
                    dates.insert(at.date_naive());
                    bounds.push(at.timestamp());
                }
            }
        }
        let first = bounds.iter().min().copied();
        let last = bounds.iter().max().copied();
        let span = dates
            .first()
            .zip(dates.last())
            .map(|(a, b)| (*b - *a).num_days())
            .unwrap_or(0);
        let ready = dates.len() >= 2
            && (["bloodWork", "daily"].contains(&self.category.as_str()) || span >= 30);
        let monthly = self.category != "bloodWork" && span > 90;
        let mut series = Vec::new();
        for (name, samples) in self.samples {
            let mut buckets = BTreeMap::<chrono::NaiveDate, BTreeSet<(i64, u64)>>::new();
            for (at, value) in samples {
                let Some(date) = timezone
                    .timestamp_opt(at, 0)
                    .single()
                    .map(|v| v.date_naive())
                else {
                    continue;
                };
                buckets
                    .entry(date)
                    .or_default()
                    .insert((at, value.to_bits()));
            }
            let mut plotted = BTreeMap::<chrono::NaiveDate, Vec<f64>>::new();
            for (date, samples) in buckets {
                let values: Vec<_> = samples
                    .iter()
                    .map(|(_, bits)| f64::from_bits(*bits))
                    .collect();
                if self.category == "bloodWork" {
                    // Dated lab results remain individual observations, with no interpolation.
                    for (at, bits) in samples {
                        plotted
                            .entry(
                                timezone
                                    .timestamp_opt(at, 0)
                                    .single()
                                    .expect("validated timestamp")
                                    .date_naive(),
                            )
                            .or_default()
                            .push(f64::from_bits(bits));
                    }
                } else {
                    let day_mean = values.iter().sum::<f64>() / values.len() as f64;
                    let bucket = if monthly {
                        date.with_day(1).expect("first of month")
                    } else {
                        date
                    };
                    plotted.entry(bucket).or_default().push(day_mean);
                }
            }
            let mut points = Vec::new();
            for (date, values) in plotted {
                let at = timezone
                    .from_local_datetime(&date.and_hms_opt(12, 0, 0).expect("valid noon"))
                    .earliest()
                    .map(|v| v.timestamp());
                let Some(at) = at else { continue };
                if self.category == "bloodWork" {
                    for value in values {
                        points.push(json!({"at":at,"value":value,"date":date.to_string()}));
                    }
                } else {
                    points.push(json!({"at":at,"value":values.iter().sum::<f64>() / values.len() as f64,"date":date.to_string()}));
                }
            }
            series.push(json!({"title":name,"points":points}));
        }
        let mut needed: Vec<String> = self
            .missing
            .into_iter()
            .map(|(reason, count)| {
                if count == 0 {
                    reason
                } else {
                    format!("{reason} ({count} records)")
                }
            })
            .collect();
        if !ready {
            needed.push(if self.category == "bloodWork" {
                "At least two dated, numeric results from the same source, unit, specimen and recorded assay method.".into()
            } else {
                "Measurements from the same source on at least two dates spanning 30 days.".into()
            });
        }
        let (citation, note) = match self.category.as_str() {
            "bloodWork" => (
                EFLM,
                "Individual collected results. Recorded assay and specimen identities are kept separate. Clinical significance of change also needs analytical imprecision and within-person biological variation.",
            ),
            "daily" => (
                "https://www.heart.org/en/healthy-living/fitness/fitness-basics/target-heart-rates",
                "Median daily values from 7 completed days compared with the preceding 28; each source is kept separate. Sleep uses your saved sleep goal.",
            ),
            "heart_bp" => (
                AHA,
                "Only explicitly paired systolic and diastolic readings are included. Each day contributes equally to a monthly average. A standardized home blood pressure assessment additionally needs repeated sessions, cuff and posture information.",
            ),
            _ => (
                APPLE,
                "Daily means or equally weighted averages of observed days in each month. Each device source is kept separate. Cardio fitness estimates depend on the measurement method and personal context.",
            ),
        };
        let identity = json!([self.metric, self.source, self.unit, self.method]).to_string();
        json!({
            "id":hex::encode(Sha256::digest(identity.as_bytes())),
            "metric_id":self.metric,"title":self.title,"source":self.source,"source_name":self.source_name,
            "unit":self.unit,"category":self.category,"state":if ready {"ready"} else {"insufficient_data"},
            "reference_interval":if self.reference_intervals.len()==1 {self.reference_intervals.first().and_then(|s| serde_json::from_str::<Value>(s).ok()).unwrap_or(Value::Null)} else {Value::Null},
            "recent_start":self.recent_start,
            "series":series,"observed_days":dates.len(),"first_at":first,"last_at":last,
            "record_count":self.records,"missing":needed,"method":self.method,
            "aggregation":if self.category=="bloodWork" {"Collected results"} else if monthly {"Monthly daily averages"} else {"Daily averages"},
            "explanation":note,"citation":citation,
            "comparison_note":if self.category=="bloodWork" && self.method=="[null,null]" {"Assay and specimen were not recorded."} else {""}
        })
    }
}

pub async fn get(database: &Database, user: &str, request: Request) -> Result<Value, AppError> {
    let tz: chrono_tz::Tz = request
        .timezone
        .parse()
        .map_err(|_| AppError::Invalid("Unknown IANA timezone"))?;
    let revision = database.data_revision(user).await?;
    let as_of = now()?;
    let health = crate::metric_history::load_health(
        database,
        user,
        "HKClinicalTypeIdentifierVitalSignRecord",
    )
    .await?;
    let (clinical_pressure, labs) = database
        .cpu
        .run(move || {
            let pressure = clinical_pressure_cards(&health, as_of);
            let labs = health
                .into_iter()
                .filter(|v| v["record_type"] == "HKClinicalTypeIdentifierLabResultRecord")
                .collect();
            Ok((pressure, labs))
        })
        .await?;
    let preferences = database.wellness_preferences(user).await?;
    let profile: crate::reference_guides::Profile =
        serde_json::from_value(preferences["preferences"]["reference_profile"].clone())?;
    let sleep_minutes = preferences["preferences"]["sleep_target_minutes"].as_f64();
    let mut cards = measurement_cards(database, user, as_of).await?;
    if !clinical_pressure.is_empty() {
        cards.retain(|card| card.metric != BP || card.records > 0);
        cards.extend(clinical_pressure);
    }
    cards.extend(laboratory_cards(database, user, as_of, labs).await?);
    cards.extend(daily_cards(database, user, as_of, &request.timezone).await?);
    if revision != database.data_revision(user).await? {
        return Err(AppError::Conflict("Archive changed; refresh Trends"));
    }
    let cards = database
        .cpu
        .run(move || {
            let mut result: Vec<_> = cards.into_iter().map(|card| card.finish(tz)).collect();
            result.retain_mut(|card| {
                if let Some(change) =
                    super::trend_change::classify(card, &profile, sleep_minutes, as_of)
                {
                    card["change"] = change;
                    true
                } else {
                    false
                }
            });
            result.sort_by(|a, b| {
                (a["change"]["direction"] != "worsening")
                    .cmp(&(b["change"]["direction"] != "worsening"))
                    .then_with(|| a["title"].as_str().cmp(&b["title"].as_str()))
                    .then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
            });
            Ok(result)
        })
        .await?;
    Ok(
        json!({"cards":cards,"data_revision":revision,"computed_at":as_of,"algorithm_version":"directional-trends-v2","complete":true}),
    )
}

async fn measurement_cards(
    database: &Database,
    user: &str,
    as_of: i64,
) -> Result<Vec<Card>, AppError> {
    let mut vo2 = BTreeMap::<String, Card>::new();
    let mut stream = sqlx::query(queries::MEASUREMENT_SAMPLES)
        .bind(user)
        .bind("vo2_max")
        .bind(i64::MIN)
        .bind(as_of + 1)
        .fetch(&database.pool);
    while let Some(row) = stream.try_next().await? {
        let platform: String = row.try_get("platform")?;
        let source = format!("{platform}:{}", row.try_get::<String, _>("source_id")?);
        let card = vo2.entry(source.clone()).or_insert_with(|| {
            Card::new(
                "vo2_max",
                "VO₂ Max",
                &source,
                "",
                "ml/kg*min",
                "heart_vo2",
                String::new(),
            )
        });
        card.records += 1;
        let at: f64 = row.try_get("at")?;
        let unit: Option<String> = row.try_get("unit")?;
        if let Some(value) =
            super::series::convert(&platform, "vo2_max", unit.as_deref(), row.try_get("value")?)
        {
            card.samples
                .entry("VO₂ Max".into())
                .or_default()
                .push((at as i64, value));
        } else {
            *card
                .missing
                .entry("Missing or incompatible VO₂ Max value/unit".into())
                .or_default() += 1;
        }
    }
    drop(stream);
    let mut pairs = BTreeMap::<(String, String, i64), BTreeMap<String, BTreeSet<u64>>>::new();
    let mut stream = sqlx::query(queries::COMBINED_SAMPLES)
        .bind(user)
        .bind(BP)
        .bind(i64::MIN)
        .bind(as_of + 1)
        .fetch(&database.pool);
    while let Some(row) = stream.try_next().await? {
        let source = format!(
            "{}:{}",
            row.try_get::<String, _>("platform")?,
            row.try_get::<String, _>("source_id")?
        );
        let key = (
            source,
            row.try_get("record_id")?,
            row.try_get::<i64, _>("at")?,
        );
        let components = pairs.entry(key).or_default();
        let kind: Option<String> = row.try_get("component")?;
        let raw_unit: Option<String> = row.try_get("unit")?;
        let value: Option<f64> = row.try_get("value")?;
        if let (Some(kind), Some(value), Some("mmHg")) = (
            kind,
            value.filter(|v| v.is_finite() && *v > 0.),
            raw_unit.as_deref(),
        ) {
            components.entry(kind).or_default().insert(value.to_bits());
        }
    }
    drop(stream);
    let mut pressure = BTreeMap::<String, Card>::new();
    for ((source, _, at), components) in pairs {
        let card = pressure.entry(source.clone()).or_insert_with(|| {
            Card::new(
                BP,
                "Blood Pressure",
                &source,
                "",
                "mmHg",
                "heart_bp",
                String::new(),
            )
        });
        card.records += 1;
        let sys = components
            .get("blood_pressure_systolic")
            .filter(|v| v.len() == 1)
            .and_then(|v| v.first())
            .map(|v| f64::from_bits(*v));
        let dia = components
            .get("blood_pressure_diastolic")
            .filter(|v| v.len() == 1)
            .and_then(|v| v.first())
            .map(|v| f64::from_bits(*v));
        if let (Some(sys), Some(dia)) = (sys, dia) {
            card.samples
                .entry("Systolic".into())
                .or_default()
                .push((at, sys));
            card.samples
                .entry("Diastolic".into())
                .or_default()
                .push((at, dia));
        } else {
            *card
                .missing
                .entry("Missing, unpaired or conflicting blood pressure components".into())
                .or_default() += 1;
        }
    }
    let mut result: Vec<_> = pressure.into_values().chain(vo2.into_values()).collect();
    for (metric, title, category, unit) in [
        (BP, "Blood Pressure", "heart_bp", "mmHg"),
        ("vo2_max", "VO₂ Max", "heart_vo2", "ml/kg*min"),
    ] {
        if !result.iter().any(|c| c.metric == metric) {
            result.push(Card::new(
                metric,
                title,
                "",
                "",
                "",
                category,
                String::new(),
            ));
            let card = result.last_mut().expect("inserted card");
            card.unit = unit.into();
            card.missing
                .insert("No matching measurements in the archive".into(), 0);
        }
    }
    Ok(result)
}

fn clinical_pressure_cards(records: &[Value], as_of: i64) -> Vec<Card> {
    let mut cards = BTreeMap::<String, Card>::new();
    for record in records
        .iter()
        .filter(|v| v["record_type"] == "HKClinicalTypeIdentifierVitalSignRecord")
    {
        let Some(resource) = record["payload"]["fhir"]["data_base64"]
            .as_str()
            .and_then(|v| STANDARD.decode(v).ok())
            .and_then(|v| serde_json::from_slice::<Value>(&v).ok())
        else {
            continue;
        };
        if resource["resourceType"] != "Observation" {
            continue;
        }
        let mut pair = BTreeMap::<String, BTreeSet<u64>>::new();
        for component in resource["component"].as_array().into_iter().flatten() {
            let name = component["code"]["coding"]
                .as_array()
                .into_iter()
                .flatten()
                .find_map(|code| {
                    if !matches!(
                        code["system"].as_str(),
                        Some("http://loinc.org" | "https://loinc.org")
                    ) {
                        return None;
                    }
                    match code["code"].as_str() {
                        Some("8480-6") => Some("Systolic"),
                        Some("8462-4") => Some("Diastolic"),
                        _ => None,
                    }
                });
            let Some(name) = name else {
                continue;
            };
            let values = pair.entry(name.into()).or_default();
            let quantity = &component["valueQuantity"];
            if quantity["comparator"].is_null()
                && matches!(
                    quantity["code"]
                        .as_str()
                        .or_else(|| quantity["unit"].as_str()),
                    Some("mm[Hg]" | "mmHg")
                )
                && quantity["system"]
                    .as_str()
                    .is_none_or(|v| v == "http://unitsofmeasure.org")
                && let Some(value) = quantity["value"]
                    .as_f64()
                    .filter(|v| v.is_finite() && *v > 0.)
            {
                values.insert(value.to_bits());
            }
        }
        if pair.is_empty() {
            continue;
        }
        let source = format!(
            "apple_health:{}",
            record["source_id"].as_str().unwrap_or("")
        );
        let name = record["payload"]["source_name"]
            .as_str()
            .unwrap_or("Hospital");
        let card = cards.entry(source.clone()).or_insert_with(|| {
            Card::new(
                BP,
                "Blood Pressure",
                &source,
                name,
                "mmHg",
                "heart_bp",
                "clinical:FHIR Observation".into(),
            )
        });
        card.records += 1;
        let at = resource["effectiveDateTime"]
            .as_str()
            .or_else(|| resource["effectivePeriod"]["start"].as_str())
            .and_then(|v| crate::reports::parse_time(v).ok())
            .filter(|v| *v <= as_of);
        if !matches!(
            resource["status"].as_str(),
            Some("final" | "amended" | "corrected")
        ) {
            *card
                .missing
                .entry("Final hospital result status is missing".into())
                .or_default() += 1;
        } else if at.is_none() {
            *card
                .missing
                .entry("Hospital collection date is missing, invalid or in the future".into())
                .or_default() += 1;
        } else if pair.len() != 2 || pair.values().any(|v| v.len() != 1) {
            *card
                .missing
                .entry(
                    "Complete hospital blood pressure pair with a compatible unit is missing"
                        .into(),
                )
                .or_default() += 1;
        } else {
            for (name, values) in pair {
                card.samples.entry(name).or_default().push((
                    at.expect("validated date"),
                    f64::from_bits(*values.first().expect("one value")),
                ));
            }
        }
    }
    cards.into_values().collect()
}

async fn laboratory_cards(
    database: &Database,
    user: &str,
    as_of: i64,
    health: Vec<Value>,
) -> Result<Vec<Card>, AppError> {
    let mut rows = database
        .cpu
        .run(move || {
            Ok(crate::clinical_labs::groups(&health)
                .into_values()
                .flat_map(|group| {
                    let title = group.definition["name"].as_str().unwrap_or("").to_owned();
                    group
                        .records
                        .into_iter()
                        .map(move |mut row| {
                            row["title"] = json!(title);
                            row
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>())
        })
        .await?;
    let mut report_sources = BTreeMap::new();
    let definitions = crate::laboratory::metrics();
    for observation in database.confirmed(user).await? {
        let Some(metric) = observation.resolved_metric_id.as_deref() else {
            continue;
        };
        let Some(definition) = definitions.iter().find(|d| d.metric_id == metric) else {
            continue;
        };
        if !report_sources.contains_key(&observation.report_id) {
            let report = database
                .report_summary(user, &observation.report_id)
                .await?;
            let context: Value = serde_json::from_str(&report.context_json)?;
            let name = context["source_name"]
                .as_str()
                .filter(|v| !v.trim().is_empty());
            report_sources.insert(
                observation.report_id.clone(),
                (
                    crate::sources::report_key(name, &observation.report_id),
                    name.unwrap_or(&report.original_name).to_owned(),
                ),
            );
        }
        let (source, name) = &report_sources[&observation.report_id];
        let normalized = crate::laboratory::normalized(&observation.payload);
        let at = observation
            .payload
            .sampled_at
            .as_deref()
            .and_then(|v| crate::reports::parse_time(v).ok());
        let reference = crate::laboratory::interpret(&observation.payload).reference;
        rows.push(json!({
            "metric_id":metric,"title":definition.name,"source_key":source,"source_name":name,
            "timestamp":at,"value":normalized.as_ref().map(|(value,_)|value),
            "unit":normalized.as_ref().map(|(_,unit)|unit).or(observation.payload.raw_unit.as_ref()),
            "plotted":normalized.is_some() && at.is_some(),"reference":reference,"original":{}
        }));
    }
    database
        .cpu
        .run(move || {
            let mut cards = group_labs(rows, as_of);
            if cards.is_empty() {
                let mut card = Card::new(
                    "blood_work",
                    "Blood Work",
                    "",
                    "",
                    "",
                    "bloodWork",
                    "[null,null]".into(),
                );
                card.missing.insert(
                    "No confirmed blood test results or hospital laboratory observations".into(),
                    0,
                );
                cards.push(card);
            }
            Ok(cards)
        })
        .await
}

fn coded_identity(concept: &Value) -> Value {
    let codes: BTreeSet<_> = concept["coding"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|code| Some((code["system"].as_str()?, code["code"].as_str()?)))
        .collect();
    if !codes.is_empty() {
        return json!(codes);
    }
    concept["text"]
        .as_str()
        .filter(|v| !v.trim().is_empty())
        .map(|v| json!(v.trim().to_lowercase()))
        .unwrap_or(Value::Null)
}

fn group_labs(rows: Vec<Value>, as_of: i64) -> Vec<Card> {
    let mut cards = BTreeMap::<String, Card>::new();
    for row in rows {
        let metric = row["metric_id"].as_str().unwrap_or("");
        let title = row["title"].as_str().unwrap_or(metric);
        if crate::browse_catalog::classify(metric, title, true) != "bloodWork" {
            continue;
        }
        let source = row["source_key"].as_str().unwrap_or("");
        let name = row["source_name"].as_str().unwrap_or("");
        let unit = row["unit"].as_str().unwrap_or("");
        let method = json!([
            coded_identity(&row["method"]),
            coded_identity(&row["specimen"]["type"])
        ])
        .to_string();
        let key = json!([metric, source, unit, method]).to_string();
        let card = cards
            .entry(key)
            .or_insert_with(|| Card::new(metric, title, source, name, unit, "bloodWork", method));
        card.records += 1;
        let at = row["timestamp"].as_i64();
        let value = row["value"]
            .as_f64()
            .or_else(|| row["value"].as_str()?.parse::<f64>().ok())
            .filter(|v| v.is_finite());
        let reason = if at.is_none() {
            "Collection date is missing"
        } else if at.is_some_and(|at| at > as_of) {
            "Collection date is in the future"
        } else if unit.is_empty() || unit == "Not reported" {
            "Measurement unit is missing"
        } else if value.is_none() || row["plotted"] != true {
            "Exact numeric value or compatible unit is missing"
        } else {
            ""
        };
        if reason.is_empty() {
            let value = value.expect("validated value");
            let mut reference_point = row.clone();
            if !row["original"].is_null() && row["original"]["clinical_observation"].is_null() {
                reference_point["original"] = json!({"clinical_observation":row["original"]});
            }
            let printed = crate::reference_guides::report_assessment(&reference_point, value, unit)
                .or_else(|| {
                    crate::reference_guides::clinical_assessment(&reference_point, value, unit)
                });
            let bounds = printed
                .map(|p| json!({"lower":p["lower"],"upper":p["upper"]}).to_string())
                .unwrap_or_default();
            card.reference_intervals.insert(bounds);
            card.samples
                .entry(title.into())
                .or_default()
                .push((at.expect("validated date"), value));
        } else {
            *card.missing.entry(reason.into()).or_default() += 1;
        }
    }
    cards.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laboratory_cards_require_distinct_dates_and_keep_methods_sources_and_units_separate() {
        let base = json!({"metric_id":"apob","title":"Apolipoprotein B","source_key":"report:lab",
            "source_name":"Lab","timestamp":1609459200,"value":"90","unit":"mg/dL","plotted":true});
        let mut next = base.clone();
        next["timestamp"] = json!(1640995200);
        next["value"] = json!("80");
        next["specimen"] = json!({"reference":"Specimen/second-collection"});
        let mut method = next.clone();
        method["method"] = json!({"text":"different assay"});
        let mut source = next.clone();
        source["source_key"] = json!("report:other");
        let mut missing = base.clone();
        missing["timestamp"] = Value::Null;
        missing["value"] = Value::Null;
        missing["plotted"] = json!(false);
        let cards: Vec<_> = group_labs(
            vec![base.clone(), base, next, method, source, missing],
            1700000000,
        )
        .into_iter()
        .map(|c| c.finish(chrono_tz::UTC))
        .collect();
        assert_eq!(cards.len(), 3);
        assert_eq!(cards.iter().filter(|c| c["state"] == "ready").count(), 1);
        let ready = cards.iter().find(|c| c["state"] == "ready").unwrap();
        assert_eq!(ready["series"][0]["points"].as_array().unwrap().len(), 2);
        assert_eq!(ready["observed_days"], 2);
        assert!(!ready["missing"].as_array().unwrap().is_empty());
        let ids: BTreeSet<_> = cards.iter().map(|c| c["id"].as_str().unwrap()).collect();
        assert_eq!(ids.len(), cards.len());
    }
}

#[cfg(test)]
mod clinical_tests {
    use super::*;

    #[test]
    fn hospital_pressure_uses_only_complete_final_loinc_pairs() {
        let mut records = Vec::new();
        for date in ["2020-01-01", "2021-01-01"] {
            let resource = json!({"resourceType":"Observation","status":"final","effectiveDateTime":date,
            "component":[
                {"code":{"coding":[{"system":"http://loinc.org","code":"8480-6"}]},"valueQuantity":{"value":120,"code":"mm[Hg]","system":"http://unitsofmeasure.org"}},
                {"code":{"coding":[{"system":"http://loinc.org","code":"8462-4"}]},"valueQuantity":{"value":80,"code":"mm[Hg]","system":"http://unitsofmeasure.org"}}
            ]});
            records.push(json!({"record_type":"HKClinicalTypeIdentifierVitalSignRecord","source_id":"hospital",
                "payload":{"source_name":"Hospital","fhir":{"data_base64":STANDARD.encode(resource.to_string())}}}));
        }
        let mut incomplete = json!({"resourceType":"Observation","status":"final","effectiveDateTime":"2022-01-01",
            "component":[{"code":{"coding":[{"system":"http://loinc.org","code":"8480-6"}]},"valueQuantity":{"value":199,"code":"mm[Hg]"}}]});
        records.push(
            json!({"record_type":"HKClinicalTypeIdentifierVitalSignRecord","source_id":"hospital",
            "payload":{"fhir":{"data_base64":STANDARD.encode(incomplete.to_string())}}}),
        );
        incomplete["status"] = json!("preliminary");
        records.push(
            json!({"record_type":"HKClinicalTypeIdentifierVitalSignRecord","source_id":"hospital",
            "payload":{"fhir":{"data_base64":STANDARD.encode(incomplete.to_string())}}}),
        );
        let cards = clinical_pressure_cards(&records, 1700000000);
        assert_eq!(cards.len(), 1);
        let card = cards.into_iter().next().unwrap().finish(chrono_tz::UTC);
        assert_eq!(card["state"], "ready");
        assert_eq!(card["observed_days"], 2);
        assert_eq!(card["series"].as_array().unwrap().len(), 2);
        assert_eq!(card["missing"].as_array().unwrap().len(), 2);
    }
}

async fn daily_cards(
    database: &Database,
    user: &str,
    as_of: i64,
    timezone: &str,
) -> Result<Vec<Card>, AppError> {
    let tz: chrono_tz::Tz = timezone
        .parse()
        .map_err(|_| AppError::Invalid("Unknown IANA timezone"))?;
    let today = tz
        .timestamp_opt(as_of, 0)
        .single()
        .ok_or(AppError::Internal)?
        .date_naive();
    let first = today - chrono::Days::new(35);
    let last = today - chrono::Days::new(1);
    let recent = tz
        .from_local_datetime(
            &(today - chrono::Days::new(7))
                .and_hms_opt(0, 0, 0)
                .ok_or(AppError::Internal)?,
        )
        .earliest()
        .ok_or(AppError::Internal)?
        .timestamp();
    let mut cards = Vec::new();
    for (kind, title) in [
        ("sleep", "Sleep"),
        ("resting_heart_rate", "Resting Heart Rate"),
    ] {
        let aggregate = database
            .daily_period(user, kind, first, last, timezone)
            .await?;
        let response = super::select_metric(kind, &last.to_string(), &[], &aggregate)?;
        let mut grouped = BTreeMap::<(String, String), BTreeMap<String, BTreeSet<u64>>>::new();
        for day in response["history"].as_array().into_iter().flatten() {
            let date = day["date"].as_str().unwrap_or("");
            for row in day["alternatives"].as_array().into_iter().flatten() {
                if let (Some(source), Some(unit), Some(value)) = (
                    row["source"].as_str(),
                    row["unit"].as_str(),
                    row["value"].as_f64().filter(|v| v.is_finite() && *v >= 0.),
                ) {
                    grouped
                        .entry((source.into(), unit.into()))
                        .or_default()
                        .entry(date.into())
                        .or_default()
                        .insert(value.to_bits());
                }
            }
        }
        for ((source, unit), days) in grouped {
            let mut card = Card::new(kind, title, &source, "", &unit, "daily", String::new());
            card.recent_start = Some(recent);
            for (date, values) in days {
                if values.len() != 1 {
                    continue;
                }
                let Ok(date) = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d") else {
                    continue;
                };
                let Some(at) = tz
                    .from_local_datetime(&date.and_hms_opt(12, 0, 0).expect("valid noon"))
                    .earliest()
                else {
                    continue;
                };
                card.samples.entry(title.into()).or_default().push((
                    at.timestamp(),
                    f64::from_bits(*values.first().expect("one value")),
                ));
                card.records += 1;
            }
            cards.push(card);
        }
    }
    Ok(cards)
}
