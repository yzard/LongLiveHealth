use crate::{add_user, archive, confirm, fixture, observation_payload};
use longlivehealth::laboratory::normalized;
use longlivehealth::reports::{ReviewItem, ReviewRequest, TrendRequest, trends};

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
                status: "confirmed".into(),
                payload: observation_payload(),
            },
            ReviewItem {
                observation_id: None,
                expected_revision: None,
                status: "pending".into(),
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
                    status: "confirmed".into(),
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
                status: "confirmed".into(),
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
                    status: "pending".into(),
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
        ("confirmed", (1, 0)),
        ("pending", (0, 1)),
        ("confirmed", (1, 0)),
        ("rejected", (0, 1)),
        ("confirmed", (1, 0)),
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
                status: status.into(),
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
        assert_eq!(document["observations"][0]["status"], status);
        assert_eq!(document["observations"][0]["observation_id"], id);
        assert_eq!(document["observations"][0]["payload"]["raw_result"], "120");
        let history = ldl_history(&state.database, &user.user_id).await;
        let points = history["points"].as_array().unwrap();
        if status == "confirmed" {
            assert_eq!(points.len(), 1);
            assert_eq!(points[0]["observation_id"], id);
        } else {
            assert!(points.is_empty());
            assert!(history["records"].as_array().unwrap().is_empty());
        }
        assert_eq!(
            state.database.confirmed(&user.user_id).await.unwrap().len(),
            usize::from(status == "confirmed")
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
                observations: ["pending", "confirmed"]
                    .into_iter()
                    .map(|status| ReviewItem {
                        observation_id: None,
                        expected_revision: None,
                        status: status.into(),
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
async fn rescan_replaces_results_preserves_original_and_rejects_stale_or_active_requests() {
    let (_directory, state) = fixture().await;
    let alice = add_user(&state, "alice").await;
    let bob = add_user(&state, "bob").await;
    let report = archive(&state, "alice").await;
    confirm(&state, &alice.user_id, &report).await;
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
                    status: "pending".into(),
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
    assert!(
        state
            .database
            .observations(&alice.user_id, &report)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        state
            .database
            .confirmed(&alice.user_id)
            .await
            .unwrap()
            .is_empty()
    );
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
                observations: vec![unsupported, mapped, unknown, undated, qualitative]
                    .into_iter()
                    .map(|payload| ReviewItem {
                        observation_id: None,
                        expected_revision: None,
                        status: "pending".into(),
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
                    status: "confirmed".into(),
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
        ("confirmed", "2026-08-15"),
        ("pending", "2026-09-01T23:30:00-05:00"),
        ("rejected", "2026-10-01"),
    ]
    .into_iter()
    .map(|(status, date)| {
        let mut payload = observation_payload();
        payload.sampled_at = Some(date.into());
        ReviewItem {
            observation_id: None,
            expected_revision: None,
            status: status.into(),
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
                    status: "pending".into(),
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
            status: "confirmed".into(),
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
