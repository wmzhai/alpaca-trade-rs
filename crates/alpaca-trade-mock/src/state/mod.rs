use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

mod account;
mod activities;
mod executions;
mod market_data;
mod positions;

use activities::{ActivityEvent, ActivityEventKind};
use positions::{ProjectedPosition, project_position};

use alpaca_data::Client as DataClient;
use alpaca_data::options::{
    Snapshot as OptionSnapshot, SnapshotsRequest as OptionSnapshotsRequest,
};
use alpaca_data::stocks::{Snapshot as StockSnapshot, SnapshotsRequest as StockSnapshotsRequest};
use alpaca_trade::Decimal;
use alpaca_trade::orders::{
    CancelAllOrderResult, OptionLegRequest, Order, OrderClass, OrderSide, OrderStatus, OrderType,
    PositionIntent, StopLoss, TakeProfit, TimeInForce,
};
use chrono::Utc;
use parking_lot::RwLock;
use uuid::Uuid;

pub use account::{AccountProfile, CashLedger};
pub use executions::ExecutionFact;
pub use market_data::{DEFAULT_STOCK_SYMBOL, InstrumentSnapshot, OrdersMarketSnapshot, mid_price};
pub use positions::{InstrumentPosition, OpenLot, PositionBook, PositionSide};

const API_KEY_CANDIDATES: [&str; 2] = ["ALPACA_TRADE_API_KEY", "APCA_API_KEY_ID"];
const SECRET_KEY_CANDIDATES: [&str; 2] = ["ALPACA_TRADE_SECRET_KEY", "APCA_API_SECRET_KEY"];

#[derive(Debug, Clone, Default)]
pub struct MockTradingState {
    inner: Arc<RwLock<HashMap<String, VirtualAccountState>>>,
}

#[derive(Debug, Clone)]
pub struct VirtualAccountState {
    pub(crate) account_profile: AccountProfile,
    pub(crate) cash_ledger: CashLedger,
    pub(crate) orders: HashMap<String, StoredOrder>,
    pub(crate) client_order_ids: HashMap<String, String>,
    pub(crate) executions: Vec<ExecutionFact>,
    pub(crate) positions: PositionBook,
    pub(crate) activities: Vec<ActivityEvent>,
    pub(crate) sequence_clock: u64,
}

impl MockTradingState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn account_count(&self) -> usize {
        self.inner.read().len()
    }

    pub fn ensure_account(&self, api_key: &str) -> VirtualAccountState {
        let mut inner = self.inner.write();
        inner
            .entry(api_key.to_owned())
            .or_insert_with(|| VirtualAccountState::new(api_key))
            .clone()
    }
}

impl VirtualAccountState {
    fn new(api_key: &str) -> Self {
        Self {
            account_profile: AccountProfile::new(api_key),
            cash_ledger: CashLedger::seeded_default(),
            orders: HashMap::new(),
            client_order_ids: HashMap::new(),
            executions: Vec::new(),
            positions: PositionBook::default(),
            activities: Vec::new(),
            sequence_clock: 0,
        }
    }

    fn next_sequence(&mut self) -> u64 {
        self.sequence_clock += 1;
        self.sequence_clock
    }

    pub fn account_profile(&self) -> &AccountProfile {
        &self.account_profile
    }

    pub fn cash_ledger(&self) -> &CashLedger {
        &self.cash_ledger
    }

    pub fn execution_count(&self) -> usize {
        self.executions.len()
    }

    pub fn activity_count(&self) -> usize {
        self.activities.len()
    }

    pub fn positions(&self) -> &PositionBook {
        &self.positions
    }
}

#[derive(Clone)]
pub struct OrdersState {
    trading_state: MockTradingState,
    api_key: String,
    market_snapshot: OrdersMarketSnapshot,
    data_client: Option<DataClient>,
}

#[derive(Debug, Clone)]
pub(crate) struct StoredOrder {
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
    MarketDataUnavailable(String),
}

impl OrdersState {
    pub fn new(
        trading_state: MockTradingState,
        api_key: impl Into<String>,
        market_snapshot: OrdersMarketSnapshot,
    ) -> Self {
        Self {
            trading_state,
            api_key: api_key.into(),
            market_snapshot,
            data_client: data_client_from_environment(),
        }
    }

    pub fn account_snapshot(&self) -> VirtualAccountState {
        self.trading_state.ensure_account(&self.api_key)
    }

    pub fn project_account(&self) -> alpaca_trade::account::Account {
        let account = self.account_snapshot();
        account::project_account(&account)
    }

    pub(crate) async fn list_positions(&self) -> Result<Vec<ProjectedPosition>, OrdersStateError> {
        let account = self.account_snapshot();
        let open_positions = account.positions().list_open_positions();
        let market_quotes = self
            .resolve_market_quotes(
                &open_positions
                    .iter()
                    .map(|position| position.instrument_identity.symbol.clone())
                    .collect::<Vec<_>>(),
            )
            .await?;

        let mut positions = open_positions
            .iter()
            .map(|position| {
                let market_snapshot = market_quotes
                    .get(&position.instrument_identity.symbol)
                    .ok_or_else(|| {
                        OrdersStateError::MarketDataUnavailable(format!(
                            "mock position valuation for {} requires live market data",
                            position.instrument_identity.symbol
                        ))
                    })?;
                Ok(project_position(position, market_snapshot))
            })
            .collect::<Result<Vec<_>, OrdersStateError>>()?;
        positions.sort_by(|left, right| left.symbol.cmp(&right.symbol));
        Ok(positions)
    }

    pub(crate) async fn get_position(
        &self,
        symbol_or_asset_id: &str,
    ) -> Result<ProjectedPosition, OrdersStateError> {
        let account = self.account_snapshot();
        let position = account
            .positions()
            .find_open_position(symbol_or_asset_id)
            .ok_or_else(|| {
                OrdersStateError::NotFound(format!("position {symbol_or_asset_id} was not found"))
            })?;
        let market_quotes = self
            .resolve_market_quotes(std::slice::from_ref(&position.instrument_identity.symbol))
            .await?;
        let market_snapshot = market_quotes
            .get(&position.instrument_identity.symbol)
            .ok_or_else(|| {
                OrdersStateError::MarketDataUnavailable(format!(
                    "mock position valuation for {} requires live market data",
                    position.instrument_identity.symbol
                ))
            })?;
        Ok(project_position(&position, market_snapshot))
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
        let market_quotes = self
            .resolve_market_quotes(&requested_symbols(
                input.symbol.as_deref(),
                input.legs.as_deref(),
            ))
            .await?;
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
                .ok_or_else(|| {
                    OrdersStateError::MarketDataUnavailable(format!(
                        "mock order creation for {symbol} requires live market data"
                    ))
                })?
                .asset_class
        };
        let effective_asset_id = if order_class == OrderClass::Mleg {
            String::new()
        } else {
            mock_asset_id(&symbol)
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
            order_type,
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

        let mut inner = self.trading_state.inner.write();
        let account = inner
            .entry(self.api_key.clone())
            .or_insert_with(|| VirtualAccountState::new(&self.api_key));
        if account.client_order_ids.contains_key(&client_order_id) {
            return Err(OrdersStateError::Conflict(format!(
                "client_order_id {client_order_id} already exists"
            )));
        }
        account
            .client_order_ids
            .insert(client_order_id, order_id.clone());
        account.orders.insert(
            order_id,
            StoredOrder {
                order: order.clone(),
                request_side: request_side.clone(),
            },
        );
        record_create_effects(account, &order, &request_side);

        Ok(order)
    }

    pub fn list_orders(&self, filter: ListOrdersFilter) -> Vec<Order> {
        let inner = self.trading_state.inner.read();
        let Some(account) = inner.get(&self.api_key) else {
            return Vec::new();
        };
        let symbol_filter = filter.symbols.map(|symbols| {
            symbols
                .into_iter()
                .map(|symbol| symbol.trim().to_owned())
                .filter(|symbol| !symbol.is_empty())
                .collect::<HashSet<_>>()
        });

        let mut orders = account
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
        self.trading_state
            .inner
            .read()
            .get(&self.api_key)
            .and_then(|account| account.orders.get(order_id))
            .map(|stored| stored.order.clone())
    }

    pub fn get_by_client_order_id(&self, client_order_id: &str) -> Option<Order> {
        let inner = self.trading_state.inner.read();
        let account = inner.get(&self.api_key)?;
        let order_id = account.client_order_ids.get(client_order_id)?;
        account
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
            let inner = self.trading_state.inner.read();
            let account = inner.get(&self.api_key).ok_or_else(|| {
                OrdersStateError::NotFound(format!("order {order_id} was not found"))
            })?;
            account.orders.get(order_id).cloned().ok_or_else(|| {
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
            .await?;
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

        let mut inner = self.trading_state.inner.write();
        let account = inner
            .entry(self.api_key.clone())
            .or_insert_with(|| VirtualAccountState::new(&self.api_key));
        if replacement_client_order_id != current.order.client_order_id
            && account
                .client_order_ids
                .contains_key(&replacement_client_order_id)
        {
            return Err(OrdersStateError::Conflict(format!(
                "client_order_id {replacement_client_order_id} already exists"
            )));
        }
        let request_side = {
            let current = account.orders.get_mut(order_id).ok_or_else(|| {
                OrdersStateError::NotFound(format!("order {order_id} was not found"))
            })?;
            if is_terminal_status(&current.order.status) {
                return Err(OrdersStateError::Conflict(format!(
                    "order {order_id} is no longer replaceable"
                )));
            }
            let request_side = current.request_side.clone();
            mark_order_replaced(&mut current.order, &replacement, &now);
            request_side
        };
        account
            .client_order_ids
            .insert(replacement_client_order_id, replacement_order_id.clone());
        account.orders.insert(
            replacement_order_id,
            StoredOrder {
                order: replacement.clone(),
                request_side: request_side.clone(),
            },
        );
        record_replace_effects(account, order_id, &replacement, &request_side);

        Ok(replacement)
    }

    pub fn cancel_order(&self, order_id: &str) -> Result<(), OrdersStateError> {
        let mut inner = self.trading_state.inner.write();
        let account = inner
            .entry(self.api_key.clone())
            .or_insert_with(|| VirtualAccountState::new(&self.api_key));
        let order = {
            let stored = account.orders.get_mut(order_id).ok_or_else(|| {
                OrdersStateError::NotFound(format!("order {order_id} was not found"))
            })?;
            if is_terminal_status(&stored.order.status) {
                return Err(OrdersStateError::Conflict(format!(
                    "order {order_id} is already terminal"
                )));
            }
            mark_order_canceled(&mut stored.order);
            stored.order.clone()
        };
        record_cancel_effects(account, &order);
        Ok(())
    }

    pub fn cancel_all_orders(&self) -> Vec<CancelAllOrderResult> {
        let mut inner = self.trading_state.inner.write();
        let account = inner
            .entry(self.api_key.clone())
            .or_insert_with(|| VirtualAccountState::new(&self.api_key));
        let mut order_ids = account
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

        let mut results = Vec::with_capacity(order_ids.len());
        for order_id in order_ids {
            let order = {
                let stored = match account.orders.get_mut(&order_id) {
                    Some(stored) => stored,
                    None => continue,
                };
                mark_order_canceled(&mut stored.order);
                stored.order.clone()
            };
            record_cancel_effects(account, &order);
            results.push(build_cancel_all_result(order_id, 200, Some(order)));
        }
        results
    }

    pub fn market_snapshot(&self) -> OrdersMarketSnapshot {
        self.market_snapshot.clone()
    }

    async fn resolve_market_quotes(
        &self,
        symbols: &[String],
    ) -> Result<HashMap<String, InstrumentSnapshot>, OrdersStateError> {
        if symbols.is_empty() {
            return Ok(HashMap::new());
        }

        let mut quotes = HashMap::new();

        let data_client = self.data_client.as_ref().ok_or_else(|| {
            OrdersStateError::MarketDataUnavailable(
                "mock orders require live market data credentials via alpaca-data".to_owned(),
            )
        })?;

        let stock_symbols = symbols
            .iter()
            .filter(|symbol| !looks_like_option_symbol(symbol))
            .cloned()
            .collect::<Vec<_>>();
        if !stock_symbols.is_empty() {
            let snapshots = data_client
                .stocks()
                .snapshots(StockSnapshotsRequest {
                    symbols: stock_symbols.clone(),
                    ..StockSnapshotsRequest::default()
                })
                .await
                .map_err(|error| {
                    OrdersStateError::MarketDataUnavailable(format!(
                        "stock snapshots request failed: {error}"
                    ))
                })?;
            for symbol in stock_symbols {
                let snapshot = snapshots.get(&symbol).ok_or_else(|| {
                    OrdersStateError::MarketDataUnavailable(format!(
                        "stock snapshots response did not include {symbol}"
                    ))
                })?;
                let instrument = live_stock_snapshot(snapshot).ok_or_else(|| {
                    OrdersStateError::MarketDataUnavailable(format!(
                        "stock snapshot for {symbol} is missing a usable live bid/ask"
                    ))
                })?;
                quotes.insert(symbol, instrument);
            }
        }

        let option_symbols = symbols
            .iter()
            .filter(|symbol| looks_like_option_symbol(symbol))
            .cloned()
            .collect::<Vec<_>>();
        if !option_symbols.is_empty() {
            let response = data_client
                .options()
                .snapshots(OptionSnapshotsRequest {
                    symbols: option_symbols.clone(),
                    ..OptionSnapshotsRequest::default()
                })
                .await
                .map_err(|error| {
                    OrdersStateError::MarketDataUnavailable(format!(
                        "options snapshots request failed: {error}"
                    ))
                })?;
            for symbol in option_symbols {
                let snapshot = response.snapshots.get(&symbol).ok_or_else(|| {
                    OrdersStateError::MarketDataUnavailable(format!(
                        "options snapshots response did not include {symbol}"
                    ))
                })?;
                let instrument = live_option_snapshot(snapshot).ok_or_else(|| {
                    OrdersStateError::MarketDataUnavailable(format!(
                        "options snapshot for {symbol} is missing a usable live bid/ask"
                    ))
                })?;
                quotes.insert(symbol, instrument);
            }
        }

        Ok(quotes)
    }
}

fn record_create_effects(
    account: &mut VirtualAccountState,
    order: &Order,
    request_side: &OrderSide,
) {
    if order.status == OrderStatus::Filled {
        record_filled_effects(account, order, request_side, None);
        return;
    }

    let sequence = account.next_sequence();
    account.activities.push(ActivityEvent::new(
        sequence,
        ActivityEventKind::New,
        order.id.clone(),
        order.client_order_id.clone(),
        None,
        order.status.clone(),
        order.symbol.clone(),
        order.asset_class.clone(),
        order.updated_at.clone(),
        Decimal::ZERO,
    ));
}

fn record_replace_effects(
    account: &mut VirtualAccountState,
    replaced_order_id: &str,
    replacement: &Order,
    request_side: &OrderSide,
) {
    let sequence = account.next_sequence();
    account.activities.push(ActivityEvent::new(
        sequence,
        ActivityEventKind::Replaced,
        replacement.id.clone(),
        replacement.client_order_id.clone(),
        Some(replaced_order_id.to_owned()),
        replacement.status.clone(),
        replacement.symbol.clone(),
        replacement.asset_class.clone(),
        replacement.updated_at.clone(),
        Decimal::ZERO,
    ));

    if replacement.status == OrderStatus::Filled {
        record_filled_effects(account, replacement, request_side, Some(replaced_order_id));
    }
}

fn record_cancel_effects(account: &mut VirtualAccountState, order: &Order) {
    let sequence = account.next_sequence();
    account.activities.push(ActivityEvent::new(
        sequence,
        ActivityEventKind::Canceled,
        order.id.clone(),
        order.client_order_id.clone(),
        None,
        order.status.clone(),
        order.symbol.clone(),
        order.asset_class.clone(),
        order.updated_at.clone(),
        Decimal::ZERO,
    ));
}

fn record_filled_effects(
    account: &mut VirtualAccountState,
    order: &Order,
    request_side: &OrderSide,
    related_order_id: Option<&str>,
) {
    let cash_delta = cash_delta_for_filled_order(order, request_side);
    account.cash_ledger.apply_delta(cash_delta);
    let execution_facts = execution_facts_from_order(account, order, request_side);
    for execution in &execution_facts {
        account.positions.apply_execution(execution);
    }
    account.executions.extend(execution_facts);
    let sequence = account.next_sequence();
    account.activities.push(ActivityEvent::new(
        sequence,
        ActivityEventKind::Filled,
        order.id.clone(),
        order.client_order_id.clone(),
        related_order_id.map(str::to_owned),
        order.status.clone(),
        order.symbol.clone(),
        order.asset_class.clone(),
        order
            .filled_at
            .clone()
            .unwrap_or_else(|| order.updated_at.clone()),
        cash_delta,
    ));
}

fn cash_delta_for_filled_order(order: &Order, request_side: &OrderSide) -> Decimal {
    let notional = order.filled_avg_price.unwrap_or(Decimal::ZERO) * order.filled_qty;
    match request_side {
        OrderSide::Buy | OrderSide::Unspecified => -notional,
        OrderSide::Sell => notional,
        _ => Decimal::ZERO,
    }
}

fn execution_facts_from_order(
    account: &mut VirtualAccountState,
    order: &Order,
    request_side: &OrderSide,
) -> Vec<ExecutionFact> {
    let filled_at = order
        .filled_at
        .clone()
        .unwrap_or_else(|| order.updated_at.clone());

    if order.order_class == OrderClass::Mleg {
        return order
            .legs
            .as_ref()
            .map(|legs| {
                legs.iter()
                    .map(|leg| {
                        ExecutionFact::new(
                            account.next_sequence(),
                            leg.id.clone(),
                            Some(order.id.clone()),
                            leg.asset_id.clone(),
                            leg.symbol.clone(),
                            leg.asset_class.clone(),
                            leg.side.clone(),
                            leg.position_intent.clone(),
                            leg.filled_qty,
                            leg.filled_avg_price.unwrap_or(Decimal::ZERO),
                            leg.filled_at.clone().unwrap_or_else(|| filled_at.clone()),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
    }

    vec![ExecutionFact::new(
        account.next_sequence(),
        order.id.clone(),
        None,
        order.asset_id.clone(),
        order.symbol.clone(),
        order.asset_class.clone(),
        request_side.clone(),
        order.position_intent.clone(),
        order.filled_qty,
        order.filled_avg_price.unwrap_or(Decimal::ZERO),
        filled_at,
    )]
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

    order.status = OrderStatus::New;
    order.filled_qty = Decimal::ZERO;
    order.filled_avg_price = None;
    order.filled_at = None;
    sync_nested_legs(order, market_quotes, None, OrderStatus::New);
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

fn mock_asset_id(symbol: &str) -> String {
    format!("mock-asset-{symbol}")
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
                    .unwrap_or_else(|| mock_asset_id(&leg.symbol)),
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

fn live_stock_snapshot(snapshot: &StockSnapshot) -> Option<InstrumentSnapshot> {
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
        .and_then(|quote| quote.ap.or(quote.bp))
        .and_then(decimal_from_market_data)
        .or(latest_trade_price)?;
    let previous_close = snapshot
        .prevDailyBar
        .as_ref()
        .and_then(|bar| bar.c)
        .and_then(decimal_from_market_data)
        .or_else(|| {
            snapshot
                .dailyBar
                .as_ref()
                .and_then(|bar| bar.c)
                .and_then(decimal_from_market_data)
        })
        .or(latest_trade_price);
    Some(InstrumentSnapshot {
        asset_class: "us_equity".to_owned(),
        bid,
        ask,
        previous_close,
    })
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
    let previous_close = snapshot
        .prevDailyBar
        .as_ref()
        .and_then(|bar| bar.c)
        .and_then(decimal_from_market_data)
        .or_else(|| {
            snapshot
                .dailyBar
                .as_ref()
                .and_then(|bar| bar.c)
                .and_then(decimal_from_market_data)
        })
        .or(latest_trade_price);
    Some(InstrumentSnapshot {
        asset_class: "us_option".to_owned(),
        bid,
        ask,
        previous_close,
    })
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
        "status": OrderStatus::New,
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
    use tokio::sync::Mutex;

    static LIVE_MARKET_DATA_TEST_MUTEX: Mutex<()> = Mutex::const_new(());

    fn test_state(market_snapshot: OrdersMarketSnapshot) -> OrdersState {
        OrdersState {
            trading_state: MockTradingState::new(),
            api_key: "mock-unit-test".to_owned(),
            market_snapshot,
            data_client: None,
        }
    }

    fn live_test_data_client() -> DataClient {
        data_client_from_environment()
            .expect("mock market-data-dependent tests require Alpaca credentials for alpaca-data")
    }

    async fn live_stock_mid(symbol: &str) -> Decimal {
        let snapshot = live_stock_snapshot(&live_test_data_client(), symbol)
            .await
            .expect("latest live stock quote should be available for mock midpoint tests");
        mid_price(snapshot.bid, snapshot.ask)
    }

    #[derive(Debug, Clone)]
    struct LiveOptionQuote {
        symbol: String,
        expiration_date: String,
        strike_price: Decimal,
        bid: Decimal,
        ask: Decimal,
    }

    #[derive(Debug, Clone)]
    struct LiveCallSpread {
        buy_symbol: String,
        sell_symbol: String,
        buy_mid: Decimal,
        sell_mid: Decimal,
    }

    fn parse_option_symbol_for_test(symbol: &str) -> Option<(String, Decimal)> {
        let root_len = symbol.len().checked_sub(15)?;
        let expiration_date = symbol.get(root_len..root_len + 6)?.to_owned();
        if symbol.get(root_len + 6..root_len + 7)? != "C" {
            return None;
        }
        let strike = symbol.get(root_len + 7..)?;
        let strike_price = Decimal::from_str(strike).ok()? / Decimal::new(1000, 0);
        Some((expiration_date, strike_price))
    }

    fn live_option_quote(symbol: String, snapshot: OptionSnapshot) -> Option<LiveOptionQuote> {
        let (expiration_date, strike_price) = parse_option_symbol_for_test(&symbol)?;
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
        if bid <= Decimal::ZERO || ask <= Decimal::ZERO || ask < bid {
            return None;
        }
        Some(LiveOptionQuote {
            symbol,
            expiration_date,
            strike_price,
            bid,
            ask,
        })
    }

    async fn live_call_spread() -> LiveCallSpread {
        let data_client = live_test_data_client();
        let today = Utc::now().date_naive();
        let latest_expiration = today
            .checked_add_days(chrono::Days::new(21))
            .expect("option lookahead window should be valid");
        let response = data_client
            .options()
            .chain_all(alpaca_data::options::ChainRequest {
                underlying_symbol: DEFAULT_STOCK_SYMBOL.to_owned(),
                expiration_date_gte: Some(today.to_string()),
                expiration_date_lte: Some(latest_expiration.to_string()),
                limit: Some(1_000),
                ..alpaca_data::options::ChainRequest::default()
            })
            .await
            .expect("optionchain request should succeed for mock midpoint tests");

        let mut quotes = response
            .snapshots
            .into_iter()
            .filter_map(|(symbol, snapshot)| live_option_quote(symbol, snapshot))
            .collect::<Vec<_>>();
        quotes.sort_by(|left, right| {
            left.expiration_date
                .cmp(&right.expiration_date)
                .then_with(|| left.strike_price.cmp(&right.strike_price))
        });

        for (index, lower) in quotes.iter().enumerate() {
            for higher in quotes.iter().skip(index + 1) {
                if higher.expiration_date != lower.expiration_date
                    || higher.strike_price <= lower.strike_price
                {
                    continue;
                }

                let buy_mid = mid_price(lower.bid, lower.ask);
                let sell_mid = mid_price(higher.bid, higher.ask);
                if buy_mid <= sell_mid {
                    continue;
                }

                return LiveCallSpread {
                    buy_symbol: lower.symbol.clone(),
                    sell_symbol: higher.symbol.clone(),
                    buy_mid,
                    sell_mid,
                };
            }
        }

        panic!("optionchain did not return a quoted call spread with positive net debit");
    }

    #[tokio::test]
    async fn stock_buy_limit_uses_midpoint_as_fill_threshold() {
        let _guard = LIVE_MARKET_DATA_TEST_MUTEX.lock().await;
        let stock_mid = live_stock_mid(DEFAULT_STOCK_SYMBOL).await;
        let below_mid = (stock_mid - Decimal::new(1, 2)).round_dp(2);
        let at_mid = stock_mid.round_dp(2);
        let instrument = InstrumentSnapshot::equity(stock_mid, stock_mid);
        let market_quotes = HashMap::from([(DEFAULT_STOCK_SYMBOL.to_owned(), instrument)]);

        let now = now_string();
        let mut accepted = build_order(NewOrderSpec {
            id: Uuid::new_v4().to_string(),
            client_order_id: Uuid::new_v4().to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            submitted_at: now.clone(),
            expires_at: expires_at_for(TimeInForce::Day),
            asset_id: Uuid::new_v4().to_string(),
            symbol: DEFAULT_STOCK_SYMBOL.to_owned(),
            asset_class: "us_equity".to_owned(),
            notional: None,
            qty: Some(Decimal::new(1, 0)),
            order_class: OrderClass::Simple,
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            position_intent: None,
            time_in_force: TimeInForce::Day,
            limit_price: Some(below_mid),
            stop_price: None,
            extended_hours: false,
            trail_percent: None,
            trail_price: None,
            ratio_qty: None,
            legs: None,
            replaces: None,
            take_profit: None,
            stop_loss: None,
        });
        apply_fill_rules(&mut accepted, &OrderSide::Buy, &market_quotes);
        assert_eq!(accepted.status, OrderStatus::New);

        let mut filled = build_order(NewOrderSpec {
            id: Uuid::new_v4().to_string(),
            client_order_id: Uuid::new_v4().to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            submitted_at: now,
            expires_at: expires_at_for(TimeInForce::Day),
            asset_id: Uuid::new_v4().to_string(),
            symbol: DEFAULT_STOCK_SYMBOL.to_owned(),
            asset_class: "us_equity".to_owned(),
            notional: None,
            qty: Some(Decimal::new(1, 0)),
            order_class: OrderClass::Simple,
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            position_intent: None,
            time_in_force: TimeInForce::Day,
            limit_price: Some(at_mid),
            stop_price: None,
            extended_hours: false,
            trail_percent: None,
            trail_price: None,
            ratio_qty: None,
            legs: None,
            replaces: None,
            take_profit: None,
            stop_loss: None,
        });
        apply_fill_rules(&mut filled, &OrderSide::Buy, &market_quotes);
        assert_eq!(filled.status, OrderStatus::Filled);
        assert_eq!(filled.filled_avg_price, Some(at_mid));
    }

    #[tokio::test]
    async fn mleg_limit_uses_combo_midpoint_for_fill_decision() {
        let _guard = LIVE_MARKET_DATA_TEST_MUTEX.lock().await;
        let spread = live_call_spread().await;
        let combo_mid = (spread.buy_mid - spread.sell_mid).round_dp(2);
        let below_combo_mid = (combo_mid - Decimal::new(1, 2)).round_dp(2);
        let legs = vec![
            OptionLegRequest {
                symbol: spread.buy_symbol.clone(),
                ratio_qty: 1,
                side: Some(OrderSide::Buy),
                position_intent: Some(PositionIntent::BuyToOpen),
            },
            OptionLegRequest {
                symbol: spread.sell_symbol.clone(),
                ratio_qty: 1,
                side: Some(OrderSide::Sell),
                position_intent: Some(PositionIntent::SellToOpen),
            },
        ];
        let market_quotes = HashMap::from([
            (
                spread.buy_symbol.clone(),
                InstrumentSnapshot::option(spread.buy_mid, spread.buy_mid),
            ),
            (
                spread.sell_symbol.clone(),
                InstrumentSnapshot::option(spread.sell_mid, spread.sell_mid),
            ),
        ]);
        let now = now_string();
        let accepted_legs = build_leg_orders_from_requests(
            &legs,
            Some(Decimal::new(1, 0)),
            OrderType::Limit,
            TimeInForce::Day,
            &now,
            None,
        );
        let mut accepted = build_order(NewOrderSpec {
            id: Uuid::new_v4().to_string(),
            client_order_id: Uuid::new_v4().to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            submitted_at: now.clone(),
            expires_at: expires_at_for(TimeInForce::Day),
            asset_id: Uuid::new_v4().to_string(),
            symbol: String::new(),
            asset_class: String::new(),
            notional: None,
            qty: Some(Decimal::new(1, 0)),
            order_class: OrderClass::Mleg,
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            position_intent: None,
            time_in_force: TimeInForce::Day,
            limit_price: Some(below_combo_mid),
            stop_price: None,
            extended_hours: false,
            trail_percent: None,
            trail_price: None,
            ratio_qty: None,
            legs: Some(accepted_legs),
            replaces: None,
            take_profit: None,
            stop_loss: None,
        });
        apply_fill_rules(&mut accepted, &OrderSide::Buy, &market_quotes);
        assert_eq!(accepted.status, OrderStatus::New);

        let filled_legs = build_leg_orders_from_requests(
            &legs,
            Some(Decimal::new(1, 0)),
            OrderType::Limit,
            TimeInForce::Day,
            &now,
            None,
        );
        let mut filled = build_order(NewOrderSpec {
            id: Uuid::new_v4().to_string(),
            client_order_id: Uuid::new_v4().to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
            submitted_at: now,
            expires_at: expires_at_for(TimeInForce::Day),
            asset_id: Uuid::new_v4().to_string(),
            symbol: String::new(),
            asset_class: String::new(),
            notional: None,
            qty: Some(Decimal::new(1, 0)),
            order_class: OrderClass::Mleg,
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            position_intent: None,
            time_in_force: TimeInForce::Day,
            limit_price: Some(combo_mid),
            stop_price: None,
            extended_hours: false,
            trail_percent: None,
            trail_price: None,
            ratio_qty: None,
            legs: Some(filled_legs),
            replaces: None,
            take_profit: None,
            stop_loss: None,
        });
        apply_fill_rules(&mut filled, &OrderSide::Buy, &market_quotes);
        assert_eq!(filled.status, OrderStatus::Filled);
        assert_eq!(filled.filled_avg_price, Some(combo_mid));
        let filled_legs = filled.legs.expect("filled mleg should include child legs");
        assert_eq!(filled_legs.len(), 2);
        assert!(
            filled_legs
                .iter()
                .all(|leg| leg.status == OrderStatus::Filled)
        );
        assert_eq!(filled_legs[0].filled_avg_price, Some(spread.buy_mid));
        assert_eq!(filled_legs[1].filled_avg_price, Some(spread.sell_mid));
    }

    #[tokio::test]
    async fn create_order_fails_without_live_market_data() {
        let state = test_state(OrdersMarketSnapshot::default());

        let error = state
            .create_order(CreateOrderInput {
                symbol: Some("SPY".to_owned()),
                qty: Some(Decimal::new(1, 0)),
                side: Some(OrderSide::Buy),
                order_type: Some(OrderType::Market),
                time_in_force: Some(TimeInForce::Day),
                ..CreateOrderInput::default()
            })
            .await
            .expect_err("mock orders must fail when live market data is unavailable");

        assert!(matches!(error, OrdersStateError::MarketDataUnavailable(_)));
    }

    #[tokio::test]
    async fn create_order_quote_failure_leaves_state_untouched() {
        let state = test_state(OrdersMarketSnapshot::default());

        let error = state
            .create_order(CreateOrderInput {
                symbol: Some("SPY".to_owned()),
                qty: Some(Decimal::new(1, 0)),
                side: Some(OrderSide::Buy),
                order_type: Some(OrderType::Market),
                time_in_force: Some(TimeInForce::Day),
                client_order_id: Some("quote-failure-no-write".to_owned()),
                ..CreateOrderInput::default()
            })
            .await
            .expect_err("mock orders must fail when live market data is unavailable");

        let account = state.account_snapshot();
        assert!(matches!(error, OrdersStateError::MarketDataUnavailable(_)));
        assert!(account.orders.is_empty());
        assert!(account.client_order_ids.is_empty());
        assert_eq!(
            account.cash_ledger.cash_balance(),
            Decimal::new(1_000_000, 0)
        );
        assert!(account.executions.is_empty());
        assert!(account.activities.is_empty());
    }

    #[tokio::test]
    async fn replace_order_quote_failure_leaves_original_order_untouched() {
        let state = test_state(OrdersMarketSnapshot::default());
        let now = now_string();
        let existing = build_order(NewOrderSpec {
            id: "replace-source-order".to_owned(),
            client_order_id: "replace-source-client".to_owned(),
            created_at: now.clone(),
            updated_at: now.clone(),
            submitted_at: now,
            expires_at: expires_at_for(TimeInForce::Day),
            asset_id: Uuid::new_v4().to_string(),
            symbol: DEFAULT_STOCK_SYMBOL.to_owned(),
            asset_class: "us_equity".to_owned(),
            notional: None,
            qty: Some(Decimal::new(1, 0)),
            order_class: OrderClass::Simple,
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            position_intent: None,
            time_in_force: TimeInForce::Day,
            limit_price: Some(Decimal::new(1, 0)),
            stop_price: None,
            extended_hours: false,
            trail_percent: None,
            trail_price: None,
            ratio_qty: None,
            legs: None,
            replaces: None,
            take_profit: None,
            stop_loss: None,
        });
        {
            let mut inner = state.trading_state.inner.write();
            let account = inner
                .entry(state.api_key.clone())
                .or_insert_with(|| VirtualAccountState::new(&state.api_key));
            account
                .client_order_ids
                .insert(existing.client_order_id.clone(), existing.id.clone());
            account.orders.insert(
                existing.id.clone(),
                StoredOrder {
                    order: existing.clone(),
                    request_side: OrderSide::Buy,
                },
            );
            account.activities.push(ActivityEvent::new(
                1,
                ActivityEventKind::New,
                existing.id.clone(),
                existing.client_order_id.clone(),
                None,
                existing.status.clone(),
                existing.symbol.clone(),
                existing.asset_class.clone(),
                existing.updated_at.clone(),
                Decimal::ZERO,
            ));
            account.sequence_clock = 1;
        }

        let error = state
            .replace_order(
                &existing.id,
                ReplaceOrderInput {
                    limit_price: Some(Decimal::new(2, 0)),
                    ..ReplaceOrderInput::default()
                },
            )
            .await
            .expect_err("replace should fail when live quote lookup fails");

        let account = state.account_snapshot();
        let stored = account
            .orders
            .get(&existing.id)
            .expect("original order should still exist");
        assert!(matches!(error, OrdersStateError::MarketDataUnavailable(_)));
        assert_eq!(stored.order.status, OrderStatus::New);
        assert_eq!(stored.order.limit_price, existing.limit_price);
        assert!(stored.order.replaced_by.is_none());
        assert_eq!(account.orders.len(), 1);
        assert_eq!(
            account
                .client_order_ids
                .get(&existing.client_order_id)
                .map(String::as_str),
            Some(existing.id.as_str())
        );
        assert_eq!(
            account.cash_ledger.cash_balance(),
            Decimal::new(1_000_000, 0)
        );
        assert!(account.executions.is_empty());
        assert_eq!(account.activities.len(), 1);
    }
}
