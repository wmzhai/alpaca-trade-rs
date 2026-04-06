#![recursion_limit = "256"]

use alpaca_trade::Decimal;
use alpaca_trade::orders::{
    CancelAllOrderResult, Order, OrderClass, OrderSide, OrderStatus, OrderType, PositionIntent,
};
use serde_json::json;

#[test]
fn order_deserializes_current_official_single_order_shape() {
    let payload = json!({
        "id": "904837e3-3b76-47ec-b432-046db621571b",
        "client_order_id": "phase7-orders-stock-limit-1",
        "created_at": "2026-04-06T15:04:05Z",
        "updated_at": "2026-04-06T15:04:05Z",
        "submitted_at": "2026-04-06T15:04:05Z",
        "filled_at": null,
        "expired_at": null,
        "expires_at": null,
        "canceled_at": null,
        "failed_at": null,
        "replaced_at": null,
        "replaced_by": null,
        "replaces": null,
        "asset_id": "b0b6dd9d-8b9b-48a9-ba46-b9d54906e415",
        "symbol": "SPY",
        "asset_class": "us_equity",
        "notional": 250.00,
        "qty": "2",
        "filled_qty": "0",
        "filled_avg_price": null,
        "order_class": "simple",
        "order_type": "limit",
        "type": "limit",
        "side": "buy",
        "position_intent": "buy_to_open",
        "time_in_force": "day",
        "limit_price": "499.25",
        "stop_price": null,
        "status": "accepted",
        "extended_hours": false,
        "legs": null,
        "trail_percent": null,
        "trail_price": null,
        "hwm": null,
        "subtag": null,
        "source": null
    });

    let order: Order = serde_json::from_value(payload).expect("order payload should deserialize");

    assert_eq!(order.symbol, "SPY");
    assert_eq!(order.client_order_id, "phase7-orders-stock-limit-1");
    assert_eq!(order.notional, Some(Decimal::new(25000, 2)));
    assert_eq!(order.qty, Some(Decimal::new(2, 0)));
    assert_eq!(order.order_class, OrderClass::Simple);
    assert_eq!(order.order_type, OrderType::Limit);
    assert_eq!(order.r#type, OrderType::Limit);
    assert_eq!(order.side, OrderSide::Buy);
    assert_eq!(order.position_intent, Some(PositionIntent::BuyToOpen));
    assert_eq!(order.status, OrderStatus::Accepted);
}

#[test]
fn order_deserializes_legs_take_profit_and_stop_loss_shapes() {
    let payload = json!({
        "id": "root-order-id",
        "client_order_id": "phase7-orders-bracket-1",
        "created_at": "2026-04-06T15:04:05Z",
        "updated_at": "2026-04-06T15:04:05Z",
        "submitted_at": "2026-04-06T15:04:05Z",
        "filled_at": null,
        "expired_at": null,
        "expires_at": null,
        "canceled_at": null,
        "failed_at": null,
        "replaced_at": null,
        "replaced_by": null,
        "replaces": null,
        "asset_id": "asset-id",
        "symbol": "SPY",
        "asset_class": "us_equity",
        "notional": null,
        "qty": "1",
        "filled_qty": "0",
        "filled_avg_price": null,
        "order_class": "bracket",
        "order_type": "limit",
        "type": "limit",
        "side": "buy",
        "position_intent": null,
        "time_in_force": "day",
        "limit_price": "498.00",
        "stop_price": null,
        "status": "new",
        "extended_hours": false,
        "take_profit": {
            "limit_price": "510.00"
        },
        "stop_loss": {
            "stop_price": "492.00",
            "limit_price": "491.50"
        },
        "legs": [{
            "id": "leg-order-id",
            "client_order_id": "phase7-orders-bracket-leg-1",
            "created_at": "2026-04-06T15:04:05Z",
            "updated_at": "2026-04-06T15:04:05Z",
            "submitted_at": "2026-04-06T15:04:05Z",
            "filled_at": null,
            "expired_at": null,
            "expires_at": null,
            "canceled_at": null,
            "failed_at": null,
            "replaced_at": null,
            "replaced_by": null,
            "replaces": null,
            "asset_id": "asset-id",
            "symbol": "SPY",
            "asset_class": "us_equity",
            "notional": null,
            "qty": "1",
            "filled_qty": "0",
            "filled_avg_price": null,
            "order_class": "simple",
            "order_type": "limit",
            "type": "limit",
            "side": "sell",
            "position_intent": null,
            "time_in_force": "day",
            "limit_price": "510.00",
            "stop_price": null,
            "status": "held",
            "extended_hours": false,
            "legs": null,
            "trail_percent": null,
            "trail_price": null,
            "hwm": null,
            "subtag": null,
            "source": null
        }],
        "trail_percent": null,
        "trail_price": null,
        "hwm": null,
        "subtag": null,
        "source": null
    });

    let order: Order = serde_json::from_value(payload).expect("nested order payload should deserialize");

    assert_eq!(order.order_class, OrderClass::Bracket);
    assert_eq!(order.take_profit.expect("take profit").limit_price, Decimal::new(51000, 2));
    let stop_loss = order.stop_loss.expect("stop loss");
    assert_eq!(stop_loss.stop_price, Decimal::new(49200, 2));
    assert_eq!(stop_loss.limit_price, Some(Decimal::new(49150, 2)));
    let leg = order.legs.expect("legs").pop().expect("leg");
    assert_eq!(leg.side, OrderSide::Sell);
    assert_eq!(leg.status, OrderStatus::Held);
}

#[test]
fn cancel_all_result_deserializes_official_batch_shape() {
    let payload = json!([
        {
            "id": "904837e3-3b76-47ec-b432-046db621571b",
            "status": 200,
            "body": {
                "id": "904837e3-3b76-47ec-b432-046db621571b",
                "client_order_id": "phase7-orders-cancel-all-1",
                "created_at": "2026-04-06T15:04:05Z",
                "updated_at": "2026-04-06T15:04:05Z",
                "submitted_at": "2026-04-06T15:04:05Z",
                "filled_at": null,
                "expired_at": null,
                "expires_at": null,
                "canceled_at": null,
                "failed_at": null,
                "replaced_at": null,
                "replaced_by": null,
                "replaces": null,
                "asset_id": "asset-id",
                "symbol": "SPY",
                "asset_class": "us_equity",
                "notional": null,
                "qty": "1",
                "filled_qty": "0",
                "filled_avg_price": null,
                "order_class": "simple",
                "order_type": "limit",
                "type": "limit",
                "side": "buy",
                "position_intent": null,
                "time_in_force": "day",
                "limit_price": "400.00",
                "stop_price": null,
                "status": "canceled",
                "extended_hours": false,
                "legs": null,
                "trail_percent": null,
                "trail_price": null,
                "hwm": null,
                "subtag": null,
                "source": null
            }
        }
    ]);

    let results: Vec<CancelAllOrderResult> =
        serde_json::from_value(payload).expect("cancel_all payload should deserialize");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, 200);
    assert_eq!(results[0].body.as_ref().expect("body").status, OrderStatus::Canceled);
}
