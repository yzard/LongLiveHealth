use longlivehealth::{
    apple_health::{browse_catalog, definitions, normalize, projection},
    health::ConnectionRequest,
    wellness::measurement_chart::{HistoryRequest, Request},
};
use serde_json::json;

#[test]
fn apple_units_are_dimensionally_checked_and_percent_is_not_scaled_twice() {
    for (value, from, to, expected) in [
        (1000.0, "mg", "g", 1.0),
        (50.0, "%", "%", 50.0),
        (2.0, "kcal/hr·kg", "kcal/(kg*hr)", 2.0),
        (3.6, "km/hr", "m/s", 1.0),
        (32.0, "degF", "degC", 0.0),
    ] {
        assert!((normalize(value, from, to).unwrap() - expected).abs() < 1e-10);
    }
    assert_eq!(normalize(1.0, "kg", "m"), None);
    assert_eq!(normalize(f64::NAN, "ms", "ms"), None);
    assert_eq!(normalize(101.0, "%", "%"), None);
    assert_eq!(normalize(-10.0, "degC", "degC"), Some(-10.0));
    let kind = "HKQuantityTypeIdentifierWalkingAsymmetryPercentage";
    let p=projection(kind,&json!({"raw_quantity":"2 %","series":[{"start_at":100,"quantity":"1 %"},{"start_at":105,"quantity":"3 %"},{"start_at":110,"quantity":"invalid %"}]})).unwrap();
    assert_eq!(p["value"], 2.0);
    assert_eq!(p["quantity_samples"].as_array().unwrap().len(), 3);
    assert_eq!(p["quantity_samples"][1]["value"], 3.0);
    assert!(p["quantity_samples"][2]["value"].is_null());
}

#[tokio::test]
async fn every_apple_quantity_is_queryable_and_catalog_excludes_administrative_types() {
    let (_directory, state) = crate::fixture().await;
    let user = crate::add_user(&state, "all-apple").await;
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
    for d in definitions().iter().filter(|d| d.family == "quantity") {
        let value = if ["body_fat", "oxygen_saturation"].contains(&d.kind.as_str()) {
            0.5
        } else {
            10.0
        };
        let mut record = crate::health_batch::record(
            &d.kind,
            json!({"value":value,"unit":d.unit,"raw_quantity":format!("10 {}",d.unit),"raw_archive":{"format":"nskeyedarchiver-secure-v1","data":"synthetic"}}),
        );
        record.record_type = d.kind.clone();
        let mut batch = crate::health_batch::batch(&connection.connection_id, vec![record]);
        batch.record_type = d.kind.clone();
        state
            .database
            .sync_health(&user.user_id, batch)
            .await
            .unwrap();
        for period in ["H", "D", "W", "M", "6M", "Y"] {
            let chart = state
                .database
                .measurement_chart(
                    &user.user_id,
                    Request {
                        record_type: d.kind.clone(),
                        timezone: "UTC".into(),
                        period: period.into(),
                        start_at: 0,
                        end_at: 86400,
                    },
                )
                .await
                .unwrap();
            assert_eq!(chart["valid_samples"], 1, "{} {period}", d.kind);
            assert_eq!(chart["complete"], true);
        }
        let history = state
            .database
            .measurement_history(
                &user.user_id,
                HistoryRequest {
                    record_type: d.kind.clone(),
                    start_at: 0,
                    end_at: 86400,
                    offset: 0,
                    expected_revision: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(history["points"].as_array().unwrap().len(), 1, "{}", d.kind);
    }
    let sources = state
        .database
        .wellness_sources(&user.user_id)
        .await
        .unwrap();
    assert_eq!(
        sources["browse_metrics"].as_array().unwrap().len(),
        definitions()
            .iter()
            .filter(|d| d.family == "quantity")
            .count()
    );
    let blocked = browse_catalog(&[
        json!({"platform":"apple_health","record_type":"HKClinicalTypeIdentifierCoverageRecord","record_count":1}),
        json!({"platform":"apple_health","record_type":"specimen","record_count":1}),
    ]);
    assert!(blocked.is_empty());
}

#[tokio::test]
async fn quantity_series_backfill_keeps_each_sample_and_replay_identity() {
    let (_directory, state) = crate::fixture().await;
    let user = crate::add_user(&state, "all-series").await;
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
    let kind = "HKQuantityTypeIdentifierWalkingStepLength";
    let make = || {
        let mut record = crate::health_batch::record(
            "steps-length",
            json!({"raw_quantity":"70 cm","quantity_count":2,"series":[{"start_at":1,"quantity":"60 cm"},{"start_at":2,"quantity":"80 cm"}],"raw_archive":{"format":"nskeyedarchiver-secure-v1","data":"synthetic"}}),
        );
        record.record_type = kind.into();
        record.end_at = 3;
        let mut b = crate::health_batch::batch(&connection.connection_id, vec![record]);
        b.record_type = kind.into();
        b
    };
    state
        .database
        .sync_health(&user.user_id, make())
        .await
        .unwrap();
    let pool = sqlx::SqlitePool::connect(&format!(
        "sqlite://{}",
        state.config.data_dir.join("database.sqlite").display()
    ))
    .await
    .unwrap();
    sqlx::query("UPDATE health_records SET payload_json=json_remove(payload_json,'$.payload.value','$.payload.unit','$.payload.quantity_samples') WHERE user_id=?").bind(&user.user_id).execute(&pool).await.unwrap();
    assert_eq!(
        state.database.index_apple_quantities().await.unwrap()["updated"],
        1
    );
    assert_eq!(
        state.database.index_apple_quantities().await.unwrap()["updated"],
        0
    );
    state
        .database
        .sync_health(&user.user_id, make())
        .await
        .unwrap();
    let result = state
        .database
        .measurement_history(
            &user.user_id,
            HistoryRequest {
                record_type: kind.into(),
                start_at: 0,
                end_at: 86400,
                offset: 0,
                expected_revision: None,
            },
        )
        .await
        .unwrap();
    let p = result["points"].as_array().unwrap();
    assert_eq!(p.len(), 2);
    assert_eq!(p[0]["value"], 0.8);
    assert_eq!(p[1]["value"], 0.6);
    assert_eq!(p[0]["unit"], "m");
    pool.close().await;
}

#[tokio::test]
async fn health_event_history_is_paged_and_account_scoped() {
    use longlivehealth::wellness::records::{Cursor, ListRequest};
    let (_directory, state) = crate::fixture().await;
    let user = crate::add_user(&state, "events").await;
    let other = crate::add_user(&state, "other-events").await;
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
    let kind = "HKCategoryTypeIdentifierAppleStandHour";
    let records = (0..120)
        .map(|i| {
            let mut r =
                crate::health_batch::record(&format!("event-{i:03}"), json!({"category":0}));
            r.record_type = kind.into();
            r
        })
        .collect();
    let mut b = crate::health_batch::batch(&connection.connection_id, records);
    b.record_type = kind.into();
    state.database.sync_health(&user.user_id, b).await.unwrap();
    let mut cursor: Option<Cursor> = None;
    let mut total = 0;
    loop {
        let result = state
            .database
            .specialty_records(
                &user.user_id,
                ListRequest {
                    kind: kind.into(),
                    after: cursor,
                },
            )
            .await
            .unwrap();
        total += result["records"].as_array().unwrap().len();
        if result["next_cursor"].is_null() {
            break;
        }
        cursor = Some(serde_json::from_value(result["next_cursor"].clone()).unwrap());
    }
    assert_eq!(total, 120);
    assert!(
        state
            .database
            .specialty_records(
                &other.user_id,
                ListRequest {
                    kind: kind.into(),
                    after: None
                }
            )
            .await
            .unwrap()["records"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn measurement_bounds_include_manual_weight_but_ignore_deleted_and_unrelated_rows() {
    let (_directory, state) = crate::fixture().await;
    let user = crate::add_user(&state, "bounds").await;
    let pool = sqlx::SqlitePool::connect(&format!(
        "sqlite://{}",
        state.config.data_dir.join("database.sqlite").display()
    ))
    .await
    .unwrap();
    for (id, platform, kind, start, end, deleted, payload) in [
        ("a", "apple_health", "body_mass", 100, 200, 0, json!({})),
        (
            "b",
            "manual",
            "body",
            50,
            250,
            0,
            json!({"payload":{"entry":{"content":{"weight_kg":70}}}}),
        ),
        (
            "c",
            "manual",
            "body",
            1,
            500,
            0,
            json!({"payload":{"entry":{"content":{"waist_cm":90}}}}),
        ),
        ("d", "apple_health", "body_mass", 0, 1000, 1, json!({})),
        ("e", "apple_health", "heart_rate", 150, 170, 0, json!({})),
    ] {
        sqlx::query("INSERT INTO health_records(user_id,platform,source_id,record_id,record_type,start_at,end_at,version,deleted,payload_json) VALUES(?,?,'bounds',?,?,?,?,1,?,?)")
            .bind(&user.user_id).bind(platform).bind(id).bind(kind).bind(start).bind(end).bind(deleted).bind(payload.to_string())
            .execute(&pool).await.unwrap();
    }
    for (kind, expected) in [
        ("body_mass", (Some(50i64), Some(250i64))),
        ("heart_rate", (Some(150), Some(170))),
        ("vo2_max", (None, None)),
    ] {
        let result = state
            .database
            .measurement_chart(
                &user.user_id,
                Request {
                    record_type: kind.into(),
                    timezone: "UTC".into(),
                    period: "D".into(),
                    start_at: 0,
                    end_at: 1100,
                },
            )
            .await
            .unwrap();
        let bounds = (result["first_at"].as_i64(), result["last_at"].as_i64());
        assert_eq!(bounds, expected, "{kind}");
    }
}
