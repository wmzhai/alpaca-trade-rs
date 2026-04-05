use alpaca_trade::{Client, Error};
#[path = "support/http_server.rs"]
mod http_server;

use http_server::TestServer;

fn account_json() -> &'static str {
    r#"{"id":"acct-1","account_number":"010203ABCD","status":"ACTIVE"}"#
}

#[tokio::test]
async fn account_get_hits_official_path_and_sends_auth_headers() {
    let server = TestServer::spawn(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        account_json().len(),
        account_json()
    )]);

    let account = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect("account request should succeed");

    assert_eq!(account.id, "acct-1");

    let request = server.into_single_request();
    assert_eq!(request.request_line, "GET /v2/account HTTP/1.1");
    assert!(request.body.is_empty());
    assert_eq!(
        request.headers.get("apca-api-key-id"),
        Some(&"key".to_owned())
    );
    assert_eq!(
        request.headers.get("apca-api-secret-key"),
        Some(&"secret".to_owned())
    );
}

#[tokio::test]
async fn account_get_maps_429_to_rate_limited() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 429 Too Many Requests\r\nx-request-id: req-account-429-1\r\nretry-after: 0\r\ncontent-length: 9\r\nconnection: close\r\n\r\nslow down"
            .to_owned(),
        "HTTP/1.1 429 Too Many Requests\r\nx-request-id: req-account-429-2\r\nretry-after: 17\r\ncontent-length: 9\r\nconnection: close\r\n\r\nslow down"
            .to_owned(),
    ]);

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect_err("429 response must fail");

    match error {
        Error::RateLimited(meta) => {
            assert_eq!(meta.endpoint, "account.get");
            assert_eq!(meta.method, "GET");
            assert_eq!(meta.status, Some(429));
            assert_eq!(meta.request_id.as_deref(), Some("req-account-429-2"));
            assert_eq!(meta.retry_after.as_deref(), Some("17"));
            assert_eq!(meta.body.as_deref(), Some("slow down"));
        }
        other => panic!("expected rate limited error, got {other:?}"),
    }

    let requests = server.into_requests();
    assert_eq!(requests.len(), 2, "GET retries should issue two requests");
    for request in requests {
        assert_eq!(request.request_line, "GET /v2/account HTTP/1.1");
        assert!(request.body.is_empty());
    }
}

#[tokio::test]
async fn account_get_maps_non_success_status_to_http_status() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 503 Service Unavailable\r\nx-request-id: req-account-503-1\r\ncontent-length: 15\r\nconnection: close\r\n\r\nservice offline"
            .to_owned(),
        "HTTP/1.1 503 Service Unavailable\r\nx-request-id: req-account-503-2\r\ncontent-length: 15\r\nconnection: close\r\n\r\nservice offline"
            .to_owned(),
    ]);

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect_err("503 response must fail");

    match error {
        Error::HttpStatus(meta) => {
            assert_eq!(meta.endpoint, "account.get");
            assert_eq!(meta.method, "GET");
            assert_eq!(meta.status, Some(503));
            assert_eq!(meta.request_id.as_deref(), Some("req-account-503-2"));
            assert_eq!(meta.body.as_deref(), Some("service offline"));
        }
        other => panic!("expected http status error, got {other:?}"),
    }

    let requests = server.into_requests();
    assert_eq!(requests.len(), 2, "GET retries should issue two requests");
    for request in requests {
        assert_eq!(request.request_line, "GET /v2/account HTTP/1.1");
        assert!(request.body.is_empty());
    }
}

#[tokio::test]
async fn account_get_maps_malformed_json_to_deserialize() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nx-request-id: req-account-json-1\r\ncontent-length: 15\r\nconnection: close\r\n\r\n{not valid json"
            .to_owned(),
    ]);

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect_err("invalid json must fail");

    match error {
        Error::Deserialize { message, meta } => {
            assert!(!message.is_empty());
            assert_eq!(meta.endpoint, "account.get");
            assert_eq!(meta.method, "GET");
            assert_eq!(meta.status, Some(200));
            assert_eq!(meta.request_id.as_deref(), Some("req-account-json-1"));
            assert_eq!(meta.body.as_deref(), Some("{not valid json"));
        }
        other => panic!("expected deserialize error, got {other:?}"),
    }

    let request = server.into_single_request();
    assert_eq!(request.request_line, "GET /v2/account HTTP/1.1");
    assert!(request.body.is_empty());
}
