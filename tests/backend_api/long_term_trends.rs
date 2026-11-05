use crate::{
    add_user, fixture,
    health_batch::{batch, record},
};
use longlivehealth::{health::ConnectionRequest, wellness::trends};
use serde_json::json;

#[tokio::test]
async fn long_term_trends_preserve_complete_history_explicit_pairs_and_account_scope() {
    let (_dir, state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let other = add_user(&state, "bob").await;
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
    state.database.save_wellness_preferences(&user.user_id, serde_json::from_value(json!({
        "expected_version":0,"batch_id":uuid::Uuid::new_v4().to_string(),
        "preferences":{"reference_profile":{"birth_date":"1980-01-01","reference_sex":"male"},"source_aliases":{},"source_priority":{},"favorite_metrics":[],"overview_show_all":true,"sleep_target_minutes":420}
    })).unwrap()).await.unwrap();
    let now = longlivehealth::authentication::now().unwrap();
    let mut records = Vec::new();
    for (index, at) in [now - 3 * 366 * 86400, now - 366 * 86400, now - 3 * 86400]
        .into_iter()
        .enumerate()
    {
        for (kind, id, payload) in [
            (
                "vo2_max",
                format!("fitness-{index}"),
                json!({"value":30+index*5,"unit":"ml/kg*min"}),
            ),
            (
                "blood_pressure_systolic",
                format!("sys-{index}"),
                json!({"value":120+index*15,"unit":"mmHg"}),
            ),
            (
                "blood_pressure_diastolic",
                format!("dia-{index}"),
                json!({"value":80+index*5,"unit":"mmHg"}),
            ),
            (
                "HKCorrelationTypeIdentifierBloodPressure",
                format!("pair-{index}"),
                json!({"related_sample_ids":[format!("sys-{index}"),format!("dia-{index}")]}),
            ),
        ] {
            let mut r = record(&id, payload);
            r.record_type = kind.into();
            r.start_at = at;
            r.end_at = at + 1;
            records.push(r);
        }
    }
    let mut r = record("unpaired", json!({"value":199,"unit":"mmHg"}));
    r.record_type = "blood_pressure_systolic".into();
    r.start_at = now - 3 * 86400;
    r.end_at = now - 3 * 86400 + 1;
    records.push(r);
    let mut groups = std::collections::BTreeMap::<String, Vec<_>>::new();
    for record in records {
        groups
            .entry(record.record_type.clone())
            .or_default()
            .push(record);
    }
    for (kind, records) in groups {
        let mut input = batch(&connection.connection_id, records);
        input.record_type = kind;
        state
            .database
            .sync_health(&user.user_id, input)
            .await
            .unwrap();
    }
    let result = trends::get(
        &state.database,
        &user.user_id,
        trends::Request {
            timezone: "America/New_York".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(result["complete"], true);
    let cards = result["cards"].as_array().unwrap();
    let vo2 = cards.iter().find(|v| v["metric_id"] == "vo2_max").unwrap();
    assert_eq!(vo2["state"], "ready");
    assert_eq!(vo2["observed_days"], 3);
    assert_eq!(vo2["series"][0]["points"].as_array().unwrap().len(), 3);
    let bp = cards
        .iter()
        .find(|v| v["title"] == "Blood Pressure")
        .unwrap();
    assert_eq!(bp["change"]["direction"], "worsening");
    assert_eq!(vo2["change"]["direction"], "improving");
    assert_eq!(
        cards[0]["metric_id"],
        "HKCorrelationTypeIdentifierBloodPressure"
    );
    assert_eq!(bp["state"], "ready");
    assert_eq!(bp["observed_days"], 3);
    assert_eq!(bp["series"].as_array().unwrap().len(), 2);
    assert!(!bp["missing"].as_array().unwrap().is_empty());
    for series in bp["series"].as_array().unwrap() {
        assert!(
            series["points"]
                .as_array()
                .unwrap()
                .iter()
                .all(|p| p["value"].as_f64().unwrap() < 180.)
        );
    }
    let empty = trends::get(
        &state.database,
        &other.user_id,
        trends::Request {
            timezone: "UTC".into(),
        },
    )
    .await
    .unwrap();
    assert!(
        empty["cards"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["state"] == "insufficient_data")
    );
    assert!(
        trends::get(
            &state.database,
            &user.user_id,
            trends::Request {
                timezone: "invalid".into()
            }
        )
        .await
        .is_err()
    );
}
