//! Public client distribution. Only a fixed packaged artifact is served, never user data.
use crate::error::AppError;
use axum::{
    body::Body,
    http::header,
    response::{IntoResponse, Response},
};
use std::path::PathBuf;

pub async fn android_package() -> Result<Response, AppError> {
    let executable = std::env::current_exe()?;
    let directory = executable
        .parent()
        .ok_or(AppError::Internal)?
        .join("downloads");
    packaged_android(directory).await
}

pub async fn packaged_android(directory: PathBuf) -> Result<Response, AppError> {
    let file = match tokio::fs::File::open(directory.join("longlivehealth.apk")).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(AppError::NotFound);
        }
        Err(error) => return Err(error.into()),
    };
    let length = file.metadata().await?.len();
    Ok((
        [
            (
                header::CONTENT_TYPE,
                "application/vnd.android.package-archive".to_owned(),
            ),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=longlivehealth.apk".to_owned(),
            ),
            (header::CONTENT_LENGTH, length.to_string()),
            (header::CACHE_CONTROL, "no-cache".to_owned()),
        ],
        Body::from_stream(tokio_util::io::ReaderStream::new(file)),
    )
        .into_response())
}
