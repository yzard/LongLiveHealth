use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn response_language_changes_the_label_and_preserves_the_error_code() {
    let (_directory, state) = crate::fixture().await;
    let app = longlivehealth::app::create_application(state);
    for (accept, language, message) in [
        (
            "en, zh-Hans;q=0.5",
            "en",
            "Authentication required or credentials invalid",
        ),
        ("zh-Hans, en;q=0.5", "zh-Hans", "需要身份验证或凭据无效"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post("/api/v1/reports/list")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::ACCEPT_LANGUAGE, accept)
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()[header::CONTENT_LANGUAGE], language);
        let bytes = to_bytes(response.into_body(), 8 * 1024).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"]["code"], "unauthorized");
        assert_eq!(body["error"]["message"], message);
    }
}
