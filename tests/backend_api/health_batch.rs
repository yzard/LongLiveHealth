use crate::{add_user, fixture};
use longlivehealth::health::{ConnectionRequest, HealthRecordInput, SyncRequest};
use serde_json::json;
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};

pub fn record(id: &str, payload: serde_json::Value) -> HealthRecordInput {
    HealthRecordInput {
        record_id: id.into(),
        source_id: "synthetic".into(),
        record_type: "synthetic_raw".into(),
        start_at: 1,
        end_at: 2,
        version: 1,
        deleted: false,
        payload,
    }
}
pub fn batch(connection: &str, records: Vec<HealthRecordInput>) -> SyncRequest {
    SyncRequest {
        connection_id: connection.into(),
        batch_id: uuid::Uuid::new_v4().to_string(),
        record_type: "synthetic_raw".into(),
        coverage_status: "observed".into(),
        records,
    }
}

#[tokio::test]
async fn streaming_digest_matches_existing_canonical_bytes_and_invalid_late_record_is_atomic() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
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
    let input = batch(
        &connection.connection_id,
        vec![record(
            "valid",
            json!({"unicode":"µ 中文", "nested":{"unknown":[1,2,"\\n"]}}),
        )],
    );
    let encoded = serde_json::to_vec(&input).unwrap();
    let batch_id = input.batch_id.clone();
    state
        .database
        .sync_health(&user.user_id, input)
        .await
        .unwrap();
    let mut connection_sql = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(state.config.data_dir.join("database.sqlite")),
    )
    .await
    .unwrap();
    let stored: String = sqlx::query_scalar("SELECT digest FROM sync_batches WHERE batch_id = ?")
        .bind(batch_id)
        .fetch_one(&mut connection_sql)
        .await
        .unwrap();
    assert_eq!(stored, longlivehealth::authentication::digest(&encoded));
    let mut invalid = record("bad", json!({}));
    invalid.version = 0;
    let result = state
        .database
        .sync_health(
            &user.user_id,
            batch(
                &connection.connection_id,
                vec![record("must-not-exist", json!({"value":2})), invalid],
            ),
        )
        .await;
    assert!(matches!(
        result,
        Err(longlivehealth::error::AppError::Invalid(_))
    ));
    let rows = state
        .database
        .health_records(&user.user_id, 100, 0)
        .await
        .unwrap();
    assert_eq!(rows["records"].as_array().unwrap().len(), 1);
    assert_eq!(
        rows["records"][0]["record"]["payload"],
        json!({"unicode":"µ 中文", "nested":{"unknown":[1,2,"\\n"]}})
    );
    let files = state
        .database
        .health_raw_files(
            &user.user_id,
            &longlivehealth::models::ListRequest {
                after_id: None,
                limit: 100,
            },
        )
        .await
        .unwrap();
    assert_eq!(files["files"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn large_individual_payload_upload_replay_and_read_are_lossless() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
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
    let payload = json!({"blob":"x".repeat(41 * 1024 * 1024)});
    let input = batch(
        &connection.connection_id,
        vec![record("large-route", payload.clone())],
    );
    let bytes = serde_json::to_vec(&input).unwrap();
    state
        .database
        .sync_health(&user.user_id, input)
        .await
        .unwrap();
    state
        .database
        .sync_health(&user.user_id, serde_json::from_slice(&bytes).unwrap())
        .await
        .unwrap();
    let result = state
        .database
        .health_records(&user.user_id, 1, 0)
        .await
        .unwrap();
    assert_eq!(result["records"].as_array().unwrap().len(), 1);
    assert_eq!(result["records"][0]["record"]["payload"], payload);
}

#[tokio::test]
async fn large_batches_have_no_record_count_ceiling_and_invalid_tail_is_atomic() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "batch-limit").await;
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
    let mut connection_sql = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(state.config.data_dir.join("database.sqlite")),
    )
    .await
    .unwrap();
    let mut records: Vec<_> = (0..10001)
        .map(|i| record(&i.to_string(), json!({"value":i})))
        .collect();
    records.last_mut().unwrap().version = 0;
    assert!(matches!(
        state
            .database
            .sync_health(&user.user_id, batch(&connection.connection_id, records))
            .await,
        Err(longlivehealth::error::AppError::Invalid(_))
    ));
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM health_records")
        .fetch_one(&mut connection_sql)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let records = (0..10001)
        .map(|i| record(&i.to_string(), json!({"value":i})))
        .collect();
    state
        .database
        .sync_health(&user.user_id, batch(&connection.connection_id, records))
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM health_records")
        .fetch_one(&mut connection_sql)
        .await
        .unwrap();
    assert_eq!(count, 10001);
}

#[tokio::test]
async fn originals_stage_without_writer_lock_and_publication_failure_rolls_back() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "staging").await;
    let connection = state
        .database
        .create_connection(
            &user.user_id,
            ConnectionRequest {
                platform: "health_connect".into(),
                installation_id: uuid::Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap();
    let mut sql = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(state.config.data_dir.join("database.sqlite")),
    )
    .await
    .unwrap();
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut sql)
        .await
        .unwrap();
    let input = batch(
        &connection.connection_id,
        (0..32)
            .map(|i| record(&i.to_string(), json!({"original": "x".repeat(4096)})))
            .collect(),
    );
    let retry: SyncRequest = serde_json::from_value(serde_json::to_value(&input).unwrap()).unwrap();
    let database = state.database.clone();
    let user_id = user.user_id.clone();
    let task = tokio::spawn(async move { database.sync_health(&user_id, input).await });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let mut files = tokio::fs::read_dir(state.config.data_dir.join("tmp"))
                .await
                .unwrap();
            let mut count = 0;
            while files.next_entry().await.unwrap().is_some() {
                count += 1;
            }
            if count == 32 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("originals must stage while another writer owns SQLite");
    assert!(!task.is_finished());
    let obstacle = state
        .config
        .data_dir
        .join("raw")
        .join(&user.user_id)
        .join("google_health");
    tokio::fs::create_dir_all(obstacle.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(&obstacle, b"synthetic publication failure")
        .await
        .unwrap();
    sqlx::query("COMMIT").execute(&mut sql).await.unwrap();
    assert!(task.await.unwrap().is_err());
    for table in [
        "health_records",
        "health_originals",
        "sync_batches",
        "sync_coverage",
    ] {
        // Closed test-only table list.
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&mut sql)
            .await
            .unwrap();
        assert_eq!(count, 0, "{table} must roll back");
    }
    tokio::fs::remove_file(obstacle).await.unwrap();
    let replay: SyncRequest =
        serde_json::from_value(serde_json::to_value(&retry).unwrap()).unwrap();
    assert_eq!(
        state
            .database
            .sync_health(&user.user_id, retry)
            .await
            .unwrap()["total_records"],
        32
    );
    assert_eq!(
        state
            .database
            .sync_health(&user.user_id, replay)
            .await
            .unwrap()["replayed"],
        true
    );
}

#[tokio::test]
#[ignore = "isolated disk benchmark; run with TMPDIR on the deployment filesystem"]
async fn sync_disk_benchmark() {
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "benchmark").await;
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
    for run in 0..3 {
        let input = batch(
            &connection.connection_id,
            (0..1000)
                .map(|i| record(&format!("{run}-{i}"), json!({"original":"x".repeat(3000)})))
                .collect(),
        );
        let started = std::time::Instant::now();
        state
            .database
            .sync_health(&user.user_id, input)
            .await
            .unwrap();
        println!(
            "synthetic 1000-record disk batch {run}: {} ms",
            started.elapsed().as_millis()
        );
    }
}

#[tokio::test]
async fn apple_archive_representation_changes_do_not_duplicate_originals_or_current_count() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
    for platform in ["apple_health", "health_connect"] {
        let connection = state
            .database
            .create_connection(
                &user.user_id,
                ConnectionRequest {
                    platform: platform.into(),
                    installation_id: uuid::Uuid::new_v4().to_string(),
                },
            )
            .await
            .unwrap();
        let payload = |data: &str, value: i64| json!({"value":value,"unit":"count","raw_archive":{"format":"nskeyedarchiver-secure-v1","encoding":"base64","data":data}});
        state
            .database
            .sync_health(
                &user.user_id,
                batch(
                    &connection.connection_id,
                    vec![record(platform, payload("YQ==", 3))],
                ),
            )
            .await
            .unwrap();
        let result = state
            .database
            .sync_health(
                &user.user_id,
                batch(
                    &connection.connection_id,
                    vec![record(platform, payload("YWFhYQ==", 3))],
                ),
            )
            .await;
        assert!(result.is_ok());
        if platform == "apple_health" {
            // A fresh batch with the same encoded record remains a no-op.
            state
                .database
                .sync_health(
                    &user.user_id,
                    batch(
                        &connection.connection_id,
                        vec![record(platform, payload("YWFhYQ==", 3))],
                    ),
                )
                .await
                .unwrap();
        }
        // Real structured data changes still conflict and cannot overwrite the record.
        assert!(matches!(
            state
                .database
                .sync_health(
                    &user.user_id,
                    batch(
                        &connection.connection_id,
                        vec![record(platform, payload("YWFhYQ==", 4))]
                    )
                )
                .await,
            Err(longlivehealth::error::AppError::Conflict(_))
        ));
    }
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(state.config.data_dir.join("database.sqlite")),
    )
    .await
    .unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM health_records WHERE platform = 'apple_health'")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(count, 1);
    let projection: String =
        sqlx::query_scalar("SELECT payload_json FROM health_records WHERE platform='apple_health'")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    let projection: serde_json::Value = serde_json::from_str(&projection).unwrap();
    assert!(projection["payload"].get("raw_archive").is_none());
    assert_eq!(projection["content_hash"].as_str().unwrap().len(), 64);
    let raw_ids: Vec<String> =
        sqlx::query_scalar("SELECT raw_id FROM health_originals WHERE platform='apple_health'")
            .fetch_all(&mut connection)
            .await
            .unwrap();
    assert_eq!(raw_ids.len(), 1);
    let original: serde_json::Value = serde_json::from_slice(
        &state
            .database
            .health_raw_bytes(&user.user_id, &raw_ids[0])
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(original["payload"]["raw_archive"]["data"], "YQ==");
}

#[tokio::test]
async fn query_projection_keeps_details_in_raw_and_matches_offline_digest() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
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
    let payload = json!({"value":3.5,"unit":"count","series":[1,1.0,-0.0,9007199254740993_i64,"µ 中文",true,null],
        "device":{"name":"Watch"},"raw_archive":{"format":"nskeyedarchiver-secure-v1","encoding":"base64","data":"YQ=="}});
    state
        .database
        .sync_health(
            &user.user_id,
            batch(
                &connection.connection_id,
                vec![record("projection", payload.clone())],
            ),
        )
        .await
        .unwrap();
    let mut sql = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(state.config.data_dir.join("database.sqlite")),
    )
    .await
    .unwrap();
    let text: String = sqlx::query_scalar("SELECT payload_json FROM health_records")
        .fetch_one(&mut sql)
        .await
        .unwrap();
    let projection: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(projection["payload"], json!({"value":3.5,"unit":"count"}));
    assert_eq!(
        projection["content_hash"],
        "b64eb0da43733256197a4c2f8b28109dfeadf396a9a7a091c6bdff5fcfcaf256"
    );
    let original = state
        .database
        .health_records(&user.user_id, 10, 0)
        .await
        .unwrap();
    assert_eq!(original["records"][0]["record"]["payload"], payload);
    let mut changed = payload;
    changed["series"][0] = json!(2);
    assert!(
        state
            .database
            .sync_health(
                &user.user_id,
                batch(
                    &connection.connection_id,
                    vec![record("projection", changed)]
                )
            )
            .await
            .is_err()
    );
}

#[tokio::test]
async fn sync_http_accepts_a_batch_larger_than_forty_mib() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "large-http").await;
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
    let token = crate::token(&state, "large-http").await;
    let router = longlivehealth::app::create_application(state.clone());
    let records: Vec<_> = (0..2)
        .map(|i| record(&i.to_string(), json!({"blob":"x".repeat(21 * 1024 * 1024)})))
        .collect();
    let input = serde_json::to_value(batch(&connection.connection_id, records)).unwrap();
    let (status, result) =
        crate::request(&router, "/api/v1/health/sync", Some(&token), input).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{result}");
    assert_eq!(result["total_records"], 2);
}

#[tokio::test]
async fn mixed_http_batch_is_atomic_replayable_and_updates_each_type() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "mixed-http").await;
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
    let token = crate::token(&state, "mixed-http").await;
    let router = longlivehealth::app::create_application(state.clone());
    let records: Vec<_> = (0..10_000)
        .map(|i| {
            let mut value = record(&i.to_string(), json!({"value": i}));
            value.record_type = if i == 0 {
                "weight"
            } else if i < 4 {
                "sleep"
            } else {
                "heart"
            }
            .into();
            value
        })
        .collect();
    let body = json!({"connection_id":connection.connection_id,"batch_id":uuid::Uuid::new_v4().to_string(),
        "coverage":[{"record_type":"weight","status":"observed"},{"record_type":"sleep","status":"observed"},
        {"record_type":"heart","status":"observed"},{"record_type":"empty","status":"no_visible_samples"}], "records":records});
    let mut invalid = body.clone();
    invalid["records"][9999]["record_type"] = json!("unlisted");
    assert_eq!(
        crate::request(&router, "/api/v1/health/sync", Some(&token), invalid)
            .await
            .0,
        axum::http::StatusCode::BAD_REQUEST
    );
    assert_eq!(
        state.database.health_coverage(&user.user_id).await.unwrap()["total_records"],
        0
    );
    for replayed in [false, true] {
        let (status, result) =
            crate::request(&router, "/api/v1/health/sync", Some(&token), body.clone()).await;
        assert_eq!(status, axum::http::StatusCode::OK, "{result}");
        assert_eq!(result["total_records"], 10_000);
        assert_eq!(result["replayed"], replayed);
    }
    let coverage = state.database.health_coverage(&user.user_id).await.unwrap();
    assert_eq!(coverage["coverage"].as_array().unwrap().len(), 4);
    let mut deleted = record("0", json!({}));
    deleted.record_type = "weight".into();
    deleted.source_id = "*".into();
    deleted.deleted = true;
    let mut added = record("new", json!({"value":1}));
    added.record_type = "heart".into();
    let mut changes = body.clone();
    changes["batch_id"] = json!(uuid::Uuid::new_v4().to_string());
    changes["records"] = json!([deleted, added]);
    let (status, result) =
        crate::request(&router, "/api/v1/health/sync", Some(&token), changes).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{result}");
    assert_eq!(result["total_records"], 10_000);
}

#[tokio::test]
async fn identical_ecg_archive_accepts_adjacent_float_but_rejects_changed_measurements() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "ecg-roundtrip").await;
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
    let original = json!({"raw_archive":{"format":"nskeyedarchiver-secure-v1","data":"same-immutable-archive"},
        "series":[{"lead_i_volts":0.0001,"time_since_start":1.0}], "source_name":"watch"});
    let make = |payload| {
        let mut r = record("ecg", payload);
        r.record_type = "electrocardiogram".into();
        let mut b = batch(&connection.connection_id, vec![r]);
        b.record_type = "electrocardiogram".into();
        b
    };
    state
        .database
        .sync_health(&user.user_id, make(original.clone()))
        .await
        .unwrap();
    let mut adjacent = original.clone();
    adjacent["series"][0]["lead_i_volts"] = json!(f64::from_bits(0.0001_f64.to_bits() + 1));
    state
        .database
        .sync_health(&user.user_id, make(adjacent))
        .await
        .unwrap();
    for (field, value) in [
        ("voltage", json!(0.0002)),
        ("time", json!(1.001)),
        ("archive", json!("changed")),
        ("source", json!("other")),
    ] {
        let mut changed = original.clone();
        match field {
            "voltage" => changed["series"][0]["lead_i_volts"] = value,
            "time" => changed["series"][0]["time_since_start"] = value,
            "archive" => {
                changed["raw_archive"]["data"] = value;
                changed["series"][0]["lead_i_volts"] =
                    json!(f64::from_bits(0.0001_f64.to_bits() + 1));
            }
            _ => {
                changed["source_name"] = value;
                changed["series"][0]["lead_i_volts"] = json!(0.0002);
            }
        }
        assert!(matches!(
            state
                .database
                .sync_health(&user.user_id, make(changed))
                .await,
            Err(longlivehealth::error::AppError::Conflict(_))
        ));
    }
    assert_eq!(
        state.database.health_coverage(&user.user_id).await.unwrap()["total_records"],
        1
    );
}

#[tokio::test]
async fn medication_fractional_versions_preserve_raw_and_order_updates() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "medication").await;
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
    let token = crate::token(&state, "medication").await;
    let router = longlivehealth::app::create_application(state.clone());
    let make = |version: f64, value: i64| {
        json!({"connection_id":connection.connection_id,"batch_id":uuid::Uuid::new_v4().to_string(),
        "coverage":[{"record_type":"medication_dose_event","status":"observed"}],
        "records":[{"record_id":"dose","source_id":"health","record_type":"medication_dose_event","version":version,
        "start_at":1,"end_at":1,"deleted":false,"payload":{"value":value}}]})
    };
    let first = make(684724783.304679, 1);
    for body in [
        first.clone(),
        first,
        make(684724783.304749, 2),
        make(684724783.3047, 1),
        make(684724784.0, 3),
    ] {
        let (status, result) =
            crate::request(&router, "/api/v1/health/sync", Some(&token), body).await;
        assert_eq!(status, axum::http::StatusCode::OK, "{result}");
        assert_eq!(result["total_records"], 1);
    }
    let listed = state
        .database
        .health_records(&user.user_id, 100, 0)
        .await
        .unwrap();
    assert_eq!(listed["records"][0]["record"]["payload"]["value"], 3);
    assert_eq!(
        listed["records"][0]["record"]["version"],
        json!(684724784.0)
    );
    let (status, _) = crate::request(
        &router,
        "/api/v1/health/sync",
        Some(&token),
        make(684724784.0, 4),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CONFLICT);
    let (status, _) =
        crate::request(&router, "/api/v1/health/sync", Some(&token), make(-1.5, 4)).await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
}
