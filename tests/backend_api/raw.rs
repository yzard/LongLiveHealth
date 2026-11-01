use crate::{add_user, fixture};
use axum::http::StatusCode;
use longlivehealth::{
    health::{ConnectionRequest, HealthRecordInput, SyncRequest},
    models::ListRequest,
};
use serde_json::{Value, json};

#[tokio::test]
async fn raw_health_is_indexed_replayable_exported_isolated_and_deleted() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let other = add_user(&state, "bob").await;
    let token = crate::token(&state, "alice").await;
    let other_token = crate::token(&state, "bob").await;
    let app = longlivehealth::app::create_application(state.clone());
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
        let record = HealthRecordInput {
            record_id: "raw-sample".into(),
            source_id: "original-device".into(),
            record_type: "unsupported_numeric_type".into(),
            start_at: 100,
            end_at: 101,
            version: 1,
            deleted: false,
            payload: json!({"raw_archive":{"format":"nskeyedarchiver-secure-v1","data":"opaque".repeat(100_000)},"nested":{"flag":true,"values":[1,"original",null]},"series":[{"timestamp":100.25,"quantity":"4 count"}]}),
        };
        let request = SyncRequest {
            connection_id: connection.connection_id,
            batch_id: uuid::Uuid::new_v4().to_string(),
            record_type: record.record_type.clone(),
            coverage_status: "observed".into(),
            records: vec![record.clone()],
        };
        let encoded = serde_json::to_value(&request).unwrap();
        assert_eq!(
            crate::request(&app, "/api/v1/health/sync", Some(&token), encoded.clone())
                .await
                .0,
            StatusCode::OK
        );
        assert_eq!(
            crate::request(&app, "/api/v1/health/sync", Some(&token), encoded.clone())
                .await
                .1["replayed"],
            true
        );
        let files = state
            .database
            .health_raw_files(
                &user.user_id,
                &ListRequest {
                    after_id: None,
                    limit: 100,
                },
            )
            .await
            .unwrap();
        let file = files["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| file["platform"] == platform)
            .unwrap();
        let raw_id = file["raw_id"].as_str().unwrap();
        let path = file["relative_path"].as_str().unwrap();
        let source = if platform == "apple_health" {
            "apple_health"
        } else {
            "google_health"
        };
        assert!(path.starts_with(&format!("raw/{}/{source}/", user.user_id)));
        let bytes = state
            .database
            .health_raw_bytes(&user.user_id, raw_id)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&bytes).unwrap(),
            serde_json::to_value(&record).unwrap()
        );
        assert!(
            state
                .database
                .health_raw_bytes(&other.user_id, raw_id)
                .await
                .is_err()
        );
        assert_eq!(
            crate::request(
                &app,
                "/api/v1/health/coverage",
                Some(&other_token),
                json!({})
            )
            .await
            .1["coverage"],
            json!([])
        );
        let export = state.database.request_export(&user.user_id).await.unwrap();
        longlivehealth::maintenance::export_next(&state)
            .await
            .unwrap();
        let export_path = state
            .config
            .data_dir
            .join(format!("exports/{}/{export}.zip", user.user_id));
        let mut zip = zip::ZipArchive::new(std::fs::File::open(export_path).unwrap()).unwrap();
        assert!(zip.by_name(path).is_ok());
        drop(zip);
        let mut deletion: SyncRequest = serde_json::from_value(encoded).unwrap();
        deletion.batch_id = uuid::Uuid::new_v4().to_string();
        deletion.records[0].deleted = true;
        deletion.records[0].source_id = "*".into();
        assert_eq!(
            crate::request(
                &app,
                "/api/v1/health/sync",
                Some(&token),
                serde_json::to_value(deletion).unwrap()
            )
            .await
            .0,
            StatusCode::OK
        );
        assert_eq!(
            state
                .database
                .health_raw_bytes(&user.user_id, raw_id)
                .await
                .is_ok(),
            platform == "apple_health"
        );
        longlivehealth::maintenance::cleanup(&state).await.unwrap();
        assert_eq!(
            state.config.data_dir.join(path).exists(),
            platform == "apple_health"
        );
    }
}

#[test]
fn raw_cleanup_paths_cannot_escape_a_source_and_owner() {
    let user = uuid::Uuid::new_v4();
    assert!(longlivehealth::raw::valid_relative_path(&format!(
        "raw/{user}/function_health/image"
    )));
    for path in [
        format!("raw/function_health/{user}/image"),
        format!("raw/{user}/function_health/nested/image"),
        format!("raw/{user}/function_health/../image"),
    ] {
        assert!(!longlivehealth::raw::valid_relative_path(&path));
    }
    for path in [
        "raw/../../etc/passwd",
        "raw/function_health/not-a-user/image",
        "raw/unknown/user/file",
        "raw/function_health/../file",
        "raw\\photos\\file",
    ] {
        assert!(!longlivehealth::raw::valid_relative_path(path));
    }
}

#[tokio::test]
async fn report_source_groups_originals_and_source_changes_preserve_bytes() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let file_id = crate::archive(&state, "alice").await;
    let old = state.database.file(&user.user_id, &file_id).await.unwrap();
    let bytes = std::fs::read(state.config.data_dir.join(&old.relative_path)).unwrap();
    for source in ["Function Health", "Other Clinic", "Function Health"] {
        let report = state
            .database
            .report_summary(&user.user_id, &file_id)
            .await
            .unwrap();
        state
            .database
            .review(
                &user.user_id,
                longlivehealth::reports::ReviewRequest {
                    report_id: file_id.clone(),
                    expected_revision: report.revision,
                    context: Some(serde_json::from_value(json!({"source_name":source})).unwrap()),
                    observations: vec![],
                },
            )
            .await
            .unwrap();
        let file = state.database.file(&user.user_id, &file_id).await.unwrap();
        assert!(file.relative_path.starts_with(&format!(
            "raw/{}/{}/",
            user.user_id,
            longlivehealth::raw::source_folder(source)
        )));
        assert!(file.relative_path.ends_with(".png"));
        assert_eq!(
            std::fs::read(state.config.data_dir.join(&file.relative_path)).unwrap(),
            bytes
        );
        assert!(state.config.data_dir.join(&file.relative_path).exists());
    }
    longlivehealth::maintenance::cleanup(&state).await.unwrap();
    longlivehealth::raw::remove_orphans(&state.database, &state.config.data_dir)
        .await
        .unwrap();
    let file = state.database.file(&user.user_id, &file_id).await.unwrap();
    assert_eq!(
        std::fs::read(state.config.data_dir.join(file.relative_path)).unwrap(),
        bytes
    );
    assert!(!state.config.data_dir.join(old.relative_path).exists());
}

#[tokio::test]
async fn category_frames_retain_versions_and_deletions_and_recover_failed_commit() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "frames").await;
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
    let batch = |records| SyncRequest {
        connection_id: connection.connection_id.clone(),
        batch_id: uuid::Uuid::new_v4().to_string(),
        record_type: "steps".into(),
        coverage_status: "observed".into(),
        records,
    };
    let record = |id: &str, version, deleted| HealthRecordInput {
        record_id: id.into(),
        source_id: "watch".into(),
        record_type: "steps".into(),
        start_at: 100,
        end_at: 101,
        version,
        deleted,
        payload: json!({"value": version * 10, "unit":"count", "raw_archive": {"data":"x".repeat(3000)}}),
    };
    let records = (0..1000)
        .map(|i| record(&i.to_string(), 1, false))
        .collect();
    let started = std::time::Instant::now();
    state
        .database
        .sync_health(&user.user_id, batch(records))
        .await
        .unwrap();
    eprintln!("1000 framed originals: {:?}", started.elapsed());
    let path = state
        .config
        .data_dir
        .join(format!("raw/{}/apple_health/steps.frames", user.user_id));
    let initial = std::fs::read(&path).unwrap();
    assert_eq!(
        std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
        1
    );
    let pool = sqlx::SqlitePool::connect(&format!(
        "sqlite://{}",
        state.config.data_dir.join("database.sqlite").display()
    ))
    .await
    .unwrap();
    sqlx::query("CREATE TRIGGER reject_batch BEFORE INSERT ON sync_batches BEGIN SELECT RAISE(ABORT, 'test interrupted commit'); END").execute(&pool).await.unwrap();
    assert!(
        state
            .database
            .sync_health(&user.user_id, batch(vec![record("failed", 1, false)]))
            .await
            .is_err()
    );
    assert!(std::fs::metadata(&path).unwrap().len() > initial.len() as u64);
    sqlx::query("DROP TRIGGER reject_batch")
        .execute(&pool)
        .await
        .unwrap();
    let request = batch(vec![record("0", 2, false)]);
    let encoded = serde_json::to_value(&request).unwrap();
    state
        .database
        .sync_health(&user.user_id, request)
        .await
        .unwrap();
    let after_update = std::fs::read(&path).unwrap();
    assert!(after_update.starts_with(&initial));
    state
        .database
        .sync_health(&user.user_id, serde_json::from_value(encoded).unwrap())
        .await
        .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), after_update);
    state
        .database
        .sync_health(&user.user_id, batch(vec![record("0", 2, true)]))
        .await
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(&after_update));
    let mut offset = 0;
    let mut frames = Vec::new();
    while offset < bytes.len() {
        let length = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()) as usize;
        frames.push(
            serde_json::from_slice::<Value>(&bytes[offset + 8..offset + 8 + length]).unwrap(),
        );
        offset += 8 + length;
    }
    assert_eq!(frames.len(), 1002);
    assert_eq!(frames.last().unwrap()["deleted"], true);
    assert_eq!(frames.last().unwrap()["record_id"], "0");
    assert!(!frames.iter().any(|frame| frame["record_id"] == "failed"));
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM health_records WHERE deleted=0")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 999);
    let raw_id: String =
        sqlx::query_scalar("SELECT raw_id FROM health_originals WHERE raw_offset=0")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(
            &state
                .database
                .health_raw_bytes(&user.user_id, &raw_id)
                .await
                .unwrap()
        )
        .unwrap()["version"],
        1
    );
    // Export validates retained revisions, tombstones, and shared category membership.
    let export = state.database.request_export(&user.user_id).await.unwrap();
    longlivehealth::maintenance::export_next(&state)
        .await
        .unwrap();
    let output = std::process::Command::new("python3")
        .arg("../archive/check_export.py")
        .args(["--max-bytes", "100000000"])
        .arg(
            state
                .config
                .data_dir
                .join(format!("exports/{}/{export}.zip", user.user_id)),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let restored = state.config.data_dir.join("restored-archive");
    let output = std::process::Command::new("python3")
        .arg("../archive/restore_export.py")
        .arg(
            state
                .config
                .data_dir
                .join(format!("exports/{}/{export}.zip", user.user_id)),
        )
        .arg("--destination")
        .arg(&restored)
        .args(["--username", "restored", "--max-bytes", "100000000"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(restored.join(format!("raw/{}/apple_health/steps.frames", user.user_id)))
            .unwrap(),
        bytes
    );
    // Never return other records after a corrupted frame prefix.
    use std::io::{Seek, Write};
    let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(std::io::SeekFrom::Start(0)).unwrap();
    file.write_all(&1_u64.to_le_bytes()).unwrap();
    assert!(
        state
            .database
            .health_raw_bytes(&user.user_id, &raw_id)
            .await
            .is_err()
    );
}
