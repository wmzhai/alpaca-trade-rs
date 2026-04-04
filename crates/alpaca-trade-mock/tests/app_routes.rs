use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok() {
    let app = alpaca_trade_mock::build_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn account_requires_auth_headers() {
    let app = alpaca_trade_mock::build_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v2/account")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn account_returns_seed_payload_when_authed() {
    let app = alpaca_trade_mock::build_app();

    let request = Request::builder()
        .uri("/v2/account")
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.expect("request should succeed");
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json should parse");
    assert_eq!(json["status"], "ACTIVE");
}
