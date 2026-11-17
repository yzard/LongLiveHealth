use chrono::TimeZone;
use longlivehealth::insights::{Context, Request, prevent_cvd, recovery, snapshot};
use longlivehealth::reports::ReportResultStatus;
use serde_json::json;

fn context() -> Context {
    Context {
        known_cvd: Some(false),
        diabetes: Some(false),
        current_smoking: Some(false),
        antihypertensive: Some(false),
        statin: Some(false),
        egfr_2021_confirmed: Some(true),
    }
}

#[test]
fn prevent_centered_covariates_match_published_sex_specific_intercepts() {
    // All transformed covariates are zero in the CDC base formula at this point.
    for (female, intercept) in [(true, -3.307728_f64), (false, -3.031168_f64)] {
        let expected = 100.0 / (1.0 + (-intercept).exp());
        let actual = prevent_cvd(
            55.0,
            female,
            4.8 / 0.02586,
            1.3 / 0.02586,
            130.0,
            90.0,
            &context(),
        )
        .unwrap();
        assert!((expected - actual).abs() < 1e-10);
    }
    let baseline = prevent_cvd(55., false, 200., 50., 130., 90., &context()).unwrap();
    let mut smoking = context();
    smoking.current_smoking = Some(true);
    assert!(prevent_cvd(55., false, 200., 50., 130., 90., &smoking).unwrap() > baseline);
}

#[test]
fn prevent_never_imputes_unknowns_clamps_inputs_or_applies_to_existing_cvd() {
    for (age, tc, hdl, sbp, gfr) in [
        (29., 200., 50., 130., 90.),
        (80., 200., 50., 130., 90.),
        (55., 400., 50., 130., 90.),
        (55., 200., 50., 250., 90.),
        (55., 200., 50., 130., 10.),
        (55., f64::NAN, 50., 130., 90.),
    ] {
        assert!(prevent_cvd(age, false, tc, hdl, sbp, gfr, &context()).is_none());
    }
    for index in 0..7 {
        let mut c = context();
        match index {
            0 => c.diabetes = None,
            1 => c.current_smoking = None,
            2 => c.antihypertensive = None,
            3 => c.statin = None,
            4 => c.known_cvd = None,
            5 => c.known_cvd = Some(true),
            _ => c.egfr_2021_confirmed = Some(false),
        }
        assert!(prevent_cvd(55., false, 200., 50., 130., 90., &c).is_none());
    }
}

#[test]
fn recovery_uses_disjoint_calendar_windows_without_pooling_devices_or_hrv_types() {
    let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
    let tz = chrono_tz::America::New_York;
    let as_of = tz
        .from_local_datetime(&today.and_hms_opt(12, 0, 0).unwrap())
        .single()
        .unwrap()
        .timestamp();
    let make = |kind: &str, source: &str, start: u64, end: u64, old: f64, new: f64| {
        let points: Vec<_> = (start..end)
            .map(|offset| {
                let day = today - chrono::Days::new(offset);
                let at = tz
                    .from_local_datetime(&day.and_hms_opt(12, 0, 0).unwrap())
                    .single()
                    .unwrap()
                    .timestamp();
                json!({"at":at,"value":if offset<=7 {new} else {old}})
            })
            .collect();
        json!({"id":format!("{kind}:{source}"),"metric_id":kind,"title":kind,"unit":"ms","source":source,"series":[{"points":points}]})
    };
    let cards = vec![
        make("hrv_sdnn", "old_watch", 8, 36, 50., 50.),
        make("hrv_sdnn", "new_watch", 1, 8, 100., 100.),
        make("hrv_rmssd", "ring", 1, 36, 50., 35.),
        make("resting_heart_rate", "ring", 1, 36, 60., 70.),
        make("sleep", "ring", 1, 36, 8. * 3600., 6. * 3600.),
    ];
    let result = recovery(&cards, Some(420.), as_of, today, tz);
    assert_eq!(result["status"], "Multiple recovery signals need review");
    let signals = result["signals"].as_array().unwrap();
    assert_eq!(signals[0]["recent_days"], 0);
    assert_eq!(signals[1]["baseline_days"], 0);
    assert_eq!(signals[2]["baseline_days"], 28);
    assert_eq!(signals[2]["recent_days"], 7);
    assert_eq!(signals[2]["cue"], "lower_hrv");
    assert_eq!(signals[4]["cue"], "below_sleep_goal");
    assert_eq!(signals[1]["cue"], "insufficient_comparable_days");
    let descriptive_only = vec![
        make("workout", "ring", 1, 36, 60., 60.),
        make("active_energy", "ring", 1, 36, 300., 300.),
        make(longlivehealth::recovery_hrv::KIND, "ring", 1, 36, 50., 50.),
    ];
    assert_eq!(
        recovery(&descriptive_only, None, as_of, today, tz)["status"],
        "More comparable data needed"
    );
}

#[tokio::test]
async fn empty_archive_reports_missing_inputs_without_inventing_risk_or_health_values() {
    let (_directory, state) = crate::fixture().await;
    let user = crate::add_user(&state, "insights-empty").await;
    let output = snapshot(
        &state,
        &user.user_id,
        Request {
            timezone: "America/New_York".into(),
        },
    )
    .await
    .unwrap();
    assert!(output["cardiovascular"]["risk_percent"].is_null());
    assert!(
        output["cardiovascular"]["missing"]
            .as_array()
            .unwrap()
            .len()
            >= 10
    );
    assert_eq!(output["training"]["status"], "More comparable data needed");
    assert!(
        snapshot(
            &state,
            &user.user_id,
            Request {
                timezone: "invalid".into()
            }
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn integrated_snapshot_uses_confirmed_labs_paired_pressure_and_report_ranges() {
    use longlivehealth::{
        health::ConnectionRequest,
        reports::{ReportResultChange, ReportSubmission},
    };
    let (_directory, state) = crate::fixture().await;
    let user = crate::add_user(&state, "insights-inputs").await;
    let now = longlivehealth::authentication::now().unwrap();
    let at = now - 2 * 86400;
    let collected = chrono::DateTime::from_timestamp(at, 0)
        .unwrap()
        .to_rfc3339();
    let report = crate::archive(&state, "insights-inputs").await;
    let mut observations = Vec::new();
    for (metric, name, value, unit, reference, status) in [
        (
            "lipoprotein_a",
            "Lipoprotein (a)",
            "<10",
            "nmol/L",
            "<75",
            ReportResultStatus::Submitted,
        ),
        (
            "total_cholesterol",
            "Total Cholesterol",
            "200",
            "mg/dL",
            "<200",
            ReportResultStatus::Submitted,
        ),
        (
            "hdl_cholesterol",
            "HDL Cholesterol",
            "50",
            "mg/dL",
            ">40",
            ReportResultStatus::Submitted,
        ),
        (
            "egfr_creatinine_unspecified",
            "eGFR",
            "90",
            "mL/min/1.73m2",
            ">60",
            ReportResultStatus::Submitted,
        ),
        (
            "apob",
            "Apolipoprotein B",
            "130",
            "mg/dL",
            "<90",
            ReportResultStatus::Submitted,
        ),
        (
            "total_cholesterol",
            "Total Cholesterol",
            "310",
            "mg/dL",
            "<200",
            ReportResultStatus::Pending,
        ),
    ] {
        let mut payload = crate::observation_payload();
        payload.metric_id = Some(metric.into());
        payload.raw_name = name.into();
        payload.raw_result = value.into();
        payload.raw_unit = Some(unit.into());
        payload.reference_range = Some(reference.into());
        payload.sampled_at = Some(if metric == "total_cholesterol" && value == "200" {
            collected[..10].to_owned()
        } else {
            collected.clone()
        });
        payload.report_flag = None;
        observations.push(ReportResultChange {
            observation_id: None,
            expected_revision: None,
            status,
            payload,
        });
    }
    state
        .database
        .submit_report_changes(
            &user.user_id,
            ReportSubmission {
                report_id: report,
                expected_revision: 1,
                context: Some(
                    serde_json::from_value(json!({"source_name":"Example Laboratory"})).unwrap(),
                ),
                observations,
            },
        )
        .await
        .unwrap();
    state.database.save_wellness_preferences(&user.user_id,serde_json::from_value(json!({"expected_version":0,"batch_id":uuid::Uuid::new_v4().to_string(),"preferences":{"reference_profile":{"birth_date":"1971-01-01","reference_sex":"male"},"cardiovascular":context(),"source_aliases":{},"source_priority":{},"sleep_target_minutes":420}})).unwrap()).await.unwrap();
    let connection = state
        .database
        .create_connection(
            &user.user_id,
            ConnectionRequest {
                platform: "apple_health".into(),
                installation_id: uuid::Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap();
    for (kind, id, payload) in [
        (
            "blood_pressure_systolic",
            "sys",
            json!({"value":130,"unit":"mmHg"}),
        ),
        (
            "blood_pressure_diastolic",
            "dia",
            json!({"value":80,"unit":"mmHg"}),
        ),
        (
            "HKCorrelationTypeIdentifierBloodPressure",
            "pair",
            json!({"related_sample_ids":["sys","dia"]}),
        ),
    ] {
        let mut record = crate::health_batch::record(id, payload);
        record.record_type = kind.into();
        record.start_at = at;
        record.end_at = at + 1;
        let mut batch = crate::health_batch::batch(&connection.connection_id, vec![record]);
        batch.record_type = kind.into();
        state
            .database
            .sync_health(&user.user_id, batch)
            .await
            .unwrap();
    }
    let output = snapshot(
        &state,
        &user.user_id,
        Request {
            timezone: "UTC".into(),
        },
    )
    .await
    .unwrap();
    let cardiac = &output["cardiovascular"];
    assert!(
        cardiac["risk_percent"].as_f64().is_some(),
        "{}",
        cardiac["missing"]
    );
    let censored = cardiac["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["metric_id"] == "lipoprotein_a")
        .unwrap();
    assert_eq!(censored["state"], "reported");
    assert_eq!(censored["printed_value"], "<10");
    assert!(censored["value"].is_null());
    assert!(censored["reason"].as_str().unwrap().contains("nmol/L"));
    assert_eq!(cardiac["inputs"][0]["value"], 200.);
    assert_eq!(
        cardiac["inputs"][0]["at"],
        longlivehealth::reports::parse_time(&collected[..10]).unwrap()
    );
    assert_eq!(cardiac["inputs"][0]["sampled_at"], collected[..10]);
    assert!(
        cardiac["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["title"] == "Blood Pressure · Systolic")
    );
    assert!(
        cardiac["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["metric_id"] == "blood_pressure_diastolic" && i["value"] == 80.)
    );
    assert!(
        cardiac["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["title"] == "Apolipoprotein B"
                && f["assessment"]["kind"] == "report_reference")
    );
    let other = crate::add_user(&state, "insights-other").await;
    let empty = snapshot(
        &state,
        &other.user_id,
        Request {
            timezone: "UTC".into(),
        },
    )
    .await
    .unwrap();
    assert!(empty["cardiovascular"]["risk_percent"].is_null());
}

#[test]
fn dashboard_keeps_six_domains_real_values_and_unknown_scores() {
    use longlivehealth::insights::dashboard;
    let as_of = 1_800_000_000;
    let samples: Vec<_> = (0..16)
        .map(|day| json!({"at":as_of-day*86400,"value":60.+day as f64}))
        .collect();
    let cards = vec![
        json!({"id":"rhr:a","metric_id":"resting_heart_rate","source":"a","unit":"count/min","series":[{"points":samples}]}),
        json!({"id":"rhr:b","metric_id":"resting_heart_rate","source":"b","unit":"count/min","series":[{"points":[{"at":as_of-86400,"value":90.}]}]}),
    ];
    let training = json!({"signals":[],"references":[],"status":"More comparable data needed","method":"Evidence only"});
    let sleep = json!({"sources":[{"source":"watch","sessions":[
        {"state":"ready","wake_date":"2027-01-15","start_at":as_of-32000,"end_at":as_of-1000,"asleep_seconds":28800},
        {"state":"ready","wake_date":"2027-01-15","start_at":as_of-600,"end_at":as_of-1,"asleep_seconds":599},
        {"state":"conflicting_states","wake_date":"2027-01-16","end_at":as_of,"asleep_seconds":null}]}]});
    let result = dashboard::build(
        &cards,
        &json!({"cards":[]}),
        &training,
        &sleep,
        Some(420.),
        as_of,
    );
    assert_eq!(result.as_array().unwrap().len(), 6);
    assert_eq!(result[0]["session"]["asleep_seconds"], 28800);
    assert_eq!(result[0]["goal_minutes"], 420.);
    assert!(result[0]["score"].is_null());
    assert!(result[1]["score"].is_null());
    let vitals = &result[2]["evidence"];
    assert_eq!(vitals[0]["source"], "a");
    assert_eq!(vitals[0]["baseline_low"], 61.);
    assert_eq!(vitals[0]["baseline_high"], 75.);
    assert!(vitals[1]["baseline_low"].is_null());
    assert_eq!(vitals[1]["latest"], 90.);
    let empty = dashboard::build(
        &[],
        &json!({"cards":[]}),
        &training,
        &json!({}),
        None,
        as_of,
    );
    assert!(empty[0]["session"].is_null());
    assert!(empty[3]["evidence"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn retired_dashboard_routes_are_absent_and_preferences_keep_personal_settings() {
    use axum::http::StatusCode;
    let (directory, state) = crate::fixture().await;
    let user = crate::add_user(&state, "insights-home").await;
    let token = crate::token(&state, "insights-home").await;
    let app = longlivehealth::app::create_application(state.clone());
    for route in [
        "wellness/day",
        "wellness/trends",
        "wellness/report",
        "trends/get",
    ] {
        let response =
            crate::request(&app, &format!("/api/v1/{route}"), Some(&token), json!({})).await;
        assert_eq!(response.0, StatusCode::NOT_FOUND, "{route}");
    }
    let mut preferences = state
        .database
        .wellness_preferences(&user.user_id)
        .await
        .unwrap()["preferences"]
        .clone();
    assert!(preferences.get("favorite_metrics").is_none());
    assert!(preferences.get("overview_show_all").is_none());
    preferences["sleep_target_minutes"] = json!(420);
    let saved = state.database.save_wellness_preferences(&user.user_id, serde_json::from_value(json!({"expected_version":0,"batch_id":uuid::Uuid::new_v4(),"preferences":preferences})).unwrap()).await.unwrap();
    assert_eq!(saved["preferences"]["sleep_target_minutes"], 420);
    // Simulate a pre-upgrade on-disk record, without accepting retired fields in new API writes.
    let pool = sqlx::SqlitePool::connect(&format!(
        "sqlite:{}",
        directory.path().join("database.sqlite").display()
    ))
    .await
    .unwrap();
    let raw: String = sqlx::query_scalar("SELECT payload_json FROM health_records WHERE user_id=? AND source_id='longlivehealth.preferences'").bind(&user.user_id).fetch_one(&pool).await.unwrap();
    let mut raw: serde_json::Value = serde_json::from_str(&raw).unwrap();
    raw["payload"]["favorite_metrics"] = json!(["sleep"]);
    raw["payload"]["overview_show_all"] = json!(false);
    sqlx::query("UPDATE health_records SET payload_json=? WHERE user_id=? AND source_id='longlivehealth.preferences'").bind(raw.to_string()).bind(&user.user_id).execute(&pool).await.unwrap();
    let reloaded = state
        .database
        .wellness_preferences(&user.user_id)
        .await
        .unwrap();
    assert_eq!(reloaded["preferences"]["sleep_target_minutes"], 420);
    assert!(reloaded["preferences"].get("favorite_metrics").is_none());
    assert!(
        serde_json::from_value::<longlivehealth::wellness::preferences::Preferences>(
            raw["payload"].clone()
        )
        .is_err()
    );
}
