use alpaca_trade::account::Account;

#[test]
fn account_model_deserializes_official_shape() {
    let json = r#"
    {
      "account_blocked": false,
      "account_number": "010203ABCD",
      "buying_power": "262113.632",
      "cash": "100000.00",
      "created_at": "2019-06-12T22:47:07.99658Z",
      "currency": "USD",
      "daytrade_count": 0,
      "balance_asof": "2023-09-27",
      "daytrading_buying_power": "262113.632",
      "equity": "103820.56",
      "id": "e6fe16f3-64a4-4921-8928-cadf02f92f98",
      "initial_margin": "63480.38",
      "last_equity": "103529.24",
      "last_maintenance_margin": "38000.832",
      "long_market_value": "126960.76",
      "maintenance_margin": "38088.228",
      "multiplier": "4",
      "pattern_day_trader": false,
      "portfolio_value": "103820.56",
      "regt_buying_power": "80680.36",
      "options_buying_power": "40340.18",
      "short_market_value": "0",
      "shorting_enabled": true,
      "sma": "0",
      "status": "ACTIVE",
      "trade_suspended_by_user": false,
      "trading_blocked": false,
      "transfers_blocked": false,
      "options_approved_level": 2,
      "options_trading_level": 1,
      "intraday_adjustments": "0",
      "pending_reg_taf_fees": "0"
    }
    "#;

    let account: Account = serde_json::from_str(json).expect("json should deserialize");
    assert_eq!(account.id, "e6fe16f3-64a4-4921-8928-cadf02f92f98");
    assert_eq!(account.status, "ACTIVE");
    assert_eq!(account.currency.as_deref(), Some("USD"));
    assert_eq!(account.options_approved_level, Some(2));
}

#[test]
fn account_model_rejects_missing_required_id() {
    let json = r#"
    {
      "account_number": "010203ABCD",
      "status": "ACTIVE"
    }
    "#;

    let error = serde_json::from_str::<Account>(json).expect_err("missing id must fail");
    assert!(error.to_string().contains("missing field `id`"));
}
