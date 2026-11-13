use crate::{add_user, archive, fixture};

#[tokio::test]
async fn extraction_replay_refreshes_untouched_drafts_and_preserves_reviewed_rows() {
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
    let edits = [(&original[1], "pending"), (&original[2], "confirmed")];
    state
        .database
        .review(
            &user.user_id,
            ReviewRequest {
                new_metric: None,
                report_id: report.clone(),
                expected_revision: revision,
                context: None,
                observations: edits
                    .into_iter()
                    .map(|(row, status)| {
                        let mut payload = row.payload.clone();
                        payload.notes = Some("Reviewer annotation".into());
                        ReviewItem {
                            observation_id: Some(row.observation_id.clone()),
                            expected_revision: Some(row.revision),
                            status: status.into(),
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
            "code":{"text":label,"coding":[{"system":"https://example.org/lab","code":"synthetic"}]},
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
    let router = axum::Router::new().route(
        "/v1/chat/completions",
        axum::routing::post(move |axum::Json(request): axum::Json<serde_json::Value>| {
            let alice_metric = alice_metric.clone();
            async move {
                let input = &request["messages"][1]["content"];
                let evidence: serde_json::Value =
                    serde_json::from_str(input[0]["text"].as_str().unwrap()).unwrap();
                let catalog = evidence["laboratory_metric_catalog"].as_array().unwrap();
                assert!(catalog.contains(&json!([
                    alice_metric,
                    "Synthetic clinical marker",
                    "mg/L",
                    []
                ])));
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
    let token = crate::token(&state, "alice").await;
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
