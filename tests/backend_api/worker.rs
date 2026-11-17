use crate::{add_user, archive, fixture};
use longlivehealth::reports::ReviewStatus;

#[tokio::test]
async fn new_confirmed_metric_and_loinc_are_used_in_the_next_report_rescan() {
    use longlivehealth::{
        metric_history::{HistoryRequest, history},
        reports::{NewMetric, ReviewItem, ReviewRequest, UnmappedChoice},
    };
    use serde_json::{Value, json};
    use std::sync::{Arc, Mutex};

    let (_directory, mut state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let received = Arc::new(Mutex::new(Vec::<Value>::new()));
    let requests = received.clone();
    let provider = axum::Router::new().route(
        "/v1/chat/completions",
        axum::routing::post(move |axum::Json(request): axum::Json<Value>| {
            let requests = requests.clone();
            async move {
                let evidence: Value = serde_json::from_str(
                    request["messages"][1]["content"][0]["text"]
                        .as_str()
                        .unwrap(),
                )
                .unwrap();
                let metric = evidence["laboratory_metric_catalog"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|entry| entry[1] == "pH of Urine")
                    .cloned();
                if let Some(metric) = metric.as_ref() {
                    assert_eq!(
                        evidence["browse_metric_index"][metric[1].as_str().unwrap()],
                        json!(["50560-2"])
                    );
                    assert!(
                        request["messages"][1]["content"][1]["image_url"]["url"]
                            .as_str()
                            .unwrap()
                            .starts_with("data:image/png;base64,")
                    );
                }
                let mut requests = requests.lock().unwrap();
                let first = requests.is_empty();
                requests.push(metric.clone().unwrap_or(Value::Null));
                let date = if first { "2018-06-01" } else { "2019-06-01" };
                let mut row = crate::observation_payload();
                row.raw_name = if first {
                    "pH (50560-2)"
                } else {
                    "Urine pH (50560-2)"
                }
                .into();
                row.raw_result = if first { "6.0" } else { "6.5" }.into();
                row.raw_unit = None;
                row.sampled_at = Some(date.into());
                row.source.quote = format!("{} {} {date}", row.raw_name, row.raw_result);
                row.metric_id = None;
                row.ocr_metric_candidate = metric
                    .as_ref()
                    .and_then(|entry| entry[0].as_str())
                    .map(str::to_owned);
                row.ocr_new_metric = metric.is_none().then(|| NewMetric {
                    name: "pH of Urine".into(),
                    standard_unit: "".into(),
                });
                axum::Json(super::provider::completion(json!({
                    "observations": [row], "warnings": [], "source_name": "Fixture Lab",
                })))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    Arc::make_mut(&mut state.config).llm.base_url =
        format!("http://{}/v1", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, provider).await.unwrap() });
    let app = longlivehealth::app::create_application(state.clone());
    let token = crate::token(&state, "alice").await;
    let mut reports = Vec::new();
    for line in ["pH 6.0 2018-06-01", "Urine pH 6.5 2019-06-01"] {
        let (status, uploaded) = crate::upload(
            &app,
            &token,
            &uuid::Uuid::new_v4().to_string(),
            &super::documents::text_pdf(&[&[line]]),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        reports.push(uploaded["file"]["file_id"].as_str().unwrap().to_owned());
        assert!(longlivehealth::worker::extract_next(&state).await.unwrap());
    }
    assert_ne!(reports[0], reports[1]);
    assert_eq!(*received.lock().unwrap(), [Value::Null, Value::Null]);

    let change = |report_id: &str, document: &Value, status: ReviewStatus, payload| {
        let row = &document["observations"][0];
        state.database.review(
            &user.user_id,
            ReviewRequest {
                report_id: report_id.into(),
                expected_revision: document["report"]["revision"].as_i64().unwrap(),
                context: None,
                observations: vec![ReviewItem {
                    observation_id: row["observation_id"].as_str().map(str::to_owned),
                    expected_revision: row["revision"].as_i64(),
                    status,
                    payload,
                }],
            },
        )
    };
    let first = state
        .database
        .report(&user.user_id, &reports[0])
        .await
        .unwrap();
    assert_eq!(first["observations"][0]["status"], "pending");
    assert!(first["observations"][0]["resolved_metric_id"].is_null());
    let mut payload: longlivehealth::reports::ObservationPayload =
        serde_json::from_value(first["observations"][0]["payload"].clone()).unwrap();
    assert!(
        change(
            &reports[0],
            &first,
            ReviewStatus::Confirmed,
            payload.clone()
        )
        .await
        .is_err()
    );
    payload.unmapped_choice = Some(UnmappedChoice::New(payload.ocr_new_metric.clone().unwrap()));
    let saved = change(&reports[0], &first, ReviewStatus::Pending, payload.clone())
        .await
        .unwrap();
    assert!(
        state
            .database
            .custom_metrics(&user.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    let confirmed = change(&reports[0], &saved, ReviewStatus::Confirmed, payload)
        .await
        .unwrap();
    let id = confirmed["created_metric"]["metric_id"].as_str().unwrap();
    assert!(id.starts_with("custom:"));
    let revision = state
        .database
        .report_summary(&user.user_id, &reports[1])
        .await
        .unwrap()
        .revision;
    state
        .database
        .rescan_report(&user.user_id, &reports[1], revision)
        .await
        .unwrap();
    assert!(longlivehealth::worker::extract_next(&state).await.unwrap());
    let entry = received.lock().unwrap()[2].clone();
    assert_eq!(entry[0], id);
    assert_eq!(entry[2], "1");
    assert_eq!(entry[4], json!(["50560-2"]));
    let second = state
        .database
        .report(&user.user_id, &reports[1])
        .await
        .unwrap();
    assert_eq!(second["observations"][0]["status"], "pending");
    assert_eq!(
        second["observations"][0]["payload"]["ocr_metric_candidate"],
        id
    );
    assert!(second["observations"][0]["payload"]["metric_id"].is_null());
    assert_eq!(second["observations"][0]["resolved_metric_id"], id);
    let payload = serde_json::from_value(second["observations"][0]["payload"].clone()).unwrap();
    let after = change(&reports[1], &second, ReviewStatus::Confirmed, payload)
        .await
        .unwrap();
    assert_eq!(after["observations"][0]["payload"]["metric_id"], id);
    let history = history(
        &state.database,
        &user.user_id,
        HistoryRequest {
            metric_id: id.into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(history["records"].as_array().unwrap().len(), 2);
    for (point, report) in history["points"].as_array().unwrap().iter().zip(&reports) {
        assert_eq!(point["report_id"], *report);
    }
    assert_eq!(
        state
            .database
            .custom_metrics(&user.user_id)
            .await
            .unwrap()
            .len(),
        1
    );
    server.abort();
}

#[tokio::test]
async fn retry_preserves_reviewed_rows_but_rescan_replaces_them_with_fresh_ocr() {
    use longlivehealth::reports::{ReviewItem, ReviewRequest};
    use serde_json::{Value, json};
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    use std::sync::atomic::{AtomicUsize, Ordering};

    let (directory, mut state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let report = archive(&state, "alice").await;
    let calls = std::sync::Arc::new(AtomicUsize::new(0));
    let router = axum::Router::new().route(
        "/v1/chat/completions",
        axum::routing::post(move |_: axum::Json<Value>| {
            let replay = calls.fetch_add(1, Ordering::SeqCst) > 0;
            async move {
                let observations: Vec<_> = [
                    "Review fixture A",
                    "Review fixture B",
                    "Review fixture C",
                    "Review fixture D",
                ]
                .into_iter()
                .take(if replay { 4 } else { 3 })
                .map(|name| {
                    let mut row = crate::observation_payload();
                    row.raw_name = name.into();
                    row.source.quote = if replay {
                        format!("{name} result")
                    } else {
                        name.into()
                    };
                    row.ocr_mapping_note = Some(
                        if replay {
                            "Refreshed note"
                        } else {
                            "Original note"
                        }
                        .into(),
                    );
                    row
                })
                .collect();
                axum::Json(super::provider::completion(
                    json!({"observations":observations,"warnings":[]}),
                ))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut config = (*state.config).clone();
    config.llm.base_url = format!("http://{}/v1", listener.local_addr().unwrap());
    state.config = std::sync::Arc::new(config);
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    assert!(longlivehealth::worker::extract_next(&state).await.unwrap());
    let mut original = state
        .database
        .observations(&user.user_id, &report)
        .await
        .unwrap();
    original.sort_by_key(|row| row.payload.raw_name.clone());
    let revision = state
        .database
        .report_summary(&user.user_id, &report)
        .await
        .unwrap()
        .revision;
    let edits = [
        (&original[1], ReviewStatus::Pending),
        (&original[2], ReviewStatus::Confirmed),
    ];
    state
        .database
        .review(
            &user.user_id,
            ReviewRequest {
                report_id: report.clone(),
                expected_revision: revision,
                context: None,
                observations: edits
                    .into_iter()
                    .map(|(row, status)| {
                        let mut payload = row.payload.clone();
                        payload.notes = Some("Reviewer annotation".into());
                        if status == ReviewStatus::Confirmed {
                            payload.metric_id = Some("ldl_cholesterol".into());
                        }
                        ReviewItem {
                            observation_id: Some(row.observation_id.clone()),
                            expected_revision: Some(row.revision),
                            status,
                            payload,
                        }
                    })
                    .collect(),
            },
        )
        .await
        .unwrap();
    let mut db = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(directory.path().join("database.sqlite")),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE jobs SET status='queued' WHERE file_id=?")
        .bind(&report)
        .execute(&mut db)
        .await
        .unwrap();
    assert!(longlivehealth::worker::extract_next(&state).await.unwrap());
    let mut rows = state
        .database
        .observations(&user.user_id, &report)
        .await
        .unwrap();
    rows.sort_by_key(|row| row.payload.raw_name.clone());
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].observation_id, original[0].observation_id);
    assert_eq!(
        rows[0].payload.source.quote,
        original[0].payload.source.quote
    );
    assert_eq!(
        rows[0].payload.ocr_mapping_note.as_deref(),
        Some("Refreshed note")
    );
    for (row, expected_status) in rows[1..=2].iter().zip(["pending", "confirmed"]) {
        assert_eq!(row.status, expected_status);
        assert_eq!(row.payload.notes.as_deref(), Some("Reviewer annotation"));
        assert_eq!(
            row.payload.ocr_mapping_note.as_deref(),
            Some("Original note")
        );
    }
    assert_eq!(rows[3].status, "pending");
    assert!(rows[3].payload.metric_id.is_none());
    let revision = state
        .database
        .report_summary(&user.user_id, &report)
        .await
        .unwrap()
        .revision;
    state
        .database
        .rescan_report(&user.user_id, &report, revision)
        .await
        .unwrap();
    assert!(longlivehealth::worker::extract_next(&state).await.unwrap());
    let rescanned = state
        .database
        .observations(&user.user_id, &report)
        .await
        .unwrap();
    assert_eq!(rescanned.len(), rows.len());
    assert!(rescanned.iter().all(|row| {
        row.status == "pending"
            && row.payload.notes.is_none()
            && row.payload.metric_id.is_none()
            && row.payload.ocr_mapping_note.as_deref() == Some("Refreshed note")
            && rows
                .iter()
                .all(|before| before.observation_id != row.observation_id)
    }));
    server.abort();
}

#[tokio::test]
async fn rescan_submits_every_original_page_and_publishes_only_after_a_complete_run() {
    use serde_json::{Value, json};
    use std::sync::{Arc, Mutex};

    let (_directory, mut state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let calls = Arc::new(Mutex::new(Vec::new()));
    let received = calls.clone();
    let provider = axum::Router::new().route(
        "/v1/chat/completions",
        axum::routing::post(move |axum::Json(request): axum::Json<Value>| {
            let received = received.clone();
            async move {
                let evidence: Value = serde_json::from_str(
                    request["messages"][1]["content"][0]["text"]
                        .as_str()
                        .unwrap(),
                )
                .unwrap();
                let page = evidence["page"].as_i64().unwrap();
                assert!(
                    request["messages"][1]["content"][1]["image_url"]["url"]
                        .as_str()
                        .unwrap()
                        .starts_with("data:image/png;base64,")
                );
                let mut calls = received.lock().unwrap();
                calls.push(page);
                if calls.len() == 4 {
                    return axum::Json(super::provider::completion(
                        json!({"observations": [{"raw_name": "Glucose"}], "warnings": []}),
                    ));
                }
                let mut row = crate::observation_payload();
                row.raw_name = format!("Glucose page {page}");
                row.source.page = page;
                row.raw_result = calls.len().to_string();
                row.source.quote = format!("Glucose page {page}");
                axum::Json(super::provider::completion(
                    json!({"observations": [row], "warnings": []}),
                ))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    Arc::make_mut(&mut state.config).llm.base_url =
        format!("http://{}/v1", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, provider).await.unwrap() });
    let app = longlivehealth::app::create_application(state.clone());
    let token = crate::token(&state, "alice").await;
    let (_, response) = crate::upload(
        &app,
        &token,
        &uuid::Uuid::new_v4().to_string(),
        &super::documents::text_pdf(&[&["Glucose page 1"], &["Glucose page 2"]]),
    )
    .await;
    let report = response["file"]["file_id"].as_str().unwrap();
    let owner = &user.user_id;
    let run = || longlivehealth::worker::extract_next(&state);
    assert!(run().await.unwrap());
    assert_eq!(
        state
            .database
            .observations(owner, report)
            .await
            .unwrap()
            .len(),
        2
    );
    for (expected_status, expected_count) in [("failed", 0), ("succeeded", 2)] {
        let revision = state
            .database
            .report_summary(owner, report)
            .await
            .unwrap()
            .revision;
        state
            .database
            .rescan_report(owner, report, revision)
            .await
            .unwrap();
        assert!(
            state
                .database
                .observations(owner, report)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(run().await.unwrap());
        assert_eq!(
            state
                .database
                .job_by_file(owner, report)
                .await
                .unwrap()
                .status,
            expected_status
        );
        let rows = state.database.observations(owner, report).await.unwrap();
        assert_eq!(rows.len(), expected_count);
        for row in rows {
            assert_eq!(row.status, "pending");
            assert_eq!(
                row.payload.raw_result,
                (4 + row.payload.source.page).to_string()
            );
        }
    }
    assert_eq!(*calls.lock().unwrap(), [1, 2, 1, 2, 1, 2]);
    server.abort();
}

#[tokio::test]
async fn actual_http_extraction_creates_pending_candidates_with_provenance() {
    let (_directory, mut state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let (address, server) = super::provider::mock().await;
    let mut config = (*state.config).clone();
    config.llm.base_url = address.clone();
    state.config = std::sync::Arc::new(config);
    let report = archive(&state, "alice").await;
    assert!(longlivehealth::worker::extract_next(&state).await.unwrap());
    let rows = state
        .database
        .observations(&user.user_id, &report)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, "pending");
    assert_eq!(rows[0].payload.source.page, 1);
    assert!(rows[0].payload.metric_id.is_none());
    assert!(
        state
            .database
            .confirmed(&user.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    let archived = state.database.report(&user.user_id, &report).await.unwrap();
    assert_eq!(archived["extraction_outputs"].as_array().unwrap().len(), 1);
    assert_eq!(
        state
            .database
            .job_by_file(&user.user_id, &report)
            .await
            .unwrap()
            .status,
        "succeeded"
    );
    server.abort();
}

#[tokio::test]
async fn pdf_rendering_and_independent_ocr_service_create_reviewable_rows() {
    let (_directory, mut state) = fixture().await;
    let user = add_user(&state, "alice").await;
    use base64::{Engine, engine::general_purpose::STANDARD};
    use longlivehealth::health::{ConnectionRequest, HealthRecordInput, SyncRequest};
    use serde_json::json;
    let mut alice_metric = None;
    let bob = add_user(&state, "bob").await;
    let token = crate::token(&state, "alice").await;
    for (account, label) in [
        (&user, "Synthetic clinical marker"),
        (&bob, "Other account marker"),
    ] {
        let connection = state
            .database
            .create_connection(
                &account.user_id,
                ConnectionRequest {
                    platform: "apple_health".into(),
                    installation_id: uuid::Uuid::new_v4().to_string(),
                },
            )
            .await
            .unwrap();
        let record_type = "HKClinicalTypeIdentifierLabResultRecord";
        let fhir = json!({
            "resourceType":"Observation", "status":"final",
            "effectiveDateTime":"2026-08-03T00:00:00Z",
            "code":{"text":label,"coding":[if account.user_id == user.user_id {
                json!({"system":"http://loinc.org","code":"1968-7"})
            } else {
                json!({"system":"https://example.org/lab","code":"synthetic"})
            }]},
            "valueQuantity":{"value":2,"code":"mg/L","system":"http://unitsofmeasure.org"}
        });
        state
            .database
            .sync_health(
                &account.user_id,
                SyncRequest {
                    connection_id: connection.connection_id,
                    batch_id: uuid::Uuid::new_v4().to_string(),
                    record_type: record_type.into(),
                    coverage_status: "observed".into(),
                    records: vec![HealthRecordInput {
                        record_id: "sample".into(),
                        source_id: "clinical-provider".into(),
                        record_type: record_type.into(),
                        start_at: 1785715200,
                        end_at: 1785715200,
                        version: 1,
                        deleted: false,
                        payload: json!({"fhir":{"data_base64":STANDARD.encode(fhir.to_string())}}),
                    }],
                },
            )
            .await
            .unwrap();
        let browse = longlivehealth::metric_history::browse(&state.database, &account.user_id)
            .await
            .unwrap();
        if account.user_id == user.user_id {
            alice_metric = Some(browse["metrics"][0]["metric_id"].clone());
        }
    }
    let alice_metric = alice_metric.unwrap();
    use std::sync::atomic::{AtomicUsize, Ordering};
    let calls = std::sync::Arc::new(AtomicUsize::new(0));
    let captured_id = alice_metric.clone();
    let router = axum::Router::new().route(
        "/v1/chat/completions",
        axum::routing::post(move |axum::Json(request): axum::Json<serde_json::Value>| {
            let alice_metric = captured_id.clone();
            let first = calls.fetch_add(1, Ordering::SeqCst) == 0;
            let (name, aliases, codes) = if first {
                ("Direct bilirubin", vec![], vec!["1968-7"])
            } else {
                (
                    "Renamed direct bilirubin",
                    vec!["Direct bilirubin"],
                    vec!["1968-7", "35191-6"],
                )
            };
            async move {
                let input = &request["messages"][1]["content"];
                let evidence: serde_json::Value =
                    serde_json::from_str(input[0]["text"].as_str().unwrap()).unwrap();
                let catalog = evidence["laboratory_metric_catalog"].as_array().unwrap();
                assert!(catalog.contains(&json!([alice_metric, name, "mg/L", aliases, codes])));
                assert!(
                    !catalog
                        .iter()
                        .any(|entry| entry[1] == "Other account marker")
                );
                assert_eq!(evidence["page"], 1);
                assert!(evidence["text_layer"].is_object());
                assert!(
                    evidence["laboratory_metric_catalog"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|entry| entry[0] == "urine_appearance"
                            && entry[1] == "Appearance - urine"
                            && entry[2] == "1")
                );
                assert!(
                    input[1]["image_url"]["url"]
                        .as_str()
                        .unwrap()
                        .starts_with("data:image/png;base64,")
                );
                let mut observation = crate::observation_payload();
                observation.raw_name = "Appearance".into();
                observation.raw_result = "Clear".into();
                observation.raw_unit = None;
                observation.source.quote = "Appearance Clear".into();
                observation.ocr_section_heading = Some("Urinalysis".into());
                observation.ocr_metric_candidate = Some("urine_appearance".into());
                axum::Json(super::provider::completion(
                    serde_json::json!({"observations":[observation],"warnings":[]}),
                ))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}/v1", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = (*state.config).clone();
    config.llm.base_url = address.clone();
    state.config = std::sync::Arc::new(config);
    let app = longlivehealth::app::create_application(state.clone());
    let (status, result) = crate::upload(
        &app,
        &token,
        &uuid::Uuid::new_v4().to_string(),
        &super::files::synthetic_pdf(),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(result["file"]["content_type"], "application/pdf");
    longlivehealth::worker::extract_next(&state).await.unwrap();
    let report = result["file"]["file_id"].as_str().unwrap();
    let rows = state
        .database
        .observations(&user.user_id, report)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, "pending");
    assert_eq!(
        rows[0].resolved_metric_id.as_deref(),
        Some("urine_appearance")
    );
    assert!(rows[0].payload.metric_id.is_none());
    assert_eq!(rows[0].payload.raw_name, "Appearance");
    assert_eq!(rows[0].payload.raw_result, "Clear");
    let archived = state.database.report(&user.user_id, report).await.unwrap();
    assert_eq!(archived["extraction_outputs"].as_array().unwrap().len(), 1);
    let mut manual = crate::observation_payload();
    manual.raw_name = "Direct bilirubin (35191-6)".into();
    manual.raw_unit = Some("mg/L".into());
    manual.metric_id = alice_metric.as_str().map(str::to_owned);
    let revision = state
        .database
        .report_summary(&user.user_id, report)
        .await
        .unwrap()
        .revision;
    state
        .database
        .review(
            &user.user_id,
            longlivehealth::reports::ReviewRequest {
                report_id: report.into(),
                expected_revision: revision,
                context: None,
                observations: vec![longlivehealth::reports::ReviewItem {
                    observation_id: None,
                    expected_revision: None,
                    status: ReviewStatus::Confirmed,
                    payload: manual,
                }],
            },
        )
        .await
        .unwrap();
    let (status, details) = crate::request(
        &app,
        "/api/v1/metrics/rename",
        Some(&token),
        json!({
            "metric_id": alice_metric, "name": "Renamed direct bilirubin"
        }),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{details}");
    let revision = state
        .database
        .report_summary(&user.user_id, report)
        .await
        .unwrap()
        .revision;
    state
        .database
        .rescan_report(&user.user_id, report, revision)
        .await
        .unwrap();
    assert!(longlivehealth::worker::extract_next(&state).await.unwrap());
    let rows = state
        .database
        .observations(&user.user_id, report)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows.iter().all(|row| row.status == "pending"));
    assert_eq!(
        state
            .database
            .job_by_file(&user.user_id, report)
            .await
            .unwrap()
            .status,
        "succeeded"
    );
    server.abort();
}

#[tokio::test]
async fn malformed_ocr_is_archived_before_validation_and_obeys_owner_and_deletion() {
    use longlivehealth::reports::ExtractionOutputRequest;
    use serde_json::json;
    let (_directory, mut state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let other = add_user(&state, "bob").await;
    let body = json!({"choices":[{"message":{"content":"An original row with <5, but not JSON"},"finish_reason":"stop"}]}).to_string();
    let response = body.clone();
    let router = axum::Router::new().route(
        "/v1/chat/completions",
        axum::routing::post(move || {
            let response = response.clone();
            async move { (axum::http::StatusCode::OK, response) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = (*state.config).clone();
    config.llm.base_url = format!("http://{address}/v1");
    state.config = std::sync::Arc::new(config);
    let report_id = archive(&state, "alice").await;
    longlivehealth::worker::extract_next(&state).await.unwrap();
    assert_eq!(
        state
            .database
            .job_by_file(&user.user_id, &report_id)
            .await
            .unwrap()
            .status,
        "failed"
    );
    let report = state
        .database
        .report(&user.user_id, &report_id)
        .await
        .unwrap();
    let output = &report["extraction_outputs"][0];
    let request = || ExtractionOutputRequest {
        report_id: report_id.clone(),
        run_id: output["run_id"].as_str().unwrap().into(),
        page: 1,
        stage: "ocr".into(),
    };
    let original = state
        .database
        .extraction_output(&user.user_id, request())
        .await
        .unwrap();
    let envelope: serde_json::Value =
        serde_json::from_str(original["response_body"].as_str().unwrap()).unwrap();
    assert_eq!(envelope["raw_response_body"], body);
    assert_eq!(original["status_code"], 422);
    assert_eq!(original["content"], "An original row with <5, but not JSON");
    assert!(
        state
            .database
            .extraction_output(&other.user_id, request())
            .await
            .is_err()
    );
    state
        .database
        .delete_report(
            &user.user_id,
            &report_id,
            report["report"]["revision"].as_i64().unwrap(),
        )
        .await
        .unwrap();
    assert!(
        state
            .database
            .extraction_output(&user.user_id, request())
            .await
            .is_err()
    );
    server.abort();
}

#[tokio::test]
async fn deleted_report_rejects_late_ocr_and_clears_temporary_images() {
    use std::sync::Arc;
    let (_directory, mut state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let started = Arc::new(tokio::sync::Notify::new());
    let released = Arc::new(tokio::sync::Notify::new());
    let started_in = started.clone();
    let released_in = released.clone();
    let app = axum::Router::new().route("/v1/chat/completions",axum::routing::post(move || {
        let started = started_in.clone(); let released = released_in.clone();
        async move {
            started.notify_one(); released.notified().await;
            axum::Json(serde_json::json!({"choices":[{"message":{"content":serde_json::json!({"observations":[crate::observation_payload()],"warnings":[]}).to_string()}}]}))
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let mut config = (*state.config).clone();
    config.llm.base_url = format!("http://{address}/v1");
    state.config = Arc::new(config);
    let report = archive(&state, "alice").await;
    let worker_state = state.clone();
    let worker =
        tokio::spawn(async move { longlivehealth::worker::extract_next(&worker_state).await });
    tokio::time::timeout(std::time::Duration::from_secs(10), started.notified())
        .await
        .unwrap();
    state
        .database
        .delete_report(&user.user_id, &report, 1)
        .await
        .unwrap();
    released.notify_one();
    assert!(worker.await.unwrap().is_err());
    longlivehealth::maintenance::cleanup(&state).await.unwrap();
    assert!(
        state
            .database
            .observations(&user.user_id, &report)
            .await
            .is_err()
    );
    server.abort();
}

#[tokio::test]
async fn long_extraction_renews_lease_and_releases_renewal_on_completion() {
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    use std::{sync::Arc, time::Duration};
    const READ_LEASE: &str = "SELECT lease_until FROM jobs WHERE job_id = ?";
    const WRITE_REVISION: &str =
        "UPDATE users SET data_revision = data_revision + 1 WHERE user_id = ?";
    let (_directory, mut state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let started = Arc::new(tokio::sync::Notify::new());
    let released = Arc::new(tokio::sync::Notify::new());
    let start_in = started.clone();
    let release_in = released.clone();
    let router = axum::Router::new().route(
        "/v1/chat/completions",
        axum::routing::post(move || {
            let started = start_in.clone();
            let released = release_in.clone();
            async move {
                started.notify_one();
                released.notified().await;
                let mut observation = crate::observation_payload();
                observation.metric_id = None;
                let page = serde_json::json!({"observations":[observation],"warnings":[]});
                axum::Json(super::provider::completion(serde_json::json!({
                    "model":"synthetic-model", "engine":"strata", "prompt_version":"laboratory-page-v2",
                    "content":page.to_string(), "raw_response_body":"synthetic native output",
                    "observations":page["observations"], "warnings":[]
                })))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = (*state.config).clone();
    config.llm.base_url = format!("http://{address}/v1");
    config.jobs.lease_seconds = 3;
    state.config = Arc::new(config);
    let report_id = archive(&state, "alice").await;
    let worker_state = state.clone();
    let worker =
        tokio::spawn(async move { longlivehealth::worker::extract_next(&worker_state).await });
    tokio::time::timeout(Duration::from_secs(10), started.notified())
        .await
        .unwrap();
    let original = state
        .database
        .job_by_file(&user.user_id, &report_id)
        .await
        .unwrap();
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(state.config.data_dir.join("database.sqlite"))
            .busy_timeout(Duration::from_secs(5)),
    )
    .await
    .unwrap();
    let initial_deadline: i64 = sqlx::query_scalar(READ_LEASE)
        .bind(&original.job_id)
        .fetch_one(&mut connection)
        .await
        .unwrap();
    // Longer than the initial lease, with database writes while HTTP inference is pending.
    for _ in 0..4 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let mut transaction = connection.begin_with("BEGIN IMMEDIATE").await.unwrap();
        sqlx::query(WRITE_REVISION)
            .bind(&user.user_id)
            .execute(&mut *transaction)
            .await
            .unwrap();
        transaction.commit().await.unwrap();
    }
    let renewed = state
        .database
        .job_by_file(&user.user_id, &report_id)
        .await
        .unwrap();
    let renewed_deadline: i64 = sqlx::query_scalar(READ_LEASE)
        .bind(&original.job_id)
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert!(renewed_deadline > initial_deadline);
    assert_eq!(renewed.status, "running");
    released.notify_one();
    assert!(
        tokio::time::timeout(Duration::from_secs(10), worker)
            .await
            .unwrap()
            .unwrap()
            .unwrap()
    );
    let completed = state
        .database
        .job_by_file(&user.user_id, &report_id)
        .await
        .unwrap();
    assert_eq!(completed.status, "succeeded");
    let cleared: Option<i64> = sqlx::query_scalar(READ_LEASE)
        .bind(&original.job_id)
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert!(cleared.is_none());
    assert_eq!(
        state
            .database
            .observations(&user.user_id, &report_id)
            .await
            .unwrap()
            .len(),
        1
    );
    server.abort();
}

#[tokio::test]
async fn complete_ocr_with_review_warnings_succeeds_without_confirming_or_losing_evidence() {
    let (_directory, mut state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let mut observation = crate::observation_payload();
    observation.metric_id = None;
    let page = serde_json::json!({"observations":[observation],"warnings":["Text layer contains only the footer; values were read from the image."]});
    let output = serde_json::json!({"model":"synthetic","engine":"strata","prompt_version":"laboratory-page-v2",
        "content":page.to_string(), "raw_response_body":"synthetic full native response",
        "observations":page["observations"], "warnings":page["warnings"]});
    let router = axum::Router::new().route(
        "/v1/chat/completions",
        axum::routing::post(move || {
            let output = output.clone();
            async move { axum::Json(super::provider::completion(output)) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = (*state.config).clone();
    config.llm.base_url = format!("http://{address}/v1");
    state.config = std::sync::Arc::new(config);
    let report_id = archive(&state, "alice").await;
    longlivehealth::worker::extract_next(&state).await.unwrap();
    assert_eq!(
        state
            .database
            .job_by_file(&user.user_id, &report_id)
            .await
            .unwrap()
            .status,
        "succeeded"
    );
    let report = state
        .database
        .report(&user.user_id, &report_id)
        .await
        .unwrap();
    assert_eq!(report["pages"][0]["status"], "needs_review");
    assert_eq!(report["observations"][0]["status"], "pending");
    assert!(report["observations"][0]["payload"]["metric_id"].is_null());
    assert!(
        report["pages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("footer")
    );
    assert!(
        state
            .database
            .confirmed(&user.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    server.abort();
}

#[tokio::test]
async fn required_ocr_outage_fails_retryably_without_losing_the_original() {
    let (_directory, mut state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let mut config = (*state.config).clone();
    config.llm.base_url = format!("http://{address}/v1");
    state.config = std::sync::Arc::new(config);
    let report_id = archive(&state, "alice").await;
    let job = state
        .database
        .job_by_file(&user.user_id, &report_id)
        .await
        .unwrap();
    assert_eq!(job.status, "queued");
    assert!(job.error_code.is_none());
    assert!(longlivehealth::worker::extract_next(&state).await.unwrap());
    let failed = state
        .database
        .job(&user.user_id, &job.job_id)
        .await
        .unwrap();
    assert_eq!(failed.status, "failed");
    assert_eq!(failed.error_code.as_deref(), Some("extraction_failed"));
    let original = state
        .database
        .file(&user.user_id, &report_id)
        .await
        .unwrap();
    assert_eq!(
        tokio::fs::read(state.config.data_dir.join(original.relative_path))
            .await
            .unwrap(),
        crate::png()
    );
    let app = longlivehealth::app::create_application(state.clone());
    let (status, capabilities) =
        crate::request(&app, "/api/v1/server/status", None, serde_json::json!({})).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(capabilities["capabilities"]["document_extraction"], true);
    assert_eq!(
        state
            .database
            .retry_job(&user.user_id, &job.job_id)
            .await
            .unwrap()
            .status,
        "queued"
    );
}

#[tokio::test]
async fn multipage_extraction_survives_source_relocation_and_old_path_cleanup() {
    use serde_json::json;
    use std::sync::Arc;
    let (_directory, mut state) = fixture().await;
    let user = add_user(&state, "relocation").await;
    let old_path = Arc::new(tokio::sync::Mutex::new(None::<std::path::PathBuf>));
    let path_in = old_path.clone();
    let cleanup_state = state.clone();
    let router=axum::Router::new().route("/v1/chat/completions",axum::routing::post(move |axum::Json(request):axum::Json<serde_json::Value>| {
        let path=path_in.clone(); let state=cleanup_state.clone();
        async move {
            let page=super::provider::request_page(&request);
            if page==2 {
                longlivehealth::maintenance::cleanup(&state).await.unwrap();
                assert!(!path.lock().await.as_ref().unwrap().exists());
            }
            let mut observation=crate::observation_payload();observation.metric_id=None;observation.source.page=page;
            axum::Json(super::provider::completion(json!({"report_title":"Three page report","source_name":"Synthetic Lab","model":"fixture","engine":"fixture","prompt_version":"laboratory-page-v3","content":"synthetic page","raw_response_body":"synthetic","observations":[observation],"warnings":[]})))
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let mut config = (*state.config).clone();
    config.llm.base_url = format!("http://{address}/v1");
    state.config = Arc::new(config);
    let app = longlivehealth::app::create_application(state.clone());
    let token = crate::token(&state, "relocation").await;
    let (status, result) = crate::upload(
        &app,
        &token,
        &uuid::Uuid::new_v4().to_string(),
        &super::documents::mixed_pdf(),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let report = result["file"]["file_id"].as_str().unwrap();
    let file = state.database.file(&user.user_id, report).await.unwrap();
    *old_path.lock().await = Some(state.config.data_dir.join(file.relative_path));
    assert!(longlivehealth::worker::extract_next(&state).await.unwrap());
    assert_eq!(
        state
            .database
            .job_by_file(&user.user_id, report)
            .await
            .unwrap()
            .status,
        "succeeded"
    );
    assert_eq!(
        state
            .database
            .observations(&user.user_id, report)
            .await
            .unwrap()
            .len(),
        3
    );
    server.abort();
}

#[tokio::test]
async fn ocr_workflow_fields_are_archived_but_never_become_candidates() {
    use serde_json::json;
    let (_directory, mut state) = fixture().await;
    let user = add_user(&state, "metadata-filter").await;
    let mut metadata = crate::observation_payload();
    metadata.raw_name = "Specimen Expiration Date".into();
    metadata.raw_result = "2026-10-24".into();
    let mut status = metadata.clone();
    status.raw_name = "Status Information".into();
    status.raw_result = "Final".into();
    let rows = vec![crate::observation_payload(), metadata, status];
    let router = axum::Router::new().route("/v1/chat/completions", axum::routing::post(move || {
        let rows = rows.clone();
        async move { axum::Json(super::provider::completion(json!({"model":"test","engine":"test","prompt_version":"test","report_title":null,"source_name":null,
            "content":serde_json::to_string(&rows).unwrap(),"raw_response_body":"original response","observations":rows,"warnings":[]}))) }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = (*state.config).clone();
    config.llm.base_url = format!("http://{address}/v1");
    state.config = std::sync::Arc::new(config);
    let report = archive(&state, "metadata-filter").await;
    assert!(longlivehealth::worker::extract_next(&state).await.unwrap());
    let rows = state
        .database
        .observations(&user.user_id, &report)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].payload.raw_name,
        crate::observation_payload().raw_name
    );
    assert_eq!(
        state
            .database
            .job_by_file(&user.user_id, &report)
            .await
            .unwrap()
            .status,
        "succeeded"
    );
    assert_eq!(
        state.database.report(&user.user_id, &report).await.unwrap()["extraction_outputs"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    server.abort();
}

#[test]
fn ocr_required_fields_accept_text_values_and_report_each_missing_field() {
    use serde_json::json;
    let page = json!({"observations":[
        {"raw_name":"Antibody", "sampled_at":"2026-07-30", "raw_result":"Negative"},
        {"raw_name":"Antigen", "sampled_at":"2026-07-30", "raw_result":"Positive"},
        {"raw_name":"ApoB", "sampled_at":null, "raw_result":"120"},
        {"raw_name":" ", "sampled_at":"2026-02-30", "raw_result":""}
    ]});
    let issues = longlivehealth::ocr::validation_issues(&page.to_string(), 3, false);
    assert_eq!(issues.len(), 4);
    assert_eq!(issues[0]["item_name"], "ApoB");
    assert_eq!(issues[0]["page"], 3);
    assert_eq!(issues[0]["field"], "collection_date");
    assert_eq!(issues[1]["field"], "name");
    assert_eq!(issues[2]["reason"], "invalid_date");
    assert_eq!(issues[3]["field"], "value");
}

#[tokio::test]
async fn missing_collection_date_fails_job_and_returns_item_reason() {
    use serde_json::json;
    let (_directory, mut state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let mut observation = crate::observation_payload();
    observation.sampled_at = None;
    let page = json!({"observations":[observation],"warnings":[]});
    let response = json!({"model":"qwen3.8-flash-next-iq3_s","engine":"strata","prompt_version":"laboratory-page-v4",
        "report_title":null,"source_name":null,"content":page.to_string(),"raw_response_body":"original", "observations":page["observations"],"warnings":[]});
    let router = axum::Router::new().route(
        "/v1/chat/completions",
        axum::routing::post(move || {
            let response = response.clone();
            async move { axum::Json(super::provider::completion(response)) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = (*state.config).clone();
    config.llm.base_url = format!("http://{address}/v1");
    state.config = std::sync::Arc::new(config);
    let report = archive(&state, "alice").await;
    longlivehealth::worker::extract_next(&state).await.unwrap();
    assert_eq!(
        state
            .database
            .job_by_file(&user.user_id, &report)
            .await
            .unwrap()
            .status,
        "failed"
    );
    let result = state.database.report(&user.user_id, &report).await.unwrap();
    assert_eq!(result["validation_issues"][0]["field"], "collection_date");
    assert_eq!(result["validation_issues"][0]["page"], 1);
    assert_eq!(result["observations"].as_array().unwrap().len(), 1);
    assert!(
        state
            .database
            .confirmed(&user.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    server.abort();
}

#[tokio::test]
async fn rescan_removes_only_numeric_overlaps_and_refreshes_ocr_again() {
    use longlivehealth::reports::{ReviewItem, ReviewRequest};
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let (_directory, mut state) = fixture().await;
    let owner = add_user(&state, "alice").await;
    let owner = &owner.user_id;
    let result = |name: &str, value: &str, date: &str, unit: &str| {
        let mut row = crate::observation_payload();
        row.raw_name = name.into();
        row.raw_result = value.into();
        row.sampled_at = Some(date.into());
        row.raw_unit = Some(unit.into());
        row.metric_id = None;
        row.source.quote = format!("{name} {value} {unit}");
        row
    };
    let ldl = "LDL Cholesterol (2089-1)";
    let hdl = "HDL Cholesterol (2085-9)";
    for (source, name) in [("Clinic A", ldl), ("clinic a", ldl), ("Clinic B", hdl)] {
        let report = archive(&state, "alice").await;
        state
            .database
            .review(
                owner,
                ReviewRequest {
                    report_id: report,
                    expected_revision: 1,
                    context: Some(serde_json::from_value(json!({"source_name": source})).unwrap()),
                    observations: vec![ReviewItem {
                        observation_id: None,
                        expected_revision: None,
                        status: ReviewStatus::Confirmed,
                        payload: result(name, "120.0", "2026-08-01", "mg/dL"),
                    }],
                },
            )
            .await
            .unwrap();
        let job = state
            .database
            .claim_job(2_000_000_000, 600, 3)
            .await
            .unwrap()
            .unwrap();
        state
            .database
            .finish_job(&job, 2_000_000_001, None)
            .await
            .unwrap();
    }
    let report = archive(&state, "alice").await;
    state
        .database
        .review(
            owner,
            ReviewRequest {
                report_id: report.clone(),
                expected_revision: 1,
                context: Some(serde_json::from_value(json!({"source_name":"clinic a"})).unwrap()),
                observations: vec![],
            },
        )
        .await
        .unwrap();

    let calls = std::sync::Arc::new(AtomicUsize::new(0));
    let api = axum::Router::new().route(
        "/v1/chat/completions",
        axum::routing::post(move || {
            let revision = calls.fetch_add(1, Ordering::SeqCst);
            async move {
                let mut refreshed = result("Glucose", "99", "2026-11-01", "mg/dL");
                refreshed.notes = Some(format!("OCR run {revision}"));
                axum::Json(super::provider::completion(json!({
                    "observations":[
                        result(ldl, "120", "2026-08-01", "mg/dL"),
                        result(ldl, "120.00", "2026-08-01", "mg/dL"),
                        result(ldl, "121", "2026-08-01", "mg/dL"),
                        result(ldl, "120", "2026-11-01", "mg/dL"),
                        result(ldl, "120", "2026-08-01", "mmol/L"),
                        result(hdl, "120.00", "2026-08-01", "mg/dL"),
                        refreshed
                    ],
                    "warnings":[]
                })))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut config = (*state.config).clone();
    config.llm.base_url = format!("http://{}/v1", listener.local_addr().unwrap());
    state.config = std::sync::Arc::new(config);
    let server = tokio::spawn(async move { axum::serve(listener, api).await.unwrap() });
    assert!(longlivehealth::worker::extract_next(&state).await.unwrap());
    let initial = state.database.observations(owner, &report).await.unwrap();
    assert_eq!(initial.len(), 7);
    let mut previous = initial
        .iter()
        .find(|row| row.payload.raw_name == "Glucose")
        .unwrap()
        .observation_id
        .clone();

    for iteration in 1..=2 {
        let revision = state
            .database
            .report_summary(owner, &report)
            .await
            .unwrap()
            .revision;
        state
            .database
            .rescan_report(owner, &report, revision)
            .await
            .unwrap();
        assert!(longlivehealth::worker::extract_next(&state).await.unwrap());
        let rows = state.database.observations(owner, &report).await.unwrap();
        assert_eq!(rows.len(), 5);
        assert!(rows.iter().all(|row| row.status == "pending"));
        assert!(rows.iter().any(|row| row.payload.raw_name == hdl));
        assert!(
            rows.iter()
                .any(|row| row.payload.sampled_at.as_deref() == Some("2026-11-01"))
        );
        assert!(
            rows.iter()
                .any(|row| row.payload.raw_unit.as_deref() == Some("mmol/L"))
        );
        let refreshed = rows
            .iter()
            .find(|row| row.payload.raw_name == "Glucose")
            .unwrap();
        assert_eq!(
            refreshed.payload.notes.as_deref(),
            Some(format!("OCR run {iteration}").as_str())
        );
        assert_ne!(refreshed.observation_id, previous);
        previous = refreshed.observation_id.clone();
    }
    assert_eq!(state.database.confirmed(owner).await.unwrap().len(), 2);
    server.abort();
}
