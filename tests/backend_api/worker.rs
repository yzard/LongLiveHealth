use crate::{add_user, archive, fixture};

#[tokio::test]
async fn actual_http_extraction_creates_pending_candidates_with_provenance() {
    let (_directory, mut state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let (address, server) = super::provider::mock().await;
    let mut config = (*state.config).clone();
    config.ocr.url = address.trim_end_matches("/v1").into();
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
    let (address, server) = super::provider::mock().await;
    let mut config = (*state.config).clone();
    config.ocr.url = address.trim_end_matches("/v1").into();
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
    assert_eq!(
        state
            .database
            .observations(&user.user_id, report)
            .await
            .unwrap()
            .len(),
        1
    );
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
    let body = json!({"model":"qwen3.8-27b-ninfer-nvfp4","engine":"ninfer","content":"An original row with <5, but not JSON","raw_response_body":"Complete native failure reply","error":{"code":"invalid_structured_output"}}).to_string();
    let response = body.clone();
    let router = axum::Router::new().route(
        "/api/v1/documents/extract",
        axum::routing::post(move || {
            let response = response.clone();
            async move { (axum::http::StatusCode::UNPROCESSABLE_ENTITY, response) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = (*state.config).clone();
    config.ocr.url = format!("http://{address}");
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
    assert_eq!(original["response_body"], body);
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
    let app = axum::Router::new().route("/api/v1/documents/extract",axum::routing::post(move || {
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
    config.ocr.url = format!("http://{address}");
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
        "/api/v1/documents/extract",
        axum::routing::post(move || {
            let started = start_in.clone();
            let released = release_in.clone();
            async move {
                started.notify_one();
                released.notified().await;
                let mut observation = crate::observation_payload();
                observation.metric_id = None;
                let page = serde_json::json!({"observations":[observation],"warnings":[]});
                axum::Json(serde_json::json!({
                    "model":"synthetic-model", "engine":"ninfer", "prompt_version":"laboratory-page-v2",
                    "content":page.to_string(), "raw_response_body":"synthetic native output",
                    "observations":page["observations"], "warnings":[]
                }))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = (*state.config).clone();
    config.ocr.url = format!("http://{address}");
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
    let output = serde_json::json!({"model":"synthetic","engine":"ninfer","prompt_version":"laboratory-page-v2",
        "content":page.to_string(), "raw_response_body":"synthetic full native response",
        "observations":page["observations"], "warnings":page["warnings"]});
    let router = axum::Router::new().route(
        "/api/v1/documents/extract",
        axum::routing::post(move || {
            let output = output.clone();
            async move { axum::Json(output) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = (*state.config).clone();
    config.ocr.url = format!("http://{address}");
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
    config.ocr.url = format!("http://{address}");
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
    let router=axum::Router::new().route("/api/v1/documents/extract",axum::routing::post(move |axum::Json(request):axum::Json<serde_json::Value>| {
        let path=path_in.clone(); let state=cleanup_state.clone();
        async move {
            let page=request["page"].as_i64().unwrap();
            if page==2 {
                longlivehealth::maintenance::cleanup(&state).await.unwrap();
                assert!(!path.lock().await.as_ref().unwrap().exists());
            }
            let mut observation=crate::observation_payload();observation.metric_id=None;observation.source.page=page;
            axum::Json(json!({"report_title":"Three page report","source_name":"Synthetic Lab","model":"fixture","engine":"fixture","prompt_version":"laboratory-page-v3","content":"synthetic page","raw_response_body":"synthetic","observations":[observation],"warnings":[]}))
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let mut config = (*state.config).clone();
    config.ocr.url = format!("http://{address}");
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
    let router = axum::Router::new().route("/api/v1/documents/extract", axum::routing::post(move || {
        let rows = rows.clone();
        async move { axum::Json(json!({"model":"test","engine":"test","prompt_version":"test","report_title":null,"source_name":null,
            "content":serde_json::to_string(&rows).unwrap(),"raw_response_body":"original response","observations":rows,"warnings":[]})) }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = (*state.config).clone();
    config.ocr.url = format!("http://{address}");
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
    let issues = longlivehealth::ocr::validation_issues(&page.to_string(), 3);
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
    let response = json!({"model":"qwen3.8-27b-ninfer-nvfp4","engine":"ninfer","prompt_version":"laboratory-page-v4",
        "report_title":null,"source_name":null,"content":page.to_string(),"raw_response_body":"original", "observations":page["observations"],"warnings":[]});
    let router = axum::Router::new().route(
        "/api/v1/documents/extract",
        axum::routing::post(move || {
            let response = response.clone();
            async move { axum::Json(response) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = (*state.config).clone();
    config.ocr.url = format!("http://{address}");
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
