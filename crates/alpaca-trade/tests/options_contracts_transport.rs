use alpaca_trade::options_contracts::{ContractStatus, ContractStyle, ContractType, ListRequest};
use alpaca_trade::{Client, Decimal, Error};
use std::str::FromStr;

#[path = "support/http_server.rs"]
mod http_server;

use http_server::TestServer;

const OPTIONS_CONTRACTS_LIST_REQUEST_LINE: &str = concat!(
    "GET /v2/options/contracts?",
    "underlying_symbols=AAPL%2CSPY&",
    "show_deliverables=true&",
    "status=active&",
    "expiration_date=2025-06-20&",
    "expiration_date_gte=2025-06-01&",
    "expiration_date_lte=2025-06-30&",
    "root_symbol=AAPL&",
    "type=call&",
    "style=american&",
    "strike_price_gte=100.00&",
    "strike_price_lte=200.00&",
    "page_token=MTAwMA%3D%3D&",
    "limit=100&",
    "ppind=true HTTP/1.1"
);

fn list_response_json() -> &'static str {
    r#"{"option_contracts":[{"id":"98359ef7-5124-49f3-85ea-5cf02df6defa","symbol":"AAPL250620C00100000","name":"AAPL Jun 20 2025 100 Call","status":"active","tradable":true,"expiration_date":"2025-06-20","root_symbol":"AAPL","underlying_symbol":"AAPL","underlying_asset_id":"b0b6dd9d-8b9b-48a9-ba46-b9d54906e415","type":"call","style":"american","strike_price":"100","multiplier":"100","size":"100","open_interest":"237","open_interest_date":"2023-12-11","close_price":"148.38","close_price_date":"2023-12-11","deliverables":[{"type":"equity","symbol":"AAPL","asset_id":"b0b6dd9d-8b9b-48a9-ba46-b9d54906e415","amount":"100","allocation_percentage":"100","settlement_type":"T+2","settlement_method":"CCC","delayed_settlement":false}]}],"next_page_token":"MTAwMA=="}"#
}

fn option_contract_json() -> &'static str {
    r#"{"id":"98359ef7-5124-49f3-85ea-5cf02df6defa","symbol":"AAPL250620C00100000","name":"AAPL Jun 20 2025 100 Call","status":"active","tradable":true,"expiration_date":"2025-06-20","root_symbol":"AAPL","underlying_symbol":"AAPL","underlying_asset_id":"b0b6dd9d-8b9b-48a9-ba46-b9d54906e415","type":"call","style":"american","strike_price":"100","multiplier":"100","size":"100","open_interest":"237","open_interest_date":"2023-12-11","close_price":"148.38","close_price_date":"2023-12-11","deliverables":[{"type":"equity","symbol":"AAPL","asset_id":"b0b6dd9d-8b9b-48a9-ba46-b9d54906e415","amount":"100","allocation_percentage":"100","settlement_type":"T+2","settlement_method":"CCC","delayed_settlement":false}]}"#
}

fn list_request() -> ListRequest {
    ListRequest {
        underlying_symbols: Some(vec!["AAPL".to_owned(), "SPY".to_owned()]),
        show_deliverables: Some(true),
        status: Some(ContractStatus::Active),
        expiration_date: Some("2025-06-20".to_owned()),
        expiration_date_gte: Some("2025-06-01".to_owned()),
        expiration_date_lte: Some("2025-06-30".to_owned()),
        root_symbol: Some("AAPL".to_owned()),
        r#type: Some(ContractType::Call),
        style: Some(ContractStyle::American),
        strike_price_gte: Some(Decimal::from_str("100.00").expect("decimal should parse")),
        strike_price_lte: Some(Decimal::from_str("200.00").expect("decimal should parse")),
        page_token: Some("MTAwMA==".to_owned()),
        limit: Some(100),
        ppind: Some(true),
    }
}

#[tokio::test]
async fn options_contracts_list_hits_official_path_query_and_sends_auth_headers() {
    let server = TestServer::spawn(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        list_response_json().len(),
        list_response_json()
    )]);

    let response = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .options_contracts()
        .list(list_request())
        .await
        .expect("options contracts list request should succeed");

    assert_eq!(response.option_contracts.len(), 1);
    assert_eq!(response.option_contracts[0].symbol, "AAPL250620C00100000");
    assert_eq!(
        response.option_contracts[0].strike_price,
        Decimal::from_str("100").expect("decimal should parse")
    );
    assert_eq!(response.next_page_token.as_deref(), Some("MTAwMA=="));

    let request = server.into_single_request();
    assert_eq!(request.request_line, OPTIONS_CONTRACTS_LIST_REQUEST_LINE);
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
async fn options_contracts_get_hits_symbol_and_uuid_paths_and_sends_auth_headers() {
    let uuid = "98359ef7-5124-49f3-85ea-5cf02df6defa";
    let server = TestServer::spawn(vec![
        format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            option_contract_json().len(),
            option_contract_json()
        ),
        format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            option_contract_json().len(),
            option_contract_json()
        ),
    ]);

    let client = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build");

    let contract_by_symbol = client
        .options_contracts()
        .get("AAPL250620C00100000")
        .await
        .expect("symbol get request should succeed");
    let contract_by_uuid = client
        .options_contracts()
        .get(uuid)
        .await
        .expect("uuid get request should succeed");

    assert_eq!(contract_by_symbol.symbol, "AAPL250620C00100000");
    assert_eq!(contract_by_uuid.id, uuid);

    let requests = server.into_requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0].request_line,
        "GET /v2/options/contracts/AAPL250620C00100000 HTTP/1.1"
    );
    assert_eq!(
        requests[1].request_line,
        format!("GET /v2/options/contracts/{uuid} HTTP/1.1")
    );
    for request in requests {
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
}

#[tokio::test]
async fn options_contracts_list_maps_429_to_rate_limited() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 429 Too Many Requests\r\nx-request-id: req-options-contracts-list-429-1\r\nretry-after: 0\r\ncontent-length: 9\r\nconnection: close\r\n\r\nslow down"
            .to_owned(),
        "HTTP/1.1 429 Too Many Requests\r\nx-request-id: req-options-contracts-list-429-2\r\nretry-after: 17\r\ncontent-length: 9\r\nconnection: close\r\n\r\nslow down"
            .to_owned(),
    ]);

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .options_contracts()
        .list(list_request())
        .await
        .expect_err("429 response must fail");

    match error {
        Error::RateLimited(meta) => {
            assert_eq!(meta.endpoint, "options_contracts.list");
            assert_eq!(meta.method, "GET");
            assert_eq!(meta.status, Some(429));
            assert_eq!(
                meta.request_id.as_deref(),
                Some("req-options-contracts-list-429-2")
            );
            assert_eq!(meta.retry_after.as_deref(), Some("17"));
            assert_eq!(meta.body.as_deref(), Some("slow down"));
        }
        other => panic!("expected rate limited error, got {other:?}"),
    }

    let requests = server.into_requests();
    assert_eq!(requests.len(), 2, "GET retries should issue two requests");
    for request in requests {
        assert_eq!(request.request_line, OPTIONS_CONTRACTS_LIST_REQUEST_LINE);
        assert!(request.body.is_empty());
    }
}

#[tokio::test]
async fn options_contracts_get_maps_503_to_http_status() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 503 Service Unavailable\r\nx-request-id: req-options-contracts-get-503-1\r\ncontent-length: 15\r\nconnection: close\r\n\r\nservice offline"
            .to_owned(),
        "HTTP/1.1 503 Service Unavailable\r\nx-request-id: req-options-contracts-get-503-2\r\ncontent-length: 15\r\nconnection: close\r\n\r\nservice offline"
            .to_owned(),
    ]);

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .options_contracts()
        .get("AAPL250620C00100000")
        .await
        .expect_err("503 response must fail");

    match error {
        Error::HttpStatus(meta) => {
            assert_eq!(meta.endpoint, "options_contracts.get");
            assert_eq!(meta.method, "GET");
            assert_eq!(meta.status, Some(503));
            assert_eq!(
                meta.request_id.as_deref(),
                Some("req-options-contracts-get-503-2")
            );
            assert_eq!(meta.body.as_deref(), Some("service offline"));
        }
        other => panic!("expected http status error, got {other:?}"),
    }

    let requests = server.into_requests();
    assert_eq!(requests.len(), 2, "GET retries should issue two requests");
    for request in requests {
        assert_eq!(
            request.request_line,
            "GET /v2/options/contracts/AAPL250620C00100000 HTTP/1.1"
        );
        assert!(request.body.is_empty());
    }
}

#[tokio::test]
async fn options_contracts_list_maps_malformed_json_to_deserialize() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nx-request-id: req-options-contracts-json-1\r\ncontent-length: 15\r\nconnection: close\r\n\r\n{not valid json"
            .to_owned(),
    ]);

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .options_contracts()
        .list(list_request())
        .await
        .expect_err("invalid json must fail");

    match error {
        Error::Deserialize { message, meta } => {
            assert!(!message.is_empty());
            assert_eq!(meta.endpoint, "options_contracts.list");
            assert_eq!(meta.method, "GET");
            assert_eq!(meta.status, Some(200));
            assert_eq!(
                meta.request_id.as_deref(),
                Some("req-options-contracts-json-1")
            );
            assert_eq!(meta.body.as_deref(), Some("{not valid json"));
        }
        other => panic!("expected deserialize error, got {other:?}"),
    }

    let request = server.into_single_request();
    assert_eq!(request.request_line, OPTIONS_CONTRACTS_LIST_REQUEST_LINE);
    assert!(request.body.is_empty());
}

#[tokio::test]
async fn options_contracts_get_rejects_invalid_path_segment_before_send() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}".to_owned(),
    ]);

    for value in [
        "/",
        "%2F",
        " AAPL250620C00100000 ",
        " AAPL250620C00100000",
        "AAPL250620C00100000 ",
    ] {
        let error = Client::builder()
            .api_key("key")
            .secret_key("secret")
            .base_url(server.base_url())
            .build()
            .expect("client should build")
            .options_contracts()
            .get(value)
            .await
            .expect_err("invalid path segments must fail before send");

        match error {
            Error::InvalidRequest(message) => {
                assert!(message.contains("symbol_or_id"));
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
