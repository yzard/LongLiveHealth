use crate::{fixture, request};
use axum::http::StatusCode;
use longlivehealth::{app::create_application, authentication::initialize_admin};
use serde_json::json;

#[tokio::test]
async fn bootstrap_rotation_permissions_and_reset() {
    let (_directory, state) = fixture().await;
    initialize_admin(&state.database, false).await.unwrap();
    let app = create_application(state.clone());
    let (status, login) = request(
        &app,
        "/api/v1/session/login",
        None,
        json!({"username":"admin","password":"admin"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(login["user"]["must_change_password"], true);
    let token = login["token"].as_str().unwrap();
    for path in [
        "/api/v1/reports/list",
        "/api/v1/users/list",
        "/api/v1/user/delete",
    ] {
        assert_eq!(
            request(&app, path, Some(token), json!({})).await.0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        request(
            &app,
            "/api/v1/user/password/change",
            Some(token),
            json!({"current_password":"admin","new_password":"short"})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(
            &app,
            "/api/v1/user/password/change",
            Some(token),
            json!({"current_password":"admin","new_password":"new-admin-password"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "/api/v1/user/get", Some(token), json!({}))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    initialize_admin(&state.database, false).await.unwrap();
    let (status, login) = request(
        &app,
        "/api/v1/session/login",
        None,
        json!({"username":"admin","password":"new-admin-password"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(login["user"]["must_change_password"], false);
    let token = login["token"].as_str().unwrap();
    assert_eq!(
        request(
            &app,
            "/api/v1/user/delete",
            Some(token),
            json!({"confirmation":"admin"})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert!(
        state
            .database
            .change_credentials("admin", None)
            .await
            .is_err()
    );
    assert_eq!(
        request(
            &app,
            "/api/v1/users/delete",
            Some(token),
            json!({"user_id":login["user"]["user_id"],"confirmation":"admin"})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, user) = request(
        &app,
        "/api/v1/users/create",
        Some(token),
        json!({"username":"alice","password":"alice-password-123"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, alice) = request(
        &app,
        "/api/v1/session/login",
        None,
        json!({"username":"alice","password":"alice-password-123"}),
    )
    .await;
    for path in [
        "/api/v1/users/list",
        "/api/v1/users/create",
        "/api/v1/users/delete",
    ] {
        let body = match path {
            "/api/v1/users/list" => json!({"limit":100}),
            "/api/v1/users/create" => json!({"username":"bob","password":"bob-password-123"}),
            _ => json!({"user_id":user["user_id"],"confirmation":"alice"}),
        };
        assert_eq!(
            request(&app, path, alice["token"].as_str(), body).await.0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        request(
            &app,
            "/api/v1/users/delete",
            Some(token),
            json!({"user_id":user["user_id"],"confirmation":"alice"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "/api/v1/user/get", alice["token"].as_str(), json!({}))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    initialize_admin(&state.database, true).await.unwrap();
    assert_eq!(
        request(&app, "/api/v1/user/get", Some(token), json!({}))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let (status, login) = request(
        &app,
        "/api/v1/session/login",
        None,
        json!({"username":"admin","password":"admin"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(login["user"]["must_change_password"], true);
    state.shutdown().await.unwrap();
}
