use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Invalid(&'static str),
    #[error("Authentication required or credentials invalid")]
    Unauthorized,
    #[error("Administrator permission required or protected account")]
    Forbidden,
    #[error("Change your password before continuing")]
    PasswordChangeRequired,
    #[error("Resource not found")]
    NotFound,
    #[error("{0}")]
    Conflict(&'static str),
    #[error("Request exceeds configured limits")]
    TooLarge,
    #[error("Try again later")]
    RateLimited,
    #[error("Provider temporarily unavailable")]
    ProviderUnavailable,
    #[error("Storage operation failed")]
    Database(#[from] sqlx::Error),
    #[error("File operation failed")]
    Io(#[from] std::io::Error),
    #[error("Internal operation failed")]
    Internal,
}

impl From<serde_json::Error> for AppError {
    fn from(_: serde_json::Error) -> Self {
        Self::Internal
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let storage_failure = storage_failure(&self);
        let (status, code) = match &self {
            Self::Invalid(_) => (StatusCode::BAD_REQUEST, "invalid_request"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            Self::PasswordChangeRequired => (StatusCode::FORBIDDEN, "password_change_required"),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Self::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            Self::TooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "too_large"),
            Self::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            Self::ProviderUnavailable => (StatusCode::SERVICE_UNAVAILABLE, "provider_unavailable"),
            Self::Database(_) | Self::Io(_) | Self::Internal => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
            }
        };
        let code = storage_failure.map_or(code, |(code, _)| code);
        // Describe known storage causes, without exposing paths, SQL or private data.
        let message = if let Some((_, message)) = storage_failure {
            message.to_owned()
        } else if status.is_server_error() {
            "Internal operation failed".to_owned()
        } else {
            self.to_string()
        };
        (
            status,
            Json(ErrorBody {
                error: ErrorDetail { code, message },
            }),
        )
            .into_response()
    }
}

fn storage_failure(error: &AppError) -> Option<(&'static str, &'static str)> {
    let denied = (
        "storage_permission_denied",
        "Server storage access denied: the service user does not have permission to read or write a required file or directory. Check its access permissions.",
    );
    let read_only = (
        "storage_read_only",
        "Server storage is read-only: the service cannot save data. Check the data volume and database write permissions.",
    );
    match error {
        AppError::Io(error) | AppError::Database(sqlx::Error::Io(error)) => match error.kind() {
            std::io::ErrorKind::PermissionDenied => Some(denied),
            std::io::ErrorKind::ReadOnlyFilesystem => Some(read_only),
            _ => None,
        },
        AppError::Database(sqlx::Error::Database(error)) => {
            match error
                .code()
                .and_then(|code| code.parse::<i32>().ok())
                .map(|code| code & 0xff)
            {
                Some(3) => Some(denied),    // SQLITE_PERM
                Some(8) => Some(read_only), // SQLITE_READONLY and its extended codes
                _ => None,
            }
        }
        _ => None,
    }
}
