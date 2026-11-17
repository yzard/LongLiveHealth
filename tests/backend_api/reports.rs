use crate::{add_user, archive, confirm, fixture, observation_payload};
use longlivehealth::laboratory::normalized;
use longlivehealth::reports::ReviewStatus;
use longlivehealth::reports::{ReviewItem, ReviewRequest, TrendRequest, trends};

#[tokio::test]
async fn report_review_api_rejects_retired_status_without_writing_a_revision() {
    use axum::http::StatusCode;
    use serde_json::json;

    let (directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let report_id = archive(&state, "alice").await;
    let token = crate::token(&state, "alice").await;
    let router = longlivehealth::app::create_application(state.clone());
    let (status, response) = crate::request(
        &router,
        "/api/v1/reports/review",
        Some(&token),
        json!({
            "report_id": report_id,
            "expected_revision": 1,
            "observations": [
                {"status": "confirmed", "payload": observation_payload()},
                {"status": "rejected", "payload": observation_payload()}
            ]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{response}");
    let report = state
        .database
        .report(&user.user_id, &report_id)
        .await
        .unwrap();
    assert_eq!(report["report"]["revision"], 1);
    assert_eq!(observation_counts(directory.path()).await, (0, 0));
}

#[tokio::test]
async fn partial_review_history_and_conflicts_are_transactional() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let bob = add_user(&state, "bob").await;
    let report = archive(&state, "alice").await;
    let request = ReviewRequest {
        new_metric: None,
        report_id: report.clone(),
        expected_revision: 1,
        context: None,
        observations: vec![
            ReviewItem {
                observation_id: None,
                expected_revision: None,
                status: ReviewStatus::Confirmed,
                payload: observation_payload(),
            },
            ReviewItem {
                observation_id: None,
                expected_revision: None,
                status: ReviewStatus::Pending,
                payload: observation_payload(),
            },
        ],
    };
    state
        .database
        .review(&alice.user_id, request)
        .await
        .unwrap();
    assert_eq!(
        state
            .database
            .confirmed(&alice.user_id)
            .await
            .unwrap()
            .len(),
        1
    );
    let rows = state
        .database
        .observations(&alice.user_id, &report)
        .await
        .unwrap();
    let id = rows
        .iter()
        .find(|row| row.status == "confirmed")
        .unwrap()
        .observation_id
        .clone();
    let mut edited = observation_payload();
    edited.raw_result = "130".into();
    state
        .database
        .review(
            &alice.user_id,
            ReviewRequest {
                new_metric: None,
                report_id: report.clone(),
                expected_revision: 2,
                context: None,
                observations: vec![ReviewItem {
                    observation_id: Some(id.clone()),
                    expected_revision: Some(1),
                    status: ReviewStatus::Confirmed,
                    payload: edited,
                }],
            },
        )
        .await
        .unwrap();
    assert_eq!(
        state
            .database
            .observation_history(&alice.user_id, &id)
            .await
            .unwrap()["history"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        state
            .database
            .observation_history(&bob.user_id, &id)
            .await
            .is_err()
    );
    assert!(
        state
            .database
            .review(
                &alice.user_id,
                ReviewRequest {
                    new_metric: None,
                    report_id: report.clone(),
                    expected_revision: 2,
                    context: None,
                    observations: vec![]
                }
            )
            .await
            .is_err()
    );
    assert_eq!(
        state
            .database
            .report_summary(&alice.user_id, &report)
            .await
            .unwrap()
            .revision,
        3
    );
    let result = trends(
        &state.database,
        &alice.user_id,
        TrendRequest {
            metric_ids: vec!["ldl_cholesterol".into()],
            start_at: None,
            end_at: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(result["points"][0]["value"], "130");
}

#[tokio::test]
async fn duplicate_relations_exclude_points_and_forbid_cycles() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let first = archive(&state, "alice").await;
    let second = archive(&state, "alice").await;
    confirm(&state, &user.user_id, &first).await;
    confirm(&state, &user.user_id, &second).await;
    state
        .database
        .relate_reports(&user.user_id, &first, Some(&second), "duplicate", 2)
        .await
        .unwrap();
    assert_eq!(
        state.database.confirmed(&user.user_id).await.unwrap().len(),
        1
    );
    assert!(
        state
            .database
            .relate_reports(&user.user_id, &second, Some(&first), "duplicate", 2)
            .await
            .is_err()
    );
    state
        .database
        .relate_reports(&user.user_id, &first, None, "duplicate", 3)
        .await
        .unwrap();
    assert_eq!(
        state.database.confirmed(&user.user_id).await.unwrap().len(),
        2
    );
}

#[test]
fn unit_conversion_never_rewrites_comparators_or_unknown_units() {
    let mut payload = observation_payload();
    payload.raw_unit = Some("mmol/L".into());
    payload.raw_result = "2.586".into();
    assert_eq!(
        normalized(&payload).unwrap(),
        ("100".into(), "mg/dL".into())
    );
    payload.raw_result = "<2.586".into();
    assert!(normalized(&payload).is_none());
    payload.raw_result = "2.586".into();
    payload.raw_unit = Some("unknown".into());
    assert!(normalized(&payload).is_none());
    payload.source.page = 2;
    assert!(payload.validate(1).is_err());
}

#[tokio::test]
async fn confirmed_units_share_scale_and_unknown_values_report_specific_reasons() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let report = archive(&state, "alice").await;
    let values = [
        ("2.586", "mmol/L"),
        ("100", "mg/dL"),
        ("<2.586", "mmol/L"),
        ("positive", "mg/dL"),
        ("5", "unknown"),
    ];
    let observations = values
        .into_iter()
        .map(|(value, unit)| {
            let mut payload = observation_payload();
            payload.raw_result = value.into();
            payload.raw_unit = Some(unit.into());
            payload.reference_range = Some("<2.586 mmol/L".into());
            ReviewItem {
                observation_id: None,
                expected_revision: None,
                status: ReviewStatus::Confirmed,
                payload,
            }
        })
        .collect();
    let result = state
        .database
        .review(
            &user.user_id,
            ReviewRequest {
                new_metric: None,
                report_id: report,
                expected_revision: 1,
                context: None,
                observations,
            },
        )
        .await
        .unwrap();
    assert_eq!(result["observations"].as_array().unwrap().len(), 5);
    let derived = trends(
        &state.database,
        &user.user_id,
        TrendRequest {
            metric_ids: vec!["ldl_cholesterol".into()],
            start_at: None,
            end_at: None,
        },
    )
    .await
    .unwrap();
    let points = derived["points"].as_array().unwrap();
    assert_eq!(points.len(), 2);
    assert!(
        points
            .iter()
            .all(|point| point["value"] == "100" && point["reference"]["upper"] == "100")
    );
    assert_eq!(derived["incomparable_count"], 3);
    let reasons = derived["excluded"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["reason"].as_str().unwrap())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        reasons,
        ["comparison", "unparsed_value", "unsupported_unit"]
            .into_iter()
            .collect()
    );
}

#[tokio::test]
async fn ocr_identity_autofills_and_relocates_without_overwriting_user_edits() {
    use serde_json::json;
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "autoname").await;
    let report = archive(&state, "autoname").await;
    state
        .database
        .autofill_report_identity(
            &user.user_id,
            &report,
            Some("Annual laboratory report".into()),
            Some("Function Health".into()),
        )
        .await
        .unwrap();
    let first = state.database.report(&user.user_id, &report).await.unwrap();
    assert_eq!(first["context"]["source_name"], "Function Health");
    assert_eq!(first["context"]["report_title"], "Annual laboratory report");
    let source_dir = state
        .config
        .data_dir
        .join("raw")
        .join(&user.user_id)
        .join("function_health");
    assert!(source_dir.exists());
    state
        .database
        .review(
            &user.user_id,
            ReviewRequest {
                new_metric: None,
                report_id: report.clone(),
                expected_revision: first["report"]["revision"].as_i64().unwrap(),
                context: Some(
                    serde_json::from_value(
                        json!({"source_name":"My Clinic","report_title":"My report"}),
                    )
                    .unwrap(),
                ),
                observations: vec![],
            },
        )
        .await
        .unwrap();
    state
        .database
        .autofill_report_identity(
            &user.user_id,
            &report,
            Some("Retry title".into()),
            Some("Other Clinic".into()),
        )
        .await
        .unwrap();
    let after = state.database.report(&user.user_id, &report).await.unwrap();
    assert_eq!(after["context"]["source_name"], "My Clinic");
    assert_eq!(after["context"]["report_title"], "My report");
    assert!(
        state
            .database
            .autofill_report_identity("different-user", &report, Some("Other".into()), None)
            .await
            .is_err()
    );
}

async fn observation_counts(root: &std::path::Path) -> (i64, i64) {
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    let mut official = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(root.join("database.sqlite")),
    )
    .await
    .unwrap();
    let mut drafts = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(root.join("draft.sqlite")),
    )
    .await
    .unwrap();
    (
        sqlx::query_scalar("SELECT COUNT(*) FROM observations")
            .fetch_one(&mut official)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT COUNT(*) FROM observations")
            .fetch_one(&mut drafts)
            .await
            .unwrap(),
    )
}

#[tokio::test]
async fn reviewing_moves_rows_between_sqlite_databases_and_updates_browse_history() {
    let (directory, state) = fixture().await;
    let user = add_user(&state, "draft-owner").await;
    let other = add_user(&state, "other-owner").await;
    let report = archive(&state, "draft-owner").await;
    let mut document = state
        .database
        .review(
            &user.user_id,
            ReviewRequest {
                new_metric: None,
                report_id: report.clone(),
                expected_revision: 1,
                context: None,
                observations: vec![ReviewItem {
                    observation_id: None,
                    expected_revision: None,
                    status: ReviewStatus::Pending,
                    payload: observation_payload(),
                }],
            },
        )
        .await
        .unwrap();
    assert_eq!(observation_counts(directory.path()).await, (0, 1));
    assert!(
        state
            .database
            .confirmed(&user.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        state
            .database
            .observations(&other.user_id, &report)
            .await
            .is_err()
    );
    let id = document["observations"][0]["observation_id"]
        .as_str()
        .unwrap()
        .to_owned();
    for (status, counts) in [
        (ReviewStatus::Confirmed, (1, 0)),
        (ReviewStatus::Pending, (0, 1)),
        (ReviewStatus::Confirmed, (1, 0)),
    ] {
        let revision = document["observations"][0]["revision"].as_i64().unwrap();
        let request = |expected| ReviewRequest {
            new_metric: None,
            report_id: report.clone(),
            expected_revision: document["report"]["revision"].as_i64().unwrap(),
            context: None,
            observations: vec![ReviewItem {
                observation_id: Some(id.clone()),
                expected_revision: Some(expected),
                status,
                payload: observation_payload(),
            }],
        };
        let before = observation_counts(directory.path()).await;
        assert!(
            state
                .database
                .review(&user.user_id, request(revision + 10))
                .await
                .is_err()
        );
        assert_eq!(observation_counts(directory.path()).await, before);
        document = state
            .database
            .review(&user.user_id, request(revision))
            .await
            .unwrap();
        assert_eq!(observation_counts(directory.path()).await, counts);
        assert_eq!(
            document["observations"][0]["status"],
            serde_json::to_value(status).unwrap()
        );
        assert_eq!(document["observations"][0]["observation_id"], id);
        assert_eq!(document["observations"][0]["payload"]["raw_result"], "120");
        let history = ldl_history(&state.database, &user.user_id).await;
        let points = history["points"].as_array().unwrap();
        if status == ReviewStatus::Confirmed {
            assert_eq!(points.len(), 1);
            assert_eq!(points[0]["observation_id"], id);
        } else {
            assert!(points.is_empty());
            assert!(history["records"].as_array().unwrap().is_empty());
        }
        assert_eq!(
            state.database.confirmed(&user.user_id).await.unwrap().len(),
            usize::from(status == ReviewStatus::Confirmed)
        );
    }
}

#[tokio::test]
async fn deleting_report_removes_official_and_draft_results() {
    let (directory, state) = fixture().await;
    let user = add_user(&state, "delete-owner").await;
    let other = add_user(&state, "delete-other").await;
    let report = archive(&state, "delete-owner").await;
    let document = state
        .database
        .review(
            &user.user_id,
            ReviewRequest {
                new_metric: None,
                report_id: report.clone(),
                expected_revision: 1,
                context: None,
                observations: [ReviewStatus::Pending, ReviewStatus::Confirmed]
                    .into_iter()
                    .map(|status| ReviewItem {
                        observation_id: None,
                        expected_revision: None,
                        status,
                        payload: observation_payload(),
                    })
                    .collect(),
            },
        )
        .await
        .unwrap();
    let revision = document["report"]["revision"].as_i64().unwrap();
    assert_eq!(observation_counts(directory.path()).await, (1, 1));
    assert!(
        state
            .database
            .delete_report(&other.user_id, &report, revision)
            .await
            .is_err()
    );
    assert!(
        state
            .database
            .delete_report(&user.user_id, &report, revision + 1)
            .await
            .is_err()
    );
    assert_eq!(observation_counts(directory.path()).await, (1, 1));
    assert_eq!(
        ldl_history(&state.database, &user.user_id).await["points"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    state
        .database
        .delete_report(&user.user_id, &report, revision)
        .await
        .unwrap();
    assert_eq!(observation_counts(directory.path()).await, (0, 0));
    let history = ldl_history(&state.database, &user.user_id).await;
    assert!(history["points"].as_array().unwrap().is_empty());
    assert!(history["records"].as_array().unwrap().is_empty());
    assert!(
        state
            .database
            .confirmed(&user.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        state
            .database
            .observations(&user.user_id, &report)
            .await
            .is_err()
    );
}

async fn ldl_history(
    database: &longlivehealth::database::Database,
    user: &str,
) -> serde_json::Value {
    longlivehealth::metric_history::history(
        database,
        user,
        longlivehealth::metric_history::HistoryRequest {
            metric_id: "ldl_cholesterol".into(),
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn startup_relocates_old_pending_rows_and_recovers_an_interrupted_draft_cleanup() {
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    let (directory, state) = fixture().await;
    let user = add_user(&state, "migrate-owner").await;
    let report = archive(&state, "migrate-owner").await;
    let id = confirm(&state, &user.user_id, &report).await;
    state.database.close().await;
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(directory.path().join("database.sqlite")),
    )
    .await
    .unwrap();
    sqlx::query("ATTACH DATABASE ? AS draft")
        .bind(directory.path().join("draft.sqlite").to_str().unwrap())
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::raw_sql(
        "ALTER TABLE draft.observations DROP COLUMN ocr_revision; PRAGMA draft.user_version = 1",
    )
    .execute(&mut connection)
    .await
    .unwrap();
    sqlx::query("INSERT INTO draft.observations SELECT observation_id,user_id,report_id,current_revision,'pending',payload_json,updated_at,candidate_key FROM main.observations").execute(&mut connection).await.unwrap();
    sqlx::query("INSERT INTO main.observations SELECT 'old-pending',user_id,report_id,1,'pending',payload_json,updated_at,NULL FROM main.observations WHERE observation_id=?").bind(id).execute(&mut connection).await.unwrap();
    connection.close().await.unwrap();
    let database = longlivehealth::database::Database::open(directory.path())
        .await
        .unwrap();
    assert_eq!(observation_counts(directory.path()).await, (1, 1));
    assert_eq!(
        database
            .observations(&user.user_id, &report)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(database.confirmed(&user.user_id).await.unwrap().len(), 1);
    database.close().await;
}

#[tokio::test]
async fn rescan_clears_all_results_and_rejects_stale_or_active_requests() {
    let (directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let bob = add_user(&state, "bob").await;
    let report = archive(&state, "alice").await;
    let confirmed = confirm(&state, &alice.user_id, &report).await;
    assert_eq!(
        ldl_history(&state.database, &alice.user_id).await["points"][0]["observation_id"],
        confirmed
    );
    let original = state.database.file(&alice.user_id, &report).await.unwrap();
    let summary = state
        .database
        .report_summary(&alice.user_id, &report)
        .await
        .unwrap();
    assert!(
        state
            .database
            .rescan_report(&alice.user_id, &report, summary.revision)
            .await
            .is_err()
    );
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
    assert!(
        state
            .database
            .rescan_report(&bob.user_id, &report, summary.revision)
            .await
            .is_err()
    );
    assert!(
        state
            .database
            .rescan_report(&alice.user_id, &report, summary.revision - 1)
            .await
            .is_err()
    );
    assert_eq!(
        state
            .database
            .observations(&alice.user_id, &report)
            .await
            .unwrap()
            .len(),
        1
    );
    state
        .database
        .review(
            &alice.user_id,
            ReviewRequest {
                new_metric: None,
                report_id: report.clone(),
                expected_revision: summary.revision,
                context: None,
                observations: vec![ReviewItem {
                    observation_id: None,
                    expected_revision: None,
                    status: ReviewStatus::Pending,
                    payload: observation_payload(),
                }],
            },
        )
        .await
        .unwrap();
    let revision = summary.revision + 1;
    let queued = state
        .database
        .rescan_report(&alice.user_id, &report, revision)
        .await
        .unwrap();
    assert_eq!(queued.status, "queued");
    let rows = state
        .database
        .observations(&alice.user_id, &report)
        .await
        .unwrap();
    assert!(rows.is_empty());
    assert_eq!(observation_counts(directory.path()).await, (0, 0));
    assert!(
        ldl_history(&state.database, &alice.user_id).await["points"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let edit = ReviewRequest {
        new_metric: None,
        report_id: report.clone(),
        expected_revision: revision + 1,
        context: None,
        observations: vec![ReviewItem {
            observation_id: None,
            expected_revision: None,
            status: ReviewStatus::Pending,
            payload: observation_payload(),
        }],
    };
    assert!(matches!(
        state.database.review(&alice.user_id, edit).await,
        Err(longlivehealth::error::AppError::Conflict(_))
    ));
    assert_eq!(
        state
            .database
            .file(&alice.user_id, &report)
            .await
            .unwrap()
            .relative_path,
        original.relative_path
    );
    assert_eq!(
        state
            .database
            .report_summary(&alice.user_id, &report)
            .await
            .unwrap()
            .revision,
        revision + 1
    );
    assert!(
        state
            .database
            .rescan_report(&alice.user_id, &report, revision + 1)
            .await
            .is_err()
    );
    assert!(
        state
            .database
            .rescan_report(&alice.user_id, &report, revision)
            .await
            .is_err()
    );
    let job = state
        .database
        .claim_job(2_000_000_002, 600, 3)
        .await
        .unwrap()
        .unwrap();
    state
        .database
        .finish_job(&job, 2_000_000_003, Some("test_failure"))
        .await
        .unwrap();
    assert_eq!(observation_counts(directory.path()).await, (0, 0));
}

#[tokio::test]
async fn stored_results_display_loinc_long_common_name_without_remapping() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "loinc-names").await;
    let report = archive(&state, "loinc-names").await;
    let mut polys = observation_payload();
    polys.raw_name = "POLYS (31160-5)".into();
    polys.raw_unit = Some("%".into());
    polys.metric_id = None;
    let mut unknown = polys.clone();
    unknown.raw_name = "Unknown test (9999999-9)".into();
    state
        .database
        .review(
            &user.user_id,
            ReviewRequest {
                new_metric: None,
                report_id: report.clone(),
                expected_revision: 1,
                context: None,
                observations: vec![polys, unknown]
                    .into_iter()
                    .map(|payload| ReviewItem {
                        observation_id: None,
                        expected_revision: None,
                        status: ReviewStatus::Pending,
                        payload,
                    })
                    .collect(),
            },
        )
        .await
        .unwrap();

    let rows = state
        .database
        .observations(&user.user_id, &report)
        .await
        .unwrap();
    let polys = rows
        .iter()
        .find(|row| row.lab_test_code.as_deref() == Some("31160-5"))
        .unwrap();
    assert_eq!(
        polys.lab_test_name,
        Some("Granulocytes/Leukocytes in Blood by Manual count")
    );
    assert_eq!(polys.payload.raw_name, "POLYS (31160-5)");
    assert_eq!(polys.status, "pending");
    assert!(polys.resolved_metric_id.is_none());
    assert!(
        polys
            .browse_issues
            .iter()
            .any(|issue| issue.code == "lab_test_code_conflict")
    );
    let unknown = rows
        .iter()
        .find(|row| row.lab_test_code.as_deref() == Some("9999999-9"))
        .unwrap();
    assert!(unknown.lab_test_name.is_none());
}

#[tokio::test]
async fn report_rows_explain_browse_blockers_and_distinguish_chart_only_issues() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "review-flags").await;
    let report = archive(&state, "review-flags").await;
    let mut unsupported = observation_payload();
    unsupported.raw_name = "Globulin".into();
    unsupported.raw_unit = Some("invalid-unit".into());
    unsupported.metric_id = None;
    let mut mapped = unsupported.clone();
    mapped.metric_id = Some("globulin".into());
    let mut unknown = observation_payload();
    unknown.raw_name = "Unknown test".into();
    unknown.metric_id = None;
    let mut ketones = observation_payload();
    ketones.raw_name = "Ketones (50557-8)".into();
    ketones.raw_result = "Negative".into();
    ketones.raw_unit = Some("mg/dL".into());
    ketones.metric_id = None;
    ketones.ocr_mapping_note = Some("Printed unit conflicts with the catalog".into());
    let mut bounded = ketones.clone();
    bounded.raw_name = "Urine ketones (50557-8)".into();
    bounded.notes = Some("Reported negative threshold: <0.1 mg/dL".into());
    let mut egfr = observation_payload();
    egfr.raw_name = "eGFR (33914-3)".into();
    egfr.raw_unit = Some("mL/min/1.73m2".into());
    egfr.metric_id = None;
    egfr.ocr_mapping_note = Some("Unit mismatch against catalog entry with no unit".into());
    let mut undated = observation_payload();
    undated.sampled_at = None;
    let mut qualitative = observation_payload();
    qualitative.raw_name = "ABO Group".into();
    qualitative.raw_unit = None;
    qualitative.raw_result = "O".into();
    qualitative.metric_id = None;
    let response = state
        .database
        .review(
            &user.user_id,
            ReviewRequest {
                new_metric: None,
                report_id: report,
                expected_revision: 1,
                context: None,
                observations: vec![
                    unsupported,
                    mapped,
                    unknown,
                    ketones,
                    bounded,
                    egfr,
                    undated,
                    qualitative,
                ]
                .into_iter()
                .map(|payload| ReviewItem {
                    observation_id: None,
                    expected_revision: None,
                    status: ReviewStatus::Pending,
                    payload,
                })
                .collect(),
            },
        )
        .await
        .unwrap();
    let rows = response["observations"].as_array().unwrap();
    let find = |name: &str, mapped: bool| {
        rows.iter()
            .find(|r| {
                r["payload"]["raw_name"] == name && r["payload"]["metric_id"].is_string() == mapped
            })
            .unwrap()
    };
    let bad = find("Globulin", false);
    assert_eq!(bad["browse_issues"][0]["code"], "unsupported_unit");
    assert_eq!(bad["browse_issues"][0]["scope"], "browse");
    assert_eq!(bad["metric_suggestions"][0]["metric_id"], "globulin");
    assert_eq!(bad["metric_suggestions"][0]["unit_match"], "incompatible");
    assert!(
        bad["browse_issues"][0]["message"]
            .as_str()
            .unwrap()
            .contains("invalid-unit")
    );
    let ketones = find("Ketones (50557-8)", false);
    assert_eq!(ketones["status"], "pending");
    assert_eq!(ketones["browse_issues"][0]["code"], "unsupported_unit");
    let message = ketones["browse_issues"][0]["message"].as_str().unwrap();
    assert!(message.contains("Unit mismatch"));
    assert!(message.contains("report unit mg/dL"));
    assert!(message.contains("database standard unit unitless"));
    assert_eq!(
        ketones["metric_suggestions"][0]["metric_id"],
        "urine_ketones"
    );
    assert_eq!(
        ketones["metric_suggestions"][0]["unit_match"],
        "incompatible"
    );
    assert!(ketones.get("reported_negative_bound").is_none());
    let bounded = find("Urine ketones (50557-8)", false);
    assert_eq!(bounded["reported_negative_bound"], "<0.1 mg/dL");
    assert_eq!(bounded["payload"]["raw_result"], "Negative");
    assert!(
        bounded["browse_issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["code"] == "negative_with_unit")
    );
    let egfr = find("eGFR (33914-3)", false);
    assert_eq!(egfr["browse_issues"][0]["code"], "unmapped_metric");
    assert_eq!(egfr["browse_issues"].as_array().unwrap().len(), 1);
    assert_eq!(find("Globulin", true)["browse_issues"][0]["scope"], "chart");
    assert_eq!(
        find("Unknown test", false)["browse_issues"][0]["code"],
        "unmapped_metric"
    );
    assert_eq!(
        find("LDL Cholesterol", true)["browse_issues"][0]["code"],
        "missing_date"
    );
    assert!(
        find("ABO Group", false)["browse_issues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mut corrected: longlivehealth::reports::ObservationPayload =
        serde_json::from_value(bad["payload"].clone()).unwrap();
    corrected.raw_unit = Some("g/dL (calc)".into());
    let fixed = state
        .database
        .review(
            &user.user_id,
            ReviewRequest {
                new_metric: None,
                report_id: bad["report_id"].as_str().unwrap().into(),
                expected_revision: 2,
                context: None,
                observations: vec![ReviewItem {
                    observation_id: Some(bad["observation_id"].as_str().unwrap().into()),
                    expected_revision: Some(1),
                    status: ReviewStatus::Confirmed,
                    payload: corrected,
                }],
            },
        )
        .await
        .unwrap();
    let fixed = fixed["observations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["observation_id"] == bad["observation_id"])
        .unwrap();
    assert!(fixed["browse_issues"].as_array().unwrap().is_empty());
    assert_eq!(fixed["resolved_metric_id"], "globulin");
}

#[tokio::test]
async fn saving_mappings_keeps_ocr_evidence_and_rechecks_independent_report_issues() {
    use serde_json::{Value, json};

    let (_directory, state) = fixture().await;
    let user = add_user(&state, "mapping-review").await;
    let report_id = archive(&state, "mapping-review").await;
    let note = "Check the printed identity or scale.";
    let mut snapshot = state
        .database
        .review(
            &user.user_id,
            ReviewRequest {
                new_metric: None,
                report_id: report_id.clone(),
                expected_revision: 1,
                context: None,
                observations: [
                    ("WBC", "4.5", "10^9/L"),
                    ("Total cholesterol", "160", "mg/dL"),
                    ("Absolute neutrophil count", "3660", "10E3/uL"),
                    ("LDL Cholesterol", "TNP", "mg/dL"),
                ]
                .into_iter()
                .map(|(name, result, unit)| {
                    let mut payload = observation_payload();
                    payload.metric_id = None;
                    payload.raw_name = name.into();
                    payload.raw_result = result.into();
                    payload.raw_unit = Some(unit.into());
                    payload.ocr_mapping_note = Some(note.into());
                    ReviewItem {
                        observation_id: None,
                        expected_revision: None,
                        status: ReviewStatus::Pending,
                        payload,
                    }
                })
                .collect(),
            },
        )
        .await
        .unwrap();

    for (name, id, issue) in [
        ("WBC", "wbc", Value::Null),
        ("Total cholesterol", "total_cholesterol", Value::Null),
        (
            "Absolute neutrophil count",
            "neutrophil_count",
            json!("inconsistent_subcount"),
        ),
        (
            "LDL Cholesterol",
            "ldl_cholesterol",
            json!("test_not_performed"),
        ),
    ] {
        let row = snapshot["observations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["payload"]["raw_name"] == name)
            .unwrap();
        let observation_id = row["observation_id"].as_str().unwrap().to_owned();
        let mut payload: longlivehealth::reports::ObservationPayload =
            serde_json::from_value(row["payload"].clone()).unwrap();
        payload.metric_id = Some(id.into());
        snapshot = state
            .database
            .review(
                &user.user_id,
                ReviewRequest {
                    new_metric: None,
                    report_id: report_id.clone(),
                    expected_revision: snapshot["report"]["revision"].as_i64().unwrap(),
                    context: None,
                    observations: vec![ReviewItem {
                        observation_id: Some(observation_id.clone()),
                        expected_revision: row["revision"].as_i64(),
                        status: ReviewStatus::Pending,
                        payload,
                    }],
                },
            )
            .await
            .unwrap();
        let saved = snapshot["observations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["observation_id"] == observation_id)
            .unwrap();
        assert_eq!(saved["status"], "pending");
        assert_eq!(saved["resolved_metric_id"], id);
        assert_eq!(saved["payload"]["ocr_mapping_note"], note);
        assert_eq!(saved["browse_issues"][0]["code"], issue);
    }
    assert!(
        state
            .database
            .confirmed(&user.user_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn report_summaries_use_collection_evidence_for_the_date_column() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let report = archive(&state, "alice").await;
    let request = longlivehealth::models::ListRequest {
        after_id: None,
        limit: 10,
    };
    assert!(
        state
            .database
            .reports(&alice.user_id, &request)
            .await
            .unwrap()[0]
            .collection_date
            .is_none()
    );
    let observations = [
        (ReviewStatus::Confirmed, "2026-08-15"),
        (ReviewStatus::Pending, "2026-09-01T23:30:00-05:00"),
    ]
    .into_iter()
    .map(|(status, date)| {
        let mut payload = observation_payload();
        payload.sampled_at = Some(date.into());
        ReviewItem {
            observation_id: None,
            expected_revision: None,
            status,
            payload,
        }
    })
    .collect();
    state
        .database
        .review(
            &alice.user_id,
            ReviewRequest {
                new_metric: None,
                report_id: report.clone(),
                expected_revision: 1,
                context: None,
                observations,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        state
            .database
            .reports(&alice.user_id, &request)
            .await
            .unwrap()[0]
            .collection_date
            .as_deref(),
        Some("2026-09-01")
    );
    let evidence =
        serde_json::json!({"report_collection_date":{"date":"2026-08-15","source_page":1}});
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(state.config.data_dir.join("database.sqlite")),
    )
    .await
    .unwrap();
    sqlx::query("INSERT INTO extraction_inputs (user_id, report_id, run_id, page_number, input_json, created_at) VALUES (?, ?, 'test-run', 1, ?, 1)")
        .bind(&alice.user_id)
        .bind(&report)
        .bind(evidence.to_string())
        .execute(&mut connection)
        .await.unwrap();
    assert_eq!(
        state
            .database
            .reports(&alice.user_id, &request)
            .await
            .unwrap()[0]
            .collection_date
            .as_deref(),
        Some("2026-08-15")
    );
    assert_eq!(
        state
            .database
            .report(&alice.user_id, &report)
            .await
            .unwrap()["report"]["collection_date"],
        "2026-08-15"
    );
}

#[tokio::test]
async fn review_creates_an_owned_metric_and_links_its_confirmed_browse_history() {
    use longlivehealth::metric_history::{HistoryRequest, history};
    use longlivehealth::reports::NewMetric;

    let (_directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let other = add_user(&state, "bob").await;
    let report = archive(&state, "alice").await;
    let mut payload = observation_payload();
    payload.raw_name = "Zyphorin activity".into();
    payload.raw_result = "120".into();
    payload.raw_unit = Some("ZR".into());
    payload.source.quote = "Zyphorin activity 120 ZR".into();
    payload.metric_id = None;
    let drafted = state
        .database
        .review(
            &user.user_id,
            ReviewRequest {
                report_id: report.clone(),
                expected_revision: 1,
                context: None,
                observations: vec![ReviewItem {
                    observation_id: None,
                    expected_revision: None,
                    status: ReviewStatus::Pending,
                    payload: payload.clone(),
                }],
                new_metric: None,
            },
        )
        .await
        .unwrap();
    let draft = &drafted["observations"][0];
    assert_eq!(draft["metric_suggestions"], serde_json::json!([]));
    let request = |observation_revision| ReviewRequest {
        report_id: report.clone(),
        expected_revision: 2,
        context: None,
        observations: vec![ReviewItem {
            observation_id: Some(draft["observation_id"].as_str().unwrap().to_owned()),
            expected_revision: Some(observation_revision),
            status: ReviewStatus::Confirmed,
            payload: payload.clone(),
        }],
        new_metric: Some(NewMetric {
            name: payload.raw_name.clone(),
            standard_unit: "ZR".into(),
        }),
    };
    assert!(
        state
            .database
            .review(&user.user_id, request(2))
            .await
            .is_err()
    );
    assert!(
        state
            .database
            .custom_metrics(&user.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    let saved = state
        .database
        .review(&user.user_id, request(1))
        .await
        .unwrap();
    let id = saved["created_metric"]["metric_id"].as_str().unwrap();
    let result = &saved["observations"][0];
    assert_eq!(result["resolved_metric_id"], id);
    assert_eq!(result["payload"]["raw_unit"], "ZR");
    assert_eq!(
        result["payload"]["source"]["quote"],
        "Zyphorin activity 120 ZR"
    );
    assert_eq!(result["interpretation"]["result"]["display"], "120 ZR");

    let history_request = || HistoryRequest {
        metric_id: id.to_owned(),
    };
    let series = history(&state.database, &user.user_id, history_request())
        .await
        .unwrap();
    assert_eq!(series["metric"]["name"], "Zyphorin activity");
    assert_eq!(series["points"][0]["report_id"], report);
    assert!(
        history(&state.database, &other.user_id, history_request())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn mark_as_new_replaces_a_legacy_confirmed_display_identity_with_a_persisted_metric() {
    use longlivehealth::reports::{NewMetric, UnmappedChoice};

    let (_directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let report_id = archive(&state, "alice").await;
    let review = |revision, observation_id, expected_revision, payload| {
        state.database.review(
            &user.user_id,
            ReviewRequest {
                report_id: report_id.clone(),
                expected_revision: revision,
                context: None,
                new_metric: None,
                observations: vec![ReviewItem {
                    observation_id,
                    expected_revision,
                    status: ReviewStatus::Confirmed,
                    payload,
                }],
            },
        )
    };
    let mut payload = observation_payload();
    payload.raw_name = "pH (50560-2)".into();
    payload.raw_result = "6.0".into();
    payload.raw_unit = None;
    payload.metric_id = None;
    payload.source.quote = "pH (50560-2) 6.0".into();
    let original = review(1, None, None, payload.clone()).await.unwrap();
    let legacy = &original["observations"][0];
    assert!(legacy["payload"]["metric_id"].is_null());
    let browse = longlivehealth::metric_history::browse(&state.database, &user.user_id)
        .await
        .unwrap();
    let displayed_id = browse["metrics"][0]["metric_id"].as_str().unwrap();
    assert!(displayed_id.starts_with("reported:"));
    payload.unmapped_choice = Some(UnmappedChoice::New(NewMetric {
        name: "pH of Urine".into(),
        standard_unit: "".into(),
    }));
    let promoted = review(
        2,
        legacy["observation_id"].as_str().map(str::to_owned),
        legacy["revision"].as_i64(),
        payload,
    )
    .await
    .unwrap();
    let row = &promoted["observations"][0];
    let id = promoted["created_metric"]["metric_id"].as_str().unwrap();
    assert_ne!(id, displayed_id);
    assert_eq!(row["payload"]["metric_id"], id);
    assert_eq!(row["observation_id"], legacy["observation_id"]);
    assert_eq!(row["payload"]["source"], legacy["payload"]["source"]);
    assert_eq!(row["status"], "confirmed");
    let browse = longlivehealth::metric_history::browse(&state.database, &user.user_id)
        .await
        .unwrap();
    assert_eq!(browse["metrics"][0]["metric_id"], id);
    let history = longlivehealth::metric_history::history(
        &state.database,
        &user.user_id,
        longlivehealth::metric_history::HistoryRequest {
            metric_id: id.into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(history["metric"]["loinc"][0]["code"], "50560-2");
    assert_eq!(history["points"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn saved_mapping_intent_is_checked_before_it_can_create_a_metric() {
    use longlivehealth::reports::{NewMetric, UnmappedChoice};
    use serde_json::Value;

    let (_directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let report = archive(&state, "alice").await;
    let request = |revision, row: Option<&Value>, status: ReviewStatus, payload| ReviewRequest {
        report_id: report.clone(),
        expected_revision: revision,
        context: None,
        new_metric: None,
        observations: vec![ReviewItem {
            observation_id: row.map(|row| row["observation_id"].as_str().unwrap().to_owned()),
            expected_revision: row.and_then(|row| row["revision"].as_i64()),
            status,
            payload,
        }],
    };
    let mut payload = observation_payload();
    payload.raw_name = "Zyphorin activity".into();
    payload.raw_unit = Some("ZR".into());
    payload.ocr_metric_candidate = Some("ldl_cholesterol".into());
    payload.ocr_mapping_note = Some("The suggested identity is uncertain.".into());
    payload.metric_id = None;
    let initial = state
        .database
        .review(
            &user.user_id,
            request(1, None, ReviewStatus::Pending, payload.clone()),
        )
        .await
        .unwrap();
    let initial = &initial["observations"][0];
    assert_eq!(initial["display_name"], "Zyphorin activity");
    payload.unmapped_choice = Some(UnmappedChoice::New(NewMetric {
        name: "Zyphorin enzyme activity".into(),
        standard_unit: "ZR".into(),
    }));
    assert!(
        state
            .database
            .review(
                &user.user_id,
                request(2, Some(initial), ReviewStatus::Confirmed, payload.clone())
            )
            .await
            .is_err()
    );

    payload.raw_name = "Zyphorin activity (2093-3)".into();
    payload.raw_unit = Some("wrong unit".into());
    let saved = state
        .database
        .review(
            &user.user_id,
            request(2, Some(initial), ReviewStatus::Pending, payload.clone()),
        )
        .await
        .unwrap();
    let wrong = &saved["observations"][0];
    assert_eq!(wrong["status"], "pending");
    assert_eq!(wrong["display_name"], "Zyphorin enzyme activity");
    assert_eq!(wrong["payload"]["raw_name"], "Zyphorin activity (2093-3)");
    assert_eq!(wrong["payload"]["unmapped_choice"]["kind"], "new");
    assert!(wrong["resolved_metric_id"].is_null());
    assert_eq!(
        wrong["payload"]["ocr_mapping_note"],
        "The suggested identity is uncertain."
    );
    let issues = wrong["browse_issues"].as_array().unwrap();
    for code in ["loinc_metric_conflict", "unsupported_unit"] {
        assert!(issues.iter().any(|issue| issue["code"] == code));
    }
    assert!(
        state
            .database
            .review(
                &user.user_id,
                request(3, Some(wrong), ReviewStatus::Confirmed, payload.clone())
            )
            .await
            .is_err()
    );
    assert!(
        state
            .database
            .custom_metrics(&user.user_id)
            .await
            .unwrap()
            .is_empty()
    );

    payload.raw_name = "Zyphorin activity".into();
    payload.raw_unit = Some("ZR".into());
    let corrected = state
        .database
        .review(
            &user.user_id,
            request(3, Some(wrong), ReviewStatus::Pending, payload.clone()),
        )
        .await
        .unwrap();
    let corrected = &corrected["observations"][0];
    assert!(corrected["browse_issues"].as_array().unwrap().is_empty());
    let reopened = state.database.report(&user.user_id, &report).await.unwrap();
    assert_eq!(
        reopened["observations"][0]["display_name"],
        "Zyphorin enzyme activity"
    );
    assert_eq!(reopened["observations"][0]["status"], "pending");
    assert!(
        state
            .database
            .custom_metrics(&user.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    let confirmed = state
        .database
        .review(
            &user.user_id,
            request(4, Some(corrected), ReviewStatus::Confirmed, payload),
        )
        .await
        .unwrap();
    let row = &confirmed["observations"][0];
    assert_eq!(row["status"], "confirmed");
    assert_eq!(row["display_name"], "Zyphorin enzyme activity");
    assert_eq!(
        row["payload"]["metric_id"],
        confirmed["created_metrics"][0]["metric_id"]
    );
    assert!(row["payload"]["unmapped_choice"].is_null());
    assert_eq!(
        state
            .database
            .custom_metrics(&user.user_id)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn a_saved_new_metric_can_confirm_a_qualitative_result_with_a_compatible_unit() {
    use longlivehealth::metric_history::{HistoryRequest, history};
    use longlivehealth::reports::{NewMetric, UnmappedChoice};
    use serde_json::Value;

    let (_directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let report = archive(&state, "alice").await;
    let request = |revision, row: Option<&Value>, status: ReviewStatus, payload| ReviewRequest {
        report_id: report.clone(),
        expected_revision: revision,
        context: None,
        new_metric: None,
        observations: vec![ReviewItem {
            observation_id: row.map(|row| row["observation_id"].as_str().unwrap().to_owned()),
            expected_revision: row.and_then(|row| row["revision"].as_i64()),
            status,
            payload,
        }],
    };
    let mut payload = observation_payload();
    payload.raw_name = "Urobilinogen (50563-6)".into();
    payload.raw_result = "Negative".into();
    payload.raw_unit = Some("mg/dL".into());
    payload.reference_range = None;
    payload.report_flag = None;
    payload.metric_id = None;
    payload.unmapped_choice = Some(UnmappedChoice::New(NewMetric {
        name: "Urobilinogen in Urine".into(),
        standard_unit: "U/L".into(),
    }));
    let saved = state
        .database
        .review(
            &user.user_id,
            request(1, None, ReviewStatus::Pending, payload.clone()),
        )
        .await
        .unwrap();
    let wrong = &saved["observations"][0];
    assert!(
        wrong["browse_issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["code"] == "unsupported_unit")
    );
    assert!(
        state
            .database
            .review(
                &user.user_id,
                request(2, Some(wrong), ReviewStatus::Confirmed, payload.clone())
            )
            .await
            .is_err()
    );
    assert!(
        state
            .database
            .custom_metrics(&user.user_id)
            .await
            .unwrap()
            .is_empty()
    );

    payload.unmapped_choice = Some(UnmappedChoice::New(NewMetric {
        name: "Urobilinogen in Urine".into(),
        standard_unit: "mg/dL".into(),
    }));
    let saved = state
        .database
        .review(
            &user.user_id,
            request(2, Some(wrong), ReviewStatus::Pending, payload.clone()),
        )
        .await
        .unwrap();
    let pending = &saved["observations"][0];
    assert_eq!(pending["status"], "pending");
    assert_eq!(pending["browse_issues"].as_array().unwrap().len(), 1);
    assert_eq!(pending["browse_issues"][0]["code"], "negative_with_unit");
    let confirmed = state
        .database
        .review(
            &user.user_id,
            request(3, Some(pending), ReviewStatus::Confirmed, payload),
        )
        .await
        .unwrap();
    let row = &confirmed["observations"][0];
    assert_eq!(row["status"], "confirmed");
    assert_eq!(row["payload"]["raw_result"], "Negative");
    assert_eq!(row["payload"]["raw_unit"], "mg/dL");
    assert!(row.get("reported_negative_bound").is_none());
    let metric_id = row["resolved_metric_id"].as_str().unwrap().to_owned();
    let history = history(&state.database, &user.user_id, HistoryRequest { metric_id })
        .await
        .unwrap();
    assert_eq!(history["records"][0]["display_value"], "Negative");
    assert_eq!(history["points"], serde_json::json!([]));
}

#[tokio::test]
async fn confirming_multiple_new_results_creates_each_distinct_saved_definition_once() {
    use longlivehealth::reports::{NewMetric, UnmappedChoice};

    let (_directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let report_id = archive(&state, "alice").await;
    let observations = [
        ("Zyphorin activity", "1.1"),
        ("Zyphorin activity", "1.2"),
        ("Vivorel fraction", "2.0"),
    ]
    .into_iter()
    .map(|(name, result)| {
        let mut payload = observation_payload();
        payload.raw_name = name.into();
        payload.raw_result = result.into();
        payload.raw_unit = Some("ZR".into());
        payload.metric_id = None;
        payload.unmapped_choice = Some(UnmappedChoice::New(NewMetric {
            name: name.into(),
            standard_unit: "ZR".into(),
        }));
        ReviewItem {
            observation_id: None,
            expected_revision: None,
            status: ReviewStatus::Pending,
            payload,
        }
    })
    .collect();
    let save = |expected_revision, observations| {
        state.database.review(
            &user.user_id,
            ReviewRequest {
                report_id: report_id.clone(),
                expected_revision,
                context: None,
                new_metric: None,
                observations,
            },
        )
    };
    let draft = save(1, observations).await.unwrap();
    let changes = draft["observations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| ReviewItem {
            observation_id: row["observation_id"].as_str().map(str::to_owned),
            expected_revision: row["revision"].as_i64(),
            status: ReviewStatus::Confirmed,
            payload: serde_json::from_value(row["payload"].clone()).unwrap(),
        })
        .collect();
    let confirmed = save(2, changes).await.unwrap();
    assert_eq!(confirmed["created_metrics"].as_array().unwrap().len(), 2);
    let rows = confirmed["observations"].as_array().unwrap();
    for (name, count) in [("Zyphorin activity", 2), ("Vivorel fraction", 1)] {
        let created = confirmed["created_metrics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|metric| metric["name"] == name)
            .unwrap();
        assert_eq!(
            rows.iter()
                .filter(|row| row["status"] == "confirmed"
                    && row["payload"]["raw_name"] == name
                    && row["payload"]["metric_id"] == created["metric_id"])
                .count(),
            count
        );
    }
}

#[test]
fn count_exponent_units_use_a_verified_conversion_and_keep_the_printed_unit() {
    use longlivehealth::laboratory::interpret;
    use longlivehealth::reports::OcrNormalization;
    let mut payload = observation_payload();
    payload.raw_name = "RBC".into();
    payload.raw_result = "4.61".into();
    payload.raw_unit = Some("×10⁶/µL".into());
    payload.metric_id = None;
    payload.ocr_normalization = Some(OcrNormalization {
        result: "4610000".into(),
        unit: "10^12/L".into(),
    });
    let interpreted = interpret(&payload);
    assert_eq!(interpreted.result.display.as_deref(), Some("4.61 10^12/L"));
    assert_eq!(payload.raw_unit.as_deref(), Some("×10⁶/µL"));
    for (name, unit, expected) in [
        ("RBC (789-8)", "10E6/uL", "4.61 10^12/L"),
        ("RBC (789-8)", "x10E6/uL", "4.61 10^12/L"),
        ("WBC (6690-2)", "10E3/uL", "4.61 10^9/L"),
        ("Platelets (777-3)", "x10E3/uL", "4.61 10^9/L"),
    ] {
        payload.raw_name = name.into();
        payload.raw_unit = Some(unit.into());
        assert_eq!(
            interpret(&payload).result.display.as_deref(),
            Some(expected)
        );
        assert_eq!(payload.raw_unit.as_deref(), Some(unit));
    }
    payload.raw_unit = Some("unknown".into());
    assert!(interpret(&payload).result.display.is_none());
}
