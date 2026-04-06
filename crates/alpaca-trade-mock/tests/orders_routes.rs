use alpaca_trade::Decimal;
use alpaca_trade::orders::{
    CreateRequest, OrderSide, OrderStatus, OrderType, PositionIntent, TimeInForce,
};

fn mock_client(base_url: String) -> alpaca_trade::Client {
    alpaca_trade::Client::builder()
        .api_key("mock-api-key")
        .secret_key("mock-secret-key")
        .base_url(base_url)
        .build()
        .expect("mock client should build")
}

#[tokio::test]
async fn stock_limit_orders_cover_list_get_replace_cancel_and_alias_lookup() {
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
            limit_price: Some(Decimal::new(49900, 2)),
            client_order_id: Some("mock-stock-limit-route-1".to_owned()),
            ..CreateRequest::default()
        })
        .await
        .expect("mock create should succeed");

    assert_eq!(created.status, OrderStatus::Accepted);

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
            alpaca_trade::orders::ReplaceRequest {
                limit_price: Some(Decimal::new(49800, 2)),
                ..Default::default()
            },
        )
        .await
        .expect("mock replace should succeed");
    assert_eq!(replaced.id, created.id);
    assert_eq!(replaced.limit_price, Some(Decimal::new(49800, 2)));
    assert_eq!(replaced.status, OrderStatus::Accepted);

    client
        .orders()
        .cancel(&created.id)
        .await
        .expect("mock cancel should succeed");

    let canceled = client
        .orders()
        .get(&created.id)
        .await
        .expect("mock get after cancel should succeed");
    assert_eq!(canceled.status, OrderStatus::Canceled);
}

#[tokio::test]
async fn market_orders_fill_immediately_for_stocks_and_options() {
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
            symbol: Some("SPY260417C00550000".to_owned()),
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
}

#[tokio::test]
async fn cancel_all_returns_each_canceled_open_order() {
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
                limit_price: Some(Decimal::new(49900, 2)),
                client_order_id: Some(format!("mock-cancel-all-route-{index}")),
                ..CreateRequest::default()
            })
            .await
            .expect("mock create should succeed");
        assert_eq!(created.status, OrderStatus::Accepted);
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
