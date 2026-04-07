#[path = "../../alpaca-trade/tests/support/mod.rs"]
mod trade_support;

use alpaca_trade::Decimal;
use alpaca_trade::orders::{CreateRequest, OrderClass, OrderSide, OrderType, TimeInForce};
use reqwest::StatusCode;
use serde_json::Value;
use trade_support::orders::{
    discover_mleg_call_spread, discover_option_contract, orders_test_context, orders_test_lock,
};

fn mock_client(base_url: String, api_key: &str, secret_key: &str) -> alpaca_trade::Client {
    alpaca_trade::Client::builder()
        .api_key(api_key)
        .secret_key(secret_key)
        .base_url(base_url)
        .build()
        .expect("mock client should build")
}

async fn list_positions(base_url: &str, api_key: &str, secret_key: &str) -> Vec<Value> {
    let response = reqwest::Client::new()
        .get(format!("{base_url}/v2/positions"))
        .header("apca-api-key-id", api_key)
        .header("apca-api-secret-key", secret_key)
        .send()
        .await
        .expect("positions list request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    response
        .json()
        .await
        .expect("positions list should deserialize")
}

async fn get_position(
    base_url: &str,
    api_key: &str,
    secret_key: &str,
    symbol_or_asset_id: &str,
) -> Value {
    let response = reqwest::Client::new()
        .get(format!("{base_url}/v2/positions/{symbol_or_asset_id}"))
        .header("apca-api-key-id", api_key)
        .header("apca-api-secret-key", secret_key)
        .send()
        .await
        .expect("position get request should succeed");
    assert_eq!(response.status(), StatusCode::OK);
    response
        .json()
        .await
        .expect("position response should deserialize")
}

async fn delete_positions(
    base_url: &str,
    api_key: &str,
    secret_key: &str,
    symbol_or_asset_id: Option<&str>,
) -> reqwest::Response {
    let url = match symbol_or_asset_id {
        Some(symbol_or_asset_id) => format!("{base_url}/v2/positions/{symbol_or_asset_id}"),
        None => format!("{base_url}/v2/positions"),
    };
    reqwest::Client::new()
        .delete(url)
        .header("apca-api-key-id", api_key)
        .header("apca-api-secret-key", secret_key)
        .send()
        .await
        .expect("positions delete request should succeed")
}

async fn post_position_action(
    base_url: &str,
    api_key: &str,
    secret_key: &str,
    symbol_or_contract_id: &str,
    action: &str,
) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!(
            "{base_url}/v2/positions/{symbol_or_contract_id}/{action}"
        ))
        .header("apca-api-key-id", api_key)
        .header("apca-api-secret-key", secret_key)
        .send()
        .await
        .expect("positions action request should succeed")
}

fn find_position<'a>(positions: &'a [Value], symbol: &str) -> &'a Value {
    positions
        .iter()
        .find(|position| position["symbol"] == symbol)
        .unwrap_or_else(|| panic!("expected to find position for {symbol}"))
}

#[tokio::test]
async fn filled_stock_order_creates_readable_position() {
    let _guard = orders_test_lock().await;
    let _credentials = trade_support::trade_credentials().expect(
        "mock positions route tests require Alpaca credentials because positions valuation uses live market data",
    );
    let server = alpaca_trade_mock::spawn_test_server().await;
    let client = mock_client(
        server.base_url.clone(),
        "mock-positions-stock-key",
        "mock-positions-stock-secret",
    );

    let created = client
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some("mock-positions-stock-fill".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock market buy should fill");

    let positions = list_positions(
        &server.base_url,
        "mock-positions-stock-key",
        "mock-positions-stock-secret",
    )
    .await;
    let position = find_position(&positions, "SPY");

    assert_eq!(position["asset_class"], "us_equity");
    assert_eq!(position["side"], "long");
    assert_eq!(position["qty"], "1");
    assert_eq!(position["qty_available"], "1");
    assert_eq!(
        position["avg_entry_price"],
        created
            .filled_avg_price
            .expect("filled order should carry avg entry price")
            .to_string()
    );
}

#[tokio::test]
async fn filled_mleg_order_projects_leg_positions_not_parent_combo_position() {
    let _guard = orders_test_lock().await;
    let _credentials = trade_support::trade_credentials().expect(
        "mock positions route tests require Alpaca credentials because positions valuation uses live market data",
    );
    let context = orders_test_context().await;
    let spread = discover_mleg_call_spread(&context, "SPY")
        .await
        .expect("dynamic call spread should be discoverable");
    let server = alpaca_trade_mock::spawn_test_server().await;
    let client = mock_client(
        server.base_url.clone(),
        "mock-positions-mleg-key",
        "mock-positions-mleg-secret",
    );

    client
        .orders()
        .create(CreateRequest {
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(spread.marketable_limit_price),
            client_order_id: Some("mock-positions-mleg-fill".to_owned()),
            order_class: Some(OrderClass::Mleg),
            legs: Some(spread.legs.clone()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock marketable spread should fill");

    let positions = list_positions(
        &server.base_url,
        "mock-positions-mleg-key",
        "mock-positions-mleg-secret",
    )
    .await;

    assert_eq!(positions.len(), spread.legs.len());
    assert!(positions.iter().all(|position| position["symbol"] != ""));
    for leg in &spread.legs {
        let position = find_position(&positions, &leg.symbol);
        assert_eq!(position["asset_class"], "us_option");
        assert_eq!(position["qty"], "1");
        assert_eq!(
            position["side"],
            match leg.side {
                Some(OrderSide::Buy) => "long",
                Some(OrderSide::Sell) => "short",
                _ => panic!("spread leg should have a side"),
            }
        );
    }
}

#[tokio::test]
async fn position_lookup_is_account_local() {
    let _guard = orders_test_lock().await;
    let _credentials = trade_support::trade_credentials().expect(
        "mock positions route tests require Alpaca credentials because positions valuation uses live market data",
    );
    let server = alpaca_trade_mock::spawn_test_server().await;
    let account_a = mock_client(
        server.base_url.clone(),
        "mock-positions-lookup-a",
        "mock-positions-lookup-secret-a",
    );
    let account_b = mock_client(
        server.base_url.clone(),
        "mock-positions-lookup-b",
        "mock-positions-lookup-secret-b",
    );

    account_a
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some("mock-positions-lookup-a".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("account A market buy should fill");
    account_b
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(2, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some("mock-positions-lookup-b".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("account B market buy should fill");

    let position_a = get_position(
        &server.base_url,
        "mock-positions-lookup-a",
        "mock-positions-lookup-secret-a",
        "SPY",
    )
    .await;
    let position_b = get_position(
        &server.base_url,
        "mock-positions-lookup-b",
        "mock-positions-lookup-secret-b",
        "SPY",
    )
    .await;

    assert_eq!(position_a["qty"], "1");
    assert_eq!(position_b["qty"], "2");
    assert_eq!(position_a["asset_id"], position_b["asset_id"]);
}

#[tokio::test]
async fn close_position_by_symbol_creates_offsetting_execution_and_removes_position() {
    let _guard = orders_test_lock().await;
    let _credentials = trade_support::trade_credentials().expect(
        "mock positions route tests require Alpaca credentials because positions valuation uses live market data",
    );
    let server = alpaca_trade_mock::spawn_test_server().await;
    let client = mock_client(
        server.base_url.clone(),
        "mock-positions-close-key",
        "mock-positions-close-secret",
    );

    client
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some("mock-positions-close-source".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock market buy should fill");
    let cash_before_close = client
        .account()
        .get()
        .await
        .expect("account should be readable before closing");

    let response = delete_positions(
        &server.base_url,
        "mock-positions-close-key",
        "mock-positions-close-secret",
        Some("SPY"),
    )
    .await;
    assert!(response.status().is_success());

    let cash_after_close = client
        .account()
        .get()
        .await
        .expect("account should be readable after closing");
    assert!(
        cash_after_close.cash.expect("cash should be present")
            > cash_before_close.cash.expect("cash should be present")
    );

    let missing = reqwest::Client::new()
        .get(format!("{}/v2/positions/SPY", server.base_url))
        .header("apca-api-key-id", "mock-positions-close-key")
        .header("apca-api-secret-key", "mock-positions-close-secret")
        .send()
        .await
        .expect("position get after close should return a response");
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn close_all_positions_only_affects_the_current_account() {
    let _guard = orders_test_lock().await;
    let _credentials = trade_support::trade_credentials().expect(
        "mock positions route tests require Alpaca credentials because positions valuation uses live market data",
    );
    let server = alpaca_trade_mock::spawn_test_server().await;
    let account_a = mock_client(
        server.base_url.clone(),
        "mock-positions-close-all-a",
        "mock-positions-close-all-secret-a",
    );
    let account_b = mock_client(
        server.base_url.clone(),
        "mock-positions-close-all-b",
        "mock-positions-close-all-secret-b",
    );

    for (client, id) in [
        (&account_a, "mock-positions-close-all-order-a"),
        (&account_b, "mock-positions-close-all-order-b"),
    ] {
        client
            .orders()
            .create(CreateRequest {
                symbol: Some("SPY".to_owned()),
                qty: Some(Decimal::new(1, 0)),
                side: Some(OrderSide::Buy),
                r#type: Some(OrderType::Market),
                time_in_force: Some(TimeInForce::Day),
                client_order_id: Some(id.to_owned()),
                ..CreateRequest::default()
            })
            .await
            .expect("mock market buy should fill");
    }

    let response = delete_positions(
        &server.base_url,
        "mock-positions-close-all-a",
        "mock-positions-close-all-secret-a",
        None,
    )
    .await;
    assert!(response.status().is_success());

    let positions_a = list_positions(
        &server.base_url,
        "mock-positions-close-all-a",
        "mock-positions-close-all-secret-a",
    )
    .await;
    let positions_b = list_positions(
        &server.base_url,
        "mock-positions-close-all-b",
        "mock-positions-close-all-secret-b",
    )
    .await;
    assert!(positions_a.is_empty());
    assert_eq!(positions_b.len(), 1);
    assert_eq!(positions_b[0]["symbol"], "SPY");
}

#[tokio::test]
async fn exercise_route_requires_existing_option_position() {
    let _guard = orders_test_lock().await;
    let _credentials = trade_support::trade_credentials().expect(
        "mock positions route tests require Alpaca credentials because positions valuation uses live market data",
    );
    let server = alpaca_trade_mock::spawn_test_server().await;

    let response = post_position_action(
        &server.base_url,
        "mock-positions-exercise-key",
        "mock-positions-exercise-secret",
        "SPY260417C00550000",
        "exercise",
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn do_not_exercise_route_records_override_without_fake_market_data() {
    let _guard = orders_test_lock().await;
    let _credentials = trade_support::trade_credentials().expect(
        "mock positions route tests require Alpaca credentials because positions valuation uses live market data",
    );
    let context = orders_test_context().await;
    let contract = discover_option_contract(&context, "SPY")
        .await
        .expect("dynamic option contract should be discoverable");
    let server = alpaca_trade_mock::spawn_test_server().await;
    let client = mock_client(
        server.base_url.clone(),
        "mock-positions-dne-key",
        "mock-positions-dne-secret",
    );

    client
        .orders()
        .create(CreateRequest {
            symbol: Some(contract.symbol.clone()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some("mock-positions-dne-order".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock option market buy should fill");

    let response = post_position_action(
        &server.base_url,
        "mock-positions-dne-key",
        "mock-positions-dne-secret",
        &contract.symbol,
        "do-not-exercise",
    )
    .await;
    assert!(response.status().is_success());

    let position = get_position(
        &server.base_url,
        "mock-positions-dne-key",
        "mock-positions-dne-secret",
        &contract.symbol,
    )
    .await;
    assert_eq!(position["symbol"], contract.symbol);
    assert_eq!(position["qty"], "1");
}
