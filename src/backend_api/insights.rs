//! Deterministic, auditable prevention and training evidence; not LLM-estimated risk.
use crate::{app::AppState, authentication::CurrentUser, error::AppError};
use axum::{Json, extract::State};
use chrono::{Datelike, TimeZone};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub mod dashboard;
pub mod history;
mod trend_change;

const BP: &str = "HKCorrelationTypeIdentifierBloodPressure";
const AHA: &str =
    "https://professional.heart.org/en/guidelines-and-statements/about-prevent-calculator";
const CDC: &str = "https://wwwn.cdc.gov/KDSS/detail.aspx?Qnum=Q811&topic=1";
const HRV: &str = "https://pubmed.ncbi.nlm.nih.gov/24578692/";

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub known_cvd: Option<bool>,
    pub diabetes: Option<bool>,
    pub current_smoking: Option<bool>,
    pub antihypertensive: Option<bool>,
    pub statin: Option<bool>,
    pub egfr_2021_confirmed: Option<bool>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub timezone: String,
}

pub async fn get(
    State(state): State<AppState>,
    current: CurrentUser,
    Json(request): Json<Request>,
) -> Result<Json<Value>, AppError> {
    for attempt in 0..3 {
        match snapshot(&state, &current.user.user_id, request.clone()).await {
            Err(AppError::Conflict(_)) if attempt < 2 => {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
            result => return result.map(Json),
        }
    }
    Err(AppError::Conflict(
        "Archive is updating; refresh Insights shortly",
    ))
}

pub async fn snapshot(state: &AppState, user: &str, request: Request) -> Result<Value, AppError> {
    let revision = state.database.data_revision(user).await?;
    let preferences = state.database.wellness_preferences(user).await?;
    let profile: crate::reference_guides::Profile =
        serde_json::from_value(preferences["preferences"]["reference_profile"].clone())?;
    let context: Context =
        serde_json::from_value(preferences["preferences"]["cardiovascular"].clone())?;
    let history = history::snapshot(
        &state.database,
        user,
        history::Request {
            timezone: request.timezone.clone(),
        },
    )
    .await?;
    let as_of = history["computed_at"].as_i64().ok_or(AppError::Internal)?;
    let tz: chrono_tz::Tz = request
        .timezone
        .parse()
        .map_err(|_| AppError::Invalid("Unknown timezone"))?;
    let today = tz
        .timestamp_opt(as_of, 0)
        .single()
        .ok_or(AppError::Internal)?
        .date_naive();
    let mut cards = history["cards"]
        .as_array()
        .cloned()
        .ok_or(AppError::Internal)?;
    // RMSSD and Recovery HRV stay distinct from SDNN and from each other.
    for (kind, title) in [
        ("hrv_rmssd", "HRV RMSSD"),
        (crate::recovery_hrv::KIND, "Recovery HRV"),
        ("heart_rate", "Daily Heart Rate"),
        ("respiratory_rate", "Respiratory Rate"),
        ("oxygen_saturation", "Blood Oxygen"),
        (
            "HKQuantityTypeIdentifierAppleSleepingWristTemperature",
            "Sleeping Wrist Temperature",
        ),
    ] {
        let lower = today - chrono::Days::new(if kind == "heart_rate" { 7 } else { 35 });
        let start = tz
            .from_local_datetime(&lower.and_hms_opt(0, 0, 0).ok_or(AppError::Internal)?)
            .earliest()
            .ok_or(AppError::Internal)?
            .timestamp();
        let end = tz
            .from_local_datetime(&today.and_hms_opt(0, 0, 0).ok_or(AppError::Internal)?)
            .earliest()
            .ok_or(AppError::Internal)?
            .timestamp();
        let chart = state
            .database
            .measurement_chart(
                user,
                crate::wellness::measurement_chart::Request {
                    record_type: kind.into(),
                    timezone: request.timezone.clone(),
                    period: "M".into(),
                    start_at: start,
                    end_at: if kind == "hrv_rmssd" || kind == crate::recovery_hrv::KIND {
                        end
                    } else {
                        as_of
                    },
                },
            )
            .await?;
        let rows = chart["chart_ranges"]["reference_days"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let sources: BTreeSet<_> = rows
            .iter()
            .filter_map(|v| v["source"].as_str())
            .map(str::to_owned)
            .collect();
        for source in sources {
            let points: Vec<_> = rows
                .iter()
                .filter(|v| v["source"] == source)
                .filter_map(|v| {
                    Some(json!({"at":v["start_at"].as_i64()?,"value":v["mean"].as_f64()?}))
                })
                .collect();
            cards.push(json!({"id":format!("{kind}:{source}"),"metric_id":kind,"title":title,"unit":chart["unit"],"source":source,"series":[{"title":kind,"points":points}]}));
        }
    }
    let sleep = state
        .database
        .sleep_sessions(
            user,
            crate::wellness::sleep::SleepRequest {
                start_at: as_of - 15 * 86400,
                end_at: as_of,
                timezone: request.timezone.clone(),
            },
        )
        .await?;
    if revision != state.database.data_revision(user).await? {
        return Err(AppError::Conflict("Archive changed; refresh Insights"));
    }
    let sleep_goal = preferences["preferences"]["sleep_target_minutes"].as_f64();
    state.database.cpu.run(move || {
        let cardiac = cardiovascular(&cards, &profile, &context, as_of, today);
        let training = recovery(&cards, sleep_goal, as_of, today, tz);
        let dashboard = dashboard::build(&cards, &cardiac, &training, &sleep, sleep_goal, as_of);
        Ok(json!({"dashboard":dashboard,"cardiovascular":cardiac,"training":training,"computed_at":as_of,"data_revision":revision,"method_version":"integrated-insights-v1","timezone":tz.name()}))
    }).await
}

fn points(card: &Value, component: Option<&str>) -> Vec<(i64, f64)> {
    card["series"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|s| component.is_none_or(|name| s["title"] == name))
        .flat_map(|s| s["points"].as_array().into_iter().flatten())
        .filter_map(|p| {
            Some((
                p["at"].as_i64()?,
                p["value"].as_f64().filter(|v| v.is_finite())?,
            ))
        })
        .collect()
}

fn latest(
    cards: &[Value],
    metric: &str,
    component: Option<&str>,
    unit: &str,
    as_of: i64,
    max_days: i64,
) -> Value {
    let mut rows = Vec::new();
    for card in cards.iter().filter(|c| c["metric_id"] == metric) {
        if card["unit"] != unit {
            continue;
        }
        for (at, value) in points(card, component) {
            if at <= as_of && as_of - at <= max_days * 86400 {
                let point = card["series"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .flat_map(|s| s["points"].as_array().into_iter().flatten())
                    .find(|p| p["at"].as_i64() == Some(at) && p["value"].as_f64() == Some(value));
                rows.push(json!({"sampled_at":point.map(|p| p["sampled_at"].clone()),"at":at,"value":value,"unit":unit,"source":card["source"],"source_name":card["source_name"],"evidence_id":card["id"],"title":card["title"],"report_assessment":card["series"].as_array().into_iter().flatten().flat_map(|s| s["points"].as_array().into_iter().flatten()).find(|p| p["at"].as_i64()==Some(at) && p["value"].as_f64()==Some(value)).map(|p| p["report_assessment"].clone())}));
            }
        }
    }
    rows.sort_by_key(|r| r["at"].as_i64());
    // A censored or qualitative latest result is evidence, not an absent measurement.
    // Never replace it with an older exact value or coerce an inequality into equality.
    let mut reported: Vec<_> = cards
        .iter()
        .filter(|c| c["metric_id"] == metric)
        .flat_map(|c| {
            c["reported_results"]
                .as_array()
                .into_iter()
                .flatten()
                .map(move |r| (c, r))
        })
        .filter(|(_, r)| {
            r["at"]
                .as_i64()
                .is_some_and(|at| at <= as_of && as_of - at <= max_days * 86400)
        })
        .filter(|(_, r)| {
            r["printed_value"]
                .as_str()
                .is_some_and(|s| !s.trim().is_empty())
        })
        .collect();
    reported.sort_by_key(|(_, r)| r["at"].as_i64());
    if let Some((card, record)) = reported.last()
        && rows
            .last()
            .is_none_or(|last| record["at"].as_i64() >= last["at"].as_i64())
    {
        let date = chrono::DateTime::from_timestamp(record["at"].as_i64().unwrap(), 0)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();
        return json!({"state":"reported","metric_id":metric,"title":card["title"],"at":record["at"],"source":card["source"],"source_name":card["source_name"],"unit":record["unit"],"printed_value":record["printed_value"],"sampled_at":record["sampled_at"],
            "reason":format!("Reported {} {} · {} · {}. {}. Not used as an exact numeric input.",record["printed_value"].as_str().unwrap_or(""),record["unit"].as_str().unwrap_or(""),date,card["source_name"].as_str().unwrap_or(""),record["reason"].as_str().unwrap_or(""))});
    }
    let Some(last) = rows.last() else {
        return json!({"state":"missing","metric_id":metric,"reason":format!("No compatible {metric} measurement within {max_days} days")});
    };
    if rows
        .iter()
        .filter(|r| r["at"] == last["at"])
        .any(|r| r["value"] != last["value"])
    {
        return json!({"state":"conflicting","metric_id":metric,"reason":format!("Conflicting {metric} results at the latest collection time; select a clinically representative result")});
    }
    let mut result = last.clone();
    result["state"] = json!("available");
    result["metric_id"] = json!(metric);
    result
}

/// Independent implementation of the base 10-year total-CVD equation published by CDC.
/// No ASCVD cutoffs, optional predictors, substituted values or out-of-domain clamping.
pub fn prevent_cvd(
    age: f64,
    female: bool,
    tc: f64,
    hdl: f64,
    sbp: f64,
    egfr: f64,
    context: &Context,
) -> Option<f64> {
    if context.known_cvd != Some(false)
        || context.egfr_2021_confirmed != Some(true)
        || !(30.0..=79.0).contains(&age)
        || !(130.0..=320.0).contains(&tc)
        || !(20.0..=100.0).contains(&hdl)
        || hdl >= tc
        || !(90.0..=200.0).contains(&sbp)
        || !(15.0..=140.0).contains(&egfr)
    {
        return None;
    }
    let d = f64::from(u8::from(context.diabetes?));
    let s = f64::from(u8::from(context.current_smoking?));
    let t = f64::from(u8::from(context.antihypertensive?));
    let st = f64::from(u8::from(context.statin?));
    let a = (age - 55.0) / 10.0;
    let n = (tc - hdl) * 0.02586 - 3.5;
    let h = (hdl * 0.02586 - 1.3) / 0.3;
    let low = (sbp.min(110.0) - 110.0) / 20.0;
    let high = (sbp.max(110.0) - 130.0) / 20.0;
    let g = (egfr.min(60.0) - 60.0) / -15.0;
    let g2 = (egfr.max(60.0) - 90.0) / -15.0;
    let x = if female {
        -3.307728 + 0.7939329 * a + 0.0305239 * n - 0.1606857 * h - 0.2394003 * low
            + 0.360078 * high
            + 0.8667604 * d
            + 0.5360739 * s
            + 0.6045917 * g
            + 0.0433769 * g2
            + 0.3151672 * t
            - 0.1477655 * st
            - 0.0663612 * t * high
            + 0.1197879 * st * n
            - 0.0819715 * a * n
            + 0.0306769 * a * h
            - 0.0946348 * a * high
            - 0.27057 * a * d
            - 0.078715 * a * s
            - 0.1637806 * a * g
    } else {
        -3.031168 + 0.7688528 * a + 0.0736174 * n - 0.0954431 * h - 0.4347345 * low
            + 0.3362658 * high
            + 0.7692857 * d
            + 0.4386871 * s
            + 0.5378979 * g
            + 0.0164827 * g2
            + 0.288879 * t
            - 0.1337349 * st
            - 0.0475924 * t * high
            + 0.150273 * st * n
            - 0.0517874 * a * n
            + 0.0191169 * a * h
            - 0.1049477 * a * high
            - 0.2251948 * a * d
            - 0.0895067 * a * s
            - 0.1543702 * a * g
    };
    Some(100.0 / (1.0 + (-x).exp()))
}

fn cardiovascular(
    cards: &[Value],
    profile: &crate::reference_guides::Profile,
    context: &Context,
    as_of: i64,
    date: chrono::NaiveDate,
) -> Value {
    let born = profile
        .birth_date
        .as_deref()
        .and_then(|v| chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").ok());
    let age = born.filter(|b| *b <= date).map(|b| {
        date.year() - b.year() - i32::from((date.month(), date.day()) < (b.month(), b.day()))
    });
    let mut inputs = vec![
        latest(cards, "total_cholesterol", None, "mg/dL", as_of, 365),
        latest(cards, "hdl_cholesterol", None, "mg/dL", as_of, 365),
        latest(cards, BP, Some("Systolic"), "mmHg", as_of, 90),
        latest(
            cards,
            "egfr_creatinine_unspecified",
            None,
            "mL/min/1.73m2",
            as_of,
            365,
        ),
    ];
    let mut missing: Vec<String> = inputs
        .iter()
        .filter(|v| v["state"] != "available")
        .filter_map(|v| v["reason"].as_str().map(str::to_owned))
        .collect();
    if age.is_none() {
        missing.push("Date of birth in Reference Settings".into());
    }
    if !matches!(profile.reference_sex.as_deref(), Some("male" | "female")) {
        missing.push("Biological sex in Reference Settings".into());
    }
    for (name, value) in [
        ("Known cardiovascular disease", context.known_cvd),
        ("Diabetes diagnosis", context.diabetes),
        ("Current smoking", context.current_smoking),
        ("Blood pressure medication", context.antihypertensive),
        ("Statin use", context.statin),
        (
            "eGFR uses the CKD-EPI 2021 creatinine equation",
            context.egfr_2021_confirmed,
        ),
    ] {
        if value.is_none() {
            missing.push(format!("Confirm: {name}"));
        }
    }
    if context.egfr_2021_confirmed == Some(false) {
        missing
            .push("An eGFR measured using CKD-EPI 2021 is required for this implementation".into());
    }
    let values: Option<Vec<f64>> = inputs.iter().map(|v| v["value"].as_f64()).collect();
    let risk = if missing.is_empty() {
        values.and_then(|v| {
            prevent_cvd(
                f64::from(age?),
                profile.reference_sex.as_deref() == Some("female"),
                v[0],
                v[1],
                v[2],
                v[3],
                context,
            )
        })
    } else {
        None
    };
    let eligibility = if context.known_cvd == Some(true) {
        "Not applicable: PREVENT estimates first events in people without established cardiovascular disease."
    } else if risk.is_none() && missing.is_empty() {
        "The available inputs are outside the validated base-model domain; no risk percentage is calculated."
    } else {
        "Base-model 10-year risk of total cardiovascular disease, including ASCVD and heart failure. This is not a diagnosis or an ASCVD-only risk percentage."
    };
    // Assess both components from the same paired source/day; only SBP enters PREVENT.
    if inputs[2]["state"] == "available" {
        let paired: Vec<_> = cards
            .iter()
            .filter(|c| c["id"] == inputs[2]["evidence_id"])
            .cloned()
            .collect();
        let mut diastolic = latest(&paired, BP, Some("Diastolic"), "mmHg", as_of, 90);
        if diastolic["at"] == inputs[2]["at"] {
            diastolic["metric_id"] = json!("blood_pressure_diastolic");
            diastolic["title"] = json!("Blood Pressure · Diastolic");
            inputs.push(diastolic);
        }
        inputs[2]["title"] = json!("Blood Pressure · Systolic");
    }
    for metric in [
        "ldl_cholesterol",
        "apob",
        "non_hdl_cholesterol",
        "triglycerides",
        "lipoprotein_a",
        "hs_crp",
        "hba1c",
    ] {
        let unit = crate::laboratory::metrics()
            .into_iter()
            .find(|m| m.metric_id == metric)
            .map(|m| m.standard_unit)
            .unwrap_or_default();
        inputs.push(latest(cards, metric, None, unit, as_of, 365));
    }
    let selected: Vec<Value> = cards
        .iter()
        .filter(|c| {
            [
                "total_cholesterol",
                "hdl_cholesterol",
                "ldl_cholesterol",
                "apob",
                "non_hdl_cholesterol",
                "triglycerides",
                "lipoprotein_a",
                "hs_crp",
                "hba1c",
                "vo2_max",
                BP,
            ]
            .contains(&c["metric_id"].as_str().unwrap_or(""))
        })
        .cloned()
        .collect();
    let mut findings = Vec::new();
    for input in &inputs {
        if let (Some(metric), Some(value), Some(unit), Some(at)) = (
            input["metric_id"].as_str(),
            input["value"].as_f64(),
            input["unit"].as_str(),
            input["at"].as_i64(),
        ) {
            let general = crate::reference_guides::evaluate(
                &crate::reference_guides::EvaluationRequest {
                    metric_id: if metric == BP {
                        "blood_pressure_systolic".into()
                    } else {
                        metric.into()
                    },
                    value,
                    unit: unit.into(),
                    measured_at: at,
                    context: Default::default(),
                },
                profile,
            );
            let evaluation = if !input["report_assessment"].is_null() {
                input["report_assessment"].clone()
            } else {
                general
            };
            if matches!(
                evaluation["status"].as_str(),
                Some(
                    "high"
                        | "low"
                        | "very_high"
                        | "very_low"
                        | "borderline_high"
                        | "high_risk"
                        | "elevated"
                )
            ) {
                findings.push(json!({"title":input["title"],"text":format!("{} {} · {}",value,unit,evaluation["label"].as_str().unwrap_or("Outside guide")),"evidence_id":input["evidence_id"],"assessment":evaluation}));
            }
        }
    }
    json!({"title":"Cardiovascular Review","risk_percent":risk,"risk_label":"10-year total CVD risk","age":age,"inputs":inputs,"findings":findings,"missing":missing,"explanation":eligibility,"cards":selected,
        "actions":["Review elevated lipid, ApoB and blood pressure findings together with a clinician; a favorable wearable trend does not cancel these findings.","Confirm representative blood pressure, current diagnoses and medication status before using the risk estimate for prevention planning."],
        "method":"PREVENT base total-CVD equation; independently implemented from the CDC-published formula. Latest compatible laboratory data within 365 days and paired blood pressure daily mean within 90 days; these freshness windows are application rules, not AHA thresholds. Inputs can come from different dates and are displayed for review. ApoB, Lp(a), HRV and heart-rate volume are contextual evidence, not added to the equation. No optional HbA1c, UACR or SDI equation is used.",
        "references":[{"title":"AHA PREVENT — scope and validation","url":AHA},{"title":"CDC — published base CVD equation and coefficients","url":CDC},{"title":"AHA — October 2026 update (base equations unaffected)","url":"https://professional.heart.org/-/media/PHD-Files/Guidelines-and-Statements/PREVENT/PREVENT-Update-FAQ-101426.pdf"}]})
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    Some((values[(values.len() - 1) / 2] + values[values.len() / 2]) / 2.0)
}

pub fn recovery(
    cards: &[Value],
    sleep_goal: Option<f64>,
    as_of: i64,
    today: chrono::NaiveDate,
    tz: chrono_tz::Tz,
) -> Value {
    let recent = today - chrono::Days::new(7);
    let baseline = today - chrono::Days::new(35);
    let mut signals = Vec::new();
    for card in cards.iter().filter(|c| {
        [
            "sleep",
            "resting_heart_rate",
            "hrv_sdnn",
            "hrv_rmssd",
            crate::recovery_hrv::KIND,
            "workout",
            "active_energy",
        ]
        .contains(&c["metric_id"].as_str().unwrap_or(""))
    }) {
        let mut old = Vec::new();
        let mut new = Vec::new();
        for (at, value) in points(card, None) {
            let Some(date) = tz.timestamp_opt(at, 0).single().map(|d| d.date_naive()) else {
                continue;
            };
            if date >= recent && date < today {
                new.push(value);
            } else if date >= baseline && date < recent {
                old.push(value);
            }
        }
        let (before_count, after_count) = (old.len(), new.len());
        let before = median(&mut old);
        let after = median(&mut new);
        let comparable = before_count >= 14 && after_count >= 3;
        let delta = before
            .zip(after)
            .and_then(|(a, b)| (a != 0.0).then_some((b - a) / a * 100.0));
        let kind = card["metric_id"].as_str().unwrap_or("");
        let cue = if comparable {
            match kind {
                "resting_heart_rate" if delta.is_some_and(|v| v >= 10.0) => "elevated_resting_rate",
                "hrv_sdnn" | "hrv_rmssd" if delta.is_some_and(|v| v <= -10.0) => "lower_hrv",
                "sleep" if after.zip(sleep_goal).is_some_and(|(a, g)| a < g * 60.0) => {
                    "below_sleep_goal"
                }
                "workout" | "active_energy" if delta.is_some_and(|v| v >= 20.0) => {
                    "higher_observed_activity"
                }
                _ => "no_review_cue",
            }
        } else {
            "insufficient_comparable_days"
        };
        signals.push(json!({"id":card["id"],"title":card["title"],"metric_id":kind,"source":card["source"],"source_name":card["source_name"],"unit":card["unit"],"baseline":before,"recent":after,"baseline_days":before_count,"recent_days":after_count,"change_percent":delta,"cue":cue,"series":card["series"]}));
    }
    let cues: BTreeSet<_> = signals.iter().filter_map(|s| s["cue"].as_str()).collect();
    let kinds: BTreeSet<_> = signals
        .iter()
        .filter(|s| s["cue"] != "insufficient_comparable_days")
        .filter_map(|s| s["metric_id"].as_str())
        .collect();
    let strain = cues.contains("lower_hrv") && cues.contains("elevated_resting_rate");
    let status = if strain
        && (cues.contains("below_sleep_goal") || cues.contains("higher_observed_activity"))
    {
        "Multiple recovery signals need review"
    } else if cues.contains("lower_hrv")
        || cues.contains("elevated_resting_rate")
        || cues.contains("below_sleep_goal")
    {
        "Changes to review in context"
    } else if !kinds.contains("resting_heart_rate")
        || !kinds.contains("sleep")
        || (!kinds.contains("hrv_sdnn") && !kinds.contains("hrv_rmssd"))
    {
        "More comparable data needed"
    } else {
        "No combined review signal detected"
    };
    let mut missing = Vec::new();
    for (kind, label) in [
        ("resting_heart_rate", "Resting heart rate"),
        ("sleep", "Sleep"),
        ("workout", "Workout duration"),
    ] {
        if !kinds.contains(kind) {
            missing.push(format!(
                "{label}: at least 14 observed baseline days and 3 recent days from the same source"
            ));
        }
    }
    if !kinds.contains("hrv_sdnn") && !kinds.contains("hrv_rmssd") {
        missing
            .push("Comparable HRV baseline from the same source and measurement protocol".into());
    }
    if sleep_goal.is_none() {
        missing.push("Sleep goal in Reference Settings".into());
    }
    missing.push("Daily fatigue, soreness, illness, stress and session effort are not included in this first assessment; they can explain or contradict wearable changes.".into());
    let fitness: Vec<_> = cards
        .iter()
        .filter(|c| c["metric_id"] == "vo2_max")
        .cloned()
        .collect();
    if fitness.is_empty() {
        missing.push("VO₂ Max: no compatible cardio-fitness measurements are available".into());
    }
    json!({"fitness":fitness,"title":"Training & Recovery","status":status,"signals":signals,"missing":missing,"computed_at":as_of,
        "actions":if strain {vec!["Review recent training and sleep alongside fatigue, soreness and illness before increasing load.","Repeat measurements under the same conditions; persistent or symptomatic changes warrant clinical review."]} else {vec!["Keep measurement timing and device consistent, and record session effort and how recovered you feel.","Use these observations with training goals and symptoms; absence of a signal is not clearance for intense exercise."]},
        "method":"Last 7 completed local days versus the preceding 28, compared separately per source and metric using daily medians. At least 3 recent and 14 baseline observed days are required. Missing days are not zero/rest days. Review cues of ±10% for HRV/resting heart rate and +20% for observed activity are transparent application heuristics, not validated disease, injury or readiness thresholds. SDNN, RMSSD and Recovery HRV are never pooled; Recovery HRV is shown descriptively because its acquisition protocol is not established. Device/protocol changes can confound comparisons. No acute:chronic workload injury probability or universal recovery score is calculated.",
        "references":[{"title":"Buchheit 2014 — interpreting HR, HRV and recovery together","url":HRV},{"title":"Kiviniemi 2007 — randomized HRV-guided endurance training study","url":"https://pubmed.ncbi.nlm.nih.gov/17849143/"},{"title":"Plews 2014 — measurement frequency and HRV monitoring","url":"https://pubmed.ncbi.nlm.nih.gov/24334285/"}]})
}
