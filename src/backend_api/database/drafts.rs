use super::{Database, queries};
use crate::error::AppError;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::{path::Path, time::Duration};

pub(super) async fn initialize(path: &Path) -> Result<(), AppError> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .synchronous(SqliteSynchronous::Full)
                .busy_timeout(Duration::from_secs(5)),
        )
        .await?;
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let version: i64 = sqlx::query_scalar(queries::SCHEMA_VERSION)
        .fetch_one(&mut *transaction)
        .await?;
    match version {
        0 => {
            sqlx::raw_sql(include_str!("draft_schema.sql"))
                .execute(&mut *transaction)
                .await?;
        }
        1 | 2 => {
            if version == 1 {
                sqlx::raw_sql(include_str!("draft_ocr_revision.sql"))
                .execute(&mut *transaction)
                .await?;
            }
            sqlx::raw_sql(include_str!("draft_pending_migration.sql"))
                .execute(&mut *transaction)
                .await?;
        }
        3 => {}
        _ => {
            return Err(AppError::Conflict(
                "Unsupported report draft database version",
            ));
        }
    }
    transaction.commit().await?;
    pool.close().await;
    Ok(())
}

impl Database {
    pub(crate) async fn reconcile_report_drafts(&self) -> Result<(), AppError> {
        // Each phase commits separately: never delete the only durable copy across WAL databases.
        sqlx::query(queries::MIGRATE_REPORT_DRAFTS)
            .execute(&self.pool)
            .await?;
        sqlx::query(queries::REMOVE_SUPERSEDED_OFFICIAL)
            .execute(&self.pool)
            .await?;
        sqlx::query(queries::REMOVE_SUPERSEDED_DRAFTS)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn previous_rejections_return_to_manual_review_without_losing_the_result() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("draft.sqlite");
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options.clone())
            .await
            .unwrap();
        let prior_schema = include_str!("draft_schema.sql")
            .replace(
                "CHECK (status = 'pending')",
                "CHECK (status IN ('pending', 'rejected'))",
            )
            .replace("user_version = 3", "user_version = 2");
        sqlx::raw_sql(&prior_schema).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO observations (observation_id,user_id,report_id,current_revision,status,payload_json) VALUES ('excluded','user','report',1,'rejected',?1), ('draft','user','report',3,'pending',?1)")
            .bind(r#"{"raw_result":"42"}"#)
            .execute(&pool).await.unwrap();
        pool.close().await;

        initialize(&path).await.unwrap();
        initialize(&path).await.unwrap();

        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .unwrap();
        let rows: Vec<(String, String, i64, String)> = sqlx::query_as(
            "SELECT observation_id,status,current_revision,payload_json FROM observations ORDER BY observation_id",
        ).fetch_all(&pool).await.unwrap();
        assert_eq!(
            rows[0],
            (
                "draft".into(),
                "pending".into(),
                3,
                r#"{"raw_result":"42"}"#.into()
            )
        );
        assert_eq!(
            (&*rows[1].0, &*rows[1].1, rows[1].2),
            ("excluded", "pending", 2)
        );
        let payload: serde_json::Value = serde_json::from_str(&rows[1].3).unwrap();
        assert_eq!(payload["raw_result"], "42");
        assert!(
            payload["ocr_mapping_note"]
                .as_str()
                .is_some_and(|note| !note.is_empty())
        );
        assert!(
            sqlx::query("UPDATE observations SET status='rejected'")
                .execute(&pool)
                .await
                .is_err()
        );
    }
}
