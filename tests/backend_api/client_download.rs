use axum::{body::to_bytes, http::header};
use longlivehealth::{client_download::packaged_android, error::AppError};

#[tokio::test]
async fn apk_download_streams_only_packaged_file_and_reports_missing_artifact() {
    let directory = tempfile::tempdir().unwrap();
    assert!(matches!(
        packaged_android(directory.path().into()).await,
        Err(AppError::NotFound)
    ));
    let bytes = b"PK\x03\x04synthetic package";
    std::fs::write(directory.path().join("longlivehealth.apk"), bytes).unwrap();
    let response = packaged_android(directory.path().into()).await.unwrap();
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "application/vnd.android.package-archive"
    );
    assert_eq!(
        response.headers()[header::CONTENT_LENGTH],
        bytes.len().to_string()
    );
    assert_eq!(
        to_bytes(response.into_body(), 1024).await.unwrap().as_ref(),
        bytes
    );
}
