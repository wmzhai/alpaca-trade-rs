#[path = "../../alpaca-trade/tests/support/mod.rs"]
mod trade_support;

use alpaca_trade::Decimal;
use alpaca_trade::orders::{OrderSide, OrderStatus, OrderType, TimeInForce};
use alpaca_trade_mock::state::{CreateOrderInput, MockTradingState, OrdersState, ReplaceOrderInput};
use trade_support::orders::{orders_test_context, orders_test_lock, stock_price_context};

#[test]
fn mock_trading_state_starts_without_materialized_accounts() {
    let state = MockTradingState::new();

    assert_eq!(state.account_count(), 0);
}

#[test]
fn virtual_account_is_created_on_first_access_with_default_cash() {
    let state = MockTradingState::new();

    let account = state.ensure_account("mock-key-a");

    assert_eq!(account.account_profile().id, "mock-key-a");
    assert_eq!(
        account.cash_ledger().cash_balance(),
        Decimal::new(1_000_000, 0)
    );
}

#[test]
fn different_api_keys_resolve_to_different_virtual_accounts() {
    let state = MockTradingState::new();

    let first = state.ensure_account("mock-key-a");
    let second = state.ensure_account("mock-key-b");

    assert_ne!(first.account_profile().id, second.account_profile().id);
}

#[tokio::test]
async fn non_marketable_limit_order_is_recorded_as_new_without_cash_change() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;
    let stock = stock_price_context(&context, "SPY")
        .await
        .expect("live stock quote should be available for state tests");
    let state = OrdersState::new(
        MockTradingState::new(),
        "mock-state-account-a",
        alpaca_trade_mock::OrdersMarketSnapshot::default(),
    );

    let cash_before = state.account_snapshot().cash_ledger().cash_balance();

    let created = state
        .create_order(CreateOrderInput {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            order_type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(stock.non_marketable_buy_limit_price),
            client_order_id: Some("mock-state-resting-new".to_owned()),
            ..CreateOrderInput::default()
        })
        .await
        .expect("resting order should be created");

    let account = state.account_snapshot();
    assert_eq!(created.status, OrderStatus::New);
    assert_eq!(account.cash_ledger().cash_balance(), cash_before);
    assert_eq!(account.execution_count(), 0);
    assert_eq!(account.activity_count(), 1);
}

#[tokio::test]
async fn marketable_order_writes_execution_and_changes_cash() {
    let _guard = orders_test_lock().await;
    let _context = orders_test_context().await;
    let state = OrdersState::new(
        MockTradingState::new(),
        "mock-state-account-b",
        alpaca_trade_mock::OrdersMarketSnapshot::default(),
    );

    let cash_before = state.account_snapshot().cash_ledger().cash_balance();

    let created = state
        .create_order(CreateOrderInput {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            order_type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some("mock-state-filled-market".to_owned()),
            ..CreateOrderInput::default()
        })
        .await
        .expect("market order should be created");

    let account = state.account_snapshot();
    assert_eq!(created.status, OrderStatus::Filled);
    assert_eq!(account.execution_count(), 1);
    assert_eq!(account.activity_count(), 1);
    assert_ne!(account.cash_ledger().cash_balance(), cash_before);
}

#[tokio::test]
async fn create_quote_failure_leaves_account_state_untouched() {
    let _guard = orders_test_lock().await;
    let _context = orders_test_context().await;
    let state = OrdersState::new(
        MockTradingState::new(),
        "mock-state-account-c",
        alpaca_trade_mock::OrdersMarketSnapshot::default(),
    );

    let cash_before = state.account_snapshot().cash_ledger().cash_balance();

    let error = state
        .create_order(CreateOrderInput {
            symbol: Some("MOCKQUOTEFAIL".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            order_type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some("mock-state-quote-failure".to_owned()),
            ..CreateOrderInput::default()
        })
        .await
        .expect_err("unknown symbol should fail quote resolution");

    let account = state.account_snapshot();
    assert!(matches!(
        error,
        alpaca_trade_mock::state::OrdersStateError::MarketDataUnavailable(_)
    ));
    assert_eq!(account.cash_ledger().cash_balance(), cash_before);
    assert_eq!(account.execution_count(), 0);
    assert_eq!(account.activity_count(), 0);
    assert!(state.list_orders(Default::default()).is_empty());
    assert!(state.get_by_client_order_id("mock-state-quote-failure").is_none());
}

#[tokio::test]
async fn non_filled_replace_emits_activity_without_changing_cash() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;
    let stock = stock_price_context(&context, "SPY")
        .await
        .expect("live stock quote should be available for state tests");
    let state = OrdersState::new(
        MockTradingState::new(),
        "mock-state-account-d",
        alpaca_trade_mock::OrdersMarketSnapshot::default(),
    );

    let created = state
        .create_order(CreateOrderInput {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            order_type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(stock.non_marketable_buy_limit_price),
            client_order_id: Some("mock-state-replace-source".to_owned()),
            ..CreateOrderInput::default()
        })
        .await
        .expect("resting order should be created");
    let cash_before = state.account_snapshot().cash_ledger().cash_balance();

    let replaced = state
        .replace_order(
            &created.id,
            ReplaceOrderInput {
                limit_price: Some(stock.more_conservative_buy_limit_price),
                ..ReplaceOrderInput::default()
            },
        )
        .await
        .expect("resting replacement should succeed");

    let account = state.account_snapshot();
    let original = state
        .get_order(&created.id)
        .expect("original order should remain queryable");
    assert_eq!(replaced.status, OrderStatus::New);
    assert_eq!(original.status, OrderStatus::Replaced);
    assert_eq!(account.cash_ledger().cash_balance(), cash_before);
    assert_eq!(account.execution_count(), 0);
    assert_eq!(account.activity_count(), 2);
}

#[tokio::test]
async fn cancel_emits_activity_without_changing_cash() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;
    let stock = stock_price_context(&context, "SPY")
        .await
        .expect("live stock quote should be available for state tests");
    let state = OrdersState::new(
        MockTradingState::new(),
        "mock-state-account-e",
        alpaca_trade_mock::OrdersMarketSnapshot::default(),
    );

    let created = state
        .create_order(CreateOrderInput {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            order_type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(stock.non_marketable_buy_limit_price),
            client_order_id: Some("mock-state-cancel-source".to_owned()),
            ..CreateOrderInput::default()
        })
        .await
        .expect("resting order should be created");
    let cash_before = state.account_snapshot().cash_ledger().cash_balance();

    state
        .cancel_order(&created.id)
        .expect("cancel should succeed for resting order");

    let account = state.account_snapshot();
    let canceled = state
        .get_order(&created.id)
        .expect("canceled order should remain queryable");
    assert_eq!(canceled.status, OrderStatus::Canceled);
    assert_eq!(account.cash_ledger().cash_balance(), cash_before);
    assert_eq!(account.execution_count(), 0);
    assert_eq!(account.activity_count(), 2);
}
