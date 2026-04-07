mod support;

use alpaca_trade::orders::{
    CreateRequest, ListRequest, OrderClass, OrderSide, OrderStatus, OrderType, PositionIntent,
    QueryOrderStatus, ReplaceRequest, TimeInForce,
};
use support::orders::{
    CleanupTracker, MultiLegOrderContext, cleanup_open_orders, discover_mleg_call_spread,
    discover_mleg_iron_condor, discover_mleg_put_spread, discover_option_contract,
    orders_test_context, orders_test_lock, stock_price_context, wait_for_order_statuses,
    wait_for_order_terminal_state,
};

async fn exercise_mleg_limit_lifecycle(
    context: &support::orders::OrdersTestContext,
    cleanup: &mut CleanupTracker,
    suite: &str,
    multi_leg: &MultiLegOrderContext,
) -> Result<(), alpaca_trade::Error> {
    let create_request = CreateRequest {
        qty: Some(alpaca_trade::Decimal::new(1, 0)),
        side: Some(OrderSide::Buy),
        r#type: Some(OrderType::Limit),
        time_in_force: Some(TimeInForce::Day),
        limit_price: Some(multi_leg.non_marketable_limit_price),
        client_order_id: Some(context.next_client_order_id("mleg-limit", suite)),
        order_class: Some(OrderClass::Mleg),
        legs: Some(multi_leg.legs.clone()),
        ..CreateRequest::default()
    };
    let order = context.trade_client.orders().create(create_request).await?;
    cleanup.record_order_id(order.id.clone());
    cleanup.record_client_order_id(order.client_order_id.clone());

    let fetched = context.trade_client.orders().get(&order.id).await?;
    assert_eq!(fetched.id, order.id);
    assert_eq!(fetched.order_class, OrderClass::Mleg);

    let listed = context
        .trade_client
        .orders()
        .list(ListRequest {
            status: Some(QueryOrderStatus::Open),
            limit: Some(100),
            ..ListRequest::default()
        })
        .await?;
    assert!(
        listed
            .iter()
            .any(|listed_order| listed_order.id == order.id)
    );

    let fetched_by_client_order_id = context
        .trade_client
        .orders()
        .get_by_client_order_id(&order.client_order_id)
        .await?;
    assert_eq!(fetched_by_client_order_id.id, order.id);

    let replaceable = wait_for_order_statuses(
        &context.trade_client,
        &order.id,
        &[
            OrderStatus::New,
            OrderStatus::Accepted,
            OrderStatus::PendingNew,
        ],
    )
    .await?;
    let replaced = context
        .trade_client
        .orders()
        .replace(
            &replaceable.id,
            ReplaceRequest {
                limit_price: Some(multi_leg.more_conservative_limit_price),
                ..ReplaceRequest::default()
            },
        )
        .await?;
    let active_order_id = if replaced.id == order.id {
        order.id.clone()
    } else {
        assert_eq!(replaced.replaces.as_deref(), Some(order.id.as_str()));
        cleanup.record_order_id(replaced.id.clone());
        replaced.id.clone()
    };

    context
        .trade_client
        .orders()
        .cancel(&active_order_id)
        .await?;
    let canceled = wait_for_order_terminal_state(&context.trade_client, &active_order_id).await?;
    assert_eq!(canceled.status, OrderStatus::Canceled);
    Ok(())
}

async fn close_filled_mleg_legs(
    context: &support::orders::OrdersTestContext,
    cleanup: &mut CleanupTracker,
    suite: &str,
    multi_leg: &MultiLegOrderContext,
) -> Result<(), alpaca_trade::Error> {
    for (index, leg) in multi_leg.legs.iter().enumerate() {
        let (side, position_intent) = match (
            leg.side.clone().expect("mleg leg side should be present"),
            leg.position_intent
                .clone()
                .expect("mleg leg position intent should be present"),
        ) {
            (OrderSide::Buy, PositionIntent::BuyToOpen) => {
                (OrderSide::Sell, PositionIntent::SellToClose)
            }
            (OrderSide::Sell, PositionIntent::SellToOpen) => {
                (OrderSide::Buy, PositionIntent::BuyToClose)
            }
            (side, intent) => panic!("unexpected open mleg leg shape: {side:?} / {intent:?}"),
        };

        let close = context
            .trade_client
            .orders()
            .create(CreateRequest {
                symbol: Some(leg.symbol.clone()),
                qty: Some(alpaca_trade::Decimal::new(i64::from(leg.ratio_qty), 0)),
                side: Some(side),
                r#type: Some(OrderType::Market),
                time_in_force: Some(TimeInForce::Day),
                client_order_id: Some(
                    context.next_client_order_id("mleg-close", &format!("{suite}-leg-{index}")),
                ),
                position_intent: Some(position_intent),
                ..CreateRequest::default()
            })
            .await?;
        cleanup.record_order_id(close.id.clone());
        cleanup.record_client_order_id(close.client_order_id.clone());

        let closed = wait_for_order_terminal_state(&context.trade_client, &close.id).await?;
        assert_eq!(closed.status, OrderStatus::Filled);
    }

    Ok(())
}

async fn exercise_mleg_marketable_fill(
    context: &support::orders::OrdersTestContext,
    cleanup: &mut CleanupTracker,
    suite: &str,
    multi_leg: &MultiLegOrderContext,
) -> Result<(), alpaca_trade::Error> {
    let create_request = CreateRequest {
        qty: Some(alpaca_trade::Decimal::new(1, 0)),
        side: Some(OrderSide::Buy),
        r#type: Some(OrderType::Limit),
        time_in_force: Some(TimeInForce::Day),
        limit_price: Some(multi_leg.marketable_limit_price),
        client_order_id: Some(context.next_client_order_id("mleg-fill", suite)),
        order_class: Some(OrderClass::Mleg),
        legs: Some(multi_leg.legs.clone()),
        ..CreateRequest::default()
    };
    let order = context.trade_client.orders().create(create_request).await?;
    cleanup.record_order_id(order.id.clone());
    cleanup.record_client_order_id(order.client_order_id.clone());

    let fetched = context.trade_client.orders().get(&order.id).await?;
    assert_eq!(fetched.id, order.id);

    let listed = context
        .trade_client
        .orders()
        .list(ListRequest {
            status: Some(QueryOrderStatus::All),
            limit: Some(100),
            ..ListRequest::default()
        })
        .await?;
    assert!(
        listed
            .iter()
            .any(|listed_order| listed_order.id == order.id)
    );

    let fetched_by_client_order_id = context
        .trade_client
        .orders()
        .get_by_client_order_id(&order.client_order_id)
        .await?;
    assert_eq!(fetched_by_client_order_id.id, order.id);

    let filled = wait_for_order_terminal_state(&context.trade_client, &order.id).await?;
    assert_eq!(filled.status, OrderStatus::Filled);

    close_filled_mleg_legs(context, cleanup, suite, multi_leg).await
}

#[tokio::test]
async fn orders_mutating_stock_limit_create_get_replace_cancel_and_lookup_by_client_order_id() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;

    let mut cleanup = CleanupTracker::new(false);
    let result: Result<(), alpaca_trade::Error> = async {
        let quote = stock_price_context(&context, "SPY")
            .await
            .expect("stock quote context should be available");
        let order = context
            .trade_client
            .orders()
            .create(CreateRequest {
                symbol: Some("SPY".to_owned()),
                qty: Some(alpaca_trade::Decimal::new(1, 0)),
                side: Some(alpaca_trade::orders::OrderSide::Buy),
                r#type: Some(OrderType::Limit),
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(quote.non_marketable_buy_limit_price),
                client_order_id: Some(
                    context.next_client_order_id("stock-limit", "lookup-by-client-order-id"),
                ),
                ..CreateRequest::default()
            })
            .await?;
        cleanup.record_order_id(order.id.clone());
        cleanup.record_client_order_id(order.client_order_id.clone());

        let fetched = context.trade_client.orders().get(&order.id).await?;
        assert_eq!(fetched.id, order.id);

        let listed = context
            .trade_client
            .orders()
            .list(ListRequest {
                status: Some(QueryOrderStatus::Open),
                limit: Some(50),
                symbols: Some(vec!["SPY".to_owned()]),
                ..ListRequest::default()
            })
            .await?;
        assert!(
            listed
                .iter()
                .any(|listed_order| listed_order.id == order.id)
        );

        let fetched_by_client_order_id = context
            .trade_client
            .orders()
            .get_by_client_order_id(&order.client_order_id)
            .await?;
        assert_eq!(fetched_by_client_order_id.id, order.id);

        let replaceable = wait_for_order_statuses(
            &context.trade_client,
            &order.id,
            &[
                OrderStatus::New,
                OrderStatus::Accepted,
                OrderStatus::PendingNew,
            ],
        )
        .await?;

        let replaced = context
            .trade_client
            .orders()
            .replace(
                &replaceable.id,
                alpaca_trade::orders::ReplaceRequest {
                    limit_price: Some(quote.more_conservative_buy_limit_price),
                    ..alpaca_trade::orders::ReplaceRequest::default()
                },
            )
            .await?;
        let active_order_id = if replaced.id == order.id {
            order.id.clone()
        } else {
            assert_eq!(replaced.replaces.as_deref(), Some(order.id.as_str()));
            cleanup.record_order_id(replaced.id.clone());
            replaced.id.clone()
        };

        context
            .trade_client
            .orders()
            .cancel(&active_order_id)
            .await?;
        let canceled =
            wait_for_order_terminal_state(&context.trade_client, &active_order_id).await?;
        assert_eq!(canceled.status, OrderStatus::Canceled);
        Ok(())
    }
    .await;

    cleanup_open_orders(&context, &cleanup).await;
    result.expect("stock limit order flow should succeed");
}

#[tokio::test]
async fn orders_mutating_stock_market_order_reaches_terminal_state() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;

    let mut cleanup = CleanupTracker::new(false);
    let result: Result<(), alpaca_trade::Error> = async {
        let opened = context
            .trade_client
            .orders()
            .create(CreateRequest {
                symbol: Some("SPY".to_owned()),
                qty: Some(alpaca_trade::Decimal::new(1, 0)),
                side: Some(alpaca_trade::orders::OrderSide::Buy),
                r#type: Some(OrderType::Market),
                time_in_force: Some(TimeInForce::Day),
                client_order_id: Some(context.next_client_order_id("stock-market", "open")),
                ..CreateRequest::default()
            })
            .await?;
        cleanup.record_order_id(opened.id.clone());

        let filled = wait_for_order_terminal_state(&context.trade_client, &opened.id).await?;
        assert_eq!(filled.status, OrderStatus::Filled);
        if context.is_mock() {
            let account = context.trade_client.account().get().await?;
            assert!(account.cash.is_some());
        }

        let closed = context
            .trade_client
            .orders()
            .create(CreateRequest {
                symbol: Some("SPY".to_owned()),
                qty: Some(alpaca_trade::Decimal::new(1, 0)),
                side: Some(alpaca_trade::orders::OrderSide::Sell),
                r#type: Some(OrderType::Market),
                time_in_force: Some(TimeInForce::Day),
                client_order_id: Some(context.next_client_order_id("stock-market", "close")),
                ..CreateRequest::default()
            })
            .await?;
        cleanup.record_order_id(closed.id.clone());

        let close_filled = wait_for_order_terminal_state(&context.trade_client, &closed.id).await?;
        assert_eq!(close_filled.status, OrderStatus::Filled);
        Ok(())
    }
    .await;

    cleanup_open_orders(&context, &cleanup).await;
    result.expect("stock market order flow should succeed");
}

#[tokio::test]
async fn orders_mutating_option_limit_create_get_replace_cancel_and_lookup_by_client_order_id() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;

    let mut cleanup = CleanupTracker::new(false);
    let result: Result<(), alpaca_trade::Error> = async {
        let contract = discover_option_contract(&context, "SPY")
            .await
            .expect("tradable option contract should be discoverable");
        let order = context
            .trade_client
            .orders()
            .create(CreateRequest {
                symbol: Some(contract.symbol.clone()),
                qty: Some(alpaca_trade::Decimal::new(1, 0)),
                side: Some(alpaca_trade::orders::OrderSide::Buy),
                r#type: Some(OrderType::Limit),
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(contract.non_marketable_buy_limit_price),
                client_order_id: Some(
                    context.next_client_order_id("option-limit", "lookup-by-client-order-id"),
                ),
                position_intent: Some(PositionIntent::BuyToOpen),
                ..CreateRequest::default()
            })
            .await?;
        cleanup.record_order_id(order.id.clone());
        cleanup.record_client_order_id(order.client_order_id.clone());

        let fetched = context.trade_client.orders().get(&order.id).await?;
        assert_eq!(fetched.id, order.id);

        let listed = context
            .trade_client
            .orders()
            .list(ListRequest {
                status: Some(QueryOrderStatus::Open),
                limit: Some(50),
                symbols: Some(vec![contract.symbol.clone()]),
                ..ListRequest::default()
            })
            .await?;
        assert!(
            listed
                .iter()
                .any(|listed_order| listed_order.id == order.id)
        );

        let fetched_by_client_order_id = context
            .trade_client
            .orders()
            .get_by_client_order_id(&order.client_order_id)
            .await?;
        assert_eq!(fetched_by_client_order_id.id, order.id);

        let replaceable = wait_for_order_statuses(
            &context.trade_client,
            &order.id,
            &[
                OrderStatus::New,
                OrderStatus::Accepted,
                OrderStatus::PendingNew,
            ],
        )
        .await?;
        let replacement_client_order_id =
            context.next_client_order_id("option-limit", "replacement");
        let replaced = context
            .trade_client
            .orders()
            .replace(
                &replaceable.id,
                alpaca_trade::orders::ReplaceRequest {
                    limit_price: Some(contract.more_conservative_buy_limit_price),
                    client_order_id: Some(replacement_client_order_id.clone()),
                    ..alpaca_trade::orders::ReplaceRequest::default()
                },
            )
            .await?;
        let active_order_id = if replaced.id == order.id {
            order.id.clone()
        } else {
            assert_eq!(replaced.replaces.as_deref(), Some(order.id.as_str()));
            cleanup.record_order_id(replaced.id.clone());
            replaced.id.clone()
        };

        context
            .trade_client
            .orders()
            .cancel(&active_order_id)
            .await?;
        let canceled =
            wait_for_order_terminal_state(&context.trade_client, &active_order_id).await?;
        assert_eq!(canceled.status, OrderStatus::Canceled);
        Ok(())
    }
    .await;

    cleanup_open_orders(&context, &cleanup).await;
    result.expect("option limit order flow should succeed");
}

#[tokio::test]
async fn orders_mutating_option_market_order_reaches_terminal_state() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;

    let mut cleanup = CleanupTracker::new(false);
    let result: Result<(), alpaca_trade::Error> = async {
        let contract = discover_option_contract(&context, "SPY")
            .await
            .expect("tradable option contract should be discoverable");
        let opened = context
            .trade_client
            .orders()
            .create(CreateRequest {
                symbol: Some(contract.symbol.clone()),
                qty: Some(alpaca_trade::Decimal::new(1, 0)),
                side: Some(alpaca_trade::orders::OrderSide::Buy),
                r#type: Some(OrderType::Market),
                time_in_force: Some(TimeInForce::Day),
                client_order_id: Some(context.next_client_order_id("option-market", "open")),
                position_intent: Some(PositionIntent::BuyToOpen),
                ..CreateRequest::default()
            })
            .await?;
        cleanup.record_order_id(opened.id.clone());

        let filled = wait_for_order_terminal_state(&context.trade_client, &opened.id).await?;
        assert_eq!(filled.status, OrderStatus::Filled);

        let closed = context
            .trade_client
            .orders()
            .create(CreateRequest {
                symbol: Some(contract.symbol.clone()),
                qty: Some(alpaca_trade::Decimal::new(1, 0)),
                side: Some(alpaca_trade::orders::OrderSide::Sell),
                r#type: Some(OrderType::Market),
                time_in_force: Some(TimeInForce::Day),
                client_order_id: Some(context.next_client_order_id("option-market", "close")),
                position_intent: Some(PositionIntent::SellToClose),
                ..CreateRequest::default()
            })
            .await?;
        cleanup.record_order_id(closed.id.clone());

        let close_filled = wait_for_order_terminal_state(&context.trade_client, &closed.id).await?;
        assert_eq!(close_filled.status, OrderStatus::Filled);
        Ok(())
    }
    .await;

    cleanup_open_orders(&context, &cleanup).await;
    result.expect("option market order flow should succeed");
}

#[tokio::test]
async fn orders_mutating_mleg_call_spread_limit_create_get_replace_cancel_and_lookup_by_client_order_id()
 {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;

    let mut cleanup = CleanupTracker::new(false);
    let result: Result<(), alpaca_trade::Error> = async {
        let multi_leg = discover_mleg_call_spread(&context, "SPY")
            .await
            .expect("quoted call spread should be discoverable");

        exercise_mleg_limit_lifecycle(&context, &mut cleanup, "call-spread", &multi_leg).await
    }
    .await;

    cleanup_open_orders(&context, &cleanup).await;
    result.expect("mleg call spread flow should succeed");
}

#[tokio::test]
async fn orders_mutating_mleg_put_spread_marketable_limit_reaches_terminal_state() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;

    let mut cleanup = CleanupTracker::new(false);
    let result: Result<(), alpaca_trade::Error> = async {
        let multi_leg = discover_mleg_put_spread(&context, "SPY")
            .await
            .expect("quoted put spread should be discoverable");

        exercise_mleg_marketable_fill(&context, &mut cleanup, "put-spread", &multi_leg).await
    }
    .await;

    cleanup_open_orders(&context, &cleanup).await;
    result.expect("mleg put spread fill flow should succeed");
}

#[tokio::test]
async fn orders_mutating_mleg_iron_condor_marketable_limit_reaches_terminal_state() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;

    let mut cleanup = CleanupTracker::new(false);
    let result: Result<(), alpaca_trade::Error> = async {
        let multi_leg = discover_mleg_iron_condor(&context, "SPY")
            .await
            .expect("quoted iron condor should be discoverable");

        exercise_mleg_marketable_fill(&context, &mut cleanup, "iron-condor", &multi_leg).await
    }
    .await;

    cleanup_open_orders(&context, &cleanup).await;
    result.expect("mleg iron condor fill flow should succeed");
}

#[tokio::test]
async fn orders_mutating_cancel_all_clears_test_orders_in_active_runtime() {
    let _guard = orders_test_lock().await;
    let context = orders_test_context().await;

    let mut cleanup = CleanupTracker::new(true);
    let result: Result<(), alpaca_trade::Error> = async {
        let quote = stock_price_context(&context, "SPY")
            .await
            .expect("stock quote context should be available");

        for index in 0..2 {
            let order = context
                .trade_client
                .orders()
                .create(CreateRequest {
                    symbol: Some("SPY".to_owned()),
                    qty: Some(alpaca_trade::Decimal::new(1, 0)),
                    side: Some(alpaca_trade::orders::OrderSide::Buy),
                    r#type: Some(OrderType::Limit),
                    time_in_force: Some(TimeInForce::Day),
                    limit_price: Some(quote.non_marketable_buy_limit_price),
                    client_order_id: Some(
                        context.next_client_order_id("cancel-all", &format!("order-{index}")),
                    ),
                    ..CreateRequest::default()
                })
                .await?;
            cleanup.record_order_id(order.id.clone());
        }

        let results = context.trade_client.orders().cancel_all().await?;
        assert!(!results.is_empty());
        Ok(())
    }
    .await;

    cleanup_open_orders(&context, &cleanup).await;
    result.expect("cancel_all flow should succeed");
}
