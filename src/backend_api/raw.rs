use crate::{
    database::{Database, queries},
    error::AppError,
};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::{collections::HashSet, path::Path};
use tokio::io::AsyncWriteExt;

const RESERVED_SOURCES: &[&str] = &[
    "apple_health",
    "google_health",
    "longlivehealth",
    "gpx",
    "tcx",
    "fit",
];

pub fn source_folder(name: &str) -> String {
    let normalized = name.trim().to_lowercase();
    let mut slug: String = normalized
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if normalized
        .chars()
        .any(|ch| !ch.is_ascii_alphanumeric() && !matches!(ch, ' ' | '-' | '_'))
    {
        slug.truncate(slug.len().min(80));
        slug.push('-');
        slug.push_str(&hex::encode(Sha256::digest(name.trim().as_bytes()))[..16]);
    }
    if slug.is_empty() || RESERVED_SOURCES.contains(&slug.as_str()) {
        slug = format!("provider_{slug}");
    }
    slug
}

pub fn document_path(
    user_id: &str,
    file_id: &str,
    content_type: &str,
    source_name: Option<&str>,
) -> String {
    let source = source_name
        .map(source_folder)
        .unwrap_or_else(|| "unassigned".into());
    let extension = match content_type {
        "application/pdf" => "pdf",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/heic" => "heic",
        "image/heif" => "heif",
        _ => "bin",
    };
    format!("raw/{user_id}/{source}/{file_id}.{extension}")
}

pub fn valid_source_folder(source: &str) -> bool {
    !source.is_empty()
        && source.len() <= 180
        && source
            .bytes()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == b'_' || ch == b'-')
}

pub fn health_path(
    user_id: &str,
    platform: &str,
    source_id: &str,
    revision_id: &str,
) -> Result<String, AppError> {
    let source = match platform {
        "apple_health" => "apple_health",
        "health_connect" => "google_health",
        "manual" => "longlivehealth",
        "file_import" => match source_id {
            "gpx-1.1" => "gpx",
            "tcx-2" => "tcx",
            "fit" => "fit",
            _ => return Err(AppError::Invalid("Unknown file import source")),
        },
        _ => return Err(AppError::Invalid("Unknown health platform")),
    };
    Ok(format!("raw/{user_id}/{source}/{revision_id}.json"))
}

pub fn valid_relative_path(path: &str) -> bool {
    let parts = path.split('/').collect::<Vec<_>>();
    parts.len() == 4
        && parts[0] == "raw"
        && valid_source_folder(parts[2])
        && uuid::Uuid::parse_str(parts[1]).is_ok()
        && parts
            .iter()
            .all(|part| !part.is_empty() && *part != "." && *part != ".." && !part.contains('\\'))
}

pub async fn sync_directory(path: &Path) -> Result<(), AppError> {
    #[cfg(unix)]
    tokio::fs::File::open(path).await?.sync_all().await?;
    Ok(())
}

/// Write and flush a new original before the transaction publishes its index.
pub async fn archive(
    data_dir: &Path,
    temporary_files: &crate::temporary::TemporaryFiles,
    relative_path: &str,
    bytes: &[u8],
) -> Result<(), AppError> {
    if !valid_relative_path(relative_path) {
        return Err(AppError::Internal);
    }
    let staged = stage(temporary_files, data_dir, bytes).await?;
    publish(data_dir, relative_path, &staged).await?;
    sync_archive_directories(data_dir, &[relative_path]).await
}

/// Flush original bytes before acquiring a database writer. The lease prevents cleanup
/// from removing a staged file while publication is pending.
pub(crate) async fn stage(
    temporary_files: &crate::temporary::TemporaryFiles,
    data_dir: &Path,
    bytes: &[u8],
) -> Result<crate::temporary::TemporaryFile, AppError> {
    let temporary =
        temporary_files.reserve(data_dir.join("tmp").join(uuid::Uuid::new_v4().to_string()))?;
    let mut output = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(temporary.path())
        .await?;
    output.write_all(bytes).await?;
    output.sync_all().await?;
    Ok(temporary)
}

pub(crate) async fn publish(
    data_dir: &Path,
    relative_path: &str,
    staged: &crate::temporary::TemporaryFile,
) -> Result<(), AppError> {
    if !valid_relative_path(relative_path) {
        return Err(AppError::Internal);
    }
    let target = data_dir.join(relative_path);
    tokio::fs::create_dir_all(target.parent().ok_or(AppError::Internal)?).await?;
    tokio::fs::rename(staged.path(), target).await?;
    Ok(())
}

/// All originals must be durable before their database indexes commit. Each directory
/// needs one flush per batch, rather than two flushes per individual record.
pub(crate) async fn sync_archive_directories(
    data_dir: &Path,
    relative_paths: &[&str],
) -> Result<(), AppError> {
    let mut directories = std::collections::BTreeSet::new();
    for relative in relative_paths {
        let target = data_dir.join(relative);
        let mut directory = target.parent().ok_or(AppError::Internal)?;
        loop {
            if !directory.starts_with(data_dir) {
                return Err(AppError::Internal);
            }
            directories.insert(directory.to_path_buf());
            if directory == data_dir {
                break;
            }
            directory = directory.parent().ok_or(AppError::Internal)?;
        }
    }
    // Include raw/ itself when a new provider folder is created.
    for directory in directories.iter().rev() {
        sync_directory(directory).await?;
    }
    Ok(())
}

pub async fn remove_orphans(database: &Database, data_dir: &Path) -> Result<(), AppError> {
    let paths: HashSet<String> = sqlx::query(queries::RAW_PATHS)
        .fetch_all(&database.pool)
        .await?
        .into_iter()
        .map(|row| row.get("relative_path"))
        .collect();
    let mut users = tokio::fs::read_dir(data_dir.join("raw")).await?;
    while let Some(user) = users.next_entry().await? {
        let user_id = user.file_name().to_string_lossy().into_owned();
        if !user.file_type().await?.is_dir() || uuid::Uuid::parse_str(&user_id).is_err() {
            continue;
        }
        let mut sources = tokio::fs::read_dir(user.path()).await?;
        while let Some(source) = sources.next_entry().await? {
            let source_name = source.file_name().to_string_lossy().into_owned();
            if !source.file_type().await?.is_dir() || !valid_source_folder(&source_name) {
                continue;
            }
            let mut files = tokio::fs::read_dir(source.path()).await?;
            while let Some(file) = files.next_entry().await? {
                if !file.file_type().await?.is_file() {
                    continue;
                }
                let relative = format!(
                    "raw/{user_id}/{source_name}/{}",
                    file.file_name().to_string_lossy()
                );
                if !paths.contains(&relative) {
                    tokio::fs::remove_file(file.path()).await?;
                }
            }
        }
    }
    Ok(())
}
