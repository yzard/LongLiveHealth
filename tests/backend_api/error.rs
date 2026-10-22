use axum::response::IntoResponse;
use http_body_util::BodyExt;
use longlivehealth::error::AppError;

#[tokio::test]
async fn internal_errors_do_not_disclose_driver_or_file_details() {
    let response =
        AppError::Io(std::io::Error::other("PRIVATE_HEALTH_VALUE at secret/path")).into_response();
    assert_eq!(response.status(), 500);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(!body.contains("PRIVATE"));
    assert!(!body.contains("secret"));
    assert!(body.contains("internal_error"));
}

#[tokio::test]
async fn storage_permission_failures_explain_the_cause_without_private_paths() {
    for (error, code, reason) in [
        (
            AppError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "PRIVATE secret/path",
            )),
            "storage_permission_denied",
            "permission to read or write",
        ),
        (
            AppError::Database(sqlx::Error::Io(std::io::Error::from(
                std::io::ErrorKind::PermissionDenied,
            ))),
            "storage_permission_denied",
            "permission to read or write",
        ),
        (
            AppError::Io(std::io::Error::from(std::io::ErrorKind::ReadOnlyFilesystem)),
            "storage_read_only",
            "read-only",
        ),
    ] {
        let response = error.into_response();
        assert_eq!(response.status(), 500);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"]["code"], code);
        assert!(body["error"]["message"].as_str().unwrap().contains(reason));
        assert!(!body.to_string().contains("PRIVATE"));
        assert!(!body.to_string().contains("secret/path"));
    }
}

#[tokio::test]
async fn readonly_database_returns_an_actionable_storage_error() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query("PRAGMA query_only = ON")
        .execute(&pool)
        .await
        .unwrap();
    let error = sqlx::query("CREATE TABLE permission_test (id INTEGER)")
        .execute(&pool)
        .await
        .unwrap_err();
    let response = AppError::Database(error).into_response();
    assert_eq!(response.status(), 500);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["error"]["code"], "storage_read_only");
}
