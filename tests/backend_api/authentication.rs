use crate::{add_user, fixture};
use longlivehealth::{
    authentication::{digest, hash_password, login, normalize_username, now},
    models::LoginRequest,
};

#[tokio::test]
async fn passwords_sessions_and_revoke_are_real() {
    let (_directory, state) = fixture().await;
    add_user(&state, "alice").await;
    assert!(hash_password("short".into()).await.is_err());
    assert_eq!(normalize_username(" ALICE ").unwrap(), "alice");
    assert!(normalize_username("../alice").is_err());
    assert!(normalize_username("a").is_err());
    for username in ["alice", "missing"] {
        assert!(
            login(
                &state,
                LoginRequest {
                    username: username.into(),
                    password: "wrong-password".into()
                }
            )
            .await
            .is_err()
        );
    }
    let session = login(
        &state,
        LoginRequest {
            username: "Alice".into(),
            password: "correct-password-123".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(session.token.len(), 64);
    let token_hash = digest(session.token.as_bytes());
    assert_eq!(
        state
            .database
            .session_user(&token_hash, now().unwrap())
            .await
            .unwrap()
            .username,
        "alice"
    );
    assert!(
        state
            .database
            .session_user(&token_hash, session.expires_at)
            .await
            .is_err()
    );
    state.database.revoke_session(&token_hash).await.unwrap();
    assert!(
        state
            .database
            .session_user(&token_hash, now().unwrap())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn password_reset_invalidates_inflight_credentials_and_old_sessions() {
    let (_directory, state) = fixture().await;
    add_user(&state, "alice").await;
    let old = state.database.credentials("alice").await.unwrap().unwrap();
    let session = crate::token(&state, "alice").await;
    let new_hash = hash_password("a-new-password-456".into()).await.unwrap();
    state
        .database
        .change_credentials("alice", Some(&new_hash))
        .await
        .unwrap();
    assert!(
        state
            .database
            .session_user(&digest(session.as_bytes()), now().unwrap())
            .await
            .is_err()
    );
    assert!(
        state
            .database
            .create_session("stale", now().unwrap() + 100, None, &old, now().unwrap())
            .await
            .is_err()
    );
    assert!(
        login(
            &state,
            LoginRequest {
                username: "alice".into(),
                password: "correct-password-123".into()
            }
        )
        .await
        .is_err()
    );
    assert!(
        login(
            &state,
            LoginRequest {
                username: "alice".into(),
                password: "a-new-password-456".into()
            }
        )
        .await
        .is_ok()
    );
    state
        .database
        .change_credentials("alice", None)
        .await
        .unwrap();
    assert!(
        login(
            &state,
            LoginRequest {
                username: "alice".into(),
                password: "a-new-password-456".into()
            }
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn rotating_refresh_recovers_lost_response_and_revokes_replay() {
    use longlivehealth::{authentication::refresh, models::RefreshRequest};
    let (_directory, state) = fixture().await;
    add_user(&state, "alice").await;
    let first = login(
        &state,
        LoginRequest {
            username: "alice".into(),
            password: "correct-password-123".into(),
        },
    )
    .await
    .unwrap();
    let next = "ab".repeat(32);
    let request = || RefreshRequest {
        refresh_token: first.refresh_token.clone(),
        next_refresh_token: next.clone(),
    };
    let (left, right) = tokio::join!(refresh(&state, request()), refresh(&state, request()));
    let rotated = left.unwrap();
    assert_eq!(rotated.token, right.unwrap().token);
    assert_ne!(rotated.token, first.token);
    assert_eq!(rotated.refresh_token, next);
    assert!(rotated.expires_at <= now().unwrap() + 3600);
    assert_eq!(rotated.refresh_after, rotated.expires_at - 1800);
    // In-flight work may finish with the old access token for only five minutes.
    assert!(
        state
            .database
            .session_user(&digest(first.token.as_bytes()), now().unwrap())
            .await
            .is_ok()
    );
    assert!(
        state
            .database
            .session_user(&digest(first.token.as_bytes()), now().unwrap() + 301)
            .await
            .is_err()
    );
    // Refresh secrets cannot be used as API bearer credentials.
    assert!(
        state
            .database
            .session_user(&digest(next.as_bytes()), now().unwrap())
            .await
            .is_err()
    );
    // A consumed credential paired with a different successor revokes the whole login.
    assert!(
        refresh(
            &state,
            RefreshRequest {
                refresh_token: first.refresh_token,
                next_refresh_token: "cd".repeat(32)
            }
        )
        .await
        .is_err()
    );
    assert!(
        state
            .database
            .session_user(&digest(rotated.token.as_bytes()), now().unwrap())
            .await
            .is_err()
    );
    assert!(
        refresh(
            &state,
            RefreshRequest {
                refresh_token: next,
                next_refresh_token: "ef".repeat(32)
            }
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn refresh_survives_expired_access_but_not_logout_reset_or_inactivity() {
    use longlivehealth::{authentication::refresh, models::RefreshRequest};
    let (directory, state) = fixture().await;
    add_user(&state, "alice").await;
    let pool = sqlx::SqlitePool::connect(&format!(
        "sqlite:{}",
        directory.path().join("database.sqlite").display()
    ))
    .await
    .unwrap();
    for reason in ["access", "logout", "reset", "inactive"] {
        let first = login(
            &state,
            LoginRequest {
                username: "alice".into(),
                password: "correct-password-123".into(),
            },
        )
        .await
        .unwrap();
        let hash = digest(first.token.as_bytes());
        match reason {
            "access" => {
                sqlx::query("UPDATE sessions SET expires_at = 0 WHERE token_hash = ?")
                    .bind(&hash)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            "logout" => {
                state
                    .database
                    .revoke_refresh_session(&digest(first.refresh_token.as_bytes()))
                    .await
                    .unwrap();
            }
            "reset" => {
                state
                    .database
                    .change_credentials(
                        "alice",
                        Some(&hash_password("correct-password-123".into()).await.unwrap()),
                    )
                    .await
                    .unwrap();
            }
            _ => {
                sqlx::query("UPDATE sessions SET refresh_expires_at = 0 WHERE token_hash = ?")
                    .bind(&hash)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
        }
        let result = refresh(
            &state,
            RefreshRequest {
                refresh_token: first.refresh_token,
                next_refresh_token: "12".repeat(32),
            },
        )
        .await;
        assert_eq!(result.is_ok(), reason == "access");
    }
    pool.close().await;
}

#[tokio::test]
async fn refresh_routes_keep_password_change_and_logout_revocation_effective() {
    use crate::request;
    use axum::http::StatusCode;
    use serde_json::json;
    let (_directory, state) = fixture().await;
    add_user(&state, "alice").await;
    let app = longlivehealth::app::create_application(state.clone());
    let (_, first) = request(
        &app,
        "/api/v1/session/login",
        None,
        json!({"username":"alice","password":"correct-password-123"}),
    )
    .await;
    let next = "34".repeat(32);
    let (status, rotated) = request(
        &app,
        "/api/v1/session/refresh",
        None,
        json!({"refresh_token":first["refresh_token"],"next_refresh_token":next}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = request(
        &app,
        "/api/v1/user/password/change",
        first["token"].as_str(),
        json!({"current_password":"correct-password-123","new_password":"a-new-password-456"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK); // Request began just before rotation.
    assert_eq!(
        request(
            &app,
            "/api/v1/user/get",
            rotated["token"].as_str(),
            json!({})
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &app,
            "/api/v1/session/refresh",
            None,
            json!({"refresh_token":next,"next_refresh_token":"56".repeat(32)})
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let (_, second) = request(
        &app,
        "/api/v1/session/login",
        None,
        json!({"username":"alice","password":"a-new-password-456"}),
    )
    .await;
    assert_eq!(
        request(
            &app,
            "/api/v1/session/revoke",
            None,
            json!({"refresh_token":second["refresh_token"]})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &app,
            "/api/v1/user/get",
            second["token"].as_str(),
            json!({})
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}
