use axum::{
    body::Body,
    body::to_bytes,
    http::{Request, StatusCode},
};
use serde_json::json;
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
async fn orders_route_requires_apca_api_key_headers() {
    let app = alpaca_trade_mock::build_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v2/orders")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn orders_are_isolated_per_apca_api_key() {
    let app = alpaca_trade_mock::build_app_with_market_snapshot(
        alpaca_trade_mock::OrdersMarketSnapshot::default().with_instrument(
            "SPY",
            alpaca_trade_mock::InstrumentSnapshot::equity(
                alpaca_trade::Decimal::new(100, 0),
                alpaca_trade::Decimal::new(101, 0),
            ),
        ),
    );

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v2/orders")
                .header("content-type", "application/json")
                .header("apca-api-key-id", "mock-key-a")
                .header("apca-api-secret-key", "mock-secret-a")
                .body(Body::from(
                    json!({
                        "symbol": "SPY",
                        "qty": "1",
                        "side": "buy",
                        "type": "limit",
                        "time_in_force": "day",
                        "limit_price": "99",
                        "client_order_id": "route-account-a-order-1"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("create request should succeed");
    assert_eq!(create_response.status(), StatusCode::OK);

    let account_a_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v2/orders")
                .header("apca-api-key-id", "mock-key-a")
                .header("apca-api-secret-key", "mock-secret-a")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("list request for account A should succeed");
    assert_eq!(account_a_response.status(), StatusCode::OK);
    let account_a_orders: Vec<alpaca_trade::orders::Order> = serde_json::from_slice(
        &to_bytes(account_a_response.into_body(), usize::MAX)
            .await
            .expect("account A response body should be readable"),
    )
    .expect("account A response should deserialize");
    assert_eq!(account_a_orders.len(), 1);

    let account_b_response = app
        .oneshot(
            Request::builder()
                .uri("/v2/orders")
                .header("apca-api-key-id", "mock-key-b")
                .header("apca-api-secret-key", "mock-secret-b")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("list request for account B should succeed");
    assert_eq!(account_b_response.status(), StatusCode::OK);
    let account_b_orders: Vec<alpaca_trade::orders::Order> = serde_json::from_slice(
        &to_bytes(account_b_response.into_body(), usize::MAX)
            .await
            .expect("account B response body should be readable"),
    )
    .expect("account B response should deserialize");
    assert!(account_b_orders.is_empty());
}
