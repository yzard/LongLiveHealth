use longlivehealth::{
    health::StoredHealthRecord,
    wellness::steps::{combined_day, device_priority},
};
use serde_json::json;

fn sample(source: &str, from: i64, to: i64, value: f64) -> StoredHealthRecord {
    StoredHealthRecord {
        platform: "apple_health".into(),
        source_id: source.into(),
        record_id: format!("{source}-{from}-{to}"),
        record_type: "steps".into(),
        start_at: from,
        end_at: to,
        version: 1,
        payload_json: json!({"payload":{"source_name":source,"value":value,"unit":"count"}})
            .to_string(),
    }
}

#[test]
fn steps_add_morning_watch_afternoon_band_and_evening_watch() {
    let records = vec![
        sample("Apple Watch", 0, 100, 1000.0),
        sample("Fitbit", 100, 200, 2000.0),
        sample("Apple Watch", 200, 300, 3000.0),
    ];
    let result = combined_day(&records, 0, 300).unwrap();
    assert_eq!(result["value"], 6000.0);
    assert_eq!(result["contributions"]["apple_health:Apple Watch"], 4000.0);
    assert_eq!(result["contributions"]["apple_health:Fitbit"], 2000.0);
}

#[test]
fn steps_priority_only_replaces_overlapping_parts_and_is_order_independent() {
    let mut records = vec![
        sample("iPhone", 0, 400, 400.0),
        sample("Fitbit", 50, 350, 600.0),
        sample("Apple Watch", 100, 200, 300.0),
        sample("Apple Watch", 250, 300, 150.0),
    ];
    let result = combined_day(&records, 0, 400).unwrap();
    // Phone 0..50 + 350..400; band 50..100 + 200..250 + 300..350; watch 150 seconds.
    assert_eq!(result["value"], 850.0);
    assert_eq!(result["contributions"]["apple_health:iPhone"], 100.0);
    assert_eq!(result["contributions"]["apple_health:Fitbit"], 300.0);
    records.reverse();
    assert_eq!(result, combined_day(&records, 0, 400).unwrap());
    // Day/window boundaries proportionally clip original samples, not their rates.
    assert_eq!(combined_day(&records, 75, 125).unwrap()["value"], 125.0);
}

#[test]
fn steps_ties_are_deterministic_same_source_samples_sum_and_missing_is_not_zero() {
    let records = vec![
        sample("Fitbit A", 0, 100, 100.0),
        sample("Fitbit B", 0, 100, 200.0),
        sample("Fitbit A", 50, 100, 50.0),
    ];
    assert_eq!(combined_day(&records, 0, 100).unwrap()["value"], 150.0);
    assert!(combined_day(&records, 100, 200).unwrap()["value"].is_null());
    assert_eq!(
        combined_day(&[sample("iPhone", 0, 100, 0.0)], 0, 100).unwrap()["value"],
        0.0
    );
    assert!(combined_day(&[sample("iPhone", 0, 100, -1.0)], 0, 100).unwrap()["value"].is_null());
}

#[test]
fn steps_device_metadata_beats_app_name_and_phone_is_last() {
    assert_eq!(
        device_priority(
            &json!({"source_name":"Health","device":{"manufacturer":"Apple Inc.","model":"Watch"}})
        ),
        0
    );
    assert_eq!(
        device_priority(&json!({"source_name":"Apple Watch","device":{"model":"iPhone"}})),
        3
    );
    for name in [
        "Google Fitbit",
        "华为手表",
        "Polar chest strap",
        "Garmin watch",
    ] {
        assert_eq!(device_priority(&json!({"source_name":name})), 1);
    }
    assert_eq!(device_priority(&json!({"source_name":"Unknown"})), 2);
    let records = vec![
        sample("Unknown", 0, 100, 200.0),
        sample("iPhone", 0, 100, 500.0),
    ];
    assert_eq!(combined_day(&records, 0, 100).unwrap()["value"], 200.0);
}

#[test]
fn steps_aggregate_preserves_source_charts_and_combines_local_dst_day() {
    use chrono::{NaiveDate, TimeZone};
    use longlivehealth::health::aggregate_records;
    let timezone = chrono_tz::America::New_York;
    let start = timezone
        .with_ymd_and_hms(2026, 3, 8, 0, 0, 0)
        .unwrap()
        .timestamp();
    let end = timezone
        .with_ymd_and_hms(2026, 3, 9, 0, 0, 0)
        .unwrap()
        .timestamp();
    assert_eq!(end - start, 23 * 3600);
    let records = vec![
        sample("Apple Watch", start, start + 3600, 1000.0),
        sample("Fitbit", start + 3600, end, 22000.0),
        sample("iPhone", start, end, 46000.0),
    ];
    let day = NaiveDate::from_ymd_opt(2026, 3, 8).unwrap();
    let result = aggregate_records(records, "steps", day, day, timezone).unwrap();
    assert_eq!(result["combined_days"][0]["value"], 23000.0);
    assert_eq!(result["days"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn steps_daily_view_reads_device_names_and_replaces_old_cached_algorithm() {
    use longlivehealth::{
        health::{ConnectionRequest, HealthRecordInput, SyncRequest},
        wellness::DayRequest,
    };
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    let (_directory, state) = crate::fixture().await;
    let user = crate::add_user(&state, "steps-intervals").await;
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
    let start = longlivehealth::reports::parse_time("2026-10-05").unwrap();
    let records = [
        ("z-watch", "Apple Watch", 0, 100, 1000.0),
        ("b-band", "Fitbit", 100, 200, 2000.0),
        ("a-phone", "iPhone", 0, 300, 300.0),
    ]
    .into_iter()
    .map(|(id, name, from, to, value)| HealthRecordInput {
        record_id: id.into(),
        source_id: id.into(),
        record_type: "steps".into(),
        start_at: start + from,
        end_at: start + to,
        version: 1,
        deleted: false,
        payload: json!({"source_name":name,"value":value,"unit":"count"}),
    })
    .collect();
    state
        .database
        .sync_health(
            &user.user_id,
            SyncRequest {
                connection_id: connection.connection_id,
                batch_id: uuid::Uuid::new_v4().to_string(),
                record_type: "steps".into(),
                coverage_status: "observed".into(),
                records,
            },
        )
        .await
        .unwrap();
    let request = || DayRequest {
        date: "2026-10-05".into(),
        timezone: "UTC".into(),
        source_priority: Default::default(),
    };
    let first = state
        .database
        .wellness_day(&user.user_id, request())
        .await
        .unwrap();
    let steps = first["metrics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|metric| metric["record_type"] == "steps")
        .unwrap();
    assert_eq!(steps["current"]["combined"]["value"], 3100.0);
    let mut db = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(state.config.data_dir.join("database.sqlite")),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE daily_views SET result_json=json_set(result_json, '$.algorithm_version', 'wellness-day-v1') WHERE user_id=?").bind(&user.user_id).execute(&mut db).await.unwrap();
    let refreshed = state
        .database
        .wellness_day(&user.user_id, request())
        .await
        .unwrap();
    assert_eq!(refreshed["algorithm_version"], "wellness-day-v5");
    assert_eq!(refreshed["metrics"], first["metrics"]);
}
