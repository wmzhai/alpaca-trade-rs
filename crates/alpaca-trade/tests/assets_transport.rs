use alpaca_trade::assets::ListRequest;
use alpaca_trade::{Client, Decimal, Error};
use std::str::FromStr;
#[path = "support/http_server.rs"]
mod http_server;

use http_server::TestServer;

fn assets_json() -> &'static str {
    r#"[{"id":"b0b6dd9d-8b9b-48a9-ba46-b9d54906e415","class":"us_equity","exchange":"NASDAQ","symbol":"AAPL","name":"Apple Inc. Common Stock","status":"active","tradable":true,"marginable":true,"shortable":true,"easy_to_borrow":true,"fractionable":true,"maintenance_margin_requirement":30.0,"margin_requirement_long":"30","margin_requirement_short":"100","attributes":["has_options"]}]"#
}

fn asset_json() -> &'static str {
    r#"{"id":"b0b6dd9d-8b9b-48a9-ba46-b9d54906e415","class":"us_equity","exchange":"NASDAQ","symbol":"AAPL","name":"Apple Inc. Common Stock","status":"active","tradable":true,"marginable":true,"shortable":true,"easy_to_borrow":true,"fractionable":true,"maintenance_margin_requirement":30.0,"margin_requirement_long":"30","margin_requirement_short":"100","attributes":["has_options"]}"#
}

fn list_request() -> ListRequest {
    ListRequest {
        status: Some("active".to_owned()),
        asset_class: Some("us_equity".to_owned()),
        exchange: Some("NASDAQ".to_owned()),
        attributes: Some(vec!["has_options".to_owned()]),
    }
}

#[tokio::test]
async fn assets_list_hits_official_path_query_and_sends_auth_headers() {
    let server = TestServer::spawn(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        assets_json().len(),
        assets_json()
    )]);

    let assets = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .assets()
        .list(list_request())
        .await
        .expect("assets list request should succeed");

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].symbol, "AAPL");
    assert_eq!(
        assets[0].maintenance_margin_requirement,
        Some(Decimal::from_str("30.0").expect("decimal should parse"))
    );
    assert_eq!(
        assets[0].margin_requirement_long,
        Some(Decimal::from_str("30").expect("decimal should parse"))
    );

    let request = server.into_single_request();
    assert_eq!(
        request.request_line,
        "GET /v2/assets?status=active&asset_class=us_equity&exchange=NASDAQ&attributes=has_options HTTP/1.1"
    );
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
async fn asset_get_hits_official_path_and_sends_auth_headers() {
    let server = TestServer::spawn(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        asset_json().len(),
        asset_json()
    )]);

    let asset = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .assets()
        .get("AAPL")
        .await
        .expect("asset get request should succeed");

    assert_eq!(asset.symbol, "AAPL");
    assert_eq!(
        asset.margin_requirement_short,
        Some(Decimal::from_str("100").expect("decimal should parse"))
    );

    let request = server.into_single_request();
    assert_eq!(request.request_line, "GET /v2/assets/AAPL HTTP/1.1");
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
async fn assets_list_maps_429_to_rate_limited() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 429 Too Many Requests\r\nx-request-id: req-assets-429-1\r\nretry-after: 0\r\ncontent-length: 9\r\nconnection: close\r\n\r\nslow down"
            .to_owned(),
        "HTTP/1.1 429 Too Many Requests\r\nx-request-id: req-assets-429-2\r\nretry-after: 17\r\ncontent-length: 9\r\nconnection: close\r\n\r\nslow down"
            .to_owned(),
    ]);

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .assets()
        .list(list_request())
        .await
        .expect_err("429 response must fail");

    match error {
        Error::RateLimited(meta) => {
            assert_eq!(meta.endpoint, "assets.list");
            assert_eq!(meta.method, "GET");
            assert_eq!(meta.status, Some(429));
            assert_eq!(meta.request_id.as_deref(), Some("req-assets-429-2"));
            assert_eq!(meta.retry_after.as_deref(), Some("17"));
            assert_eq!(meta.body.as_deref(), Some("slow down"));
        }
        other => panic!("expected rate limited error, got {other:?}"),
    }

    let requests = server.into_requests();
    assert_eq!(requests.len(), 2, "GET retries should issue two requests");
    for request in requests {
        assert_eq!(
            request.request_line,
            "GET /v2/assets?status=active&asset_class=us_equity&exchange=NASDAQ&attributes=has_options HTTP/1.1"
        );
        assert!(request.body.is_empty());
    }
}

#[tokio::test]
async fn asset_get_maps_non_success_status_to_http_status() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 503 Service Unavailable\r\nx-request-id: req-asset-503-1\r\ncontent-length: 15\r\nconnection: close\r\n\r\nservice offline"
            .to_owned(),
        "HTTP/1.1 503 Service Unavailable\r\nx-request-id: req-asset-503-2\r\ncontent-length: 15\r\nconnection: close\r\n\r\nservice offline"
            .to_owned(),
    ]);

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .assets()
        .get("AAPL")
        .await
        .expect_err("503 response must fail");

    match error {
        Error::HttpStatus(meta) => {
            assert_eq!(meta.endpoint, "assets.get");
            assert_eq!(meta.method, "GET");
            assert_eq!(meta.status, Some(503));
            assert_eq!(meta.request_id.as_deref(), Some("req-asset-503-2"));
            assert_eq!(meta.body.as_deref(), Some("service offline"));
        }
        other => panic!("expected http status error, got {other:?}"),
    }

    let requests = server.into_requests();
    assert_eq!(requests.len(), 2, "GET retries should issue two requests");
    for request in requests {
        assert_eq!(request.request_line, "GET /v2/assets/AAPL HTTP/1.1");
        assert!(request.body.is_empty());
    }
}

#[tokio::test]
async fn assets_list_maps_malformed_json_to_deserialize() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nx-request-id: req-assets-json-1\r\ncontent-length: 15\r\nconnection: close\r\n\r\n{not valid json"
            .to_owned(),
    ]);

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .assets()
        .list(list_request())
        .await
        .expect_err("invalid json must fail");

    match error {
        Error::Deserialize { message, meta } => {
            assert!(!message.is_empty());
            assert_eq!(meta.endpoint, "assets.list");
            assert_eq!(meta.method, "GET");
            assert_eq!(meta.status, Some(200));
            assert_eq!(meta.request_id.as_deref(), Some("req-assets-json-1"));
            assert_eq!(meta.body.as_deref(), Some("{not valid json"));
        }
        other => panic!("expected deserialize error, got {other:?}"),
    }

    let request = server.into_single_request();
    assert_eq!(
        request.request_line,
        "GET /v2/assets?status=active&asset_class=us_equity&exchange=NASDAQ&attributes=has_options HTTP/1.1"
    );
    assert!(request.body.is_empty());
}

#[tokio::test]
async fn asset_get_rejects_invalid_path_segment_before_send() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}".to_owned(),
    ]);

    for value in ["AAPL/US", "AAPL%2FUS"] {
        let error = Client::builder()
            .api_key("key")
            .secret_key("secret")
            .base_url(server.base_url())
            .build()
            .expect("client should build")
            .assets()
            .get(value)
            .await
            .expect_err("reserved path characters must fail before send");

        match error {
            Error::InvalidRequest(message) => {
                assert!(message.contains("symbol_or_asset_id"));
            }
            other => panic!("expected invalid request error, got {other:?}"),
        }
    }

    let requests = server.into_requests();
    assert!(
        requests.is_empty(),
        "invalid paths should not send any request"
    );
}
