use crate::{add_user, archive, confirm, fixture};
use longlivehealth::reports::ReviewStatus;

#[tokio::test]
async fn analysis_uses_confirmed_snapshot_and_becomes_stale_after_correction() {
    let (_directory, mut state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let report = archive(&state, "alice").await;
    let observation = confirm(&state, &user.user_id, &report).await;
    let (address, server) = super::provider::mock().await;
    let mut config = (*state.config).clone();
    config.analysis_enabled = true;
    config.llm.base_url = address;
    state.config = std::sync::Arc::new(config);
    let scope = longlivehealth::analysis::AnalysisRequest {
        start_date: "2026-08-01".into(),
        end_date: "2026-08-02".into(),
        timezone: "UTC".into(),
    };
    let run = longlivehealth::analysis::request_analysis(&state, &user.user_id, scope.clone())
        .await
        .unwrap();
    assert_eq!(
        longlivehealth::analysis::request_analysis(&state, &user.user_id, scope)
            .await
            .unwrap(),
        run
    );
    longlivehealth::analysis::analyze_next(&state)
        .await
        .unwrap();
    assert_eq!(
        state.database.analysis(&user.user_id, &run).await.unwrap()["status"],
        "ready"
    );
    let result = state.database.analysis(&user.user_id, &run).await.unwrap();
    assert_eq!(
        result["input"]["algorithm_version"],
        longlivehealth::health::ALGORITHM_VERSION
    );
    for aggregate in result["input"]["health"].as_object().unwrap().values() {
        assert_eq!(
            aggregate["algorithm_version"],
            result["input"]["algorithm_version"]
        );
    }
    state
        .database
        .analysis_feedback(&user.user_id, &run, "Will discuss with clinician")
        .await
        .unwrap();
    let mut changed = crate::observation_payload();
    changed.raw_result = "110".into();
    state
        .database
        .review(
            &user.user_id,
            longlivehealth::reports::ReviewRequest {
                report_id: report,
                expected_revision: 2,
                context: None,
                observations: vec![longlivehealth::reports::ReviewItem {
                    observation_id: Some(observation),
                    expected_revision: Some(1),
                    status: ReviewStatus::Confirmed,
                    payload: changed,
                }],
            },
        )
        .await
        .unwrap();
    let stale = state.database.analysis(&user.user_id, &run).await.unwrap();
    assert_eq!(stale["status"], "stale");
    assert!(stale["output"].is_null());
    server.abort();
}

#[tokio::test]
async fn report_confirmation_persists_one_background_snapshot_across_restart() {
    use axum::http::StatusCode;
    use longlivehealth::{analysis::prepare_next_report_analysis, app::create_application};
    use serde_json::json;

    let (_directory, mut state) = fixture().await;
    add_user(&state, "alice").await;
    let report = archive(&state, "alice").await;
    let token = crate::token(&state, "alice").await;
    let mut config = (*state.config).clone();
    config.analysis_enabled = true;
    state.config = std::sync::Arc::new(config.clone());
    let router = create_application(state.clone());
    let change = |status: &str| json!({"status":status,"payload":crate::observation_payload()});
    let (status, draft) = crate::request(
        &router,
        "/api/v1/reports/review",
        Some(&token),
        json!({"report_id":report,"expected_revision":1,"observations":[change("pending")]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{draft}");
    assert!(draft.get("analysis_trigger").is_none());
    assert!(!prepare_next_report_analysis(&state).await.unwrap());

    let (status, rejected) = crate::request(
        &router,
        "/api/v1/reports/review",
        Some(&token),
        json!({"report_id":report,"expected_revision":1,"observations":[change("confirmed")]}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{rejected}");
    assert!(!prepare_next_report_analysis(&state).await.unwrap());

    let (status, single) = crate::request(
        &router,
        "/api/v1/reports/review",
        Some(&token),
        json!({"report_id":report,"expected_revision":2,"observations":[change("confirmed")]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{single}");
    assert_eq!(single["analysis_trigger"]["status"], "queued");

    let (status, batch) = crate::request(&router, "/api/v1/reports/review", Some(&token),
        json!({"report_id":report,"expected_revision":3,"observations":[change("confirmed"),change("confirmed")]})).await;
    assert_eq!(status, StatusCode::OK, "{batch}");
    assert_eq!(batch["analysis_trigger"]["status"], "queued");
    let observations = batch["observations"].as_array().unwrap();
    assert_eq!(
        observations
            .iter()
            .filter(|item| item["status"] == "confirmed")
            .count(),
        3
    );
    assert_eq!(
        observations
            .iter()
            .filter(|item| item["status"] == "pending")
            .count(),
        1
    );

    let (_, before) =
        crate::request(&router, "/api/v1/analysis/list", Some(&token), json!({})).await;
    assert_eq!(before["runs"], json!([]));
    drop(router);
    state.shutdown().await.unwrap();
    drop(state);

    let state = longlivehealth::app::AppState::open(config).await.unwrap();
    assert!(prepare_next_report_analysis(&state).await.unwrap());
    assert!(!prepare_next_report_analysis(&state).await.unwrap());
    let router = create_application(state);
    let (status, after) =
        crate::request(&router, "/api/v1/analysis/list", Some(&token), json!({})).await;
    assert_eq!(status, StatusCode::OK, "{after}");
    let runs = after["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    let (_, run) = crate::request(
        &router,
        "/api/v1/analysis/get",
        Some(&token),
        json!({"run_id":runs[0]["run_id"]}),
    )
    .await;
    assert_eq!(run["input"]["observations"].as_array().unwrap().len(), 3);
}

#[test]
fn nonexistent_evidence_cannot_be_published() {
    let output:longlivehealth::analysis::AnalysisOutput=serde_json::from_value(serde_json::json!({"summary":"A concern","findings":[{"topic":"lipid_risk","title":"Concern","hypothesis":"Something","observation_ids":["invented"],"evidence_source_ids":["fake"],"other_explanations":["unknown"],"missing_information":["unknown"],"questions_for_clinician":["why"]}]})).unwrap();
    assert!(
        longlivehealth::analysis::validate_output(
            &output,
            &serde_json::json!({"observations":[],"evidence":[]})
        )
        .is_err()
    );
}

#[test]
fn provider_projection_preserves_lab_values_and_all_daily_sources() {
    use serde_json::json;
    let input = json!({"provider":{"api_key":"must-not-be-forwarded"},"evidence":[
        {"id":"observation:one:1","kind":"confirmed_report_observation","value":{
            "observation_id":"one","report_id":"report","resolved_metric_id":"apob",
            "payload":{"raw_name":"ApoB","raw_result":"<120","raw_unit":"mg/dL","sampled_at":"2026-07-30","reference_range":"<90","report_flag":"Above Range","notes":"calc","source":{"quote":"repeated report text"}},
            "interpretation":{"result":{"value":120}}}},
        {"id":"daily:date:steps","kind":"source_separated_daily_summary","value":{
            "record_type":"steps","current":{"state":"source_selection_required"},"history":[
                {"date":"2026-10-20","state":"source_selection_required","selected":null,"alternatives":[
                    {"source":"watch","value":100,"unit":"count","sample_count":5,"value_status":"available","algorithm":"sum","excluded_sample_count":0},
                    {"source":"phone","value":90,"unit":"count","sample_count":4,"value_status":"available","algorithm":"sum","excluded_sample_count":1}]}],"chart_ranges":{"duplicated":true}}}]});
    let result = longlivehealth::analysis::model_input(&input);
    assert!(result.get("provider").is_none());
    assert_eq!(result["flagged_laboratory_evidence_ids"], json!(["e1"]));
    let lab = &result["laboratory_rows"][0];
    assert_eq!(lab[0], "e1");
    assert_eq!(lab[2], "<120");
    assert_eq!(lab[5], "<90");
    assert_eq!(lab[3], "mg/dL");
    assert_eq!(lab[4], "2026-07-30");
    assert_eq!(lab[6], "Above Range");
    let daily = &result["evidence"][0]["value"];
    let summaries = daily["history_summary"].as_array().unwrap();
    let watch = summaries.iter().find(|v| v["source"] == "watch").unwrap();
    let phone = summaries.iter().find(|v| v["source"] == "phone").unwrap();
    assert_eq!(watch["daily_median"], 100.0);
    assert_eq!(phone["daily_median"], 90.0);
    assert_eq!(phone["excluded_samples"], 1);
    assert_eq!(daily["current"]["state"], "source_selection_required");
    assert!(daily.get("chart_ranges").is_none());
    assert_eq!(
        input["evidence"][0]["value"]["payload"]["raw_result"],
        "<120"
    );
}

#[test]
fn short_model_citations_restore_only_exact_snapshot_identifiers() {
    use longlivehealth::wellness::coach::{Output, restore_citations};
    use serde_json::json;
    let input = json!({"evidence":[{"id":"observation:real:2"},{"id":"daily:2026-10-20:steps"}]});
    for (ids, valid) in [
        (json!(["e1", "e2"]), true),
        (json!(["e0"]), false),
        (json!(["e3"]), false),
        (json!(["e01"]), false),
        (json!(["invented"]), false),
    ] {
        let mut output:Output=serde_json::from_value(json!({"claims":[{"text":"A fact","evidence_ids":ids}],"missing_information":[],"questions":[],"drafts":[]})).unwrap();
        assert_eq!(restore_citations(&mut output, &input).is_ok(), valid);
        if valid {
            assert_eq!(
                output.claims[0].evidence_ids,
                vec!["observation:real:2", "daily:2026-10-20:steps"]
            );
        }
    }
}

#[test]
fn clinical_source_identifiers_are_not_replaced_by_archive_aliases() {
    let input = serde_json::json!({"evidence":longlivehealth::analysis::evidence()});
    assert_eq!(
        longlivehealth::analysis::model_input(&input)["evidence"],
        input["evidence"]
    );
}

#[test]
fn complete_coach_findings_are_not_rejected_by_claim_count() {
    use longlivehealth::wellness::coach::{Output, validate};
    use serde_json::json;
    let input = json!({"evidence":[{"id":"real"}],"allow_drafts":false});
    let claims: Vec<_> = (0..22)
        .map(|i| json!({"text":format!("Finding {i}"),"evidence_ids":["real"]}))
        .collect();
    let mut output: Output = serde_json::from_value(
        json!({"claims":claims,"missing_information":[],"questions":[],"drafts":[]}),
    )
    .unwrap();
    assert!(validate(&output, &input).is_ok());
    output.claims[21].evidence_ids = vec!["invented".into()];
    assert!(validate(&output, &input).is_err());
}
