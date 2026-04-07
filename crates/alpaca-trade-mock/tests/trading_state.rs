use alpaca_trade::Decimal;

#[test]
fn virtual_account_is_created_on_first_access_with_default_cash() {
    let state = alpaca_trade_mock::state::MockTradingState::new();

    let account = state.ensure_account("mock-key-a");

    assert_eq!(account.account_profile.id, "mock-key-a");
    assert_eq!(
        account.cash_ledger.cash_balance(),
        Decimal::new(1_000_000, 0)
    );
}

#[test]
fn different_api_keys_resolve_to_different_virtual_accounts() {
    let state = alpaca_trade_mock::state::MockTradingState::new();

    let first = state.ensure_account("mock-key-a");
    let second = state.ensure_account("mock-key-b");

    assert_ne!(first.account_profile.id, second.account_profile.id);
}
