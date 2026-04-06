use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use alpaca_data::Client as DataClient;
use alpaca_data::options::{Snapshot as OptionSnapshot, SnapshotsRequest};
use alpaca_data::stocks::LatestQuoteRequest;
use alpaca_trade::Decimal;
use alpaca_trade::orders::{
    CancelAllOrderResult, OptionLegRequest, Order, OrderClass, OrderSide, OrderStatus, OrderType,
    PositionIntent, StopLoss, TakeProfit, TimeInForce,
};
use chrono::Utc;
use parking_lot::RwLock;
use uuid::Uuid;

pub const DEFAULT_STOCK_SYMBOL: &str = "SPY";
const API_KEY_CANDIDATES: [&str; 2] = ["ALPACA_TRADE_API_KEY", "APCA_API_KEY_ID"];
const SECRET_KEY_CANDIDATES: [&str; 2] = ["ALPACA_TRADE_SECRET_KEY", "APCA_API_SECRET_KEY"];

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

        Self { instruments }
    }
}

impl OrdersMarketSnapshot {
    pub fn with_instrument(
        mut self,
        symbol: impl Into<String>,
        instrument: InstrumentSnapshot,
    ) -> Self {
        self.instruments.insert(symbol.into(), instrument);
        self
    }

    pub fn instrument(&self, symbol: &str) -> InstrumentSnapshot {
        self.instruments
            .get(symbol)
            .cloned()
            .unwrap_or_else(|| default_instrument_for(symbol))
    }

    pub fn default_option_symbol(&self) -> Option<&str> {
        self.instruments.iter().find_map(|(symbol, instrument)| {
            if instrument.asset_class == "us_option" {
                Some(symbol.as_str())
            } else {
                None
            }
        })
    }
}

#[derive(Clone)]
pub struct OrdersState {
    inner: Arc<RwLock<OrdersStateInner>>,
    data_client: Option<DataClient>,
}

#[derive(Debug)]
struct OrdersStateInner {
    market_snapshot: OrdersMarketSnapshot,
    orders: HashMap<String, StoredOrder>,
    client_order_ids: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct StoredOrder {
    order: Order,
    request_side: OrderSide,
}

#[derive(Debug, Clone, Default)]
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
    pub legs: Option<Vec<OptionLegRequest>>,
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
            data_client: data_client_from_environment(),
        }
    }

    pub async fn create_order(&self, input: CreateOrderInput) -> Result<Order, OrdersStateError> {
        let order_class = input.order_class.clone().unwrap_or(OrderClass::Simple);
        let request_side = input.side.clone().unwrap_or(OrderSide::Buy);
        let order_type = input.order_type.clone().unwrap_or(OrderType::Market);
        let time_in_force = input.time_in_force.clone().unwrap_or(TimeInForce::Day);
        let symbol = input
            .symbol
            .clone()
            .unwrap_or_else(|| DEFAULT_STOCK_SYMBOL.to_owned());
        let client_order_id = input
            .client_order_id
            .clone()
            .unwrap_or_else(|| format!("mock-order-{}", Uuid::new_v4()));

        {
            let inner = self.inner.read();
            if inner.client_order_ids.contains_key(&client_order_id) {
                return Err(OrdersStateError::Conflict(format!(
                    "client_order_id {client_order_id} already exists"
                )));
            }
        }

        let market_quotes = self
            .resolve_market_quotes(&requested_symbols(
                input.symbol.as_deref(),
                input.legs.as_deref(),
            ))
            .await;
        let now = now_string();
        let order_id = Uuid::new_v4().to_string();
        let legs = if order_class == OrderClass::Mleg {
            Some(build_leg_orders_from_requests(
                input.legs.as_deref().unwrap_or(&[]),
                input.qty,
                order_type.clone(),
                time_in_force.clone(),
                &now,
                None,
            ))
        } else {
            None
        };

        let effective_symbol = if order_class == OrderClass::Mleg {
            String::new()
        } else {
            symbol.clone()
        };
        let effective_asset_class = if order_class == OrderClass::Mleg {
            String::new()
        } else {
            market_quotes
                .get(&symbol)
                .cloned()
                .unwrap_or_else(|| default_instrument_for(&symbol))
                .asset_class
        };
        let effective_asset_id = if order_class == OrderClass::Mleg {
            String::new()
        } else {
            Uuid::new_v4().to_string()
        };
        let mut order = build_order(NewOrderSpec {
            id: order_id.clone(),
            client_order_id: client_order_id.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
            submitted_at: now.clone(),
            expires_at: expires_at_for(time_in_force.clone()),
            asset_id: effective_asset_id,
            symbol: effective_symbol,
            asset_class: effective_asset_class,
            notional: input.notional,
            qty: input.qty,
            order_class: order_class.clone(),
            order_type: order_type,
            side: if order_class == OrderClass::Mleg {
                OrderSide::Unspecified
            } else {
                request_side.clone()
            },
            position_intent: if order_class == OrderClass::Mleg {
                None
            } else {
                input.position_intent
            },
            time_in_force,
            limit_price: input.limit_price,
            stop_price: input.stop_price,
            extended_hours: input.extended_hours.unwrap_or(false),
            trail_percent: input.trail_percent,
            trail_price: input.trail_price,
            ratio_qty: None,
            legs,
            replaces: None,
            take_profit: input.take_profit,
            stop_loss: input.stop_loss,
        });
        apply_fill_rules(&mut order, &request_side, &market_quotes);

        let mut inner = self.inner.write();
        inner
            .client_order_ids
            .insert(client_order_id, order_id.clone());
        inner.orders.insert(
            order_id,
            StoredOrder {
                order: order.clone(),
                request_side,
            },
        );

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
            .filter(|stored| {
                let order = &stored.order;
                matches_status_filter(order, filter.status.as_deref())
                    && symbol_filter
                        .as_ref()
                        .is_none_or(|symbols| symbols.contains(&order.symbol))
                    && filter.side.as_ref().is_none_or(|side| &order.side == side)
                    && filter
                        .asset_class
                        .as_deref()
                        .is_none_or(|asset_class| order.asset_class == asset_class)
            })
            .map(|stored| stored.order.clone())
            .collect::<Vec<_>>();
        orders.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        orders
    }

    pub fn get_order(&self, order_id: &str) -> Option<Order> {
        self.inner
            .read()
            .orders
            .get(order_id)
            .map(|stored| stored.order.clone())
    }

    pub fn get_by_client_order_id(&self, client_order_id: &str) -> Option<Order> {
        let inner = self.inner.read();
        let order_id = inner.client_order_ids.get(client_order_id)?;
        inner
            .orders
            .get(order_id)
            .map(|stored| stored.order.clone())
    }

    pub async fn replace_order(
        &self,
        order_id: &str,
        input: ReplaceOrderInput,
    ) -> Result<Order, OrdersStateError> {
        let current = {
            let inner = self.inner.read();
            inner.orders.get(order_id).cloned().ok_or_else(|| {
                OrdersStateError::NotFound(format!("order {order_id} was not found"))
            })?
        };
        if is_terminal_status(&current.order.status) {
            return Err(OrdersStateError::Conflict(format!(
                "order {order_id} is no longer replaceable"
            )));
        }

        let replacement_client_order_id = input
            .client_order_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        {
            let inner = self.inner.read();
            if replacement_client_order_id != current.order.client_order_id
                && inner
                    .client_order_ids
                    .contains_key(&replacement_client_order_id)
            {
                return Err(OrdersStateError::Conflict(format!(
                    "client_order_id {replacement_client_order_id} already exists"
                )));
            }
        }

        let now = now_string();
        let replacement_order_id = Uuid::new_v4().to_string();
        let replacement_qty = input.qty.or(current.order.qty);
        let replacement_time_in_force = input
            .time_in_force
            .clone()
            .unwrap_or_else(|| current.order.time_in_force.clone());
        let replacement_limit_price = if input.limit_price.is_some() {
            input.limit_price
        } else {
            current.order.limit_price
        };
        let replacement_stop_price = if input.stop_price.is_some() {
            input.stop_price
        } else {
            current.order.stop_price
        };
        let replacement_trail_price = input.trail.or(current.order.trail_price);
        let leg_requests = current
            .order
            .legs
            .as_deref()
            .map(option_leg_requests_from_orders)
            .unwrap_or_default();
        let market_quotes = self
            .resolve_market_quotes(&requested_symbols(
                if current.order.order_class == OrderClass::Mleg {
                    None
                } else if current.order.symbol.is_empty() {
                    None
                } else {
                    Some(current.order.symbol.as_str())
                },
                if leg_requests.is_empty() {
                    None
                } else {
                    Some(leg_requests.as_slice())
                },
            ))
            .await;
        let replacement_legs = if current.order.order_class == OrderClass::Mleg {
            Some(build_leg_orders_from_requests(
                &leg_requests,
                replacement_qty,
                current.order.r#type.clone(),
                replacement_time_in_force.clone(),
                &now,
                current.order.legs.as_deref(),
            ))
        } else {
            None
        };

        let mut replacement = build_order(NewOrderSpec {
            id: replacement_order_id.clone(),
            client_order_id: replacement_client_order_id.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
            submitted_at: now.clone(),
            expires_at: expires_at_for(replacement_time_in_force.clone()),
            asset_id: current.order.asset_id.clone(),
            symbol: current.order.symbol.clone(),
            asset_class: current.order.asset_class.clone(),
            notional: current.order.notional,
            qty: replacement_qty,
            order_class: current.order.order_class.clone(),
            order_type: current.order.r#type.clone(),
            side: current.order.side.clone(),
            position_intent: current.order.position_intent.clone(),
            time_in_force: replacement_time_in_force,
            limit_price: replacement_limit_price,
            stop_price: replacement_stop_price,
            extended_hours: current.order.extended_hours,
            trail_percent: current.order.trail_percent,
            trail_price: replacement_trail_price,
            ratio_qty: current.order.ratio_qty,
            legs: replacement_legs,
            replaces: Some(current.order.id.clone()),
            take_profit: current.order.take_profit.clone(),
            stop_loss: current.order.stop_loss.clone(),
        });
        apply_fill_rules(&mut replacement, &current.request_side, &market_quotes);

        let mut inner = self.inner.write();
        let request_side = {
            let current = inner.orders.get_mut(order_id).ok_or_else(|| {
                OrdersStateError::NotFound(format!("order {order_id} was not found"))
            })?;
            let request_side = current.request_side.clone();
            mark_order_replaced(&mut current.order, &replacement, &now);
            request_side
        };
        inner
            .client_order_ids
            .insert(replacement_client_order_id, replacement_order_id.clone());
        inner.orders.insert(
            replacement_order_id,
            StoredOrder {
                order: replacement.clone(),
                request_side,
            },
        );

        Ok(replacement)
    }

    pub fn cancel_order(&self, order_id: &str) -> Result<(), OrdersStateError> {
        let mut inner = self.inner.write();
        let stored = inner
            .orders
            .get_mut(order_id)
            .ok_or_else(|| OrdersStateError::NotFound(format!("order {order_id} was not found")))?;
        if is_terminal_status(&stored.order.status) {
            return Err(OrdersStateError::Conflict(format!(
                "order {order_id} is already terminal"
            )));
        }

        mark_order_canceled(&mut stored.order);
        Ok(())
    }

    pub fn cancel_all_orders(&self) -> Vec<CancelAllOrderResult> {
        let mut inner = self.inner.write();
        let mut order_ids = inner
            .orders
            .iter()
            .filter_map(|(order_id, stored)| {
                if is_terminal_status(&stored.order.status) {
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
                let stored = inner.orders.get_mut(&order_id)?;
                mark_order_canceled(&mut stored.order);
                Some(build_cancel_all_result(
                    order_id,
                    200,
                    Some(stored.order.clone()),
                ))
            })
            .collect()
    }

    pub fn market_snapshot(&self) -> OrdersMarketSnapshot {
        self.inner.read().market_snapshot.clone()
    }

    async fn resolve_market_quotes(
        &self,
        symbols: &[String],
    ) -> HashMap<String, InstrumentSnapshot> {
        let mut quotes = HashMap::new();

        if let Some(data_client) = self.data_client.as_ref() {
            let stock_symbols = symbols
                .iter()
                .filter(|symbol| !looks_like_option_symbol(symbol))
                .cloned()
                .collect::<Vec<_>>();
            for symbol in stock_symbols {
                if let Ok(snapshot) = live_stock_snapshot(data_client, &symbol).await {
                    quotes.insert(symbol, snapshot);
                }
            }

            let option_symbols = symbols
                .iter()
                .filter(|symbol| looks_like_option_symbol(symbol))
                .cloned()
                .collect::<Vec<_>>();
            if !option_symbols.is_empty() {
                if let Ok(response) = data_client
                    .options()
                    .snapshots(SnapshotsRequest {
                        symbols: option_symbols.clone(),
                        ..SnapshotsRequest::default()
                    })
                    .await
                {
                    for (symbol, snapshot) in response.snapshots {
                        if let Some(instrument) = live_option_snapshot(&snapshot) {
                            quotes.insert(symbol, instrument);
                        }
                    }
                }
            }
        }

        let fallback = self.inner.read().market_snapshot.clone();
        for symbol in symbols {
            quotes
                .entry(symbol.clone())
                .or_insert_with(|| fallback.instrument(symbol));
        }

        quotes
    }
}

fn apply_fill_rules(
    order: &mut Order,
    request_side: &OrderSide,
    market_quotes: &HashMap<String, InstrumentSnapshot>,
) {
    let fill_price = if order.order_class == OrderClass::Mleg {
        mleg_mid_price(order, request_side, market_quotes)
            .and_then(|mid| fill_price_from_mid(order, request_side, mid))
    } else {
        let instrument = market_quotes.get(&order.symbol).cloned();
        instrument.and_then(|instrument| {
            fill_price_from_mid(
                order,
                request_side,
                mid_price(instrument.bid, instrument.ask),
            )
        })
    };

    if let Some(fill_price) = fill_price {
        order.status = OrderStatus::Filled;
        order.filled_qty = order.qty.unwrap_or(Decimal::ZERO);
        order.filled_avg_price = Some(fill_price);
        let now = now_string();
        order.filled_at = Some(now.clone());
        order.updated_at = now;
        order.canceled_at = None;
        sync_nested_legs(order, market_quotes, Some(fill_price), OrderStatus::Filled);
        return;
    }

    order.status = OrderStatus::Accepted;
    order.filled_qty = Decimal::ZERO;
    order.filled_avg_price = None;
    order.filled_at = None;
    sync_nested_legs(order, market_quotes, None, OrderStatus::Accepted);
}

fn fill_price_from_mid(order: &Order, request_side: &OrderSide, mid: Decimal) -> Option<Decimal> {
    match order.r#type {
        OrderType::Market => Some(mid),
        OrderType::Limit => match request_side {
            OrderSide::Buy | OrderSide::Unspecified => order
                .limit_price
                .filter(|limit_price| *limit_price >= mid)
                .map(|_| mid),
            OrderSide::Sell => order
                .limit_price
                .filter(|limit_price| *limit_price <= mid)
                .map(|_| mid),
            _ => None,
        },
        _ => None,
    }
}

fn mleg_mid_price(
    order: &Order,
    request_side: &OrderSide,
    market_quotes: &HashMap<String, InstrumentSnapshot>,
) -> Option<Decimal> {
    let raw_total = order
        .legs
        .as_ref()?
        .iter()
        .try_fold(Decimal::ZERO, |total, leg| {
            let instrument = market_quotes.get(&leg.symbol)?;
            let leg_mid = mid_price(instrument.bid, instrument.ask);
            let ratio_qty = Decimal::from(leg.ratio_qty.unwrap_or(1));
            let contribution = match leg.side {
                OrderSide::Buy => leg_mid * ratio_qty,
                OrderSide::Sell => -(leg_mid * ratio_qty),
                OrderSide::Unspecified => return None,
                _ => return None,
            };
            Some(total + contribution)
        })?;

    let normalized_total = match request_side {
        OrderSide::Buy | OrderSide::Unspecified => raw_total,
        OrderSide::Sell => -raw_total,
        _ => return None,
    };

    Some(normalized_total.round_dp(2))
}

fn sync_nested_legs(
    order: &mut Order,
    market_quotes: &HashMap<String, InstrumentSnapshot>,
    fill_price: Option<Decimal>,
    status: OrderStatus,
) {
    let Some(legs) = order.legs.as_mut() else {
        return;
    };

    let now = now_string();
    for leg in legs {
        leg.updated_at = now.clone();
        leg.status = status.clone();
        match fill_price {
            Some(_) => {
                let leg_mid = market_quotes
                    .get(&leg.symbol)
                    .map(|instrument| mid_price(instrument.bid, instrument.ask));
                leg.filled_qty = leg.qty.unwrap_or(Decimal::ZERO);
                leg.filled_avg_price = leg_mid;
                leg.filled_at = Some(now.clone());
                leg.canceled_at = None;
            }
            None => {
                leg.filled_qty = Decimal::ZERO;
                leg.filled_avg_price = None;
                leg.filled_at = None;
            }
        }
    }
}

fn mark_order_canceled(order: &mut Order) {
    let now = now_string();
    order.status = OrderStatus::Canceled;
    order.updated_at = now.clone();
    order.canceled_at = Some(now.clone());
    order.filled_at = None;
    order.filled_qty = Decimal::ZERO;
    order.filled_avg_price = None;

    if let Some(legs) = order.legs.as_mut() {
        for leg in legs {
            leg.status = OrderStatus::Canceled;
            leg.updated_at = now.clone();
            leg.canceled_at = Some(now.clone());
            leg.filled_at = None;
            leg.filled_qty = Decimal::ZERO;
            leg.filled_avg_price = None;
        }
    }
}

fn mark_order_replaced(order: &mut Order, replacement: &Order, replaced_at: &str) {
    order.status = OrderStatus::Replaced;
    order.updated_at = replaced_at.to_owned();
    order.replaced_at = Some(replaced_at.to_owned());
    order.replaced_by = Some(replacement.id.clone());

    if let (Some(current_legs), Some(replacement_legs)) =
        (order.legs.as_mut(), replacement.legs.as_ref())
    {
        for (current_leg, replacement_leg) in current_legs.iter_mut().zip(replacement_legs.iter()) {
            current_leg.status = OrderStatus::Replaced;
            current_leg.updated_at = replaced_at.to_owned();
            current_leg.replaced_at = Some(replaced_at.to_owned());
            current_leg.replaced_by = Some(replacement_leg.id.clone());
        }
    }
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

fn requested_symbols(symbol: Option<&str>, legs: Option<&[OptionLegRequest]>) -> Vec<String> {
    let mut symbols = Vec::new();
    if let Some(symbol) = symbol {
        symbols.push(symbol.to_owned());
    }
    if let Some(legs) = legs {
        symbols.extend(legs.iter().map(|leg| leg.symbol.clone()));
    }
    symbols.sort();
    symbols.dedup();
    symbols
}

fn option_leg_requests_from_orders(legs: &[Order]) -> Vec<OptionLegRequest> {
    legs.iter()
        .map(|leg| OptionLegRequest {
            symbol: leg.symbol.clone(),
            ratio_qty: leg.ratio_qty.unwrap_or(1),
            side: Some(leg.side.clone()),
            position_intent: leg.position_intent.clone(),
        })
        .collect()
}

fn build_leg_orders_from_requests(
    legs: &[OptionLegRequest],
    parent_qty: Option<Decimal>,
    order_type: OrderType,
    time_in_force: TimeInForce,
    now: &str,
    previous_legs: Option<&[Order]>,
) -> Vec<Order> {
    legs.iter()
        .enumerate()
        .map(|(index, leg)| {
            let previous_leg = previous_legs.and_then(|legs| legs.get(index));
            let leg_qty = parent_qty.unwrap_or(Decimal::new(1, 0)) * Decimal::from(leg.ratio_qty);
            build_order(NewOrderSpec {
                id: Uuid::new_v4().to_string(),
                client_order_id: Uuid::new_v4().to_string(),
                created_at: now.to_owned(),
                updated_at: now.to_owned(),
                submitted_at: now.to_owned(),
                expires_at: expires_at_for(time_in_force.clone()),
                asset_id: previous_leg
                    .map(|leg| leg.asset_id.clone())
                    .unwrap_or_else(|| Uuid::new_v4().to_string()),
                symbol: leg.symbol.clone(),
                asset_class: "us_option".to_owned(),
                notional: None,
                qty: Some(leg_qty),
                order_class: OrderClass::Mleg,
                order_type: order_type.clone(),
                side: leg.side.clone().unwrap_or(OrderSide::Buy),
                position_intent: leg.position_intent.clone(),
                time_in_force: time_in_force.clone(),
                limit_price: None,
                stop_price: None,
                extended_hours: false,
                trail_percent: None,
                trail_price: None,
                ratio_qty: Some(leg.ratio_qty),
                legs: None,
                replaces: previous_leg.map(|leg| leg.id.clone()),
                take_profit: None,
                stop_loss: None,
            })
        })
        .collect()
}

fn mid_price(bid: Decimal, ask: Decimal) -> Decimal {
    ((bid + ask) / Decimal::new(2, 0)).round_dp(2)
}

async fn live_stock_snapshot(
    data_client: &DataClient,
    symbol: &str,
) -> Result<InstrumentSnapshot, ()> {
    let quote = data_client
        .stocks()
        .latest_quote(LatestQuoteRequest {
            symbol: symbol.to_owned(),
            ..LatestQuoteRequest::default()
        })
        .await
        .map_err(|_| ())?;
    let bid = quote
        .quote
        .bp
        .and_then(decimal_from_market_data)
        .ok_or(())?;
    let ask = quote
        .quote
        .ap
        .or(quote.quote.bp)
        .and_then(decimal_from_market_data)
        .ok_or(())?;
    Ok(InstrumentSnapshot::equity(bid, ask))
}

fn live_option_snapshot(snapshot: &OptionSnapshot) -> Option<InstrumentSnapshot> {
    let latest_trade_price = snapshot
        .latestTrade
        .as_ref()
        .and_then(|trade| trade.p)
        .and_then(decimal_from_market_data);
    let bid = snapshot
        .latestQuote
        .as_ref()
        .and_then(|quote| quote.bp)
        .and_then(decimal_from_market_data)
        .or(latest_trade_price)?;
    let ask = snapshot
        .latestQuote
        .as_ref()
        .and_then(|quote| quote.ap)
        .and_then(decimal_from_market_data)
        .or_else(|| {
            snapshot
                .latestQuote
                .as_ref()
                .and_then(|quote| quote.bp)
                .and_then(decimal_from_market_data)
        })
        .or(latest_trade_price)?;
    Some(InstrumentSnapshot::option(bid, ask))
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

fn decimal_from_market_data(value: f64) -> Option<Decimal> {
    Decimal::from_str(&value.to_string()).ok()
}

fn repo_root_dotenv_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.env")
}

fn normalized_value(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_owned())
}

fn read_dotenv_file(path: &Path) -> HashMap<String, String> {
    let Ok(iter) = dotenvy::from_path_iter(path) else {
        return HashMap::new();
    };

    iter.filter_map(Result::ok)
        .filter_map(|(name, value)| normalized_value(Some(&value)).map(|value| (name, value)))
        .collect()
}

fn select_credential_candidates(
    names: &[&str],
    process_values: &HashMap<String, String>,
    dotenv_values: &HashMap<String, String>,
) -> Option<String> {
    names
        .iter()
        .find_map(|name| normalized_value(process_values.get(*name).map(String::as_str)))
        .or_else(|| {
            names
                .iter()
                .find_map(|name| normalized_value(dotenv_values.get(*name).map(String::as_str)))
        })
}

fn data_client_from_environment() -> Option<DataClient> {
    let process_values = API_KEY_CANDIDATES
        .iter()
        .chain(SECRET_KEY_CANDIDATES.iter())
        .filter_map(|name| {
            normalized_value(std::env::var(name).ok().as_deref())
                .map(|value| ((*name).to_owned(), value))
        })
        .collect::<HashMap<_, _>>();
    let dotenv_values = read_dotenv_file(&repo_root_dotenv_path());

    let api_key =
        select_credential_candidates(&API_KEY_CANDIDATES, &process_values, &dotenv_values)?;
    let secret_key =
        select_credential_candidates(&SECRET_KEY_CANDIDATES, &process_values, &dotenv_values)?;

    DataClient::builder()
        .api_key(api_key)
        .secret_key(secret_key)
        .build()
        .ok()
}

fn now_string() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn expires_at_for(time_in_force: TimeInForce) -> Option<String> {
    if time_in_force == TimeInForce::Day {
        Some(format!("{}T20:15:00Z", Utc::now().date_naive()))
    } else {
        None
    }
}

#[derive(Debug, Clone)]
struct NewOrderSpec {
    id: String,
    client_order_id: String,
    created_at: String,
    updated_at: String,
    submitted_at: String,
    expires_at: Option<String>,
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
    ratio_qty: Option<u32>,
    legs: Option<Vec<Order>>,
    replaces: Option<String>,
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
        "expires_at": spec.expires_at,
        "canceled_at": null,
        "failed_at": null,
        "replaced_at": null,
        "replaced_by": null,
        "replaces": spec.replaces,
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
        "legs": spec.legs,
        "trail_percent": spec.trail_percent,
        "trail_price": spec.trail_price,
        "hwm": null,
        "ratio_qty": spec.ratio_qty,
        "take_profit": spec.take_profit,
        "stop_loss": spec.stop_loss,
        "subtag": null,
        "source": null,
    }))
    .expect("mock order json should deserialize")
}

fn build_cancel_all_result(id: String, status: u16, body: Option<Order>) -> CancelAllOrderResult {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "status": status,
        "body": body,
    }))
    .expect("mock cancel_all result json should deserialize")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state(market_snapshot: OrdersMarketSnapshot) -> OrdersState {
        OrdersState {
            inner: Arc::new(RwLock::new(OrdersStateInner {
                market_snapshot,
                orders: HashMap::new(),
                client_order_ids: HashMap::new(),
            })),
            data_client: None,
        }
    }

    #[tokio::test]
    async fn stock_buy_limit_uses_midpoint_as_fill_threshold() {
        let state = test_state(OrdersMarketSnapshot::default().with_instrument(
            "SPY",
            InstrumentSnapshot::equity(Decimal::new(50000, 2), Decimal::new(50020, 2)),
        ));

        let accepted = state
            .create_order(CreateOrderInput {
                symbol: Some("SPY".to_owned()),
                qty: Some(Decimal::new(1, 0)),
                side: Some(OrderSide::Buy),
                order_type: Some(OrderType::Limit),
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(Decimal::new(50009, 2)),
                ..CreateOrderInput::default()
            })
            .await
            .expect("below-mid buy limit should remain open");
        assert_eq!(accepted.status, OrderStatus::Accepted);

        let filled = state
            .create_order(CreateOrderInput {
                symbol: Some("SPY".to_owned()),
                qty: Some(Decimal::new(1, 0)),
                side: Some(OrderSide::Buy),
                order_type: Some(OrderType::Limit),
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(Decimal::new(50010, 2)),
                ..CreateOrderInput::default()
            })
            .await
            .expect("at-mid buy limit should fill");
        assert_eq!(filled.status, OrderStatus::Filled);
        assert_eq!(filled.filled_avg_price, Some(Decimal::new(50010, 2)));
    }

    #[tokio::test]
    async fn mleg_limit_uses_combo_midpoint_for_fill_decision() {
        let buy_leg = "SPY260417C00550000";
        let sell_leg = "SPY260417C00555000";
        let state = test_state(
            OrdersMarketSnapshot::default()
                .with_instrument(
                    buy_leg,
                    InstrumentSnapshot::option(Decimal::new(120, 2), Decimal::new(130, 2)),
                )
                .with_instrument(
                    sell_leg,
                    InstrumentSnapshot::option(Decimal::new(40, 2), Decimal::new(50, 2)),
                ),
        );
        let legs = vec![
            OptionLegRequest {
                symbol: buy_leg.to_owned(),
                ratio_qty: 1,
                side: Some(OrderSide::Buy),
                position_intent: Some(PositionIntent::BuyToOpen),
            },
            OptionLegRequest {
                symbol: sell_leg.to_owned(),
                ratio_qty: 1,
                side: Some(OrderSide::Sell),
                position_intent: Some(PositionIntent::SellToOpen),
            },
        ];

        let accepted = state
            .create_order(CreateOrderInput {
                qty: Some(Decimal::new(1, 0)),
                side: Some(OrderSide::Buy),
                order_type: Some(OrderType::Limit),
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(Decimal::new(79, 2)),
                order_class: Some(OrderClass::Mleg),
                legs: Some(legs.clone()),
                ..CreateOrderInput::default()
            })
            .await
            .expect("below combo mid should remain open");
        assert_eq!(accepted.status, OrderStatus::Accepted);

        let filled = state
            .create_order(CreateOrderInput {
                qty: Some(Decimal::new(1, 0)),
                side: Some(OrderSide::Buy),
                order_type: Some(OrderType::Limit),
                time_in_force: Some(TimeInForce::Day),
                limit_price: Some(Decimal::new(80, 2)),
                order_class: Some(OrderClass::Mleg),
                legs: Some(legs),
                ..CreateOrderInput::default()
            })
            .await
            .expect("at combo mid should fill");
        assert_eq!(filled.status, OrderStatus::Filled);
        assert_eq!(filled.filled_avg_price, Some(Decimal::new(80, 2)));
        let filled_legs = filled.legs.expect("filled mleg should include child legs");
        assert_eq!(filled_legs.len(), 2);
        assert!(
            filled_legs
                .iter()
                .all(|leg| leg.status == OrderStatus::Filled)
        );
        assert_eq!(filled_legs[0].filled_avg_price, Some(Decimal::new(125, 2)));
        assert_eq!(filled_legs[1].filled_avg_price, Some(Decimal::new(45, 2)));
    }
}
