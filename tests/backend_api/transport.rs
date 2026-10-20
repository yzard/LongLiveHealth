use crate::{add_user, fixture, request, token};
use axum::{
    body::{Body, Bytes},
    http::{Request, StatusCode},
};
use std::{sync::Arc, time::Duration};
use tower::ServiceExt;

#[tokio::test]
async fn slow_health_bodies_are_bounded_before_buffering_and_cancellation_releases_admission() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "alice").await;
    let token = token(&state, "alice").await;
    let app = longlivehealth::app::create_application(state.clone());
    let mut callers = Vec::new();
    for _ in 0..2 {
        let reading = Arc::new(tokio::sync::Notify::new());
        let read_in = reading.clone();
        let stream = futures_util::stream::once(async move {
            read_in.notify_one();
            std::future::pending::<Result<Bytes, std::io::Error>>().await
        });
        let slow = Request::builder()
            .method("POST")
            .uri("/api/v1/health/sync")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from_stream(stream))
            .unwrap();
        callers.push(tokio::spawn(app.clone().oneshot(slow)));
        tokio::time::timeout(Duration::from_secs(10), reading.notified())
            .await
            .unwrap();
    }
    assert_eq!(
        request(
            &app,
            "/api/v1/health/sync",
            Some(&token),
            serde_json::json!({})
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        request(&app, "/api/v1/server/status", None, serde_json::json!({}))
            .await
            .0,
        StatusCode::OK
    );
    for caller in callers {
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
    }
    let connection = state
        .database
        .create_connection(
            &user.user_id,
            longlivehealth::health::ConnectionRequest {
                platform: "apple_health".into(),
                installation_id: uuid::Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap();
    let input = serde_json::to_value(super::health_batch::batch(
        &connection.connection_id,
        vec![],
    ))
    .unwrap();
    let valid = Request::builder()
        .method("POST")
        .uri("/api/v1/health/sync")
        .header("content-type", "Application/JSON; charset=utf-8")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(serde_json::to_vec(&input).unwrap()))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(valid).await.unwrap().status(),
        StatusCode::OK
    );
    assert_eq!(
        request(
            &app,
            "/api/v1/health/sync",
            Some(&token),
            serde_json::json!({})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn coverage_response_is_complete_over_http_with_a_post_body() {
    let (_directory, state) = fixture().await;
    let user = add_user(&state, "coverage-http").await;
    let token = token(&state, "coverage-http").await;
    let connection = state
        .database
        .create_connection(
            &user.user_id,
            longlivehealth::health::ConnectionRequest {
                platform: "apple_health".into(),
                installation_id: uuid::Uuid::new_v4().to_string(),
            },
        )
        .await
        .unwrap();
    for i in 0..300 {
        state
            .database
            .sync_health(
                &user.user_id,
                longlivehealth::health::SyncRequest {
                    connection_id: connection.connection_id.clone(),
                    batch_id: uuid::Uuid::new_v4().to_string(),
                    record_type: format!("synthetic-type-{i}"),
                    coverage_status: "no_visible_samples".into(),
                    records: vec![],
                },
            )
            .await
            .unwrap();
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = longlivehealth::app::create_application(state.clone());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    // A proxy can forward headers and body in separate packets. The handler must
    // consume the declared POST body before sending a large response.
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
    socket.write_all(format!("POST /api/v1/health/coverage HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
    let mut premature = [0; 1];
    assert!(
        tokio::time::timeout(Duration::from_millis(100), socket.read(&mut premature))
            .await
            .is_err(),
        "coverage must consume the POST body before responding"
    );
    socket.write_all(b"{}").await.unwrap();
    let mut response = Vec::new();
    socket
        .read_to_end(&mut response)
        .await
        .expect("delayed POST body must not reset the response");
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap()
        + 4;
    let coverage: serde_json::Value = serde_json::from_slice(&response[header_end..]).unwrap();
    assert_eq!(coverage["coverage"].as_array().unwrap().len(), 300);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    for _ in 0..5 {
        let response = client
            .post(format!("http://{address}/api/v1/health/coverage"))
            .bearer_auth(&token)
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response
            .bytes()
            .await
            .expect("coverage response must arrive without a reset");
        let coverage: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(coverage["coverage"].as_array().unwrap().len(), 300);
    }
    server.abort();
}
