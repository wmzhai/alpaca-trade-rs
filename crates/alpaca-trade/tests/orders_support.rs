mod support;

use support::Credentials;
use support::orders::{
    OrdersRuntimeMode, data_client_from_trade_credentials, next_client_order_id,
    select_runtime_mode, should_run_real_cancel_all, stock_test_symbol,
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
