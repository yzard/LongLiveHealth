use serde_json::json;

#[tokio::test]
async fn aliases_are_owned_versioned_exportable_and_do_not_rename_report_sources() {
    let (_directory, state) = crate::fixture().await;
    let user = crate::add_user(&state, "alias-owner").await;
    let other = crate::add_user(&state, "alias-other").await;
    let report = crate::archive(&state, "alias-owner").await;
    state.database.review(&user.user_id, serde_json::from_value(json!({"report_id":report,"expected_revision":1,"context":{"source_name":"Function Health"},"observations":[]})).unwrap()).await.unwrap();
    let key = "report-source:function health";
    let catalog = state
        .database
        .wellness_sources(&user.user_id)
        .await
        .unwrap();
    assert_eq!(catalog["catalog"][0]["source_key"], key);
    assert_eq!(catalog["catalog"][0]["name"], "Function Health");
    let initial = state
        .database
        .wellness_preferences(&user.user_id)
        .await
        .unwrap();
    let mut preferences = initial["preferences"].clone();
    preferences["source_aliases"] = json!({key:"Annual labs","apple_health:watch":"Annual labs"});
    preferences["sleep_target_minutes"] = json!(480);
    let request =
        json!({"expected_version":0,"batch_id":uuid::Uuid::new_v4(),"preferences":preferences});
    let saved = state
        .database
        .save_wellness_preferences(
            &user.user_id,
            serde_json::from_value(request.clone()).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(saved["preferences"]["source_aliases"][key], "Annual labs");
    assert_eq!(
        state
            .database
            .wellness_preferences(&other.user_id)
            .await
            .unwrap()["preferences"]["source_aliases"],
        json!({})
    );
    assert_eq!(
        state
            .database
            .wellness_sources(&other.user_id)
            .await
            .unwrap()["catalog"],
        json!([])
    );
    let raw_path = state
        .database
        .file(&user.user_id, &report)
        .await
        .unwrap()
        .relative_path;
    assert!(raw_path.contains("/function_health/"));
    assert_eq!(
        state
            .database
            .wellness_sources(&user.user_id)
            .await
            .unwrap()["catalog"],
        catalog["catalog"]
    );
    let mut stale = request.clone();
    stale["batch_id"] = json!(uuid::Uuid::new_v4());
    stale["preferences"]["source_aliases"][key] = json!("stale");
    assert!(
        state
            .database
            .save_wellness_preferences(&user.user_id, serde_json::from_value(stale).unwrap())
            .await
            .is_err()
    );
    let mut clear = saved.clone();
    clear["expected_version"] = clear["version"].take();
    clear.as_object_mut().unwrap().remove("version");
    clear["batch_id"] = json!(uuid::Uuid::new_v4());
    clear["preferences"]["source_aliases"] = json!({});
    let cleared = state
        .database
        .save_wellness_preferences(&user.user_id, serde_json::from_value(clear).unwrap())
        .await
        .unwrap();
    assert_eq!(cleared["preferences"]["source_aliases"], json!({}));
    assert_eq!(cleared["preferences"]["sleep_target_minutes"], 480);
    for aliases in [
        json!({"unknown:source":"name"}),
        json!({key:""}),
        json!({key:"\n"}),
        json!({key:"x".repeat(101)}),
    ] {
        let mut invalid = request.clone();
        invalid["preferences"]["source_aliases"] = aliases;
        assert!(
            state
                .database
                .save_wellness_preferences(&other.user_id, serde_json::from_value(invalid).unwrap())
                .await
                .is_err()
        );
    }
}
