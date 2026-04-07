mod support;

use alpaca_trade::Decimal;
use alpaca_trade::orders::{CreateRequest, OrderSide, OrderType, TimeInForce};
use support::orders::orders_test_lock;

#[tokio::test]
async fn account_client_reads_mock_cash_after_mock_fill() {
    let _guard = orders_test_lock().await;
    let _credentials = support::trade_credentials().expect(
        "mock account client tests require Alpaca credentials because mock fills depend on live market data",
    );
    let server = alpaca_trade_mock::spawn_test_server().await;
    let client = alpaca_trade::Client::builder()
        .api_key("mock-account-key")
        .secret_key("mock-account-secret")
        .base_url(server.base_url.clone())
        .build()
        .expect("client should build");

    let before = client
        .account()
        .get()
        .await
        .expect("account before should read");
    client
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some("mock-account-client-filled-buy".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock market buy should fill");
    let after = client
        .account()
        .get()
        .await
        .expect("account after should read");

    assert!(
        after.cash.expect("cash should be present") < before.cash.expect("cash should be present")
    );
}
