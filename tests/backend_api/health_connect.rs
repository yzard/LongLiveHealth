use longlivehealth::health::{StoredHealthRecord, aggregate_records};
use serde_json::json;

fn session(stages: serde_json::Value) -> StoredHealthRecord {
    StoredHealthRecord {
        platform: "health_connect".into(),
        source_id: "watch".into(),
        record_id: "session".into(),
        record_type: "sleep".into(),
        start_at: 1767225600,
        end_at: 1767229200,
        version: 1,
        payload_json: json!({"payload":{"stages":stages}}).to_string(),
    }
}

#[test]
fn health_connect_sleep_preserves_awake_unknown_and_all_stages() {
    let rows = vec![session(json!([
        {"start_at":1767225600,"end_at":1767226200,"stage":4},
        {"start_at":1767226200,"end_at":1767226800,"stage":5},
        {"start_at":1767226800,"end_at":1767227400,"stage":6},
        {"start_at":1767227400,"end_at":1767228000,"stage":1},
        {"start_at":1767228000,"end_at":1767228600,"stage":0},
        {"start_at":1767228600,"end_at":1767229200,"stage":2}
    ]))];
    let date = "2026-01-01".parse().unwrap();
    let daily = aggregate_records(rows, "sleep", date, date, chrono_tz::UTC).unwrap();
    assert_eq!(daily["days"][0]["value"], 2400.0);
    let detail = longlivehealth::wellness::sleep::summarize(
        vec![session(json!([
            {"start_at":1767225600,"end_at":1767227400,"stage":4},
            {"start_at":1767227400,"end_at":1767229200,"stage":1}
        ]))],
        1767220000,
        1767230000,
        chrono_tz::UTC,
    )
    .unwrap();
    assert_eq!(detail["sources"][0]["sessions"][0]["asleep_seconds"], 1800);
    assert_eq!(
        detail["sources"][0]["sessions"][0]["observed_awake_seconds"],
        1800
    );
}

#[test]
fn health_connect_missing_sleep_stages_are_not_fabricated_as_sleep() {
    let detail = longlivehealth::wellness::sleep::summarize(
        vec![session(json!([]))],
        1767220000,
        1767230000,
        chrono_tz::UTC,
    )
    .unwrap();
    assert!(detail["sources"][0]["sessions"][0]["asleep_seconds"].is_null());
    assert_eq!(
        detail["sources"][0]["sessions"][0]["state"],
        "insufficient_data"
    );
}

#[test]
fn health_connect_invalid_stage_range_is_rejected() {
    let date = "2026-01-01".parse().unwrap();
    assert!(
        aggregate_records(
            vec![session(json!([
                {"start_at":1767225599,"end_at":1767226200,"stage":4}
            ]))],
            "sleep",
            date,
            date,
            chrono_tz::UTC
        )
        .is_err()
    );
}
