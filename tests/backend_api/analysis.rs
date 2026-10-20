use crate::{add_user, archive, confirm, fixture};

#[tokio::test]
async fn analysis_uses_confirmed_snapshot_and_becomes_stale_after_correction() {
    let (_directory, mut state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let report = archive(&state, "alice").await;
    let observation = confirm(&state, &user.user_id, &report).await;
    let (address, server) = super::provider::mock().await;
    let mut config = (*state.config).clone();
    config.providers.analysis.enabled = true;
    config.providers.analysis.base_url = address;
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
                    status: "confirmed".into(),
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
