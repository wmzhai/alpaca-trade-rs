use alpaca_trade::orders::{
    CreateRequest, ListRequest, OptionLegRequest, OrderClass, OrderSide, OrderType, PositionIntent,
    QueryOrderStatus, ReplaceRequest, SortDirection, StopLoss, TakeProfit, TimeInForce,
};
use alpaca_trade::{Client, Error};
use serde_json::json;

#[path = "support/http_server.rs"]
mod http_server;

use http_server::TestServer;

const ORDERS_LIST_REQUEST_LINE: &str = concat!(
    "GET /v2/orders?",
    "status=open&",
    "limit=100&",
    "after=2026-04-01T09%3A30%3A00Z&",
    "until=2026-04-06T16%3A00%3A00Z&",
    "direction=desc&",
    "nested=true&",
    "symbols=SPY%2CAAPL&",
    "side=buy&",
    "asset_class=us_equity HTTP/1.1"
);

fn order_json() -> &'static str {
    r#"{"id":"904837e3-3b76-47ec-b432-046db621571b","client_order_id":"phase7-orders-transport-1","created_at":"2026-04-06T15:04:05Z","updated_at":"2026-04-06T15:04:05Z","submitted_at":"2026-04-06T15:04:05Z","filled_at":null,"expired_at":null,"expires_at":null,"canceled_at":null,"failed_at":null,"replaced_at":null,"replaced_by":null,"replaces":null,"asset_id":"b0b6dd9d-8b9b-48a9-ba46-b9d54906e415","symbol":"SPY","asset_class":"us_equity","notional":null,"qty":"1","filled_qty":"0","filled_avg_price":null,"order_class":"simple","order_type":"limit","type":"limit","side":"buy","position_intent":null,"time_in_force":"day","limit_price":"499.25","stop_price":null,"status":"accepted","extended_hours":false,"legs":null,"trail_percent":null,"trail_price":null,"hwm":null,"ratio_qty":null,"take_profit":null,"stop_loss":null,"subtag":null,"source":null}"#
}

fn list_response_json() -> String {
    format!("[{}]", order_json())
}

fn list_request() -> ListRequest {
    ListRequest {
        status: Some(QueryOrderStatus::Open),
        limit: Some(100),
        after: Some("2026-04-01T09:30:00Z".to_owned()),
        until: Some("2026-04-06T16:00:00Z".to_owned()),
        direction: Some(SortDirection::Desc),
        nested: Some(true),
        symbols: Some(vec!["SPY".to_owned(), "AAPL".to_owned()]),
        side: Some(OrderSide::Buy),
        asset_class: Some("us_equity".to_owned()),
    }
}

fn create_request() -> CreateRequest {
    CreateRequest {
        symbol: Some("SPY".to_owned()),
        qty: Some(rust_decimal::Decimal::new(1, 0)),
        notional: None,
        side: Some(OrderSide::Buy),
        r#type: Some(OrderType::Limit),
        time_in_force: Some(TimeInForce::Day),
        limit_price: Some(rust_decimal::Decimal::new(49925, 2)),
        stop_price: None,
        trail_price: None,
        trail_percent: None,
        extended_hours: Some(false),
        client_order_id: Some("phase7-orders-create-transport-1".to_owned()),
        order_class: Some(OrderClass::Bracket),
        take_profit: Some(TakeProfit {
            limit_price: rust_decimal::Decimal::new(51000, 2),
        }),
        stop_loss: Some(StopLoss {
            stop_price: rust_decimal::Decimal::new(49200, 2),
            limit_price: Some(rust_decimal::Decimal::new(49150, 2)),
        }),
        legs: None,
        position_intent: Some(PositionIntent::BuyToOpen),
    }
}

fn replace_request() -> ReplaceRequest {
    ReplaceRequest {
        qty: Some(rust_decimal::Decimal::new(2, 0)),
        time_in_force: Some(TimeInForce::Day),
        limit_price: Some(rust_decimal::Decimal::new(50000, 2)),
        stop_price: None,
        trail: Some(rust_decimal::Decimal::new(125, 2)),
        client_order_id: Some("phase7-orders-replace-transport-1".to_owned()),
    }
}

#[tokio::test]
async fn orders_list_hits_official_path_query_and_sends_auth_headers() {
    let body = list_response_json();
    let server = TestServer::spawn(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    )]);

    let orders = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .orders()
        .list(list_request())
        .await
        .expect("orders list request should succeed");

    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].symbol, "SPY");

    let request = server.into_single_request();
    assert_eq!(request.request_line, ORDERS_LIST_REQUEST_LINE);
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
async fn orders_get_hits_official_path_and_sends_auth_headers() {
    let body = order_json();
    let server = TestServer::spawn(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    )]);

    let order = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .orders()
        .get("order-id-123")
        .await
        .expect("orders get request should succeed");

    assert_eq!(order.symbol, "SPY");

    let request = server.into_single_request();
    assert_eq!(request.request_line, "GET /v2/orders/order-id-123 HTTP/1.1");
    assert!(request.body.is_empty());
}

#[tokio::test]
async fn orders_get_by_client_order_id_hits_alias_endpoint_shape() {
    let body = order_json();
    let server = TestServer::spawn(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    )]);

    let order = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .orders()
        .get_by_client_order_id("phase7-orders-getby-1")
        .await
        .expect("orders alias request should succeed");

    assert_eq!(order.client_order_id, "phase7-orders-transport-1");

    let request = server.into_single_request();
    assert_eq!(
        request.request_line,
        "GET /v2/orders:by_client_order_id?client_order_id=phase7-orders-getby-1 HTTP/1.1"
    );
    assert!(request.body.is_empty());
}

#[tokio::test]
async fn orders_list_maps_429_to_rate_limited() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 429 Too Many Requests\r\nx-request-id: req-orders-list-429-1\r\nretry-after: 0\r\ncontent-length: 9\r\nconnection: close\r\n\r\nslow down".to_owned(),
        "HTTP/1.1 429 Too Many Requests\r\nx-request-id: req-orders-list-429-2\r\nretry-after: 17\r\ncontent-length: 9\r\nconnection: close\r\n\r\nslow down".to_owned(),
    ]);

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .orders()
        .list(list_request())
        .await
        .expect_err("429 response must fail");

    match error {
        Error::RateLimited(meta) => {
            assert_eq!(meta.endpoint, "orders.list");
            assert_eq!(meta.method, "GET");
            assert_eq!(meta.status, Some(429));
            assert_eq!(meta.request_id.as_deref(), Some("req-orders-list-429-2"));
            assert_eq!(meta.retry_after.as_deref(), Some("17"));
            assert_eq!(meta.body.as_deref(), Some("slow down"));
        }
        other => panic!("expected rate limited error, got {other:?}"),
    }

    let requests = server.into_requests();
    assert_eq!(requests.len(), 2, "GET retries should issue two requests");
}

#[tokio::test]
async fn orders_get_maps_503_to_http_status() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 503 Service Unavailable\r\nx-request-id: req-orders-get-503-1\r\ncontent-length: 15\r\nconnection: close\r\n\r\nservice offline".to_owned(),
        "HTTP/1.1 503 Service Unavailable\r\nx-request-id: req-orders-get-503-2\r\ncontent-length: 15\r\nconnection: close\r\n\r\nservice offline".to_owned(),
    ]);

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .orders()
        .get("order-id-123")
        .await
        .expect_err("503 response must fail");

    match error {
        Error::HttpStatus(meta) => {
            assert_eq!(meta.endpoint, "orders.get");
            assert_eq!(meta.method, "GET");
            assert_eq!(meta.status, Some(503));
            assert_eq!(meta.request_id.as_deref(), Some("req-orders-get-503-2"));
            assert_eq!(meta.body.as_deref(), Some("service offline"));
        }
        other => panic!("expected http status error, got {other:?}"),
    }

    let requests = server.into_requests();
    assert_eq!(requests.len(), 2, "GET retries should issue two requests");
}

#[tokio::test]
async fn orders_list_maps_malformed_json_to_deserialize() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nx-request-id: req-orders-json-1\r\ncontent-length: 15\r\nconnection: close\r\n\r\n{not valid json".to_owned(),
    ]);

    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .orders()
        .list(list_request())
        .await
        .expect_err("invalid json must fail");

    match error {
        Error::Deserialize { message, meta } => {
            assert!(!message.is_empty());
            assert_eq!(meta.endpoint, "orders.list");
            assert_eq!(meta.method, "GET");
            assert_eq!(meta.status, Some(200));
            assert_eq!(meta.request_id.as_deref(), Some("req-orders-json-1"));
            assert_eq!(meta.body.as_deref(), Some("{not valid json"));
        }
        other => panic!("expected deserialize error, got {other:?}"),
    }
}

#[tokio::test]
async fn orders_identifiers_fail_before_transport() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}".to_owned(),
    ]);

    for order_id in ["order/id", " order-id "] {
        let error = Client::builder()
            .api_key("key")
            .secret_key("secret")
            .base_url(server.base_url())
            .build()
            .expect("client should build")
            .orders()
            .get(order_id)
            .await
            .expect_err("invalid order identifiers must fail before send");

        match error {
            Error::InvalidRequest(message) => assert!(message.contains("order_id")),
            other => panic!("expected invalid request error, got {other:?}"),
        }
    }

    for client_order_id in ["client/order-id", " client-order-id "] {
        let error = Client::builder()
            .api_key("key")
            .secret_key("secret")
            .base_url(server.base_url())
            .build()
            .expect("client should build")
            .orders()
            .get_by_client_order_id(client_order_id)
            .await
            .expect_err("invalid client_order_id must fail before send");

        match error {
            Error::InvalidRequest(message) => assert!(message.contains("client_order_id")),
            other => panic!("expected invalid request error, got {other:?}"),
        }
    }

    let requests = server.into_requests();
    assert!(
        requests.is_empty(),
        "invalid identifiers should not send any request"
    );
}

#[tokio::test]
async fn orders_create_posts_official_body_shape_once() {
    let body = order_json();
    let server = TestServer::spawn(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    )]);

    let order = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .orders()
        .create(create_request())
        .await
        .expect("orders create request should succeed");

    assert_eq!(order.symbol, "SPY");

    let request = server.into_single_request();
    assert_eq!(request.request_line, "POST /v2/orders HTTP/1.1");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&request.body)
            .expect("request body must be json"),
        json!({
            "symbol": "SPY",
            "qty": "1",
            "side": "buy",
            "type": "limit",
            "time_in_force": "day",
            "limit_price": "499.25",
            "extended_hours": false,
            "client_order_id": "phase7-orders-create-transport-1",
            "order_class": "bracket",
            "take_profit": { "limit_price": "510.00" },
            "stop_loss": { "stop_price": "492.00", "limit_price": "491.50" },
            "position_intent": "buy_to_open"
        })
    );
}

#[tokio::test]
async fn orders_create_posts_official_mleg_body_shape_once() {
    let body = order_json();
    let server = TestServer::spawn(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    )]);

    let order = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .orders()
        .create(CreateRequest {
            qty: Some(rust_decimal::Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(rust_decimal::Decimal::new(-25, 2)),
            client_order_id: Some("phase7-orders-create-mleg-transport-1".to_owned()),
            order_class: Some(OrderClass::Mleg),
            legs: Some(vec![
                OptionLegRequest {
                    symbol: "SPY260417P00570000".to_owned(),
                    ratio_qty: 2,
                    side: Some(OrderSide::Sell),
                    position_intent: Some(PositionIntent::SellToOpen),
                },
                OptionLegRequest {
                    symbol: "SPY260417P00565000".to_owned(),
                    ratio_qty: 1,
                    side: Some(OrderSide::Buy),
                    position_intent: Some(PositionIntent::BuyToOpen),
                },
            ]),
            ..CreateRequest::default()
        })
        .await
        .expect("orders mleg create request should succeed");

    assert_eq!(order.symbol, "SPY");

    let request = server.into_single_request();
    assert_eq!(request.request_line, "POST /v2/orders HTTP/1.1");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&request.body)
            .expect("request body must be json"),
        json!({
            "qty": "1",
            "side": "buy",
            "type": "limit",
            "time_in_force": "day",
            "limit_price": "-0.25",
            "client_order_id": "phase7-orders-create-mleg-transport-1",
            "order_class": "mleg",
            "legs": [{
                "symbol": "SPY260417P00570000",
                "ratio_qty": "2",
                "side": "sell",
                "position_intent": "sell_to_open"
            }, {
                "symbol": "SPY260417P00565000",
                "ratio_qty": "1",
                "side": "buy",
                "position_intent": "buy_to_open"
            }]
        })
    );
}

#[tokio::test]
async fn orders_replace_patches_official_body_shape_once() {
    let body = order_json();
    let server = TestServer::spawn(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    )]);

    let order = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .orders()
        .replace("order-id-123", replace_request())
        .await
        .expect("orders replace request should succeed");

    assert_eq!(order.symbol, "SPY");

    let request = server.into_single_request();
    assert_eq!(
        request.request_line,
        "PATCH /v2/orders/order-id-123 HTTP/1.1"
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&request.body)
            .expect("request body must be json"),
        json!({
            "qty": "2",
            "time_in_force": "day",
            "limit_price": "500.00",
            "trail": "1.25",
            "client_order_id": "phase7-orders-replace-transport-1"
        })
    );
}

#[tokio::test]
async fn orders_cancel_accepts_204_and_sends_no_body() {
    let server = TestServer::spawn(vec![
        "HTTP/1.1 204 No Content\r\nconnection: close\r\n\r\n".to_owned(),
    ]);

    Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .orders()
        .cancel("order-id-123")
        .await
        .expect("orders cancel request should succeed");

    let request = server.into_single_request();
    assert_eq!(
        request.request_line,
        "DELETE /v2/orders/order-id-123 HTTP/1.1"
    );
    assert!(request.body.is_empty());
}

#[tokio::test]
async fn orders_cancel_all_deserializes_batch_body_once() {
    let body = format!(
        r#"[{{"id":"904837e3-3b76-47ec-b432-046db621571b","status":200,"body":{}}}]"#,
        order_json()
    );
    let server = TestServer::spawn(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    )]);

    let results = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .orders()
        .cancel_all()
        .await
        .expect("orders cancel_all request should succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, 200);

    let request = server.into_single_request();
    assert_eq!(request.request_line, "DELETE /v2/orders HTTP/1.1");
    assert!(request.body.is_empty());
}

#[tokio::test]
async fn orders_write_methods_do_not_retry_non_get_requests() {
    let create_server = TestServer::spawn(vec![
        "HTTP/1.1 429 Too Many Requests\r\nx-request-id: req-orders-create-429-1\r\nretry-after: 17\r\ncontent-length: 9\r\nconnection: close\r\n\r\nslow down".to_owned(),
    ]);
    let replace_server = TestServer::spawn(vec![
        "HTTP/1.1 503 Service Unavailable\r\nx-request-id: req-orders-replace-503-1\r\ncontent-length: 15\r\nconnection: close\r\n\r\nservice offline".to_owned(),
    ]);
    let cancel_all_server = TestServer::spawn(vec![
        "HTTP/1.1 503 Service Unavailable\r\nx-request-id: req-orders-cancel-all-503-1\r\ncontent-length: 15\r\nconnection: close\r\n\r\nservice offline".to_owned(),
    ]);

    let create_error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(create_server.base_url())
        .build()
        .expect("client should build")
        .orders()
        .create(create_request())
        .await
        .expect_err("create 429 must fail");
    let replace_error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(replace_server.base_url())
        .build()
        .expect("client should build")
        .orders()
        .replace("order-id-123", replace_request())
        .await
        .expect_err("replace 503 must fail");
    let cancel_all_error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(cancel_all_server.base_url())
        .build()
        .expect("client should build")
        .orders()
        .cancel_all()
        .await
        .expect_err("cancel_all 503 must fail");

    assert!(matches!(create_error, Error::RateLimited(_)));
    assert!(matches!(replace_error, Error::HttpStatus(_)));
    assert!(matches!(cancel_all_error, Error::HttpStatus(_)));

    assert_eq!(create_server.into_requests().len(), 1);
    assert_eq!(replace_server.into_requests().len(), 1);
    assert_eq!(cancel_all_server.into_requests().len(), 1);
}
