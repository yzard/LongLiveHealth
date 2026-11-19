use crate::{add_user, archive, fixture, observation_payload, seed_legacy_submitted_observation};
use base64::{Engine, engine::general_purpose::STANDARD};
use longlivehealth::reports::ReportResultStatus;
use longlivehealth::{
    health::{ConnectionRequest, HealthRecordInput, SyncRequest},
    metric_history::{HistoryRequest, history},
    reports::{NewMetric, ReportResultChange, ReportSubmission, UnmappedChoice},
};
use serde_json::{Value, json};

async fn read(state: &longlivehealth::app::AppState, user: &str) -> Value {
    history(
        &state.database,
        user,
        HistoryRequest {
            metric_id: "ldl_cholesterol".into(),
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn browse_uses_confirmed_loinc_link_for_compatible_older_submissions() {
    let (directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let name = "eGFR (Afr. Amer.)";
    let unit = "mL/min/1.73m2";
    let mut archived_reports = Vec::new();
    for (year, reported_unit, choice) in [
        (2018, unit, None),
        (2017, "mg/dL", None),
        (2016, unit, Some(UnmappedChoice::Unmapped)),
        (
            2019,
            unit,
            Some(UnmappedChoice::New(NewMetric {
                name: name.into(),
                standard_unit: unit.into(),
                loinc_code: Some("48643-1".into()),
            })),
        ),
    ] {
        let report_id = archive(&state, "alice").await;
        let mut payload = observation_payload();
        payload.raw_name = format!("{name} (48643-1)");
        payload.raw_result = "72".into();
        payload.raw_unit = Some(reported_unit.into());
        payload.sampled_at = Some(format!("{year}-10-20"));
        payload.metric_id = None;
        payload.unmapped_choice = choice;
        payload.reference_range = None;
        payload.report_flag = None;
        if year < 2019 {
            seed_legacy_submitted_observation(
                &directory,
                &user.user_id,
                &report_id,
                None,
                &payload,
            )
            .await;
            archived_reports.push(report_id);
            continue;
        }
        state
            .database
            .submit_report_changes(
                &user.user_id,
                ReportSubmission {
                    report_id: report_id.clone(),
                    expected_revision: 1,
                    context: None,
                    observations: vec![ReportResultChange {
                        observation_id: None,
                        expected_revision: None,
                        status: ReportResultStatus::Submitted,
                        payload,
                    }],
                },
            )
            .await
            .unwrap();
        archived_reports.push(report_id);
    }
    let catalog = longlivehealth::browse_catalog::get(&state.database, &user.user_id)
        .await
        .unwrap();
    let entries = catalog["entries"].as_array().unwrap();
    // The incompatible and intentionally unmapped originals remain separate.
    assert_eq!(entries.len(), 3);
    let metric_id = entries
        .iter()
        .find_map(|entry| {
            entry["kind"]
                .as_str()
                .filter(|id| id.starts_with("custom:"))
        })
        .unwrap();
    let result = history(
        &state.database,
        &user.user_id,
        HistoryRequest {
            metric_id: metric_id.into(),
        },
    )
    .await
    .unwrap();
    let records = result["records"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    let earlier = records
        .iter()
        .find(|row| row["report_id"] == archived_reports[0])
        .unwrap();
    assert!(earlier["original"]["metric_id"].is_null());
    assert_eq!(result["points"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn renamed_report_metrics_are_suggested_by_loinc_or_original_identity_and_keep_history() {
    let (directory, mut state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let token = crate::token(&state, "alice").await;
    let app = longlivehealth::app::create_application(state.clone());
    let first = archive(&state, "alice").await;
    let second = archive(&state, "alice").await;
    let row = |name: &str, year: &str, unit: Option<&str>, status| {
        let mut payload = observation_payload();
        payload.raw_name = name.into();
        payload.raw_result = "6.5".into();
        payload.raw_unit = unit.map(str::to_owned);
        payload.sampled_at = Some(format!("{year}-10-20"));
        payload.metric_id = None;
        ReportResultChange {
            observation_id: None,
            expected_revision: None,
            status,
            payload,
        }
    };
    let request = |report_id: &str, observations| ReportSubmission {
        report_id: report_id.into(),
        expected_revision: 1,
        context: Some(serde_json::from_value(json!({"source_name":"Fixture Laboratory"})).unwrap()),
        observations,
    };
    for (name, unit) in [
        ("pH (50560-2)", None),
        ("Synthetic particle ratio", Some("U/L")),
    ] {
        let payload = row(name, "2018", unit, ReportResultStatus::Submitted).payload;
        seed_legacy_submitted_observation(
            &directory,
            &alice.user_id,
            &first,
            Some("Fixture Laboratory"),
            &payload,
        )
        .await;
    }
    let catalog = longlivehealth::metric_history::browse(&state.database, &alice.user_id)
        .await
        .unwrap();
    let metrics = catalog["metrics"].as_array().unwrap();
    let rename = |printed, label| {
        let id = metrics
            .iter()
            .find(|entry| entry["name"] == printed)
            .unwrap()["metric_id"]
            .clone();
        (id, label)
    };
    let aliases = [
        rename("pH", "pH of Urine"),
        rename("Synthetic particle ratio", "My particle ratio"),
    ];
    for (id, name) in &aliases {
        assert!(id.as_str().unwrap().starts_with("reported:"));
        let (status, _) = crate::request(
            &app,
            "/api/v1/metrics/rename",
            Some(&token),
            json!({"metric_id":id,"name":name}),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
    }
    let db = sqlx::SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(directory.path().join("database.sqlite")),
    )
    .await
    .unwrap();
    sqlx::query("DELETE FROM named_report_metrics")
        .execute(&db)
        .await
        .unwrap();
    db.close().await;
    let db = longlivehealth::database::Database::open(directory.path())
        .await
        .unwrap();
    let second_report = db
        .submit_report_changes(
            &alice.user_id,
            request(
                &second,
                vec![
                    row(
                        "Urine pH (50560-2)",
                        "2019",
                        None,
                        ReportResultStatus::Pending,
                    ),
                    row(
                        "Synthetic particle ratio",
                        "2019",
                        Some("U/L"),
                        ReportResultStatus::Pending,
                    ),
                    row(
                        "pH (50560-2)",
                        "2019",
                        Some("mg/dL"),
                        ReportResultStatus::Pending,
                    ),
                ],
            ),
        )
        .await
        .unwrap();
    let rows = second_report["observations"].as_array().unwrap();
    let found = |printed: &str| {
        rows.iter()
            .find(|row| row["payload"]["raw_name"] == printed)
            .unwrap()
    };
    let mapping = |result: &Value, alias: &(Value, &str)| {
        assert_eq!(result["payload"]["ocr_metric_candidate"], alias.0);
        assert_eq!(result["ocr_suggestion_name"], alias.1);
        assert_eq!(result["metric_suggestions"][0]["metric_id"], alias.0);
        assert!(result["payload"]["metric_id"].is_null());
    };
    let urine = found("Urine pH (50560-2)");
    mapping(urine, &aliases[0]);
    mapping(found("Synthetic particle ratio"), &aliases[1]);
    let mismatch = found("pH (50560-2)");
    mapping(mismatch, &aliases[0]);
    assert!(
        mismatch["browse_issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["code"] == "unsupported_unit")
    );
    let submitted = db
        .submit_report_changes(
            &alice.user_id,
            ReportSubmission {
                report_id: second,
                expected_revision: second_report["report"]["revision"].as_i64().unwrap(),
                context: None,
                observations: vec![ReportResultChange {
                    observation_id: urine["observation_id"].as_str().map(str::to_owned),
                    expected_revision: urine["revision"].as_i64(),
                    status: ReportResultStatus::Submitted,
                    payload: serde_json::from_value({
                        let mut payload = urine["payload"].clone();
                        payload["metric_id"] = aliases[0].0.clone();
                        payload
                    })
                    .unwrap(),
                }],
            },
        )
        .await
        .unwrap();
    assert!(
        submitted["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["status"] == "submitted" && row["resolved_metric_id"] == aliases[0].0)
    );
    let series = history(
        &db,
        &alice.user_id,
        HistoryRequest {
            metric_id: aliases[0].0.as_str().unwrap().into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(series["metric"]["name"], aliases[0].1);
    assert_eq!(series["points"].as_array().unwrap().len(), 2);
    assert!(
        series["points"]
            .as_array()
            .unwrap()
            .iter()
            .all(|point| point["unit"] == "1")
    );
    let fresh_report = archive(&state, "alice").await;
    while let Some(job) = db.claim_job(2_000_000_000, 600, 3).await.unwrap() {
        db.finish_job(&job, 2_000_000_001, None).await.unwrap();
    }
    let ids = aliases.each_ref().map(|(id, _)| id.clone());
    let (sent, received) = tokio::sync::oneshot::channel();
    let sent = std::sync::Arc::new(std::sync::Mutex::new(Some(sent)));
    let provider = axum::Router::new().route(
        "/v1/chat/completions",
        axum::routing::post(move |axum::Json(request): axum::Json<Value>| {
            let sent = sent.clone();
            async move {
                let evidence: Value = serde_json::from_str(
                    request["messages"][1]["content"][0]["text"]
                        .as_str()
                        .unwrap(),
                )
                .unwrap();
                sent.lock().unwrap().take().unwrap().send(evidence).unwrap();
                axum::Json(super::provider::completion(
                    json!({"observations":[],"warnings":[]}),
                ))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    std::sync::Arc::make_mut(&mut state.config).llm.base_url =
        format!("http://{}/v1", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, provider).await.unwrap() });
    db.rescan_report(&alice.user_id, &fresh_report, 1)
        .await
        .unwrap();
    assert!(longlivehealth::worker::extract_next(&state).await.unwrap());
    let evidence = received.await.unwrap();
    let entries = evidence["laboratory_metric_catalog"].as_array().unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry == &json!([ids[0], "pH of Urine", "1", ["pH"], ["50560-2"]]))
    );
    assert!(entries.iter().any(|entry| entry
        == &json!([
            ids[1],
            "My particle ratio",
            "U/L",
            ["Synthetic particle ratio"],
            []
        ])));
    server.abort();
}

#[tokio::test]
async fn conflicting_history_units_disable_the_chart_and_list_all_reported_units() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let mut histories = Vec::new();
    for (index, (result, unit)) in [
        ("See laboratory note", Some("mg/dL")),
        ("Negative", None),
        ("120", Some("mg/dL")),
        ("3", Some("mmol/L")),
        ("10", Some("U/L")),
        ("12", Some("ng/dL")),
    ]
    .into_iter()
    .enumerate()
    {
        let mut payload = observation_payload();
        payload.raw_result = result.into();
        payload.raw_unit = unit.map(str::to_owned);
        payload.sampled_at = Some(format!("2026-08-{:02}", index + 1));
        state
            .database
            .submit_report_changes(
                &alice.user_id,
                ReportSubmission {
                    report_id: archive(&state, "alice").await,
                    expected_revision: 1,
                    context: None,
                    observations: vec![ReportResultChange {
                        observation_id: None,
                        expected_revision: None,
                        status: ReportResultStatus::Submitted,
                        payload,
                    }],
                },
            )
            .await
            .unwrap();
        histories.push(read(&state, &alice.user_id).await);
    }
    assert_eq!(histories[0]["points"], json!([]));
    assert_eq!(histories[1]["chart_unit_conflict"], json!([]));
    assert_eq!(histories[3]["chart_unit_conflict"], json!([]));
    assert_eq!(histories[3]["points"].as_array().unwrap().len(), 2);
    assert!(
        !histories[3]["chart_ranges"]["D"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mixed = &histories[5];
    assert_eq!(
        mixed["chart_unit_conflict"],
        json!(["U/L", "mg/dL", "mmol/L", "ng/dL"])
    );
    assert_eq!(mixed["chart_ranges"], json!({}));
    assert_eq!(mixed["points"].as_array().unwrap().len(), 2);
    assert_eq!(mixed["records"].as_array().unwrap().len(), 6);
}

#[tokio::test]
async fn metric_history_preserves_three_sources_normalizes_and_isolates_users() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let bob = add_user(&state, "bob").await;
    for name in ["Function Health", "Local Clinic"] {
        let report = archive(&state, "alice").await;
        let response = state
            .database
            .submit_report_changes(
                &alice.user_id,
                ReportSubmission {
                    report_id: report,
                    expected_revision: 1,
                    context: Some(serde_json::from_value(json!({"source_name":name})).unwrap()),
                    observations: vec![
                        ReportResultChange {
                            observation_id: None,
                            expected_revision: None,
                            status: ReportResultStatus::Submitted,
                            payload: observation_payload(),
                        },
                        ReportResultChange {
                            observation_id: None,
                            expected_revision: None,
                            status: ReportResultStatus::Pending,
                            payload: observation_payload(),
                        },
                    ],
                },
            )
            .await
            .unwrap();
        assert_eq!(response["context"]["source_name"], name);
    }
    let connection = state
        .database
        .create_connection(
            &alice.user_id,
            ConnectionRequest {
                platform: "apple_health".into(),
                installation_id: uuid::Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap();
    let fhir = json!({"resourceType":"Observation","status":"final", "effectiveDateTime":"2026-08-02T12:00:00Z",
        "code":{"coding":[{"system":"http://loinc.org","code":"2089-1"}]},
        "valueQuantity":{"value":3.0,"code":"mmol/L","system":"http://unitsofmeasure.org"}});
    let record = |id: &str, resource: &Value| HealthRecordInput {
        record_id: id.into(),
        source_id: "clinical-provider".into(),
        record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
        start_at: 1785672000,
        end_at: 1785672000,
        version: 1,
        deleted: false,
        payload: json!({"source_name":"Clinic through Health", "fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(resource).unwrap())}}),
    };
    let mut preliminary = fhir.clone();
    preliminary["status"] = json!("preliminary");
    let mut missing_date = fhir.clone();
    missing_date
        .as_object_mut()
        .unwrap()
        .remove("effectiveDateTime");
    let mut bounded = fhir.clone();
    bounded["valueQuantity"]["comparator"] = json!("<");
    let request = SyncRequest {
        connection_id: connection.connection_id.clone(),
        batch_id: uuid::Uuid::new_v4().to_string(),
        record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
        coverage_status: "observed".into(),
        records: vec![
            record("final", &fhir),
            record("preliminary", &preliminary),
            record("missing-date", &missing_date),
            record("bounded", &bounded),
        ],
    };
    let replay = serde_json::to_vec(&request).unwrap();
    state
        .database
        .sync_health(&alice.user_id, request)
        .await
        .unwrap();
    state
        .database
        .sync_health(&alice.user_id, serde_json::from_slice(&replay).unwrap())
        .await
        .unwrap();
    let result = read(&state, &alice.user_id).await;
    let points = result["points"].as_array().unwrap();
    assert_eq!(points.len(), 3);
    assert_eq!(result["excluded_count"], 3);
    let names: std::collections::BTreeSet<_> = points
        .iter()
        .map(|p| p["source_name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["Clinic through Health", "Function Health", "Local Clinic"]
            .into_iter()
            .collect()
    );
    let health = points
        .iter()
        .find(|p| p["channel"] == "Apple Health")
        .unwrap();
    let value: f64 = health["value"].as_str().unwrap().parse().unwrap();
    assert!((value - 116.01).abs() < 0.001);
    assert_eq!(
        health["original"]["clinical_observation"]["valueQuantity"]["value"],
        3.0
    );
    assert!(
        read(&state, &bob.user_id).await["points"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mut deleted = record("final", &fhir);
    deleted.deleted = true;
    deleted.version = 2;
    state
        .database
        .sync_health(
            &alice.user_id,
            SyncRequest {
                connection_id: connection.connection_id,
                batch_id: uuid::Uuid::new_v4().to_string(),
                record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
                coverage_status: "observed".into(),
                records: vec![deleted],
            },
        )
        .await
        .unwrap();
    assert_eq!(
        read(&state, &alice.user_id).await["points"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn report_source_rejects_empty_or_reserved_names_without_changing_revision() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let report = archive(&state, "alice").await;
    for name in [" ", "Apple Health", "clinic\nother"] {
        let result = state
            .database
            .submit_report_changes(
                &alice.user_id,
                ReportSubmission {
                    report_id: report.clone(),
                    expected_revision: 1,
                    observations: vec![],
                    context: Some(serde_json::from_value(json!({"source_name":name})).unwrap()),
                },
            )
            .await;
        assert!(matches!(
            result,
            Err(longlivehealth::error::AppError::Invalid(_))
        ));
    }
    assert_eq!(
        state
            .database
            .report_summary(&alice.user_id, &report)
            .await
            .unwrap()
            .revision,
        1
    );
}

#[tokio::test]
async fn quantity_glucose_and_clinical_component_share_report_units_and_dates() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let report = archive(&state, "alice").await;
    let mut payload = observation_payload();
    payload.metric_id = Some("glucose".into());
    payload.raw_name = "Glucose".into();
    payload.raw_result = "5.5".into();
    payload.raw_unit = Some("mmol/L".into());
    state
        .database
        .submit_report_changes(
            &alice.user_id,
            ReportSubmission {
                report_id: report.clone(),
                expected_revision: 1,
                context: Some(serde_json::from_value(json!({"source_name":"Clinic"})).unwrap()),
                observations: vec![ReportResultChange {
                    observation_id: None,
                    expected_revision: None,
                    status: ReportResultStatus::Submitted,
                    payload,
                }],
            },
        )
        .await
        .unwrap();
    let connection = state
        .database
        .create_connection(
            &alice.user_id,
            ConnectionRequest {
                platform: "apple_health".into(),
                installation_id: uuid::Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap();
    let panel = json!({"resourceType":"Observation","status":"final","effectiveDateTime":"2026-08-03T00:00:00Z",
        "component":[{"code":{"coding":[{"system":"http://loinc.org","code":"2345-7"}]},
            "valueQuantity":{"value":5.5,"code":"mmol/L","system":"http://unitsofmeasure.org"}}]});
    for (kind, payload) in [
        (
            "blood_glucose",
            json!({"value":99.1001,"unit":"mg/dL","source_name":"Meter"}),
        ),
        (
            "HKClinicalTypeIdentifierLabResultRecord",
            json!({"fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&panel).unwrap())}}),
        ),
    ] {
        state
            .database
            .sync_health(
                &alice.user_id,
                SyncRequest {
                    connection_id: connection.connection_id.clone(),
                    batch_id: uuid::Uuid::new_v4().to_string(),
                    record_type: kind.into(),
                    coverage_status: "observed".into(),
                    records: vec![HealthRecordInput {
                        record_id: kind.into(),
                        source_id: "health".into(),
                        record_type: kind.into(),
                        start_at: 1785672000,
                        end_at: 1785672000,
                        version: 1,
                        deleted: false,
                        payload,
                    }],
                },
            )
            .await
            .unwrap();
    }
    state
        .database
        .submit_report_changes(
            &alice.user_id,
            ReportSubmission {
                report_id: report,
                expected_revision: 2,
                observations: vec![],
                context: Some(
                    serde_json::from_value(json!({"source_name":"Renamed Clinic"})).unwrap(),
                ),
            },
        )
        .await
        .unwrap();
    let result = history(
        &state.database,
        &alice.user_id,
        HistoryRequest {
            metric_id: "glucose".into(),
        },
    )
    .await
    .unwrap();
    let points = result["points"].as_array().unwrap();
    assert_eq!(points.len(), 3);
    assert_eq!(result["excluded_count"], 0);
    assert!(points.iter().all(|p| p["unit"] == "mg/dL"));
    assert_eq!(
        points
            .iter()
            .filter(|p| p["source_name"] == "Apple Health")
            .count(),
        2
    );
    assert_eq!(
        points
            .iter()
            .filter(|p| p["source_name"] == "Renamed Clinic")
            .count(),
        1
    );
    let component = points
        .iter()
        .find(|p| p["point_id"].as_str().unwrap().ends_with(":component:0"))
        .unwrap();
    assert_eq!(component["sampled_at"], "2026-08-03T00:00:00Z");
    assert_eq!(
        component["original"]["clinical_observation"]["valueQuantity"]["value"],
        5.5
    );
}

#[tokio::test]
async fn known_clinical_metrics_plot_their_own_records_without_unrelated_exclusions() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "chart-matrix").await;
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
    let codes = [
        "2093-3", "2089-1", "2085-9", "2571-8", "1884-6", "4548-4", "2345-7", "718-7", "2160-0",
        "2276-4", "1751-7", "1988-5", "736-9", "787-2", "788-0", "6768-6", "6690-2",
    ];
    let coded_ids = [
        "total_cholesterol",
        "ldl_cholesterol",
        "hdl_cholesterol",
        "triglycerides",
        "apob",
        "hba1c",
        "glucose",
        "hemoglobin",
        "creatinine",
        "ferritin",
        "albumin",
        "crp",
        "lymphocyte_percent",
        "mcv",
        "rdw",
        "alp",
        "wbc",
    ];
    let metrics: Vec<_> = longlivehealth::laboratory::metrics()
        .into_iter()
        .filter(|metric| coded_ids.contains(&metric.metric_id))
        .collect();
    assert_eq!(codes.len(), metrics.len());
    let mut records = Vec::new();
    for (index, metric) in metrics.iter().enumerate() {
        let resource = json!({"resourceType":"Observation","status":"final","effectiveDateTime":"2026-08-03T00:00:00Z",
            "code":{"coding":[{"system":"http://loinc.org","code":codes[index]}]},
            "valueQuantity":{"value":5,"code":if metric.metric_id == "wbc" { "K/uL" } else { metric.standard_unit },"system":"http://unitsofmeasure.org"}});
        records.push(HealthRecordInput { record_id: metric.metric_id.into(), source_id:"synthetic".into(),
            record_type:"HKClinicalTypeIdentifierLabResultRecord".into(), start_at:1785715200, end_at:1785715200,
            version:1, deleted:false, payload:json!({"fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&resource).unwrap())}}) });
    }
    let mut unknown = records[0].clone();
    unknown.record_id = "unknown".into();
    unknown.payload = json!({"fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&json!({"resourceType":"Observation", "code":{"coding":[{"system":"http://loinc.org","code":"unmapped-test-code"}]},"valueQuantity":{"value":10,"unit":"mg/dL"}})).unwrap())}});
    records.push(unknown);
    let mut unreadable = records[0].clone();
    unreadable.record_id = "unreadable".into();
    unreadable.payload = json!({"fhir":{"data_base64":"invalid"}});
    records.push(unreadable);
    state
        .database
        .sync_health(
            &user.user_id,
            SyncRequest {
                connection_id: connection.connection_id,
                batch_id: uuid::Uuid::new_v4().to_string(),
                record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
                coverage_status: "observed".into(),
                records,
            },
        )
        .await
        .unwrap();
    for metric in metrics {
        let result = history(
            &state.database,
            &user.user_id,
            HistoryRequest {
                metric_id: metric.metric_id.into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(
            result["points"].as_array().unwrap().len(),
            1,
            "{}",
            metric.metric_id
        );
        assert_eq!(result["points"][0]["unit"], metric.standard_unit);
        assert_eq!(result["points"][0]["value"], "5");
        assert_eq!(result["excluded_count"], 0, "{}", metric.metric_id);
        assert_eq!(result["unassigned_clinical_count"], 2);
    }
}

#[tokio::test]
async fn browse_discovers_clinical_items_groups_codes_and_retains_text_and_bounds() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "clinical-browse").await;
    let bob = add_user(&state, "other-browse").await;
    let connection = state
        .database
        .create_connection(
            &alice.user_id,
            ConnectionRequest {
                platform: "apple_health".into(),
                installation_id: uuid::Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap();
    let base = json!({"resourceType":"Observation","status":"final","effectiveDateTime":"2026-08-03T00:00:00Z",
        "code":{"text":"Calcium","coding":[{"system":"http://loinc.org","code":"17861-6","display":"Calcium"}]},
        "valueQuantity":{"value":9.5,"unit":"mg/dL","system":"http://unitsofmeasure.org"}});
    let mut resources = vec![
        ("one", "Hospital", base.clone()),
        ("two", "Other Hospital", base.clone()),
    ];
    let mut molar = base.clone();
    molar["valueQuantity"]["unit"] = json!("mmol/L");
    resources.push(("molar", "Hospital", molar));
    let mut text = base.clone();
    text.as_object_mut().unwrap().remove("valueQuantity");
    text["valueString"] = json!("See note");
    resources.push(("text", "Hospital", text));
    let mut bound = base.clone();
    bound["valueQuantity"]["comparator"] = json!("<");
    resources.push(("bound", "Hospital", bound));
    let mut preliminary = base.clone();
    preliminary["status"] = json!("preliminary");
    resources.push(("preliminary", "Hospital", preliminary));
    let mut local = base.clone();
    local["code"]["coding"] = json!([{"system":"https://example.org/local","code":"1"}]);
    resources.push(("local-one", "Hospital", local.clone()));
    resources.push(("local-two", "Other Hospital", local));
    let records = resources.into_iter().map(|(id,source,r)|HealthRecordInput { record_id:id.into(),source_id:source.into(),record_type:"HKClinicalTypeIdentifierLabResultRecord".into(),start_at:1785715200,end_at:1785715200,version:1,deleted:false,payload:json!({"source_name":source,"fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&r).unwrap())}})}).collect();
    state
        .database
        .sync_health(
            &alice.user_id,
            SyncRequest {
                connection_id: connection.connection_id,
                batch_id: uuid::Uuid::new_v4().to_string(),
                record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
                coverage_status: "observed".into(),
                records,
            },
        )
        .await
        .unwrap();
    let catalog = longlivehealth::metric_history::browse(&state.database, &alice.user_id)
        .await
        .unwrap();
    let entries = catalog["metrics"].as_array().unwrap();
    assert_eq!(entries.len(), 3); // LOINC across hospitals, local codes scoped to each source.
    let calcium = entries.iter().find(|m| m["record_count"] == 5).unwrap();
    let id = calcium["metric_id"].as_str().unwrap();
    let result = history(
        &state.database,
        &alice.user_id,
        HistoryRequest {
            metric_id: id.into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(result["records"].as_array().unwrap().len(), 5);
    assert_eq!(result["points"].as_array().unwrap().len(), 3);
    assert_eq!(result["chart_unit_conflict"], json!(["mg/dL", "mmol/L"]));
    assert_eq!(result["chart_ranges"], json!({}));
    assert!(
        result["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["display_value"] == "See note")
    );
    assert!(
        result["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["display_value"] == "<9.5")
    );
    assert!(
        result["points"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["unit"] == "mmol/L")
    );
    assert!(
        longlivehealth::metric_history::browse(&state.database, &bob.user_id)
            .await
            .unwrap()["metrics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let other = history(
        &state.database,
        &bob.user_id,
        HistoryRequest {
            metric_id: id.into(),
        },
    )
    .await
    .unwrap();
    assert!(other["records"].as_array().unwrap().is_empty());
    assert!(other["points"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn equivalent_clinical_units_coalesce_display_but_keep_originals_and_distinct_results() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "unit-grouping").await;
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
    let cases = [
        (
            "basophils",
            "704-7",
            "Absolute Basophil Count",
            vec!["x10(3)/uL", "x10^9/L", "10*9/L"],
            "10^9/L",
        ),
        (
            "thyroglobulin",
            "3013-0",
            "Thyroglobulin",
            vec!["NG/mL", "ng/mL", "NG/ML"],
            "ng/mL",
        ),
        (
            "wbc",
            "6690-2",
            "White blood cell count",
            vec!["K/uL", "x10(3)/uL", "10^9/L"],
            "10^9/L",
        ),
    ];
    let mut records = Vec::new();
    for (prefix, code, name, units, _) in &cases {
        for (index, unit) in units.iter().enumerate() {
            let resource = json!({"resourceType":"Observation","status":"final","effectiveDateTime":"2026-08-03T00:00:00Z",
                "code":{"text":name,"coding":[{"system":"http://loinc.org","code":code}]},
                "valueQuantity":{"value":0.04,"unit":unit,"system":"http://unitsofmeasure.org"}});
            records.push(HealthRecordInput { record_id:format!("{prefix}-{index}"),source_id:"Hospital".into(),record_type:"HKClinicalTypeIdentifierLabResultRecord".into(),start_at:1785715200,end_at:1785715200,version:1,deleted:false,payload:json!({"source_name":"Hospital","fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&resource).unwrap())}})});
            if index == 0 {
                for variant in ["other-source", "other-value", "other-method"] {
                    let mut resource = resource.clone();
                    if variant == "other-value" {
                        resource["valueQuantity"]["value"] = json!(0.05);
                    }
                    if variant == "other-method" {
                        resource["method"] = json!({"text":"Different assay"});
                    }
                    records.push(HealthRecordInput { record_id:format!("{prefix}-{variant}"),source_id:if variant == "other-source" {"Other Hospital"} else {"Hospital"}.into(),record_type:"HKClinicalTypeIdentifierLabResultRecord".into(),start_at:1785715200,end_at:1785715200,version:1,deleted:false,payload:json!({"source_name":"Hospital","fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&resource).unwrap())}})});
                }
            }
        }
    }
    state
        .database
        .sync_health(
            &user.user_id,
            SyncRequest {
                connection_id: connection.connection_id,
                batch_id: uuid::Uuid::new_v4().to_string(),
                record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
                coverage_status: "observed".into(),
                records,
            },
        )
        .await
        .unwrap();
    let catalog = longlivehealth::metric_history::browse(&state.database, &user.user_id)
        .await
        .unwrap();
    for (_, _, name, _, unit) in cases {
        let metric = catalog["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| {
                m["name"]
                    .as_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
            })
            .unwrap();
        let result = history(
            &state.database,
            &user.user_id,
            HistoryRequest {
                metric_id: metric["metric_id"].as_str().unwrap().into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(result["records"].as_array().unwrap().len(), 4, "{name}");
        assert_eq!(result["points"].as_array().unwrap().len(), 4, "{name}");
        assert!(
            !result["chart_ranges"]["D"].as_array().unwrap().is_empty(),
            "{name}"
        );
        assert!(
            result["points"]
                .as_array()
                .unwrap()
                .iter()
                .all(|p| p["unit"] == unit)
        );
        let merged = result["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["original_records"].as_array().unwrap().len() == 3)
            .unwrap();
        assert_eq!(merged["value"], "0.04");
        assert_eq!(
            merged["original_records"][0]["observation"]["valueQuantity"]["value"],
            0.04
        );
    }
}

#[tokio::test]
async fn exact_unitless_clinical_results_plot_but_text_and_thresholds_do_not() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "unitless-results").await;
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
    let mut records = Vec::new();
    for (id, value) in [
        ("numeric", json!({"valueQuantity":{"value":12}})),
        ("text-number", json!({"valueString":"73"})),
        ("threshold", json!({"valueString":">60"})),
        ("qualitative", json!({"valueString":"Negative"})),
    ] {
        let mut r = json!({"resourceType":"Observation","status":"final","effectiveDateTime":"2026-10-21T00:00:00Z","code":{"text":id}});
        r.as_object_mut()
            .unwrap()
            .extend(value.as_object().unwrap().clone());
        records.push(HealthRecordInput{record_id:id.into(),source_id:"hospital".into(),record_type:"HKClinicalTypeIdentifierLabResultRecord".into(),start_at:1792540800,end_at:1792540800,version:1,deleted:false,payload:json!({"fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&r).unwrap())}})});
    }
    state
        .database
        .sync_health(
            &user.user_id,
            SyncRequest {
                connection_id: connection.connection_id,
                batch_id: uuid::Uuid::new_v4().to_string(),
                record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
                coverage_status: "observed".into(),
                records,
            },
        )
        .await
        .unwrap();
    let catalog = longlivehealth::metric_history::browse(&state.database, &user.user_id)
        .await
        .unwrap();
    for metric in catalog["metrics"].as_array().unwrap() {
        let result = history(
            &state.database,
            &user.user_id,
            HistoryRequest {
                metric_id: metric["metric_id"].as_str().unwrap().into(),
            },
        )
        .await
        .unwrap();
        let numeric = matches!(metric["name"].as_str().unwrap(), "numeric" | "text-number");
        assert_eq!(result["records"].as_array().unwrap().len(), 1);
        assert_eq!(
            result["chart_ranges"]["D"].as_array().unwrap().len(),
            usize::from(numeric)
        );
        if numeric {
            assert_eq!(result["points"][0]["unit"], "Not reported");
        }
    }
}

#[tokio::test]
async fn report_aliases_share_history_and_browse_without_rewriting_stored_evidence() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "alias-review").await;
    for (name, source) in [
        ("LDL-C", "Function Health"),
        ("LDL-Cholesterol", "Local Clinic"),
    ] {
        let report = archive(&state, "alias-review").await;
        let mut payload = observation_payload();
        payload.raw_name = name.into();
        payload.metric_id = None;
        let result = state
            .database
            .submit_report_changes(
                &user.user_id,
                ReportSubmission {
                    report_id: report,
                    expected_revision: 1,
                    context: Some(serde_json::from_value(json!({"source_name":source})).unwrap()),
                    observations: vec![ReportResultChange {
                        observation_id: None,
                        expected_revision: None,
                        status: ReportResultStatus::Submitted,
                        payload,
                    }],
                },
            )
            .await
            .unwrap();
        assert_eq!(result["observations"][0]["payload"]["raw_name"], name);
        assert_eq!(
            result["observations"][0]["payload"]["metric_id"],
            "ldl_cholesterol"
        );
        assert_eq!(
            result["observations"][0]["resolved_metric_id"],
            "ldl_cholesterol"
        );
    }
    let history = read(&state, &user.user_id).await;
    assert_eq!(history["points"].as_array().unwrap().len(), 2);
    assert_ne!(
        history["points"][0]["source_name"],
        history["points"][1]["source_name"]
    );
    let browse = longlivehealth::metric_history::browse(&state.database, &user.user_id)
        .await
        .unwrap();
    assert_eq!(browse["metrics"].as_array().unwrap().len(), 1);
    assert_eq!(browse["metrics"][0]["metric_id"], "ldl_cholesterol");
}

#[test]
fn every_report_catalog_name_resolves_with_its_supported_unit() {
    for definition in longlivehealth::laboratory::metrics() {
        assert_eq!(
            longlivehealth::laboratory::infer_metric(
                definition.name,
                Some(definition.standard_unit)
            ),
            Some(definition.metric_id),
            "{}",
            definition.name
        );
    }
}

#[tokio::test]
async fn browse_unifies_sodium_sources_and_excludes_workflow_fields() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "clean-browse").await;
    let report = archive(&state, "clean-browse").await;
    let mut payload = observation_payload();
    payload.raw_name = "Sodium".into();
    payload.raw_result = "140".into();
    payload.raw_unit = Some("mmol/L".into());
    payload.metric_id = None;
    state
        .database
        .submit_report_changes(
            &user.user_id,
            ReportSubmission {
                report_id: report,
                expected_revision: 1,
                context: None,
                observations: vec![ReportResultChange {
                    observation_id: None,
                    expected_revision: None,
                    status: ReportResultStatus::Submitted,
                    payload,
                }],
            },
        )
        .await
        .unwrap();
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
    let specifications = [
        (
            "Sodium",
            "2951-2",
            json!({"valueQuantity":{"value":140,"unit":"mmol/L"}}),
        ),
        (
            "sodium",
            "2951-2",
            json!({"valueQuantity":{"value":140,"unit":"mmol/L"}}),
        ),
        (
            "Unknown hospital caption",
            "45374-6",
            json!({"valueDateTime":"2026-08-05"}),
        ),
        (
            "Status Information",
            "19066-0",
            json!({"valueString":"Final"}),
        ),
        (
            "Specimen Expiration Date",
            "local-expiration",
            json!({"valueString":"Tomorrow"}),
        ),
        (
            "Antibody screen",
            "890-4",
            json!({"valueString":"Negative"}),
        ),
        (
            "Glucose in urine",
            "2350-7",
            json!({"valueString":"Negative"}),
        ),
    ];
    let records = specifications.into_iter().enumerate().map(|(i,(name,code,values))| {
        let mut resource=json!({"resourceType":"Observation","status":"final","effectiveDateTime":"2026-08-03T00:00:00Z",
            "code":{"text":name,"coding":[{"system":"http://loinc.org","code":code}]}});
        resource.as_object_mut().unwrap().extend(values.as_object().unwrap().clone());
        HealthRecordInput { record_id:format!("item-{i}"),source_id:"hospital".into(),record_type:"HKClinicalTypeIdentifierLabResultRecord".into(),start_at:1785715200,end_at:1785715200,version:1,deleted:false,payload:json!({"source_name":"Hospital","fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&resource).unwrap())}}) }
    }).collect();
    state
        .database
        .sync_health(
            &user.user_id,
            SyncRequest {
                connection_id: connection.connection_id,
                batch_id: uuid::Uuid::new_v4().to_string(),
                record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
                coverage_status: "observed".into(),
                records,
            },
        )
        .await
        .unwrap();
    let browse = longlivehealth::metric_history::browse(&state.database, &user.user_id)
        .await
        .unwrap();
    let entries = browse["metrics"].as_array().unwrap();
    assert_eq!(entries.len(), 3); // Sodium, qualitative antibody result, urine glucose.
    assert_eq!(
        entries
            .iter()
            .filter(|m| m["metric_id"] == "sodium")
            .count(),
        1
    );
    assert!(
        !entries
            .iter()
            .any(|m| m["name"].as_str().unwrap().contains("Expiration")
                || m["name"] == "Status Information")
    );
    let result = history(
        &state.database,
        &user.user_id,
        HistoryRequest {
            metric_id: "sodium".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(result["records"].as_array().unwrap().len(), 2); // Report and hospital histories.
    assert_eq!(
        result["records"][0]["original_records"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(result["points"].as_array().unwrap().len(), 2); // Report + hospital; sources preserved.
}

#[tokio::test]
async fn report_history_keeps_undated_and_non_numeric_results() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let report = archive(&state, "alice").await;
    let mut undated = observation_payload();
    undated.sampled_at = None;
    let mut text = observation_payload();
    text.raw_result = "Not available".into();
    state
        .database
        .submit_report_changes(
            &alice.user_id,
            ReportSubmission {
                report_id: report,
                expected_revision: 1,
                context: Some(
                    serde_json::from_value(json!({"source_name":"Function Health"})).unwrap(),
                ),
                observations: vec![undated, text]
                    .into_iter()
                    .map(|payload| ReportResultChange {
                        observation_id: None,
                        expected_revision: None,
                        status: ReportResultStatus::Submitted,
                        payload,
                    })
                    .collect(),
            },
        )
        .await
        .unwrap();
    let result = read(&state, &alice.user_id).await;
    assert!(result["points"].as_array().unwrap().is_empty());
    let records = result["records"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    assert!(records.iter().any(|r| r["sampled_at"].is_null()));
    assert!(
        records
            .iter()
            .any(|r| r["display_value"] == "Not available")
    );
    assert!(
        records
            .iter()
            .all(|r| r["source_name"] == "Function Health" && r["original"].is_object())
    );
}

#[tokio::test]
async fn every_function_health_marker_is_available_in_browse_after_submission() {
    let (_directory, state) = crate::fixture().await;
    let user = crate::add_user(&state, "function-labs").await;
    let report = crate::archive(&state, "function-labs").await;
    // Only marker names and units; all values/dates below are synthetic.
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("fixtures/function_health_markers.json")).unwrap();
    let mut observations = Vec::new();
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        let unit = row["unit"].as_str();
        assert_eq!(
            longlivehealth::laboratory::infer_metric(name, unit),
            row["metric_id"].as_str(),
            "{name}: {unit:?}"
        );
        let mut payload = crate::observation_payload();
        payload.raw_name = name.into();
        payload.raw_unit = unit.map(str::to_owned);
        payload.raw_result = if unit.is_some() { "1" } else { "Negative" }.into();
        payload.metric_id = None;
        observations.push(longlivehealth::reports::ReportResultChange {
            observation_id: None,
            expected_revision: None,
            status: ReportResultStatus::Submitted,
            payload,
        });
    }
    state
        .database
        .submit_report_changes(
            &user.user_id,
            longlivehealth::reports::ReportSubmission {
                report_id: report,
                expected_revision: 1,
                context: None,
                observations,
            },
        )
        .await
        .unwrap();
    let browse = longlivehealth::metric_history::browse(&state.database, &user.user_id)
        .await
        .unwrap();
    let metrics = browse["metrics"].as_array().unwrap();
    for row in &rows {
        assert!(
            metrics.iter().any(|m| m["metric_id"] == row["metric_id"]),
            "Missing from Browse: {}",
            row["name"]
        );
    }
    let globulin = history(
        &state.database,
        &user.user_id,
        HistoryRequest {
            metric_id: "globulin".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(globulin["points"].as_array().unwrap().len(), 1);
    assert_eq!(globulin["points"][0]["unit"], "g/dL");
    assert_eq!(globulin["points"][0]["value"], "1");
    assert_eq!(globulin["records"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn pdf_lab_codes_and_names_share_history_without_rewriting_evidence() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    for (source, name, date, unit) in [
        ("Portal A", "Test (11580-8)", "2026-08-01", "uIU/mL"),
        ("Portal B", "TSH", "2026-09-01", "mIU/L"),
    ] {
        let report = archive(&state, "alice").await;
        let mut payload = observation_payload();
        payload.raw_name = name.into();
        payload.raw_result = "1.5".into();
        payload.raw_unit = Some(unit.into());
        payload.sampled_at = Some(date.into());
        payload.metric_id = None;
        payload.source.quote = format!("{name} 1.5 {unit}");
        let document = state
            .database
            .submit_report_changes(
                &alice.user_id,
                ReportSubmission {
                    report_id: report,
                    expected_revision: 1,
                    context: Some(serde_json::from_value(json!({"source_name": source})).unwrap()),
                    observations: vec![ReportResultChange {
                        observation_id: None,
                        expected_revision: None,
                        status: ReportResultStatus::Submitted,
                        payload,
                    }],
                },
            )
            .await
            .unwrap();
        let row = &document["observations"][0];
        assert_eq!(row["display_name"], "Thyroid-stimulating hormone");
        assert_eq!(row["payload"]["raw_name"], name);
        assert!(
            row["payload"]["source"]["quote"]
                .as_str()
                .unwrap()
                .starts_with(name)
        );
        if source == "Portal A" {
            assert_eq!(row["lab_test_code"], "11580-8");
        }
    }
    let result = history(
        &state.database,
        &alice.user_id,
        HistoryRequest {
            metric_id: "tsh".into(),
        },
    )
    .await
    .unwrap();
    let points = result["points"].as_array().unwrap();
    assert_eq!(points.len(), 2);
    assert_eq!(points[0]["source_name"], "Portal A");
    assert_eq!(points[1]["source_name"], "Portal B");
    assert_eq!(points[0]["unit"], "mIU/L");
    assert_eq!(points[1]["unit"], "mIU/L");
}

#[test]
fn numeric_suffixes_preserve_qualifiers_and_reject_code_conflicts() {
    use longlivehealth::laboratory::{infer_metric, split_test_code};
    assert_eq!(
        split_test_code("TSH (3rd Generation) (11580-8)"),
        ("TSH (3rd Generation)", Some("11580-8"))
    );
    assert_eq!(
        split_test_code("Vitamin D (25-OH)"),
        ("Vitamin D (25-OH)", None)
    );
    assert_eq!(
        infer_metric("TSH (3rd Generation) (11580-8)", Some("mIU/L")),
        Some("tsh")
    );
    assert_eq!(infer_metric("Glucose (11580-8)", Some("mg/dL")), None);
    assert_eq!(infer_metric("Test (11580-8)", Some("mg/dL")), None);
    assert_eq!(infer_metric("Urine TSH (11580-8)", Some("mIU/L")), None);
}

#[tokio::test]
async fn browse_keeps_submitted_unmapped_results_searchable_with_original_history() {
    let (directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let bob = add_user(&state, "bob").await;
    let mut report_ids = Vec::new();
    for (index, (source, unit, value, status)) in [
        ("Lab A", None, "6.0", ReportResultStatus::Submitted),
        ("Lab A", None, "<6.5", ReportResultStatus::Submitted),
        ("Lab A", Some("mmol/L"), "7", ReportResultStatus::Submitted),
        ("Lab B", None, "8", ReportResultStatus::Submitted),
        ("Lab A", None, "9", ReportResultStatus::Pending),
    ]
    .into_iter()
    .enumerate()
    {
        let report_id = archive(&state, "alice").await;
        let mut payload = observation_payload();
        payload.raw_name = "pH (50560-2)".into();
        payload.raw_result = value.into();
        payload.raw_unit = unit.map(str::to_owned);
        payload.sampled_at = Some(format!("2018-10-2{index}"));
        payload.metric_id = None;
        if status == ReportResultStatus::Submitted {
            seed_legacy_submitted_observation(
                &directory,
                &alice.user_id,
                &report_id,
                Some(source),
                &payload,
            )
            .await;
            report_ids.push(report_id);
            continue;
        }
        let result = state
            .database
            .submit_report_changes(
                &alice.user_id,
                ReportSubmission {
                    report_id: report_id.clone(),
                    expected_revision: 1,
                    context: Some(serde_json::from_value(json!({"source_name":source})).unwrap()),
                    observations: vec![ReportResultChange {
                        observation_id: None,
                        expected_revision: None,
                        status,
                        payload,
                    }],
                },
            )
            .await
            .unwrap();
        assert!(result["observations"][0]["payload"]["metric_id"].is_null());
        report_ids.push(report_id);
    }

    let catalog = longlivehealth::browse_catalog::get(&state.database, &alice.user_id)
        .await
        .unwrap();
    let entries = catalog["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    let mut found = false;
    for entry in entries {
        assert_eq!(entry["title"], "pH");
        let metric_id = entry["kind"].as_str().unwrap();
        assert!(metric_id.starts_with("reported:"));
        let read = |owner| {
            history(
                &state.database,
                owner,
                HistoryRequest {
                    metric_id: metric_id.into(),
                },
            )
        };
        assert!(matches!(
            read(&bob.user_id).await,
            Err(longlivehealth::error::AppError::NotFound)
        ));
        let result = read(&alice.user_id).await.unwrap();
        let records = result["records"].as_array().unwrap();
        if records.len() == 2 {
            found = true;
            assert!(
                records
                    .iter()
                    .any(|row| row["report_id"] == report_ids[0] && row["display_value"] == "6.0")
            );
            assert!(
                records
                    .iter()
                    .any(|row| row["report_id"] == report_ids[1] && row["display_value"] == "<6.5")
            );
            assert_eq!(result["points"].as_array().unwrap().len(), 1);
            assert_eq!(result["points"][0]["original"]["raw_name"], "pH (50560-2)");
            assert_eq!(result["metric"]["loinc"][0]["code"], "50560-2");
            assert_eq!(
                result["metric"]["loinc"][0]["official_name"],
                "pH of Urine by Automated test strip"
            );
        } else {
            assert_eq!(records.len(), 1);
            assert!(
                records[0]["report_id"] == report_ids[2]
                    || records[0]["report_id"] == report_ids[3]
            );
        }
    }
    assert!(found);
}

#[tokio::test]
async fn browse_hides_drafts_but_reports_keep_review_issues() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let bob = add_user(&state, "bob").await;
    let report = archive(&state, "alice").await;
    let mut items = Vec::new();
    for (name, note) in [
        ("Unlisted test (99999-9)", None),
        ("TSH (11580-8)", Some("The code is difficult to read.")),
        ("Glucose (11580-8)", None),
        ("Unreadable label (11580-8)", None),
    ] {
        let mut payload = observation_payload();
        payload.raw_name = name.into();
        payload.raw_unit = Some("mIU/L".into());
        payload.metric_id = None;
        payload.ocr_mapping_note = note.map(str::to_owned);
        items.push(ReportResultChange {
            observation_id: None,
            expected_revision: None,
            status: ReportResultStatus::Pending,
            payload,
        });
    }
    let response = state
        .database
        .submit_report_changes(
            &alice.user_id,
            ReportSubmission {
                report_id: report.clone(),
                expected_revision: 1,
                context: None,
                observations: items,
            },
        )
        .await
        .unwrap();
    let observations = response["observations"].as_array().unwrap();
    let find = |name: &str| {
        observations
            .iter()
            .find(|row| row["payload"]["raw_name"] == name)
            .unwrap()
    };
    let unknown = find("Unlisted test (99999-9)");
    let uncertain = find("TSH (11580-8)");
    assert_eq!(unknown["browse_issues"][0]["code"], "unmapped_metric");
    assert_eq!(
        uncertain["browse_issues"][0]["code"],
        "metric_identity_uncertain"
    );
    assert!(
        uncertain["browse_issues"][0]["message"]
            .as_str()
            .unwrap()
            .contains("Thyroid-stimulating hormone")
    );
    assert_eq!(
        find("Glucose (11580-8)")["browse_issues"][0]["code"],
        "lab_test_code_conflict"
    );
    let possible = find("Unreadable label (11580-8)");
    assert_eq!(
        possible["browse_issues"][0]["code"],
        "metric_identity_uncertain"
    );
    assert!(
        possible["browse_issues"][0]["message"]
            .as_str()
            .unwrap()
            .contains("Thyroid-stimulating hormone")
    );
    let catalog = longlivehealth::browse_catalog::get(&state.database, &alice.user_id)
        .await
        .unwrap();
    assert!(catalog["entries"].as_array().unwrap().is_empty());
    let id = uncertain["observation_id"].as_str().unwrap();
    let metric_id = format!("report-review:{id}");
    for user in [&alice, &bob] {
        assert!(matches!(
            history(
                &state.database,
                &user.user_id,
                HistoryRequest {
                    metric_id: metric_id.clone()
                },
            )
            .await,
            Err(longlivehealth::error::AppError::NotFound)
        ));
    }

    let mut payload: longlivehealth::reports::ObservationPayload =
        serde_json::from_value(uncertain["payload"].clone()).unwrap();
    payload.metric_id = Some("tsh".into());
    let submitted = state
        .database
        .submit_report_changes(
            &alice.user_id,
            ReportSubmission {
                report_id: report.clone(),
                expected_revision: response["report"]["revision"].as_i64().unwrap(),
                context: None,
                observations: vec![ReportResultChange {
                    observation_id: Some(id.into()),
                    expected_revision: Some(1),
                    status: ReportResultStatus::Submitted,
                    payload: payload.clone(),
                }],
            },
        )
        .await
        .unwrap();
    assert!(
        history(
            &state.database,
            &alice.user_id,
            HistoryRequest { metric_id }
        )
        .await
        .is_err()
    );
    let mapped = history(
        &state.database,
        &alice.user_id,
        HistoryRequest {
            metric_id: "tsh".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(mapped["points"].as_array().unwrap().len(), 1);
    let catalog = longlivehealth::browse_catalog::get(&state.database, &alice.user_id)
        .await
        .unwrap();
    assert_eq!(catalog["entries"].as_array().unwrap().len(), 1);
    assert_eq!(catalog["entries"][0]["kind"], "tsh");
    state
        .database
        .submit_report_changes(
            &alice.user_id,
            ReportSubmission {
                report_id: report,
                expected_revision: submitted["report"]["revision"].as_i64().unwrap(),
                context: None,
                observations: vec![ReportResultChange {
                    observation_id: Some(id.into()),
                    expected_revision: Some(2),
                    status: ReportResultStatus::Pending,
                    payload,
                }],
            },
        )
        .await
        .unwrap();
    let catalog = longlivehealth::browse_catalog::get(&state.database, &alice.user_id)
        .await
        .unwrap();
    assert!(catalog["entries"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn browse_unifies_total_vitamin_d_without_combining_its_fractions() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "vitamin-d-browse").await;
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
    let mut records = Vec::new();
    for (id, name, code, status, quest, value, unit) in [
        (
            "total",
            "Vitamin D,25-OH, Total, IA",
            "62292-8",
            "final",
            true,
            20,
            "ng/mL",
        ),
        (
            "legacy-quest",
            "Vitamin D,25-OH, Total, IA",
            "1989-3",
            "final",
            true,
            20,
            "ng/mL",
        ),
        (
            "total-molar",
            "25(OH)D",
            "68438-1",
            "final",
            false,
            60,
            "nmol/L",
        ),
        (
            "d3",
            "25-hydroxyvitamin D3",
            "1989-3",
            "final",
            false,
            20,
            "ng/mL",
        ),
        (
            "d2",
            "25-hydroxyvitamin D2",
            "49054-0",
            "final",
            false,
            20,
            "ng/mL",
        ),
        (
            "preliminary",
            "Unverified Test",
            "99999-8",
            "preliminary",
            false,
            20,
            "ng/mL",
        ),
    ] {
        let mut codings = vec![json!({"system":"http://loinc.org","code":code})];
        if quest {
            codings.push(json!({"system":"urn:oid:1.2.840.114350.1.13.535.2.7.5.737384.334307","code":"55182715"}));
        }
        let r = json!({"resourceType":"Observation","status":status,"effectiveDateTime":"2026-08-02T12:00:00Z","code":{"text":name,"coding":codings},"valueQuantity":{"value":value,"code":unit}});
        records.push(HealthRecordInput {
            record_id:id.into(),source_id:"hospital".into(),record_type:"HKClinicalTypeIdentifierLabResultRecord".into(),
            start_at:1785672000,end_at:1785672000,version:1,deleted:false,
            payload:json!({"source_name":"Hospital","fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&r).unwrap())}}),
        });
    }
    state
        .database
        .sync_health(
            &user.user_id,
            SyncRequest {
                connection_id: connection.connection_id,
                batch_id: uuid::Uuid::new_v4().to_string(),
                record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
                coverage_status: "observed".into(),
                records,
            },
        )
        .await
        .unwrap();
    for (source, name, expected) in [
        ("Function Health", "Vitamin D", "vitamin_d_25_oh_total"),
        ("Other Clinic", "Vitamin D", "vitamin_d_unspecified"),
        ("Other Clinic", "25 OH Vitamin D", "vitamin_d_25_oh_total"),
        (
            "Other Clinic",
            "25-hydroxyvitamin D",
            "vitamin_d_25_oh_total",
        ),
    ] {
        let mut payload = observation_payload();
        payload.raw_name = name.into();
        payload.raw_unit = Some("ng/mL".into());
        payload.raw_result = "22".into();
        payload.metric_id = None;
        let report = archive(&state, "vitamin-d-browse").await;
        let response = state
            .database
            .submit_report_changes(
                &user.user_id,
                ReportSubmission {
                    report_id: report,
                    expected_revision: 1,
                    context: Some(serde_json::from_value(json!({"source_name":source})).unwrap()),
                    observations: vec![ReportResultChange {
                        observation_id: None,
                        expected_revision: None,
                        status: ReportResultStatus::Submitted,
                        payload,
                    }],
                },
            )
            .await
            .unwrap();
        assert_eq!(response["observations"][0]["resolved_metric_id"], expected);
    }
    let catalog = longlivehealth::browse_catalog::get(&state.database, &user.user_id)
        .await
        .unwrap();
    let entries = catalog["entries"].as_array().unwrap();
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry["presentation"] == "lab")
            .count(),
        4,
        "{entries:?}"
    );
    for (id, plotted, archived) in [
        ("vitamin_d_25_oh_total", 5, 5),
        ("vitamin_d_25_oh_d2", 1, 1),
        ("vitamin_d_25_oh_d3", 1, 2),
        ("vitamin_d_unspecified", 1, 1),
    ] {
        assert_eq!(entries.iter().filter(|e| e["kind"] == id).count(), 1);
        let result = history(
            &state.database,
            &user.user_id,
            HistoryRequest {
                metric_id: id.into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(result["points"].as_array().unwrap().len(), plotted, "{id}");
        let records = result["records"].as_array().unwrap();
        assert_eq!(records.len(), archived, "{id}");
        let pending = result["pending_chart_ranges"]["D"].as_array().unwrap();
        assert_eq!(pending.len(), usize::from(id == "vitamin_d_25_oh_d3"));
        if id == "vitamin_d_25_oh_total" {
            let codes = result["metric"]["loinc"].as_array().unwrap();
            assert_eq!(codes.len(), 2);
            assert!(codes.iter().any(|entry| entry["code"] == "62292-8"));
            assert!(codes.iter().any(|entry| entry["code"] == "68438-1"));
            assert!(result["points"].as_array().unwrap().iter().any(|point| {
                point["value"] == "24"
                    && point["unit"] == "ng/mL"
                    && point["original"]["clinical_observation"]["valueQuantity"]["code"]
                        == "nmol/L"
            }));
        } else if id == "vitamin_d_25_oh_d3" {
            let held = records
                .iter()
                .find(|record| record["status"] == "pending")
                .unwrap();
            assert_eq!(held["original"]["status"], "final");
            assert_eq!(held["original"]["code"]["coding"][0]["code"], "1989-3");
            assert!(held["review_reason"].as_str().unwrap().contains("1989-3"));
            assert_ne!(held["plotted"], true);
            assert_eq!(pending[0]["unit"], held["unit"]);
            assert_eq!(
                pending[0]["min"].as_f64(),
                held["original"]["valueQuantity"]["value"].as_f64()
            );
        }
    }
}

#[test]
fn equivalent_vitamin_d_names_preserve_analyte_and_specimen_qualifiers() {
    use longlivehealth::laboratory::infer_metric;
    for label in [
        "25 OH",
        "25 OH Vitamin D",
        "25‑OH Vit D",
        "25-hydroxy vitamin D",
        "25-hydroxyvitamin D",
        "25(OH)D",
        "Vitamin D, 25-Hydroxy, Total (62292-8)",
        "25(OH)D (68438-1)",
    ] {
        assert_eq!(
            infer_metric(label, Some("ng/mL")),
            Some("vitamin_d_25_oh_total"),
            "{label}"
        );
    }
    assert_eq!(
        infer_metric("25(OH)D2", Some("ng/mL")),
        Some("vitamin_d_25_oh_d2")
    );
    assert_eq!(
        infer_metric("25(OH)D3", Some("ng/mL")),
        Some("vitamin_d_25_oh_d3")
    );
    for label in [
        "1,25-dihydroxyvitamin D",
        "25-hydroxyvitamin D, urine",
        "25(OH)D (1989-3)",
    ] {
        assert_eq!(infer_metric(label, Some("ng/mL")), None, "{label}");
    }
}

#[test]
fn catalog_normalizes_reviewed_report_aliases_and_units() {
    use longlivehealth::laboratory::{infer_metric, test_code_conflicts};
    for (name, unit, id) in [
        (
            "IG, ABS. COUNT (53115-2)",
            "x10(3)/uL",
            "immature_granulocyte_count",
        ),
        (
            "Imm Gran Abs (51584-1)",
            "10^9/L",
            "immature_granulocyte_count",
        ),
        (
            "Immature Granulocytes (71695-1)",
            "Percent",
            "immature_granulocyte_percent",
        ),
        ("Imm Gran % (38518-7)", "%", "immature_granulocyte_percent"),
        ("Absolute NRBC (771-6)", "x10(3)/uL", "nrbc_count"),
        ("NRBC Count (30392-5)", "x10E3/uL", "nrbc_count"),
        ("NRBC % (18309-5)", "/100WBC", "nrbc_per_100_wbc"),
        (
            "Nucleated RBC Auto (58413-6)",
            "/100 WBC's",
            "nrbc_per_100_wbc",
        ),
        ("Thyroxine(T4) (3026-2)", "ug/dL", "total_t4"),
        ("Thyroxine (T4), Total (83119-8)", "µg/dL", "total_t4"),
        ("T4, Free (3024-7)", "ng/dL", "free_t4"),
        ("LD (14804-9)", "U/L", "ldh"),
        ("LDH", "U/L", "ldh"),
        (
            "VLDL Cholesterol, Calc. (13458-5)",
            "mg/dL",
            "vldl_cholesterol",
        ),
        ("VLDL, CALCULATED", "mg/dL", "vldl_cholesterol"),
        ("Chol/HDL Ratio (9830-1)", "Ratio", "cholesterol_hdl_ratio"),
        ("LDL/HDL Ratio (11054-4)", "1", "ldl_hdl_ratio"),
        ("Trig/HDL Ratio (44733-4)", "1", "triglyceride_hdl_ratio"),
        (
            "HDL as % of Cholesterol (2095-8)",
            "%",
            "hdl_percent_total_cholesterol",
        ),
        ("Alk Phos (6768-6)", "U/L", "alp"),
        ("GGTP (2324-2)", "U/L", "ggt"),
        ("Urate (3084-1)", "mg/dL", "uric_acid"),
        ("ANTI-TPO Ab (8099-4)", "[IU]/mL", "tpo_antibodies"),
        ("TSH (3016-3)", "u[IU]/mL", "tsh"),
        ("Protein (20454-5)", "1", "urine_protein"),
        ("Glucose (25428-4)", "1", "urine_glucose"),
    ] {
        assert_eq!(infer_metric(name, Some(unit)), Some(id), "{name} {unit}");
    }
    for (name, unit) in [
        ("Free T4 (83119-8)", "ng/dL"),
        ("NRBC Count (18309-5)", "/100WBC"),
        ("ANTI-THYROGLOBULIN (15210-8)", "[IU]/mL"),
        ("ANTI-TPO Ab (32786-6)", "[IU]/mL"),
        ("Neutrophils (31160-5)", "%"),
        ("Monocytes (51384-6)", "%"),
    ] {
        assert_eq!(infer_metric(name, Some(unit)), None, "{name}");
        assert!(test_code_conflicts(name), "{name}");
    }
    assert_eq!(
        infer_metric("Cast, Hyaline, Urine (50231-0)", Some("/[LPF]")),
        None
    );
}

#[tokio::test]
async fn blood_count_aliases_join_pdf_and_clinical_history() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "blood-count-aliases").await;
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
    let tests = [
        (
            "Imm Gran Abs",
            "51584-1",
            "IG, ABS. COUNT (53115-2)",
            "x10(3)/uL",
            "10^9/L",
            "immature_granulocyte_count",
        ),
        (
            "Nucleated RBC Auto",
            "58413-6",
            "NRBC % (18309-5)",
            "/100WBC",
            "/100 WBC's",
            "nrbc_per_100_wbc",
        ),
    ];
    let report = archive(&state, "blood-count-aliases").await;
    let mut records = Vec::new();
    let mut observations = Vec::new();
    for (clinical, code, printed, unit, standard_unit, id) in tests {
        let resource = json!({"resourceType":"Observation","status":"final","effectiveDateTime":"2026-08-02T12:00:00Z",
            "code":{"text":clinical,"coding":[{"system":"http://loinc.org","code":code}]},
            "valueQuantity":{"value":2,"code":standard_unit}});
        records.push(HealthRecordInput {
            record_id:id.into(),source_id:"hospital".into(),record_type:"HKClinicalTypeIdentifierLabResultRecord".into(),
            start_at:1785672000,end_at:1785672000,version:1,deleted:false,
            payload:json!({"fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&resource).unwrap())}}),
        });
        let mut payload = observation_payload();
        payload.metric_id = None;
        payload.raw_name = printed.into();
        payload.raw_result = "1".into();
        payload.raw_unit = Some(unit.into());
        observations.push(ReportResultChange {
            observation_id: None,
            expected_revision: None,
            status: ReportResultStatus::Submitted,
            payload,
        });
    }
    state
        .database
        .sync_health(
            &user.user_id,
            SyncRequest {
                connection_id: connection.connection_id,
                batch_id: uuid::Uuid::new_v4().to_string(),
                record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
                coverage_status: "observed".into(),
                records,
            },
        )
        .await
        .unwrap();
    state
        .database
        .submit_report_changes(
            &user.user_id,
            ReportSubmission {
                report_id: report,
                expected_revision: 1,
                context: None,
                observations,
            },
        )
        .await
        .unwrap();

    let browse = longlivehealth::browse_catalog::get(&state.database, &user.user_id)
        .await
        .unwrap();
    let entries = browse["entries"].as_array().unwrap();
    for (clinical, _, printed, unit, standard_unit, id) in tests {
        assert_eq!(
            entries.iter().filter(|entry| entry["kind"] == id).count(),
            1
        );
        let entry = entries.iter().find(|entry| entry["kind"] == id).unwrap();
        assert!(
            entry["aliases"]
                .as_array()
                .unwrap()
                .contains(&json!(clinical))
        );
        let result = history(
            &state.database,
            &user.user_id,
            HistoryRequest {
                metric_id: id.into(),
            },
        )
        .await
        .unwrap();
        let points = result["points"].as_array().unwrap();
        assert_eq!(points.len(), 2, "{id}");
        assert!(points.iter().all(|point| point["unit"] == standard_unit));
        assert!(
            points
                .iter()
                .any(|point| point["original"]["raw_name"] == printed
                    && point["original"]["raw_unit"] == unit)
        );
        assert!(
            points
                .iter()
                .any(|point| point["original"]["clinical_observation"]["code"]["text"] == clinical)
        );
    }
}

#[tokio::test]
async fn rename_and_review_link_pdf_to_apple_metric_without_changing_identity() {
    use axum::http::StatusCode;
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let bob = add_user(&state, "bob").await;
    let connection = state
        .database
        .create_connection(
            &alice.user_id,
            ConnectionRequest {
                platform: "apple_health".into(),
                installation_id: uuid::Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap();
    let clinical = json!({
        "resourceType": "Observation", "status": "final", "effectiveDateTime": "2026-08-02T12:00:00Z",
        "code": {"text": "Direct bilirubin", "coding": [{"system": "http://loinc.org", "code": "1968-7"}]},
        "valueQuantity": {"value": 0.2, "code": "mg/dL", "system": "http://unitsofmeasure.org"}
    });
    state.database.sync_health(&alice.user_id, SyncRequest {
        connection_id: connection.connection_id,
        batch_id: uuid::Uuid::new_v4().to_string(),
        record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
        coverage_status: "observed".into(),
        records: vec![HealthRecordInput {
            record_id: "direct-bilirubin".into(), source_id: "lab".into(),
            record_type: "HKClinicalTypeIdentifierLabResultRecord".into(),
            start_at: 1785672000, end_at: 1785672000,
            version: 1, deleted: false,
            payload: json!({"fhir":{"data_base64":STANDARD.encode(serde_json::to_vec(&clinical).unwrap())}}),
        }],
    }).await.unwrap();

    let browse = longlivehealth::metric_history::browse(&state.database, &alice.user_id)
        .await
        .unwrap();
    let id = browse["metrics"][0]["metric_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(id.starts_with("clinical:"));
    let report_id = archive(&state, "alice").await;
    let mut payload = observation_payload();
    payload.metric_id = Some(id.clone());
    payload.raw_name = "Bilirubin Direct (1968-7)".into();
    payload.raw_result = "0.3".into();

    state
        .database
        .submit_report_changes(
            &alice.user_id,
            ReportSubmission {
                report_id: report_id.clone(),
                expected_revision: 1,
                context: None,
                observations: vec![ReportResultChange {
                    observation_id: None,
                    expected_revision: None,
                    status: ReportResultStatus::Submitted,
                    payload: payload.clone(),
                }],
            },
        )
        .await
        .unwrap();

    let before = history(
        &state.database,
        &alice.user_id,
        HistoryRequest {
            metric_id: id.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(before["points"].as_array().unwrap().len(), 2);
    assert_eq!(before["records"].as_array().unwrap().len(), 2);
    let app = longlivehealth::app::create_application(state.clone());
    let alice_token = crate::token(&state, "alice").await;
    let bob_token = crate::token(&state, "bob").await;
    let rename = json!({"metric_id": id, "name": "My direct bilirubin"});
    assert_eq!(
        crate::request(
            &app,
            "/api/v1/metrics/rename",
            Some(&bob_token),
            rename.clone()
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let (status, result) =
        crate::request(&app, "/api/v1/metrics/rename", Some(&alice_token), rename).await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["metric_id"], id);
    let after = history(
        &state.database,
        &alice.user_id,
        HistoryRequest {
            metric_id: id.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(after["metric"]["name"], "My direct bilirubin");
    assert_eq!(after["metric"]["loinc"][0]["code"], "1968-7");
    assert!(
        after["metric"]["loinc"][0]["official_name"]
            .as_str()
            .unwrap()
            .contains("Bilirubin")
    );
    assert_eq!(before["points"], after["points"]);
    assert_eq!(before["records"], after["records"]);
    let (_, available) =
        crate::request(&app, "/api/v1/metrics/list", Some(&alice_token), json!({})).await;
    assert!(
        available["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|metric| metric["metric_id"] == id && metric["name"] == "My direct bilirubin")
    );
    let names: Vec<_> = available["metrics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|metric| metric["name"].as_str().unwrap().to_lowercase())
        .collect();
    assert!(names.windows(2).all(|pair| pair[0] <= pair[1]));
    let (_, catalog) = crate::request(
        &app,
        "/api/v1/browse/catalog",
        Some(&alice_token),
        json!({}),
    )
    .await;
    assert!(
        catalog["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["kind"] == id && entry["title"] == "My direct bilirubin")
    );

    payload.metric_id = None;
    payload.raw_name = "Alternate printed name (1968-7)".into();
    state
        .database
        .submit_report_changes(
            &alice.user_id,
            ReportSubmission {
                report_id: report_id.clone(),
                expected_revision: 2,
                context: None,
                observations: vec![ReportResultChange {
                    observation_id: None,
                    expected_revision: None,
                    status: ReportResultStatus::Pending,
                    payload: payload.clone(),
                }],
            },
        )
        .await
        .unwrap();
    let rows = state
        .database
        .observations(&alice.user_id, &report_id)
        .await
        .unwrap();
    let pending = rows.iter().find(|row| row.status == "pending").unwrap();
    assert_eq!(pending.resolved_metric_id.as_deref(), Some(id.as_str()));
    assert_eq!(pending.display_name, "Alternate printed name");

    let update = |status: ReportResultStatus, revision, payload| ReportSubmission {
        report_id: report_id.clone(),
        expected_revision: revision,
        context: None,
        observations: vec![ReportResultChange {
            observation_id: Some(pending.observation_id.clone()),
            expected_revision: Some(revision - 2),
            status,
            payload,
        }],
    };
    let mut conflict = payload.clone();
    conflict.metric_id = Some("total_bilirubin".into());
    state
        .database
        .submit_report_changes(
            &alice.user_id,
            update(ReportResultStatus::Pending, 3, conflict.clone()),
        )
        .await
        .unwrap();
    let rejected = state
        .database
        .submit_report_changes(
            &alice.user_id,
            update(ReportResultStatus::Submitted, 4, conflict),
        )
        .await
        .unwrap_err();
    assert!(rejected.to_string().contains("already linked"));
    let rows = state
        .database
        .observations(&alice.user_id, &report_id)
        .await
        .unwrap();
    let pending = rows.iter().find(|row| row.status == "pending").unwrap();
    assert!(
        pending
            .browse_issues
            .iter()
            .any(|issue| issue.code == "loinc_metric_conflict")
    );
    assert_eq!(
        state
            .database
            .submitted_observations(&alice.user_id)
            .await
            .unwrap()
            .len(),
        1
    );
    state
        .database
        .submit_report_changes(
            &alice.user_id,
            update(ReportResultStatus::Submitted, 4, payload.clone()),
        )
        .await
        .unwrap();
    let rows = state
        .database
        .submitted_observations(&alice.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .all(|row| row.payload.metric_id.as_deref() == Some(id.as_str()))
    );

    let bob_report = archive(&state, "bob").await;
    state
        .database
        .submit_report_changes(
            &bob.user_id,
            ReportSubmission {
                report_id: bob_report.clone(),
                expected_revision: 1,
                context: None,
                observations: vec![ReportResultChange {
                    observation_id: None,
                    expected_revision: None,
                    status: ReportResultStatus::Pending,
                    payload,
                }],
            },
        )
        .await
        .unwrap();
    let rows = state
        .database
        .observations(&bob.user_id, &bob_report)
        .await
        .unwrap();
    assert!(rows[0].resolved_metric_id.is_none());
}

#[tokio::test]
async fn rename_updates_saved_pending_title_and_preserves_unsaved_pdf_title() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let report_id = archive(&state, "alice").await;
    let mut parsed = observation_payload();
    parsed.raw_name = "Ketone, Urine (2514-8)".into();
    parsed.raw_result = "Negative".into();
    parsed.raw_unit = None;
    parsed.metric_id = None;
    parsed.ocr_metric_candidate = Some("urine_ketones".into());
    parsed.source.quote = "Ketone, Urine (2514-8) Negative".into();
    let mut mapped = parsed.clone();
    mapped.metric_id = Some("urine_ketones".into());

    let before = state
        .database
        .submit_report_changes(
            &alice.user_id,
            ReportSubmission {
                report_id: report_id.clone(),
                expected_revision: 1,
                context: None,
                observations: [
                    (ReportResultStatus::Submitted, mapped.clone()),
                    (ReportResultStatus::Pending, mapped),
                    (ReportResultStatus::Pending, parsed),
                ]
                .into_iter()
                .map(|(status, payload)| ReportResultChange {
                    observation_id: None,
                    expected_revision: None,
                    status,
                    payload,
                })
                .collect(),
            },
        )
        .await
        .unwrap();

    let app = longlivehealth::app::create_application(state.clone());
    let token = crate::token(&state, "alice").await;
    let (status, _) = crate::request(
        &app,
        "/api/v1/metrics/rename",
        Some(&token),
        json!({"metric_id":"urine_ketones","name":"Urine - Ketones"}),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let (_, after) = crate::request(
        &app,
        "/api/v1/reports/get",
        Some(&token),
        json!({"report_id":report_id}),
    )
    .await;
    let before_rows = before["observations"].as_array().unwrap();
    let after_rows = after["observations"].as_array().unwrap();
    assert_eq!(before_rows.len(), 3);
    assert_eq!(after_rows.len(), 3);
    for old in before_rows {
        let new = after_rows
            .iter()
            .find(|row| row["observation_id"] == old["observation_id"])
            .unwrap();
        assert_eq!(new["payload"], old["payload"]);
        assert_eq!(new["revision"], old["revision"]);
        assert_eq!(new["status"], old["status"]);
        assert_eq!(new["resolved_metric_id"], "urine_ketones");
        assert_eq!(new["lab_test_code"], "2514-8");
        let expected = if new["payload"]["metric_id"].is_null() {
            "Ketone, Urine"
        } else {
            "Urine - Ketones"
        };
        assert_eq!(new["display_name"], expected);
    }
}

#[tokio::test]
async fn lab_charts_keep_closed_ranges_and_binary_states_distinct() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "chart-shapes").await;
    for (index, (metric, name, result, unit)) in [
        (
            "ldl_cholesterol",
            "LDL Cholesterol",
            "2.586–5.172",
            Some("mmol/L"),
        ),
        ("ldl_cholesterol", "LDL Cholesterol", "120", Some("mg/dL")),
        ("ldl_cholesterol", "LDL Cholesterol", "<90", Some("mg/dL")),
        (
            "urine_bacteria",
            "Bacteria - urine",
            "None to Few",
            Some("/HPF"),
        ),
        (
            "urine_ketones",
            "Ketones - urine",
            "Negative",
            Some("mg/dL"),
        ),
        ("urine_ketones", "Ketones - urine", "Positive", None),
    ]
    .into_iter()
    .enumerate()
    {
        let report_id = archive(&state, "chart-shapes").await;
        let mut payload = observation_payload();
        payload.metric_id = Some(metric.into());
        payload.raw_name = name.into();
        payload.raw_result = result.into();
        payload.raw_unit = unit.map(str::to_owned);
        payload.sampled_at = Some(format!("2026-08-05T10:{index:02}:00Z"));
        state
            .database
            .submit_report_changes(
                &user.user_id,
                ReportSubmission {
                    report_id,
                    expected_revision: 1,
                    context: Some(
                        serde_json::from_value(json!({"source_name":"Same Lab"})).unwrap(),
                    ),
                    observations: vec![ReportResultChange {
                        observation_id: None,
                        expected_revision: None,
                        status: ReportResultStatus::Submitted,
                        payload,
                    }],
                },
            )
            .await
            .unwrap();
    }
    let get = |id: &str| {
        history(
            &state.database,
            &user.user_id,
            HistoryRequest {
                metric_id: id.to_owned(),
            },
        )
    };
    let numeric = get("ldl_cholesterol").await.unwrap();
    assert_eq!(numeric["points"].as_array().unwrap().len(), 1);
    assert_eq!(numeric["records"].as_array().unwrap().len(), 3);
    let bucket = &numeric["chart_ranges"]["D"][0];
    assert_eq!(bucket["unit"], "mg/dL");
    assert_eq!(bucket["min"], 100.0);
    assert_eq!(bucket["max"], 200.0);
    assert_eq!(bucket["count"], 2);
    assert!(bucket["mean"].is_null());
    assert_eq!(numeric["chart_unit_conflict"], json!([]));

    let ordered = get("urine_bacteria").await.unwrap();
    let chart = &ordered["result_charts"][0];
    assert_eq!(chart["kind"], "ordinal");
    assert_eq!(chart["unit"], "/hpf");
    assert_eq!(ordered["records"][0]["unit"], "/HPF");
    assert_eq!(chart["ranges"]["scale_labels"], json!(["None", "Few"]));
    assert_eq!(chart["ranges"]["D"][0]["latest_min"], 0.0);
    assert_eq!(chart["ranges"]["D"][0]["latest_max"], 1.0);

    let qualitative = get("urine_ketones").await.unwrap();
    let chart = &qualitative["result_charts"][0];
    assert_eq!(chart["kind"], "binary");
    assert_eq!(chart["unit"], "mg/dL");
    let buckets = chart["ranges"]["D"].as_array().unwrap();
    assert_eq!(buckets.len(), 2);
    assert_eq!(
        (buckets[0]["min"].as_f64(), buckets[0]["max"].as_f64()),
        (Some(0.0), Some(0.0))
    );
    assert_eq!(
        (buckets[1]["min"].as_f64(), buckets[1]["max"].as_f64()),
        (Some(1.0), Some(1.0))
    );
    let records = qualitative["records"].as_array().unwrap();
    assert!(
        records
            .iter()
            .any(|record| record["display_value"] == "Positive" && record["unit"].is_null())
    );
    assert!(
        records
            .iter()
            .any(|record| record["display_value"] == "Negative" && record["unit"] == "mg/dL")
    );
    assert_eq!(qualitative["chart_unit_conflict"], json!([]));
}
