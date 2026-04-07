#[path = "../../alpaca-trade/tests/support/mod.rs"]
mod trade_support;

use alpaca_trade::Decimal;
use alpaca_trade::orders::{
    CreateRequest, OrderClass, OrderSide, OrderStatus, OrderType, PositionIntent, QueryOrderStatus,
    ReplaceRequest, TimeInForce,
};
use trade_support::orders::{
    discover_mleg_call_spread, discover_mleg_iron_condor, discover_mleg_put_spread,
    discover_option_contract, orders_test_context, orders_test_lock, stock_price_context,
};

fn mock_client(base_url: String) -> alpaca_trade::Client {
    mock_client_with_api_key(base_url, "mock-api-key")
}

fn mock_client_with_api_key(base_url: String, api_key: &str) -> alpaca_trade::Client {
    alpaca_trade::Client::builder()
        .api_key(api_key)
        .secret_key("mock-secret-key")
        .base_url(base_url)
        .build()
        .expect("mock client should build")
}

fn assert_mleg_parent_shape(order: &alpaca_trade::orders::Order, expected_leg_count: usize) {
    assert_eq!(order.order_class, OrderClass::Mleg);
    assert_eq!(order.symbol, "");
    assert_eq!(order.asset_class, "");
    assert_eq!(order.side, OrderSide::Unspecified);
    assert_eq!(order.position_intent, None);
    let legs = order
        .legs
        .as_ref()
        .expect("mleg parent should include legs");
    assert_eq!(legs.len(), expected_leg_count);
    assert!(legs.iter().all(|leg| leg.order_class == OrderClass::Mleg));
    assert!(legs.iter().all(|leg| leg.asset_class == "us_option"));
    assert!(legs.iter().all(|leg| leg.limit_price.is_none()));
    assert!(legs.iter().all(|leg| leg.ratio_qty.is_some()));
}

#[tokio::test]
async fn stock_limit_orders_cover_list_get_replace_cancel_and_alias_lookup() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;
    let stock = stock_price_context(&context, "SPY")
        .await
        .expect("live stock quote should be available for mock route tests");
    let server = alpaca_trade_mock::spawn_test_server().await;
    let client = mock_client(server.base_url.clone());

    let created = client
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(stock.non_marketable_buy_limit_price),
            client_order_id: Some("mock-stock-limit-route-1".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock create should succeed");

    assert_eq!(created.status, OrderStatus::New);

    let listed = client
        .orders()
        .list(Default::default())
        .await
        .expect("mock list should succeed");
    assert!(listed.iter().any(|order| order.id == created.id));

    let fetched = client
        .orders()
        .get(&created.id)
        .await
        .expect("mock get should succeed");
    assert_eq!(fetched.id, created.id);

    let fetched_by_client_order_id = client
        .orders()
        .get_by_client_order_id("mock-stock-limit-route-1")
        .await
        .expect("mock alias get should succeed");
    assert_eq!(fetched_by_client_order_id.id, created.id);

    let replaced = client
        .orders()
        .replace(
            &created.id,
            ReplaceRequest {
                limit_price: Some(stock.more_conservative_buy_limit_price),
                ..Default::default()
            },
        )
        .await
        .expect("mock replace should succeed");
    assert_ne!(replaced.id, created.id);
    assert_eq!(replaced.replaces.as_deref(), Some(created.id.as_str()));
    assert_eq!(
        replaced.limit_price,
        Some(stock.more_conservative_buy_limit_price)
    );
    assert_eq!(replaced.status, OrderStatus::New);

    let replaced_source = client
        .orders()
        .get(&created.id)
        .await
        .expect("original order should still be queryable");
    assert_eq!(replaced_source.status, OrderStatus::Replaced);
    assert_eq!(
        replaced_source.replaced_by.as_deref(),
        Some(replaced.id.as_str())
    );

    client
        .orders()
        .cancel(&replaced.id)
        .await
        .expect("mock cancel should succeed");

    let canceled = client
        .orders()
        .get(&replaced.id)
        .await
        .expect("mock get after cancel should succeed");
    assert_eq!(canceled.status, OrderStatus::Canceled);
}

#[tokio::test]
async fn market_orders_fill_immediately_for_stocks_and_dynamically_discovered_options() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;
    let contract = discover_option_contract(&context, "SPY")
        .await
        .expect("dynamic option contract should be discoverable");

    let server = alpaca_trade_mock::spawn_test_server().await;
    let client = mock_client(server.base_url.clone());

    let stock = client
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            client_order_id: Some("mock-stock-market-route-1".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock stock market create should succeed");
    assert_eq!(stock.status, OrderStatus::Filled);

    let option = client
        .orders()
        .create(CreateRequest {
            symbol: Some(contract.symbol.clone()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Market),
            time_in_force: Some(TimeInForce::Day),
            position_intent: Some(PositionIntent::BuyToOpen),
            client_order_id: Some("mock-option-market-route-1".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock option market create should succeed");
    assert_eq!(option.status, OrderStatus::Filled);
    assert_eq!(option.asset_class, "us_option");
    assert_eq!(option.symbol, contract.symbol);
}

#[tokio::test]
async fn mleg_limit_orders_cover_create_get_list_alias_replace_and_cancel() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;
    let multi_leg = discover_mleg_call_spread(&context, "SPY")
        .await
        .expect("dynamic call spread should be discoverable");

    let server = alpaca_trade_mock::spawn_test_server().await;
    let client = mock_client(server.base_url.clone());

    let created = client
        .orders()
        .create(CreateRequest {
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(multi_leg.non_marketable_limit_price),
            client_order_id: Some("mock-mleg-limit-route-1".to_owned()),
            order_class: Some(OrderClass::Mleg),
            legs: Some(multi_leg.legs.clone()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock mleg create should succeed");
    assert_eq!(created.status, OrderStatus::New);
    assert_mleg_parent_shape(&created, multi_leg.legs.len());

    let fetched = client
        .orders()
        .get(&created.id)
        .await
        .expect("mock get should succeed");
    assert_eq!(fetched.id, created.id);
    assert_mleg_parent_shape(&fetched, multi_leg.legs.len());

    let listed = client
        .orders()
        .list(alpaca_trade::orders::ListRequest {
            status: Some(QueryOrderStatus::Open),
            ..Default::default()
        })
        .await
        .expect("mock list should succeed");
    assert!(listed.iter().any(|order| order.id == created.id));

    let fetched_by_client_order_id = client
        .orders()
        .get_by_client_order_id("mock-mleg-limit-route-1")
        .await
        .expect("mock alias get should succeed");
    assert_eq!(fetched_by_client_order_id.id, created.id);

    let created_legs = created
        .legs
        .clone()
        .expect("created order should include legs");
    let replaced = client
        .orders()
        .replace(
            &created.id,
            ReplaceRequest {
                limit_price: Some(multi_leg.more_conservative_limit_price),
                ..Default::default()
            },
        )
        .await
        .expect("mock mleg replace should succeed");

    assert_ne!(replaced.id, created.id);
    assert_eq!(replaced.replaces.as_deref(), Some(created.id.as_str()));
    assert_eq!(replaced.status, OrderStatus::New);
    assert_mleg_parent_shape(&replaced, multi_leg.legs.len());
    assert_ne!(replaced.client_order_id, created.client_order_id);

    let replaced_legs = replaced
        .legs
        .clone()
        .expect("replacement should include legs");
    assert_eq!(replaced_legs.len(), created_legs.len());
    for (old_leg, new_leg) in created_legs.iter().zip(replaced_legs.iter()) {
        assert_ne!(new_leg.id, old_leg.id);
        assert_eq!(new_leg.replaces.as_deref(), Some(old_leg.id.as_str()));
        assert_eq!(new_leg.status, OrderStatus::New);
    }

    client
        .orders()
        .cancel(&replaced.id)
        .await
        .expect("mock mleg cancel should succeed");

    let canceled = client
        .orders()
        .get(&replaced.id)
        .await
        .expect("mock get after cancel should succeed");
    assert_eq!(canceled.status, OrderStatus::Canceled);
    let canceled_legs = canceled.legs.expect("canceled mleg should keep legs");
    assert!(
        canceled_legs
            .iter()
            .all(|leg| leg.status == OrderStatus::Canceled)
    );
}

#[tokio::test]
async fn marketable_put_spread_limit_orders_fill_using_dynamic_contracts() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;
    let multi_leg = discover_mleg_put_spread(&context, "SPY")
        .await
        .expect("dynamic put spread should be discoverable");

    let server = alpaca_trade_mock::spawn_test_server().await;
    let client = mock_client(server.base_url.clone());

    let filled = client
        .orders()
        .create(CreateRequest {
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(multi_leg.marketable_limit_price),
            client_order_id: Some("mock-mleg-put-fill-route-1".to_owned()),
            order_class: Some(OrderClass::Mleg),
            legs: Some(multi_leg.legs.clone()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock marketable put spread create should succeed");

    assert_eq!(filled.status, OrderStatus::Filled);
    assert!(filled.filled_avg_price.is_some());
    let filled_legs = filled.legs.expect("filled mleg should include legs");
    assert!(
        filled_legs
            .iter()
            .all(|leg| leg.status == OrderStatus::Filled)
    );
    assert!(filled_legs.iter().all(|leg| leg.filled_avg_price.is_some()));
}

#[tokio::test]
async fn marketable_iron_condor_limit_orders_fill_using_dynamic_contracts() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;
    let multi_leg = discover_mleg_iron_condor(&context, "SPY")
        .await
        .expect("dynamic iron condor should be discoverable");

    let server = alpaca_trade_mock::spawn_test_server().await;
    let client = mock_client(server.base_url.clone());

    let filled = client
        .orders()
        .create(CreateRequest {
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(multi_leg.marketable_limit_price),
            client_order_id: Some("mock-mleg-condor-fill-route-1".to_owned()),
            order_class: Some(OrderClass::Mleg),
            legs: Some(multi_leg.legs.clone()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock marketable iron condor create should succeed");

    assert_eq!(filled.status, OrderStatus::Filled);
    assert!(filled.filled_avg_price.is_some());
    let filled_legs = filled.legs.expect("filled mleg should include legs");
    assert!(
        filled_legs
            .iter()
            .all(|leg| leg.status == OrderStatus::Filled)
    );
    assert!(filled_legs.iter().all(|leg| leg.filled_avg_price.is_some()));
}

#[tokio::test]
async fn cancel_all_returns_each_canceled_open_order() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;
    let stock = stock_price_context(&context, "SPY")
        .await
        .expect("live stock quote should be available for mock route tests");
    let server = alpaca_trade_mock::spawn_test_server().await;
    let client = mock_client(server.base_url.clone());

    for index in 0..2 {
        let created = client
            .orders()
            .create(CreateRequest {
                symbol: Some("SPY".to_owned()),
                qty: Some(Decimal::new(1, 0)),
                side: Some(OrderSide::Buy),
                r#type: Some(OrderType::Limit),
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(stock.non_marketable_buy_limit_price),
                client_order_id: Some(format!("mock-cancel-all-route-{index}")),
                ..CreateRequest::default()
            })
            .await
            .expect("mock create should succeed");
        assert_eq!(created.status, OrderStatus::New);
    }

    let canceled = client
        .orders()
        .cancel_all()
        .await
        .expect("mock cancel_all should succeed");
    assert_eq!(canceled.len(), 2);
    assert!(canceled.iter().all(|result| result.status == 200));
    assert!(canceled.iter().all(|result| {
        result
            .body
            .as_ref()
            .map(|order| order.status == OrderStatus::Canceled)
            .unwrap_or(false)
    }));
}

#[tokio::test]
async fn get_by_client_order_id_is_account_local() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;
    let stock = stock_price_context(&context, "SPY")
        .await
        .expect("live stock quote should be available for mock route tests");
    let server = alpaca_trade_mock::spawn_test_server().await;
    let first_client = mock_client_with_api_key(server.base_url.clone(), "mock-api-key-a");
    let second_client = mock_client_with_api_key(server.base_url.clone(), "mock-api-key-b");

    let first = first_client
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(stock.non_marketable_buy_limit_price),
            client_order_id: Some("shared-client-order-id".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("first mock create should succeed");
    let second = second_client
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(stock.non_marketable_buy_limit_price),
            client_order_id: Some("shared-client-order-id".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("second mock create should succeed");

    let first_lookup = first_client
        .orders()
        .get_by_client_order_id("shared-client-order-id")
        .await
        .expect("first client should resolve its own order");
    let second_lookup = second_client
        .orders()
        .get_by_client_order_id("shared-client-order-id")
        .await
        .expect("second client should resolve its own order");

    assert_eq!(first_lookup.id, first.id);
    assert_eq!(second_lookup.id, second.id);
    assert_ne!(first_lookup.id, second_lookup.id);
}

#[tokio::test]
async fn non_marketable_limit_orders_now_rest_in_new_status() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;
    let stock = stock_price_context(&context, "SPY")
        .await
        .expect("live stock quote should be available for mock route tests");
    let server = alpaca_trade_mock::spawn_test_server().await;
    let client = mock_client(server.base_url.clone());

    let created = client
        .orders()
        .create(CreateRequest {
            symbol: Some("SPY".to_owned()),
            qty: Some(Decimal::new(1, 0)),
            side: Some(OrderSide::Buy),
            r#type: Some(OrderType::Limit),
            time_in_force: Some(TimeInForce::Day),
            limit_price: Some(stock.non_marketable_buy_limit_price),
            client_order_id: Some("mock-stock-limit-route-new-status".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock create should succeed");

    assert_eq!(created.status, OrderStatus::New);
}
