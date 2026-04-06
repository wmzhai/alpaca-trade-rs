mod support;

use alpaca_trade::Client;
use support::Credentials;
use support::orders::{
    OrdersRuntimeMode, OrdersTestContext, data_client_from_trade_credentials,
    discover_option_contract, next_client_order_id, select_runtime_mode,
    should_run_real_cancel_all, stock_price_context, stock_test_symbol,
};

#[test]
fn test_order_client_order_id_prefixes_are_unique_and_traceable() {
    let first = next_client_order_id("stock limit", "lookup by client order id");
    let second = next_client_order_id("stock limit", "lookup by client order id");

    assert!(first.starts_with("phase7-orders-stock-limit-lookup-by-client-order-id-"));
    assert!(second.starts_with("phase7-orders-stock-limit-lookup-by-client-order-id-"));
    assert_ne!(first, second);
}

#[test]
fn test_paper_mode_requires_market_open_and_credentials() {
    assert_eq!(
        select_runtime_mode(true, true, true, true),
        OrdersRuntimeMode::Paper
    );

    for inputs in [
        (false, true, true, true),
        (true, false, true, true),
        (true, true, false, true),
        (true, true, true, false),
    ] {
        assert_eq!(
            select_runtime_mode(inputs.0, inputs.1, inputs.2, inputs.3),
            OrdersRuntimeMode::Mock
        );
    }
}

#[test]
fn test_cancel_all_real_mode_requires_dedicated_test_account_marker() {
    assert!(should_run_real_cancel_all(OrdersRuntimeMode::Paper, true));
    assert!(!should_run_real_cancel_all(OrdersRuntimeMode::Paper, false));
    assert!(!should_run_real_cancel_all(OrdersRuntimeMode::Mock, true));
}

#[test]
fn test_stock_symbol_selection_defaults_to_spy() {
    assert_eq!(stock_test_symbol(), "SPY");
}

#[test]
fn test_market_data_builder_uses_same_api_key_names_as_trade_support() {
    assert_eq!(
        support::trade_api_key_candidates(),
        &["ALPACA_TRADE_API_KEY", "APCA_API_KEY_ID"]
    );
    assert_eq!(
        support::trade_secret_key_candidates(),
        &["ALPACA_TRADE_SECRET_KEY", "APCA_API_SECRET_KEY"]
    );

    let client = data_client_from_trade_credentials(&Credentials {
        api_key: "api-key".to_owned(),
        secret_key: "secret-key".to_owned(),
    });

    let _ = client.stocks();
    let _ = client.options();
}

#[tokio::test]
async fn mock_stock_price_context_requires_live_market_data_client() {
    let context = OrdersTestContext {
        runtime_mode: OrdersRuntimeMode::Mock,
        trade_client: Client::builder()
            .api_key("mock-api-key")
            .secret_key("mock-secret-key")
            .base_url("http://127.0.0.1:1")
            .build()
            .expect("mock client should build"),
        data_client: None,
        market_snapshot: Some(
            alpaca_trade_mock::OrdersMarketSnapshot::default().with_instrument(
                "SPY",
                alpaca_trade_mock::InstrumentSnapshot::equity(
                    alpaca_trade::Decimal::new(50000, 2),
                    alpaca_trade::Decimal::new(50020, 2),
                ),
            ),
        ),
        mock_server: None,
    };

    let error = stock_price_context(&context, "SPY")
        .await
        .expect_err("mock stock price context must fail without live market data");
    assert!(error.contains("mock runtime requires alpaca-data"));
}

#[tokio::test]
async fn mock_option_discovery_requires_live_market_data_client() {
    let context = OrdersTestContext {
        runtime_mode: OrdersRuntimeMode::Mock,
        trade_client: Client::builder()
            .api_key("mock-api-key")
            .secret_key("mock-secret-key")
            .base_url("http://127.0.0.1:1")
            .build()
            .expect("mock client should build"),
        data_client: None,
        market_snapshot: Some(
            alpaca_trade_mock::OrdersMarketSnapshot::default().with_instrument(
                "SPY260417C00550000",
                alpaca_trade_mock::InstrumentSnapshot::option(
                    alpaca_trade::Decimal::new(120, 2),
                    alpaca_trade::Decimal::new(130, 2),
                ),
            ),
        ),
        mock_server: None,
    };

    let error = discover_option_contract(&context, "SPY")
        .await
        .expect_err("mock option discovery must fail without live market data");
    assert!(error.contains("mock runtime requires alpaca-data"));
}
