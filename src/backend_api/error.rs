use axum::{
    Json,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Invalid(&'static str),
    #[error("MapTo.csv from LOINC {version} cannot verify {incoming} against {existing}")]
    LoincMapping {
        incoming: String,
        existing: String,
        version: String,
        ambiguous: bool,
    },
    #[error("LOINC {incoming} is already linked elsewhere or ambiguous")]
    LoincCodeConflict {
        incoming: String,
        existing: String,
        native: bool,
    },
    #[error("Could not load the official LOINC reference")]
    LoincMapUnavailable { incoming: String, existing: String },
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
            Self::Invalid(_) | Self::LoincCodeConflict { .. } => {
                (StatusCode::BAD_REQUEST, "invalid_request")
            }
            Self::LoincMapping { .. } => (StatusCode::BAD_REQUEST, "loinc_mapping_unverified"),
            Self::LoincMapUnavailable { .. } => {
                (StatusCode::SERVICE_UNAVAILABLE, "loinc_map_unavailable")
            }
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
        use crate::interface_language::Language;
        let language = Language::current();
        let message = if let Some((_, message)) = self.loinc_mapping_message(language) {
            message
        } else {
            // Describe known storage causes without exposing paths, SQL or private data.
            let english = storage_failure.map_or_else(
                || {
                    if status.is_server_error() {
                        "Internal operation failed".to_owned()
                    } else {
                        self.to_string()
                    }
                },
                |(_, message)| message.to_owned(),
            );
            language.message(&english).to_owned()
        };
        (
            status,
            [(header::CONTENT_LANGUAGE, language.tag())],
            Json(ErrorBody {
                error: ErrorDetail { code, message },
            }),
        )
            .into_response()
    }
}

impl AppError {
    pub(crate) fn loinc_mapping_message(
        &self,
        language: crate::interface_language::Language,
    ) -> Option<(&'static str, String)> {
        use crate::interface_language::Language::{Chinese, English};
        match (self, language) {
            (
                Self::LoincMapping {
                    incoming,
                    existing,
                    version,
                    ambiguous,
                },
                English,
            ) => {
                let reason = if *ambiguous {
                    "The matching code has multiple official replacements."
                } else {
                    "There is no direct replacement between these codes."
                };
                Some((
                    "loinc_mapping_unverified",
                    format!(
                        "Cannot add report LOINC {incoming} to a metric with database LOINC {existing}. {reason} Source: LOINC {version} MapTo.csv. Choose the metric for the printed code or review the codes."
                    ),
                ))
            }
            (
                Self::LoincMapping {
                    incoming,
                    existing,
                    version,
                    ambiguous,
                },
                Chinese,
            ) => {
                let reason = if *ambiguous {
                    "匹配到的旧代码有多个官方替代项。"
                } else {
                    "这些代码之间没有直接替代关系。"
                };
                Some((
                    "loinc_mapping_unverified",
                    format!(
                        "无法将报告 LOINC {incoming} 添加到数据库 LOINC 为 {existing} 的指标。{reason}来源为 LOINC {version} MapTo.csv。请选择报告代码对应的指标，或核对代码。"
                    ),
                ))
            }
            (
                Self::LoincCodeConflict {
                    incoming,
                    existing,
                    native,
                },
                English,
            ) => {
                let reason = if *native {
                    "This LOINC is already used by a different or ambiguous Apple Health metric."
                } else {
                    "This LOINC is already linked to another Browse metric."
                };
                let existing = if existing.is_empty() {
                    "none"
                } else {
                    existing
                };
                Some((
                    "invalid_request",
                    format!(
                        "{reason} Report LOINC {incoming}. Selected metric's database LOINC {existing}. Check the printed code and metric mapping."
                    ),
                ))
            }
            (
                Self::LoincCodeConflict {
                    incoming,
                    existing,
                    native,
                },
                Chinese,
            ) => {
                let reason = if *native {
                    "此 LOINC 已用于其他 Apple Health 指标，或关联到多个指标。"
                } else {
                    "此 LOINC 已关联到另一个 Browse 指标。"
                };
                let existing = if existing.is_empty() { "无" } else { existing };
                Some((
                    "invalid_request",
                    format!(
                        "{reason}报告 LOINC 为 {incoming}，所选指标的数据库 LOINC 为 {existing}。请核对报告代码和指标映射。"
                    ),
                ))
            }
            (Self::LoincMapUnavailable { incoming, existing }, English) => Some((
                "loinc_map_unavailable",
                format!(
                    "Cannot verify report LOINC {incoming} against database LOINC {existing}. Load the official LOINC release archive on the server before submitting this mapping."
                ),
            )),
            (Self::LoincMapUnavailable { incoming, existing }, Chinese) => Some((
                "loinc_map_unavailable",
                format!(
                    "暂时无法核验报告 LOINC {incoming} 与数据库 LOINC {existing} 的关系。请先在服务器加载官方 LOINC 发布包，再提交此映射。"
                ),
            )),
            _ => None,
        }
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
