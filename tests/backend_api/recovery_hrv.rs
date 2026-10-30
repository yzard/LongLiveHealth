use longlivehealth::{
    health::ConnectionRequest,
    recovery_hrv::{KIND, projection},
    wellness::measurement_chart::{HistoryRequest, Request},
};
use serde_json::json;

#[test]
fn recovery_hrv_requires_a_finite_value_and_explicit_time_unit() {
    assert_eq!(
        projection(&json!({"raw_quantity":"42.125 ms"})),
        Some(json!({"value":42.125,"unit":"ms"}))
    );
    assert_eq!(
        projection(&json!({"raw_quantity":"0.042125 s"})),
        Some(json!({"value":42.125,"unit":"ms"}))
    );
    for raw in [
        "",
        "42",
        "42 bpm",
        "NaN ms",
        "inf ms",
        "-1 ms",
        "42 ms extra",
        "1e309 ms",
    ] {
        assert!(projection(&json!({"raw_quantity":raw})).is_none(), "{raw}");
    }
}

#[tokio::test]
async fn recovery_hrv_projects_new_and_archived_records_without_replay_conflicts() {
    let (_directory, state) = crate::fixture().await;
    let user = crate::add_user(&state, "recovery").await;
    let other = crate::add_user(&state, "other-recovery").await;
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
    let make = || {
        let mut records = Vec::new();
        for (id, kind, raw) in [
            ("recovery", KIND, "42.125 ms"),
            ("invalid", KIND, "unknown ms"),
        ] {
            let mut record = crate::health_batch::record(
                id,
                json!({"raw_quantity":raw,"raw_archive":{"format":"nskeyedarchiver-secure-v1","data":"synthetic"}}),
            );
            record.record_type = kind.into();
            records.push(record);
        }
        {
            let mut batch = crate::health_batch::batch(&connection.connection_id, records);
            batch.record_type = KIND.into();
            batch
        }
    };
    state
        .database
        .sync_health(&user.user_id, make())
        .await
        .unwrap();
    let chart = |user_id: String| {
        let db = &state.database;
        async move {
            db.measurement_chart(
                &user_id,
                Request {
                    record_type: KIND.into(),
                    timezone: "UTC".into(),
                    period: "D".into(),
                    start_at: 0,
                    end_at: 86400,
                },
            )
            .await
            .unwrap()
        }
    };
    let result = chart(user.user_id.clone()).await;
    assert_eq!(result["samples_read"], 2);
    assert_eq!(result["valid_samples"], 1);
    assert_eq!(result["excluded_samples"], 1);
    assert_eq!(chart(other.user_id.clone()).await["samples_read"], 0);
    let pool = sqlx::SqlitePool::connect(&format!(
        "sqlite://{}",
        state.config.data_dir.join("database.sqlite").display()
    ))
    .await
    .unwrap();
    let before: String=sqlx::query_scalar("SELECT json_extract(payload_json,'$.content_hash') FROM health_records WHERE user_id=? AND record_id='recovery'").bind(&user.user_id).fetch_one(&pool).await.unwrap();
    // Recreate the projection written by the older server; immutable original remains.
    sqlx::query("UPDATE health_records SET payload_json=json_remove(payload_json,'$.payload.value','$.payload.unit') WHERE user_id=? AND record_id='recovery'").bind(&user.user_id).execute(&pool).await.unwrap();
    assert_eq!(state.database.index_recovery_hrv().await.unwrap(), 1);
    assert_eq!(state.database.index_recovery_hrv().await.unwrap(), 0);
    let after: String=sqlx::query_scalar("SELECT json_extract(payload_json,'$.content_hash') FROM health_records WHERE user_id=? AND record_id='recovery'").bind(&user.user_id).fetch_one(&pool).await.unwrap();
    assert_eq!(before, after);
    state
        .database
        .sync_health(&user.user_id, make())
        .await
        .unwrap();
    let history = state
        .database
        .measurement_history(
            &user.user_id,
            HistoryRequest {
                record_type: KIND.into(),
                start_at: 0,
                end_at: 86400,
                offset: 0,
                expected_revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(history["points"].as_array().unwrap().len(), 2);
    assert!(
        history["points"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["value"] == 42.125 && p["unit"] == "ms")
    );
    assert!(
        history["points"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["record_id"] == "invalid" && p["value"].is_null())
    );
    pool.close().await;
}
