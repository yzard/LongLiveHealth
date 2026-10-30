//! Apple Health originals: category files of [u64 little-endian length][UTF-8 JSON].
//! Callers hold BEGIN IMMEDIATE until append buffers are durable and indexes commit.
use crate::{database::queries, error::AppError};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::{collections::BTreeMap, path::Path};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

pub struct Location {
    pub path: String,
    pub offset: Option<i64>,
    pub length: Option<i64>,
}
impl Location {
    pub(crate) fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, AppError> {
        Ok(Self {
            path: row.try_get("raw_path")?,
            offset: row.try_get("raw_offset")?,
            length: row.try_get("raw_length")?,
        })
    }
    pub async fn read(&self, root: &Path) -> Result<Vec<u8>, AppError> {
        if !crate::raw::valid_relative_path(&self.path) {
            return Err(AppError::Internal);
        }
        let mut file = tokio::fs::File::open(root.join(&self.path)).await?;
        match (self.offset, self.length) {
            (Some(offset), Some(length)) if offset >= 0 && length > 0 => {
                file.seek(std::io::SeekFrom::Start(offset as u64)).await?;
                let mut prefix = [0; 8];
                file.read_exact(&mut prefix).await?;
                if u64::from_le_bytes(prefix) != length as u64 {
                    return Err(AppError::Conflict(
                        "Raw archive frame length differs from its index",
                    ));
                }
                let end = (offset as u64)
                    .checked_add(8)
                    .and_then(|value| value.checked_add(length as u64))
                    .ok_or(AppError::Internal)?;
                if end > file.metadata().await?.len() {
                    return Err(AppError::Conflict("Raw archive frame is truncated"));
                }
                let length = usize::try_from(length).map_err(|_| AppError::Internal)?;
                let mut bytes = vec![0; length];
                file.read_exact(&mut bytes).await?;
                Ok(bytes)
            }
            (None, None) => {
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes).await?;
                Ok(bytes)
            }
            _ => Err(AppError::Internal),
        }
    }
}
struct Buffer {
    committed_end: u64,
    bytes: Vec<u8>,
}
#[derive(Default)]
pub(crate) struct Appends {
    buffers: BTreeMap<String, Buffer>,
}
impl Appends {
    pub async fn push(
        &mut self,
        tx: &mut sqlx::SqliteConnection,
        user: &str,
        category: &str,
        json: &str,
    ) -> Result<Location, AppError> {
        // Category names remain readable, including previously unseen HealthKit types.
        // A stable digest bounds filenames for types outside the normalized alphabet.
        let category =
            if crate::raw::valid_source_folder(category) && !category.starts_with("encoded-") {
                category.to_owned()
            } else {
                format!(
                    "encoded-{}",
                    hex::encode(Sha256::digest(category.as_bytes()))
                )
            };
        let path = format!("raw/{user}/apple_health/{category}.frames");
        if !self.buffers.contains_key(&path) {
            let end: Option<i64> = sqlx::query_scalar(queries::HEALTH_ARCHIVE_END)
                .bind(&path)
                .fetch_optional(&mut *tx)
                .await?;
            let end = end.unwrap_or(0);
            if end < 0 {
                return Err(AppError::Internal);
            }
            self.buffers.insert(
                path.clone(),
                Buffer {
                    committed_end: end as u64,
                    bytes: Vec::new(),
                },
            );
        }
        let buffer = self.buffers.get_mut(&path).ok_or(AppError::Internal)?;
        let offset = buffer
            .committed_end
            .checked_add(buffer.bytes.len() as u64)
            .and_then(|v| i64::try_from(v).ok())
            .ok_or(AppError::TooLarge)?;
        buffer
            .bytes
            .extend_from_slice(&(json.len() as u64).to_le_bytes());
        buffer.bytes.extend_from_slice(json.as_bytes());
        Ok(Location {
            path,
            offset: Some(offset),
            length: Some(json.len() as i64),
        })
    }
    pub async fn flush(
        self,
        root: std::path::PathBuf,
        transaction: sqlx::Transaction<'static, sqlx::Sqlite>,
    ) -> Result<(sqlx::Transaction<'static, sqlx::Sqlite>, Vec<String>), AppError> {
        if self.buffers.is_empty() {
            return Ok((transaction, Vec::new()));
        }
        // The blocking job owns the SQLite writer until every filesystem operation
        // finishes. Cancelling the HTTP request cannot release the writer while an
        // OS write is still running and corrupt a subsequent append.
        tokio::task::spawn_blocking(move || {
            use std::io::{Seek, Write};
            let mut paths = Vec::new();
            for (path, buffer) in self.buffers {
                let target = root.join(&path);
                std::fs::create_dir_all(target.parent().ok_or(AppError::Internal)?)?;
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .truncate(false)
                    .write(true)
                    .open(&target)?;
                if file.metadata()?.len() < buffer.committed_end {
                    return Err(AppError::Conflict(
                        "Raw archive is shorter than its committed index",
                    ));
                }
                // Discard only bytes from an aborted transaction, never committed frames.
                file.set_len(buffer.committed_end)?;
                file.seek(std::io::SeekFrom::Start(buffer.committed_end))?;
                file.write_all(&buffer.bytes)?;
                file.sync_all()?;
                paths.push(path);
            }
            Ok((transaction, paths))
        })
        .await
        .map_err(|_| AppError::Internal)?
    }
}
