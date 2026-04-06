use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use alpaca_trade::Decimal;
use alpaca_trade::orders::{
    CancelAllOrderResult, Order, OrderClass, OrderSide, OrderStatus, OrderType, PositionIntent,
    StopLoss, TakeProfit, TimeInForce,
};
use chrono::Utc;
use parking_lot::RwLock;
use uuid::Uuid;

pub const DEFAULT_STOCK_SYMBOL: &str = "SPY";
pub const DEFAULT_OPTION_SYMBOL: &str = "SPY260417C00550000";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrumentSnapshot {
    pub asset_class: String,
    pub bid: Decimal,
    pub ask: Decimal,
}

impl InstrumentSnapshot {
    pub fn equity(bid: Decimal, ask: Decimal) -> Self {
        Self {
            asset_class: "us_equity".to_owned(),
            bid,
            ask,
        }
    }

    pub fn option(bid: Decimal, ask: Decimal) -> Self {
        Self {
            asset_class: "us_option".to_owned(),
            bid,
            ask,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrdersMarketSnapshot {
    instruments: HashMap<String, InstrumentSnapshot>,
}

impl Default for OrdersMarketSnapshot {
    fn default() -> Self {
        let mut instruments = HashMap::new();
        instruments.insert(
            DEFAULT_STOCK_SYMBOL.to_owned(),
            InstrumentSnapshot::equity(Decimal::new(50000, 2), Decimal::new(50025, 2)),
        );
        instruments.insert(
            DEFAULT_OPTION_SYMBOL.to_owned(),
            InstrumentSnapshot::option(Decimal::new(110, 2), Decimal::new(125, 2)),
        );

        Self { instruments }
    }
}

impl OrdersMarketSnapshot {
    pub fn with_instrument(mut self, symbol: impl Into<String>, instrument: InstrumentSnapshot) -> Self {
        self.instruments.insert(symbol.into(), instrument);
        self
    }

    pub fn instrument(&self, symbol: &str) -> InstrumentSnapshot {
        self.instruments
            .get(symbol)
            .cloned()
            .unwrap_or_else(|| default_instrument_for(symbol))
    }

    pub fn default_option_symbol(&self) -> &str {
        self.instruments
            .iter()
            .find_map(|(symbol, instrument)| {
                if instrument.asset_class == "us_option" {
                    Some(symbol.as_str())
                } else {
                    None
                }
            })
            .unwrap_or(DEFAULT_OPTION_SYMBOL)
    }
}

#[derive(Debug, Clone)]
pub struct OrdersState {
    inner: Arc<RwLock<OrdersStateInner>>,
}

#[derive(Debug)]
struct OrdersStateInner {
    market_snapshot: OrdersMarketSnapshot,
    orders: HashMap<String, Order>,
    client_order_ids: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct CreateOrderInput {
    pub symbol: Option<String>,
    pub qty: Option<Decimal>,
    pub notional: Option<Decimal>,
    pub side: Option<OrderSide>,
    pub order_type: Option<OrderType>,
    pub time_in_force: Option<TimeInForce>,
    pub limit_price: Option<Decimal>,
    pub stop_price: Option<Decimal>,
    pub trail_price: Option<Decimal>,
    pub trail_percent: Option<Decimal>,
    pub extended_hours: Option<bool>,
    pub client_order_id: Option<String>,
    pub order_class: Option<OrderClass>,
    pub position_intent: Option<PositionIntent>,
    pub take_profit: Option<TakeProfit>,
    pub stop_loss: Option<StopLoss>,
}

#[derive(Debug, Clone, Default)]
pub struct ReplaceOrderInput {
    pub qty: Option<Decimal>,
    pub time_in_force: Option<TimeInForce>,
    pub limit_price: Option<Decimal>,
    pub stop_price: Option<Decimal>,
    pub trail: Option<Decimal>,
    pub client_order_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ListOrdersFilter {
    pub status: Option<String>,
    pub symbols: Option<Vec<String>>,
    pub side: Option<OrderSide>,
    pub asset_class: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrdersStateError {
    NotFound(String),
    Conflict(String),
}

impl OrdersState {
    pub fn new(market_snapshot: OrdersMarketSnapshot) -> Self {
        Self {
            inner: Arc::new(RwLock::new(OrdersStateInner {
                market_snapshot,
                orders: HashMap::new(),
                client_order_ids: HashMap::new(),
            })),
        }
    }

    pub fn create_order(&self, input: CreateOrderInput) -> Result<Order, OrdersStateError> {
        let mut inner = self.inner.write();

        let symbol = input.symbol.unwrap_or_else(|| DEFAULT_STOCK_SYMBOL.to_owned());
        let client_order_id = input
            .client_order_id
            .unwrap_or_else(|| format!("mock-order-{}", Uuid::new_v4()));
        if inner.client_order_ids.contains_key(&client_order_id) {
            return Err(OrdersStateError::Conflict(format!(
                "client_order_id {client_order_id} already exists"
            )));
        }

        let instrument = inner.market_snapshot.instrument(&symbol);
        let now = now_string();
        let order_id = Uuid::new_v4().to_string();
        let mut order = build_order(NewOrderSpec {
            id: order_id.clone(),
            client_order_id: client_order_id.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
            submitted_at: now.clone(),
            asset_id: Uuid::new_v4().to_string(),
            symbol,
            asset_class: instrument.asset_class,
            notional: input.notional,
            qty: input.qty,
            order_class: input.order_class.unwrap_or(OrderClass::Simple),
            order_type: input.order_type.unwrap_or(OrderType::Market),
            side: input.side.unwrap_or(OrderSide::Buy),
            position_intent: input.position_intent,
            time_in_force: input.time_in_force.unwrap_or(TimeInForce::Day),
            limit_price: input.limit_price,
            stop_price: input.stop_price,
            extended_hours: input.extended_hours.unwrap_or(false),
            trail_percent: input.trail_percent,
            trail_price: input.trail_price,
            take_profit: input.take_profit,
            stop_loss: input.stop_loss,
        });

        let fill_instrument = inner.market_snapshot.instrument(&order.symbol);
        apply_fill_rules(&mut order, &fill_instrument);
        inner.client_order_ids.insert(client_order_id, order_id.clone());
        inner.orders.insert(order_id, order.clone());

        Ok(order)
    }

    pub fn list_orders(&self, filter: ListOrdersFilter) -> Vec<Order> {
        let inner = self.inner.read();
        let symbol_filter = filter.symbols.map(|symbols| {
            symbols
                .into_iter()
                .map(|symbol| symbol.trim().to_owned())
                .filter(|symbol| !symbol.is_empty())
                .collect::<HashSet<_>>()
        });

        let mut orders = inner
            .orders
            .values()
            .filter(|order| {
                matches_status_filter(order, filter.status.as_deref())
                    && symbol_filter
                        .as_ref()
                        .is_none_or(|symbols| symbols.contains(&order.symbol))
                    && filter
                        .side
                        .as_ref()
                        .is_none_or(|side| &order.side == side)
                    && filter
                        .asset_class
                        .as_deref()
                        .is_none_or(|asset_class| order.asset_class == asset_class)
            })
            .cloned()
            .collect::<Vec<_>>();
        orders.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        orders
    }

    pub fn get_order(&self, order_id: &str) -> Option<Order> {
        self.inner.read().orders.get(order_id).cloned()
    }

    pub fn get_by_client_order_id(&self, client_order_id: &str) -> Option<Order> {
        let inner = self.inner.read();
        let order_id = inner.client_order_ids.get(client_order_id)?;
        inner.orders.get(order_id).cloned()
    }

    pub fn replace_order(
        &self,
        order_id: &str,
        input: ReplaceOrderInput,
    ) -> Result<Order, OrdersStateError> {
        let mut inner = self.inner.write();
        let market_snapshot = inner.market_snapshot.clone();
        let current_client_order_id = inner
            .orders
            .get(order_id)
            .ok_or_else(|| OrdersStateError::NotFound(format!("order {order_id} was not found")))?
            .client_order_id
            .clone();
        if let Some(client_order_id) = input.client_order_id.as_ref() {
            if client_order_id != &current_client_order_id
                && inner.client_order_ids.contains_key(client_order_id)
            {
                return Err(OrdersStateError::Conflict(format!(
                    "client_order_id {client_order_id} already exists"
                )));
            }
        }
        if let Some(client_order_id) = input.client_order_id.as_ref() {
            if client_order_id != &current_client_order_id {
                inner.client_order_ids.remove(&current_client_order_id);
                inner
                    .client_order_ids
                    .insert(client_order_id.clone(), order_id.to_owned());
            }
        }

        let order = inner
            .orders
            .get_mut(order_id)
            .ok_or_else(|| OrdersStateError::NotFound(format!("order {order_id} was not found")))?;
        if is_terminal_status(&order.status) {
            return Err(OrdersStateError::Conflict(format!(
                "order {order_id} is no longer replaceable"
            )));
        }

        if let Some(client_order_id) = input.client_order_id {
            if client_order_id != order.client_order_id {
                order.client_order_id = client_order_id;
            }
        }

        if let Some(qty) = input.qty {
            order.qty = Some(qty);
        }
        if let Some(time_in_force) = input.time_in_force {
            order.time_in_force = time_in_force;
        }
        if input.limit_price.is_some() {
            order.limit_price = input.limit_price;
        }
        if input.stop_price.is_some() {
            order.stop_price = input.stop_price;
        }
        if let Some(trail) = input.trail {
            order.trail_price = Some(trail);
        }
        order.updated_at = now_string();
        order.status = OrderStatus::Accepted;
        apply_fill_rules(order, &market_snapshot.instrument(&order.symbol));

        Ok(order.clone())
    }

    pub fn cancel_order(&self, order_id: &str) -> Result<(), OrdersStateError> {
        let mut inner = self.inner.write();
        let order = inner
            .orders
            .get_mut(order_id)
            .ok_or_else(|| OrdersStateError::NotFound(format!("order {order_id} was not found")))?;
        if is_terminal_status(&order.status) {
            return Err(OrdersStateError::Conflict(format!(
                "order {order_id} is already terminal"
            )));
        }

        mark_order_canceled(order);
        Ok(())
    }

    pub fn cancel_all_orders(&self) -> Vec<CancelAllOrderResult> {
        let mut inner = self.inner.write();
        let mut order_ids = inner
            .orders
            .iter()
            .filter_map(|(order_id, order)| {
                if is_terminal_status(&order.status) {
                    None
                } else {
                    Some(order_id.clone())
                }
            })
            .collect::<Vec<_>>();
        order_ids.sort();

        order_ids
            .into_iter()
            .filter_map(|order_id| {
                let order = inner.orders.get_mut(&order_id)?;
                mark_order_canceled(order);
                Some(build_cancel_all_result(order_id, 200, Some(order.clone())))
            })
            .collect()
    }

    pub fn market_snapshot(&self) -> OrdersMarketSnapshot {
        self.inner.read().market_snapshot.clone()
    }
}

fn apply_fill_rules(order: &mut Order, instrument: &InstrumentSnapshot) {
    let fill_price = match order.r#type {
        OrderType::Market => Some(match order.side {
            OrderSide::Buy => instrument.ask,
            OrderSide::Sell => instrument.bid,
            _ => instrument.ask,
        }),
        OrderType::Limit => match order.side {
            OrderSide::Buy => order
                .limit_price
                .filter(|limit_price| *limit_price >= instrument.ask)
                .map(|_| instrument.ask),
            OrderSide::Sell => order
                .limit_price
                .filter(|limit_price| *limit_price <= instrument.bid)
                .map(|_| instrument.bid),
            _ => None,
        },
        _ => None,
    };

    if let Some(fill_price) = fill_price {
        order.status = OrderStatus::Filled;
        order.filled_qty = order.qty.unwrap_or(Decimal::ZERO);
        order.filled_avg_price = Some(fill_price);
        let now = now_string();
        order.filled_at = Some(now.clone());
        order.updated_at = now;
        order.canceled_at = None;
        return;
    }

    order.status = OrderStatus::Accepted;
    order.filled_qty = Decimal::ZERO;
    order.filled_avg_price = None;
    order.filled_at = None;
}

fn mark_order_canceled(order: &mut Order) {
    let now = now_string();
    order.status = OrderStatus::Canceled;
    order.updated_at = now.clone();
    order.canceled_at = Some(now);
    order.filled_at = None;
    order.filled_qty = Decimal::ZERO;
    order.filled_avg_price = None;
}

fn matches_status_filter(order: &Order, status: Option<&str>) -> bool {
    match status {
        None | Some("all") => true,
        Some("open") => !is_terminal_status(&order.status),
        Some("closed") => is_terminal_status(&order.status),
        Some(_) => true,
    }
}

fn is_terminal_status(status: &OrderStatus) -> bool {
    matches!(
        status,
        OrderStatus::Filled
            | OrderStatus::Canceled
            | OrderStatus::Expired
            | OrderStatus::Rejected
            | OrderStatus::Replaced
    )
}

fn default_instrument_for(symbol: &str) -> InstrumentSnapshot {
    if looks_like_option_symbol(symbol) {
        return InstrumentSnapshot::option(Decimal::new(110, 2), Decimal::new(125, 2));
    }

    InstrumentSnapshot::equity(Decimal::new(50000, 2), Decimal::new(50025, 2))
}

fn looks_like_option_symbol(symbol: &str) -> bool {
    let bytes = symbol.as_bytes();
    if bytes.len() < 16 {
        return false;
    }

    bytes.iter().any(|byte| *byte == b'C' || *byte == b'P')
        && bytes.iter().filter(|byte| byte.is_ascii_digit()).count() >= 8
}

fn now_string() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[derive(Debug)]
struct NewOrderSpec {
    id: String,
    client_order_id: String,
    created_at: String,
    updated_at: String,
    submitted_at: String,
    asset_id: String,
    symbol: String,
    asset_class: String,
    notional: Option<Decimal>,
    qty: Option<Decimal>,
    order_class: OrderClass,
    order_type: OrderType,
    side: OrderSide,
    position_intent: Option<PositionIntent>,
    time_in_force: TimeInForce,
    limit_price: Option<Decimal>,
    stop_price: Option<Decimal>,
    extended_hours: bool,
    trail_percent: Option<Decimal>,
    trail_price: Option<Decimal>,
    take_profit: Option<TakeProfit>,
    stop_loss: Option<StopLoss>,
}

fn build_order(spec: NewOrderSpec) -> Order {
    serde_json::from_value(serde_json::json!({
        "id": spec.id,
        "client_order_id": spec.client_order_id,
        "created_at": spec.created_at,
        "updated_at": spec.updated_at,
        "submitted_at": spec.submitted_at,
        "filled_at": null,
        "expired_at": null,
        "expires_at": null,
        "canceled_at": null,
        "failed_at": null,
        "replaced_at": null,
        "replaced_by": null,
        "replaces": null,
        "asset_id": spec.asset_id,
        "symbol": spec.symbol,
        "asset_class": spec.asset_class,
        "notional": spec.notional,
        "qty": spec.qty,
        "filled_qty": Decimal::ZERO,
        "filled_avg_price": null,
        "order_class": spec.order_class,
        "order_type": spec.order_type,
        "type": spec.order_type,
        "side": spec.side,
        "position_intent": spec.position_intent,
        "time_in_force": spec.time_in_force,
        "limit_price": spec.limit_price,
        "stop_price": spec.stop_price,
        "status": OrderStatus::Accepted,
        "extended_hours": spec.extended_hours,
        "legs": null,
        "trail_percent": spec.trail_percent,
        "trail_price": spec.trail_price,
        "hwm": null,
        "ratio_qty": null,
        "take_profit": spec.take_profit,
        "stop_loss": spec.stop_loss,
        "subtag": null,
        "source": "alpaca-trade-mock",
    }))
    .expect("mock order json should deserialize")
}

fn build_cancel_all_result(
    id: String,
    status: u16,
    body: Option<Order>,
) -> CancelAllOrderResult {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "status": status,
        "body": body,
    }))
    .expect("mock cancel_all result json should deserialize")
}
