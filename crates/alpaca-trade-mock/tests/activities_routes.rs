#[path = "../../alpaca-trade/tests/support/mod.rs"]
mod trade_support;

use alpaca_trade::Decimal;
use alpaca_trade::orders::{CreateRequest, OrderSide, OrderType, ReplaceRequest, TimeInForce};
use serde_json::Value;
use trade_support::orders::{orders_test_context, orders_test_lock, stock_price_context};

fn mock_client(base_url: String, api_key: &str) -> alpaca_trade::Client {
    alpaca_trade::Client::builder()
        .api_key(api_key)
        .secret_key("mock-secret-key")
        .base_url(base_url)
        .build()
        .expect("mock client should build")
}

async fn list_activities(base_url: &str, api_key: &str, path: &str) -> Vec<Value> {
    let response = reqwest::Client::new()
        .get(format!("{base_url}{path}"))
        .header("apca-api-key-id", api_key)
        .header("apca-api-secret-key", "mock-secret-key")
        .send()
        .await
        .expect("activities request should succeed");
    assert!(response.status().is_success());
    response
        .json()
        .await
        .expect("activities response should deserialize")
}

#[tokio::test]
async fn activities_list_contains_create_fill_replace_and_cancel_events() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;
    let stock = stock_price_context(&context, "SPY")
        .await
        .expect("live stock quote should be available for mock activity tests");
    let server = alpaca_trade_mock::spawn_test_server().await;
    let client = mock_client(server.base_url.clone(), "mock-activities-key");

    let resting = client
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(stock.non_marketable_buy_limit_price),
            client_order_id: Some("mock-activities-resting".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("resting limit order should be created");
    let replacement = client
        .orders()
        .replace(
            &resting.id,
            ReplaceRequest {
                limit_price: Some(stock.more_conservative_buy_limit_price),
                ..ReplaceRequest::default()
            },
        )
        .await
        .expect("replacement should succeed");
    client
        .orders()
        .cancel(&replacement.id)
        .await
        .expect("replacement should cancel");
    client
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some("mock-activities-fill".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("market order should fill");

    let activities = list_activities(
        &server.base_url,
        "mock-activities-key",
        "/v2/account/activities",
    )
    .await;
    let activity_types = activities
        .iter()
        .map(|activity| activity["activity_type"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();

    assert_eq!(activity_types, vec!["FILL", "CANCELED", "REPLACED", "NEW"]);
    assert!(
        activities
            .iter()
            .all(|activity| activity["client_order_id"] != Value::Null)
    );
}

#[tokio::test]
async fn activities_by_type_filters_current_account_only() {
    let _guard = orders_test_lock().await;
    let _credentials = trade_support::trade_credentials().expect(
        "mock activity route tests require Alpaca credentials because mock fills depend on live market data",
    );
    let server = alpaca_trade_mock::spawn_test_server().await;
    let account_a = mock_client(server.base_url.clone(), "mock-activities-a");
    let account_b = mock_client(server.base_url.clone(), "mock-activities-b");

    account_a
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some("mock-activities-fill-a".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("account A fill should succeed");
    account_b
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some("mock-activities-fill-b".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("account B fill should succeed");

    let fills = list_activities(
        &server.base_url,
        "mock-activities-a",
        "/v2/account/activities/FILL",
    )
    .await;

    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0]["client_order_id"], "mock-activities-fill-a");
    assert_eq!(fills[0]["activity_type"], "FILL");
}
