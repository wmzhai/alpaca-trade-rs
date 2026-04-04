use alpaca_trade::{Client, Error};

#[tokio::test]
async fn mock_rate_limit_maps_retry_after_header() {
    let server = alpaca_trade_mock::spawn_test_server().await;

    let http = reqwest::Client::new();
    let response = http
        .post(format!("{}/__admin/faults", server.base_url))
        .json(&serde_json::json!({
            "method": "GET",
            "path": "/v2/account",
            "status": 429,
            "headers": { "retry-after": "3" },
            "body": "too many requests"
        }))
        .send()
        .await
        .expect("fault request should succeed");
    assert!(response.status().is_success());

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url.clone())
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect_err("request should fail");

    match error {
        Error::RateLimited { retry_after, body } => {
            assert_eq!(retry_after.as_deref(), Some("3"));
            assert_eq!(body.as_deref(), Some("too many requests"));
        }
        other => panic!("expected rate limited error, got {other:?}"),
    }
}

#[tokio::test]
async fn malformed_json_maps_deserialize_error() {
    let server = alpaca_trade_mock::spawn_test_server().await;

    let http = reqwest::Client::new();
    let response = http
        .post(format!("{}/__admin/faults", server.base_url))
        .json(&serde_json::json!({
            "method": "GET",
            "path": "/v2/account",
            "status": 200,
            "headers": { "content-type": "application/json" },
            "body": "not-json"
        }))
        .send()
        .await
        .expect("fault request should succeed");
    assert!(response.status().is_success());

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url.clone())
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect_err("request should fail");

    assert!(matches!(error, Error::Deserialize(_)));
}
