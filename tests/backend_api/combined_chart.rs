use crate::{
    add_user, fixture,
    health_batch::{batch, record},
};
use longlivehealth::{
    health::ConnectionRequest,
    wellness::{component_projection, measurement_chart::Request},
};
use serde_json::json;

#[tokio::test]
async fn paired_chart_resolves_exact_children_and_preserves_period_and_account_boundaries() {
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
    for (kind, id, payload) in [
        (
            "blood_pressure_systolic",
            "sys",
            json!({"value":120,"unit":"mmHg"}),
        ),
        (
            "blood_pressure_diastolic",
            "dia",
            json!({"value":80,"unit":"mmHg"}),
        ),
        (
            "blood_pressure_systolic",
            "unrelated",
            json!({"value":200,"unit":"mmHg"}),
        ),
        (
            "HKCorrelationTypeIdentifierBloodPressure",
            "pair",
            json!({"related_sample_ids":["sys","dia","sys","missing"]}),
        ),
    ] {
        let mut r = record(id, payload);
        r.record_type = kind.into();
        let mut b = batch(&connection.connection_id, vec![r]);
        b.record_type = kind.into();
        state.database.sync_health(&user.user_id, b).await.unwrap();
    }
    let history = state
        .database
        .specialty_records(
            &user.user_id,
            longlivehealth::wellness::records::ListRequest {
                kind: "HKCorrelationTypeIdentifierBloodPressure".into(),
                after: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(history["records"].as_array().unwrap().len(), 2);
    let pair = history["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["record_id"] == "pair")
        .unwrap();
    assert_eq!(pair["values"].as_array().unwrap().len(), 3); // duplicated raw relation is preserved in evidence
    for period in ["H", "D", "W", "M", "6M", "Y"] {
        let request = || Request {
            record_type: "HKCorrelationTypeIdentifierBloodPressure".into(),
            timezone: "UTC".into(),
            period: period.into(),
            start_at: 0,
            end_at: 86400,
        };
        let result = state
            .database
            .measurement_chart(&user.user_id, request())
            .await
            .unwrap();
        assert_eq!(result["components"].as_array().unwrap().len(), 2);
        assert_eq!(
            result["latest_reading"]["values"]["blood_pressure_systolic"],
            200.0
        );
        assert!(
            result["latest_reading"]["values"]["blood_pressure_diastolic"].is_null(),
            "An unpaired systolic record must not borrow another record's diastolic value"
        );
        assert_eq!(result["valid_samples"], 3);
        assert_eq!(result["excluded_samples"], 1);
        assert!(
            state
                .database
                .measurement_chart(&other.user_id, request())
                .await
                .unwrap()["components"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let mut outside = request();
        outside.start_at = 2;
        assert!(
            state
                .database
                .measurement_chart(&user.user_id, outside)
                .await
                .unwrap()["components"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn component_projection_uses_explicit_units_and_excludes_heartbeat_gaps() {
    let beats = json!({"series":[{"time_since_start":0},{"time_since_start":0.8},{"time_since_start":10,"preceded_by_gap":true},{"time_since_start":11}]});
    assert_eq!(
        component_projection::project("heartbeat_series", &beats, 0.0, 11.0)[0]["value"],
        900.0
    );
    assert_eq!(
        component_projection::project("activity_summary", &json!({"activeEnergy":100}), 0.0, 1.0),
        json!([])
    );
    let workout = component_projection::project(
        "workout",
        &json!({"duration_seconds":120,"energy_kcal":12,"distance_m":500}),
        0.0,
        120.0,
    );
    assert_eq!(workout.as_array().unwrap().len(), 3);
    assert_eq!(workout[0]["value"], 2.0);
    assert!(!component_projection::supports("electrocardiogram"));
}

#[tokio::test]
async fn manual_pressure_uses_the_same_paired_chart_and_history() {
    let (_dir, state) = fixture().await;
    let user = add_user(&state, "manual").await;
    let request=serde_json::from_value(json!({"record_id":uuid::Uuid::new_v4().to_string(),"batch_id":uuid::Uuid::new_v4().to_string(),"version":1,"at":100,"timezone":"UTC","entry":{"kind":"blood_pressure","content":{"systolic_mmhg":125,"diastolic_mmhg":82,"pulse_bpm":60,"posture":"seated","arm":"left","note":""}}})).unwrap();
    state
        .database
        .save_wellness_entry(&user.user_id, request)
        .await
        .unwrap();
    let result = state
        .database
        .measurement_chart(
            &user.user_id,
            Request {
                record_type: "HKCorrelationTypeIdentifierBloodPressure".into(),
                timezone: "UTC".into(),
                period: "D".into(),
                start_at: 0,
                end_at: 86400,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        result["latest_reading"]["values"]["blood_pressure_systolic"],
        125.0
    );
    assert_eq!(
        result["latest_reading"]["values"]["blood_pressure_diastolic"],
        82.0
    );
    let history = state
        .database
        .specialty_records(
            &user.user_id,
            longlivehealth::wellness::records::ListRequest {
                kind: "HKCorrelationTypeIdentifierBloodPressure".into(),
                after: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(history["records"].as_array().unwrap().len(), 1);
    assert_eq!(
        history["records"][0]["entry"]["content"]["systolic_mmhg"],
        125.0
    );
}
