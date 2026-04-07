#[path = "../../alpaca-trade/tests/support/mod.rs"]
mod trade_support;

use alpaca_trade::Decimal;
use alpaca_trade::account::Account;
use alpaca_trade::orders::{CreateRequest, OrderSide, OrderType, TimeInForce};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use trade_support::orders::orders_test_lock;

#[tokio::test]
async fn account_route_auto_creates_account_with_default_cash() {
    let app = alpaca_trade_mock::build_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v2/account")
                .header("apca-api-key-id", "mock-account-route-key")
                .header("apca-api-secret-key", "mock-account-route-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let account: Account = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("account response body should be readable"),
    )
    .expect("account response should deserialize");
    assert_eq!(account.id, "mock-account-route-key");
    assert_eq!(account.status, "ACTIVE");
    assert_eq!(account.currency.as_deref(), Some("USD"));
    assert_eq!(account.cash, Some(Decimal::new(1_000_000, 0)));
}

#[tokio::test]
async fn filled_buy_order_reduces_mock_account_cash() {
    let _guard = orders_test_lock().await;
    let _credentials = trade_support::trade_credentials().expect(
        "mock account route tests require Alpaca credentials because mock fills depend on live market data",
    );
    let server = alpaca_trade_mock::spawn_test_server().await;
    let client = alpaca_trade::Client::builder()
        .api_key("mock-account-cash-key")
        .secret_key("mock-account-cash-secret")
        .base_url(server.base_url.clone())
        .build()
        .expect("mock client should build");

    let before = client
        .account()
        .get()
        .await
        .expect("mock account should be readable before the fill");
    client
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some("mock-account-route-filled-buy".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock market buy should fill");
    let after = client
        .account()
        .get()
        .await
        .expect("mock account should be readable after the fill");

    assert!(
        after.cash.expect("cash should be present") < before.cash.expect("cash should be present")
    );
}
