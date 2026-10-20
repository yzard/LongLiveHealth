use longlivehealth::database::Database;
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};

#[tokio::test]
async fn current_schema_has_raw_indexes_and_enforces_extraction_owner() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path()).await.unwrap();
    database.close().await;
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(directory.path().join("database.sqlite"))
            .foreign_keys(true),
    )
    .await
    .unwrap();
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert_eq!(version, 7);
    let error = sqlx::query("INSERT INTO extraction_outputs (user_id, report_id, run_id, page_number, stage, response_body, status_code, model, adapter, created_at) VALUES ('missing', 'missing', 'run', 1, 'ocr', '{}', 200, 'model', 'adapter', 1)")
        .execute(&mut connection).await.unwrap_err();
    assert!(matches!(error, sqlx::Error::Database(error) if error.is_foreign_key_violation()));
}

#[tokio::test]
async fn legacy_schema_requires_offline_conversion_without_mutating_accounts() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path()).await.unwrap();
    database.close().await;
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(directory.path().join("database.sqlite")),
    )
    .await
    .unwrap();
    sqlx::raw_sql("INSERT INTO users(user_id,username,password_hash,created_at) VALUES ('owner','owner','hash',1); PRAGMA user_version=5;").execute(&mut connection).await.unwrap();
    connection.close().await.unwrap();
    assert!(Database::open(directory.path()).await.is_err());
}

#[tokio::test]
async fn current_schema_is_compact_and_indexes_sync_queries() {
    let directory = tempfile::tempdir().unwrap();
    let database = Database::open(directory.path()).await.unwrap();
    database.close().await;
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(directory.path().join("database.sqlite")),
    )
    .await
    .unwrap();
    let obsolete: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE name IN ('health_revisions','observation_revisions','health_records_identity','health_records_synced_active')").fetch_one(&mut connection).await.unwrap();
    assert_eq!(obsolete, 0);
    for (query, index) in [
        (
            "EXPLAIN QUERY PLAN UPDATE health_records SET deleted=1 WHERE user_id='owner' AND platform='apple_health' AND record_id='id'",
            "sqlite_autoindex_health_records_1",
        ),
        (
            "EXPLAIN QUERY PLAN SELECT raw_path FROM health_originals WHERE user_id='owner' AND platform='apple_health' AND record_id='id'",
            "health_originals_identity",
        ),
        (
            "EXPLAIN QUERY PLAN SELECT platform,source_id,record_type,COUNT(*),MIN(start_at),MAX(end_at) FROM health_records WHERE user_id='owner' AND deleted=0 GROUP BY platform,source_id,record_type",
            "health_records_sources",
        ),
    ] {
        let rows: Vec<(i64, i64, i64, String)> = sqlx::query_as(query)
            .fetch_all(&mut connection)
            .await
            .unwrap();
        assert!(rows.iter().any(|r| r.3.contains(index)), "{rows:?}");
    }
    assert_eq!(
        std::fs::read_dir(directory.path().join("raw"))
            .unwrap()
            .count(),
        0
    );
}
