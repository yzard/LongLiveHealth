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
        1 => {}
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
