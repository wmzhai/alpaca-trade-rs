use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use alpaca_data::stocks::LatestQuoteRequest;
use alpaca_data::{Client as DataClient, options};
use alpaca_trade::Client;
use alpaca_trade::calendar::ListRequest as CalendarListRequest;
use alpaca_trade::options_contracts::{
    ContractStatus, ContractType, ListRequest as OptionsContractsListRequest, OptionContract,
};
use alpaca_trade::orders::{OptionLegRequest, Order, OrderSide, OrderStatus, PositionIntent};
use alpaca_trade::{Decimal, Error};
use alpaca_trade_mock::{
    InstrumentSnapshot, OrdersMarketSnapshot, spawn_test_server_with_market_snapshot,
};
use chrono::{Days, Utc};
use tokio::sync::{Mutex, MutexGuard};
use tokio::time::sleep;

use super::Credentials;

const DEDICATED_ORDERS_TEST_ACCOUNT_ENV: &str = "ALPACA_TRADE_ORDERS_TEST_ACCOUNT";
const MIN_PRICE: Decimal = Decimal::ZERO;
const OPTION_DISCOVERY_LOOKAHEAD_DAYS: u64 = 21;
const OPTION_DISCOVERY_STRIKE_WINDOW_BPS: i64 = 750;
const OPTION_SNAPSHOT_BATCH_SIZE: usize = 100;
static CLIENT_ORDER_COUNTER: AtomicU64 = AtomicU64::new(1);
static ORDERS_MUTATING_TEST_MUTEX: Mutex<()> = Mutex::const_new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OrdersRuntimeMode {
    Paper,
    Mock,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CleanupTracker {
    pub(crate) created_order_ids: Vec<String>,
    pub(crate) created_client_order_ids: Vec<String>,
    pub(crate) allow_cancel_all: bool,
}

pub(crate) struct OrdersTestContext {
    pub(crate) runtime_mode: OrdersRuntimeMode,
    pub(crate) trade_client: Client,
    pub(crate) data_client: Option<DataClient>,
    pub(crate) market_snapshot: Option<OrdersMarketSnapshot>,
    mock_server: Option<alpaca_trade_mock::TestServer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StockPriceContext {
    pub(crate) non_marketable_buy_limit_price: Decimal,
    pub(crate) more_conservative_buy_limit_price: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OptionContractContext {
    pub(crate) symbol: String,
    pub(crate) non_marketable_buy_limit_price: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MultiLegOrderContext {
    pub(crate) underlying_symbol: String,
    pub(crate) legs: Vec<OptionLegRequest>,
    pub(crate) non_marketable_limit_price: Decimal,
    pub(crate) more_conservative_limit_price: Decimal,
    pub(crate) marketable_limit_price: Decimal,
}

#[derive(Debug, Clone, PartialEq)]
struct QuotedOptionContract {
    contract: OptionContract,
    bid: Decimal,
    ask: Decimal,
}

impl CleanupTracker {
    pub(crate) fn new(allow_cancel_all: bool) -> Self {
        Self {
            created_order_ids: Vec::new(),
            created_client_order_ids: Vec::new(),
            allow_cancel_all,
        }
    }

    pub(crate) fn record_order_id(&mut self, order_id: impl Into<String>) {
        self.created_order_ids.push(order_id.into());
    }

    pub(crate) fn record_client_order_id(&mut self, client_order_id: impl Into<String>) {
        self.created_client_order_ids.push(client_order_id.into());
    }
}

impl OrdersTestContext {
    pub(crate) fn next_client_order_id(&self, suite: &str, test_name: &str) -> String {
        let _ = &self.trade_client;
        next_client_order_id(suite, test_name)
    }

    pub(crate) fn can_run_real_cancel_all(&self) -> bool {
        should_run_real_cancel_all(
            self.runtime_mode,
            is_dedicated_orders_test_account_marker_set(),
        )
    }

    pub(crate) fn is_mock(&self) -> bool {
        self.runtime_mode == OrdersRuntimeMode::Mock
    }
}

pub(crate) fn next_client_order_id(suite: &str, test_name: &str) -> String {
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_millis();
    let counter = CLIENT_ORDER_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!(
        "phase7-orders-{}-{}-{timestamp_ms}-{counter}",
        sanitize_client_order_component(suite),
        sanitize_client_order_component(test_name),
    )
}

pub(crate) async fn orders_test_lock() -> MutexGuard<'static, ()> {
    ORDERS_MUTATING_TEST_MUTEX.lock().await
}

pub(crate) fn select_runtime_mode(
    has_credentials: bool,
    market_is_open: bool,
    has_calendar_session: bool,
    dedicated_account_marker: bool,
) -> OrdersRuntimeMode {
    if has_credentials && market_is_open && has_calendar_session && dedicated_account_marker {
        OrdersRuntimeMode::Paper
    } else {
        OrdersRuntimeMode::Mock
    }
}

pub(crate) fn should_run_real_cancel_all(
    runtime_mode: OrdersRuntimeMode,
    dedicated_account_marker: bool,
) -> bool {
    runtime_mode == OrdersRuntimeMode::Paper && dedicated_account_marker
}

pub(crate) fn is_dedicated_orders_test_account_marker_set() -> bool {
    matches!(
        std::env::var(DEDICATED_ORDERS_TEST_ACCOUNT_ENV)
            .ok()
            .as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "YES")
    )
}

pub(crate) fn stock_test_symbol() -> &'static str {
    "SPY"
}

pub(crate) fn data_client_from_trade_credentials(credentials: &Credentials) -> alpaca_data::Client {
    alpaca_data::Client::builder()
        .api_key(credentials.api_key.clone())
        .secret_key(credentials.secret_key.clone())
        .build()
        .expect("alpaca-data client should build from trade credentials")
}

pub(crate) async fn orders_test_context() -> OrdersTestContext {
    if let Some(credentials) = super::trade_credentials() {
        let trade_client = Client::builder()
            .api_key(credentials.api_key.clone())
            .secret_key(credentials.secret_key.clone())
            .paper()
            .build()
            .expect("paper trade client should build");
        let data_client = data_client_from_trade_credentials(&credentials);
        let runtime_mode = detect_orders_runtime_mode(&trade_client).await;

        if runtime_mode == OrdersRuntimeMode::Paper {
            return OrdersTestContext {
                runtime_mode,
                trade_client,
                data_client: Some(data_client),
                market_snapshot: None,
                mock_server: None,
            };
        }

        eprintln!(
            "orders mutating tests: runtime mode resolved to Mock; using alpaca-trade-mock fallback because dedicated Paper mutating coverage is unavailable without {}=1 during market hours",
            DEDICATED_ORDERS_TEST_ACCOUNT_ENV,
        );
        return build_mock_test_context(Some(&trade_client), Some(&data_client)).await;
    }

    eprintln!(
        "orders mutating tests: credentials unavailable; using alpaca-trade-mock fallback runtime"
    );
    build_mock_test_context(None, None).await
}

pub(crate) async fn detect_orders_runtime_mode(client: &Client) -> OrdersRuntimeMode {
    let dedicated_account_marker = is_dedicated_orders_test_account_marker_set();
    if !dedicated_account_marker {
        return OrdersRuntimeMode::Mock;
    }

    let clock = match client.clock().get().await {
        Ok(clock) => clock,
        Err(_) => return OrdersRuntimeMode::Mock,
    };
    if !clock.is_open {
        return OrdersRuntimeMode::Mock;
    }

    let trading_day = clock
        .timestamp
        .split_once('T')
        .map(|(date, _)| date.to_owned())
        .unwrap_or(clock.timestamp.clone());
    let has_calendar_session = match client
        .calendar()
        .list(CalendarListRequest {
            start: Some(trading_day.clone()),
            end: Some(trading_day),
        })
        .await
    {
        Ok(days) => !days.is_empty(),
        Err(_) => false,
    };

    select_runtime_mode(
        true,
        clock.is_open,
        has_calendar_session,
        dedicated_account_marker,
    )
}

pub(crate) async fn stock_price_context(
    context: &OrdersTestContext,
    symbol: &str,
) -> Result<StockPriceContext, String> {
    let ask = match context.runtime_mode {
        OrdersRuntimeMode::Paper => {
            let data_client = context
                .data_client
                .as_ref()
                .ok_or_else(|| "paper context is missing data client".to_owned())?;
            latest_stock_ask(data_client, symbol).await?
        }
        OrdersRuntimeMode::Mock => context
            .market_snapshot
            .as_ref()
            .map(|snapshot| snapshot.instrument(symbol).ask)
            .ok_or_else(|| "mock context is missing market snapshot".to_owned())?,
    };

    Ok(stock_price_context_from_ask(ask))
}

pub(crate) async fn discover_option_contract(
    context: &OrdersTestContext,
    underlying_symbol: &str,
) -> Result<OptionContractContext, String> {
    if context.is_mock() {
        let snapshot = context
            .market_snapshot
            .as_ref()
            .ok_or_else(|| "mock context is missing market snapshot".to_owned())?;
        let symbol = snapshot.default_option_symbol().to_owned();
        return Ok(OptionContractContext {
            symbol: symbol.clone(),
            non_marketable_buy_limit_price: conservative_price_below_market(
                snapshot.instrument(&symbol).ask,
            ),
        });
    }

    let data_client = context
        .data_client
        .as_ref()
        .ok_or_else(|| "paper context is missing data client".to_owned())?;
    discover_live_option_contract(&context.trade_client, data_client, underlying_symbol).await
}

pub(crate) async fn discover_mleg_call_spread(
    context: &OrdersTestContext,
    underlying_symbol: &str,
) -> Result<MultiLegOrderContext, String> {
    let data_client = context
        .data_client
        .as_ref()
        .ok_or_else(|| "paper context is missing data client".to_owned())?;
    discover_live_mleg_call_spread(&context.trade_client, data_client, underlying_symbol).await
}

pub(crate) async fn discover_mleg_put_spread(
    context: &OrdersTestContext,
    underlying_symbol: &str,
) -> Result<MultiLegOrderContext, String> {
    let data_client = context
        .data_client
        .as_ref()
        .ok_or_else(|| "paper context is missing data client".to_owned())?;
    discover_live_mleg_put_spread(&context.trade_client, data_client, underlying_symbol).await
}

pub(crate) async fn discover_mleg_iron_condor(
    context: &OrdersTestContext,
    underlying_symbol: &str,
) -> Result<MultiLegOrderContext, String> {
    let data_client = context
        .data_client
        .as_ref()
        .ok_or_else(|| "paper context is missing data client".to_owned())?;
    discover_live_mleg_iron_condor(&context.trade_client, data_client, underlying_symbol).await
}

async fn discover_live_option_contract(
    trade_client: &Client,
    data_client: &DataClient,
    underlying_symbol: &str,
) -> Result<OptionContractContext, String> {
    let contracts = trade_client
        .options_contracts()
        .list(OptionsContractsListRequest {
            underlying_symbols: Some(vec![underlying_symbol.to_owned()]),
            status: Some(ContractStatus::Active),
            limit: Some(50),
            ..OptionsContractsListRequest::default()
        })
        .await
        .map_err(|error| format!("options_contracts list failed: {error}"))?;

    let tradable_contracts = contracts
        .option_contracts
        .into_iter()
        .filter(|contract| contract.tradable)
        .collect::<Vec<_>>();
    if tradable_contracts.is_empty() {
        return Err(format!(
            "no active tradable option contracts were returned for {underlying_symbol}"
        ));
    }

    let symbols = tradable_contracts
        .iter()
        .map(|contract| contract.symbol.clone())
        .collect::<Vec<_>>();
    let snapshots = data_client
        .options()
        .snapshots(options::SnapshotsRequest {
            symbols,
            ..options::SnapshotsRequest::default()
        })
        .await
        .map_err(|error| format!("options snapshots request failed: {error}"))?;

    best_option_candidate(tradable_contracts, snapshots.snapshots).ok_or_else(|| {
        format!("no quoted option contract snapshot was available for {underlying_symbol}")
    })
}

async fn discover_live_mleg_call_spread(
    trade_client: &Client,
    data_client: &DataClient,
    underlying_symbol: &str,
) -> Result<MultiLegOrderContext, String> {
    let spot = latest_stock_ask(data_client, underlying_symbol).await?;
    let calls = discover_quoted_contracts(
        trade_client,
        data_client,
        underlying_symbol,
        ContractType::Call,
        spot,
    )
    .await?;

    find_call_spread(underlying_symbol, spot, calls)
}

async fn discover_live_mleg_put_spread(
    trade_client: &Client,
    data_client: &DataClient,
    underlying_symbol: &str,
) -> Result<MultiLegOrderContext, String> {
    let spot = latest_stock_ask(data_client, underlying_symbol).await?;
    let puts = discover_quoted_contracts(
        trade_client,
        data_client,
        underlying_symbol,
        ContractType::Put,
        spot,
    )
    .await?;

    find_put_spread(underlying_symbol, spot, puts)
}

async fn discover_live_mleg_iron_condor(
    trade_client: &Client,
    data_client: &DataClient,
    underlying_symbol: &str,
) -> Result<MultiLegOrderContext, String> {
    let spot = latest_stock_ask(data_client, underlying_symbol).await?;
    let puts = discover_quoted_contracts(
        trade_client,
        data_client,
        underlying_symbol,
        ContractType::Put,
        spot,
    )
    .await?;
    let calls = discover_quoted_contracts(
        trade_client,
        data_client,
        underlying_symbol,
        ContractType::Call,
        spot,
    )
    .await?;

    find_iron_condor(underlying_symbol, spot, puts, calls)
}

pub(crate) async fn wait_for_order_terminal_state(
    client: &Client,
    order_id: &str,
) -> Result<Order, Error> {
    let mut last_order = None;

    for _ in 0..30 {
        let order = client.orders().get(order_id).await?;
        if is_terminal_status(order.status.clone()) {
            return Ok(order);
        }
        last_order = Some(order);
        sleep(Duration::from_secs(1)).await;
    }

    Err(Error::InvalidConfiguration(format!(
        "timed out waiting for order {order_id} to reach terminal state; last status: {:?}",
        last_order.map(|order| order.status),
    )))
}

pub(crate) async fn wait_for_order_statuses(
    client: &Client,
    order_id: &str,
    expected_statuses: &[OrderStatus],
) -> Result<Order, Error> {
    let mut last_order = None;

    for _ in 0..30 {
        let order = client.orders().get(order_id).await?;
        if expected_statuses.contains(&order.status) {
            return Ok(order);
        }
        if is_terminal_status(order.status.clone()) {
            return Err(Error::InvalidConfiguration(format!(
                "order {order_id} reached unexpected terminal status {:?} before expected statuses {:?}",
                order.status, expected_statuses,
            )));
        }
        last_order = Some(order);
        sleep(Duration::from_secs(1)).await;
    }

    Err(Error::InvalidConfiguration(format!(
        "timed out waiting for order {order_id} to reach expected statuses {:?}; last status: {:?}",
        expected_statuses,
        last_order.map(|order| order.status),
    )))
}

pub(crate) async fn cleanup_open_orders(context: &OrdersTestContext, cleanup: &CleanupTracker) {
    if cleanup.allow_cancel_all
        && (context.runtime_mode == OrdersRuntimeMode::Mock || context.can_run_real_cancel_all())
    {
        let _ = context.trade_client.orders().cancel_all().await;
    }

    for order_id in cleanup.created_order_ids.iter().rev() {
        let _ = context.trade_client.orders().cancel(order_id).await;
    }
}

fn conservative_price_below_market(price: Decimal) -> Decimal {
    let floor = Decimal::new(1, 2);
    let scaled = price * Decimal::new(5, 1);
    if scaled < floor {
        floor
    } else {
        scaled.round_dp(2)
    }
}

fn stock_price_context_from_ask(ask: Decimal) -> StockPriceContext {
    StockPriceContext {
        non_marketable_buy_limit_price: conservative_price_below_market(ask),
        more_conservative_buy_limit_price: conservative_price_below_market(
            conservative_price_below_market(ask),
        ),
    }
}

async fn latest_stock_quote(
    data_client: &DataClient,
    symbol: &str,
) -> Result<(Decimal, Decimal), String> {
    let quote = data_client
        .stocks()
        .latest_quote(LatestQuoteRequest {
            symbol: symbol.to_owned(),
            ..LatestQuoteRequest::default()
        })
        .await
        .map_err(|error| format!("latest stock quote request failed: {error}"))?;
    let bid = quote
        .quote
        .bp
        .and_then(|value| Decimal::from_str(&value.to_string()).ok())
        .ok_or_else(|| format!("latest stock quote for {symbol} is missing bid price"))?;
    let ask = quote
        .quote
        .ap
        .or(quote.quote.bp)
        .and_then(|value| Decimal::from_str(&value.to_string()).ok())
        .ok_or_else(|| {
            format!("latest stock quote for {symbol} is missing both ask and bid prices")
        })?;

    Ok((bid, ask))
}

async fn latest_stock_ask(data_client: &DataClient, symbol: &str) -> Result<Decimal, String> {
    latest_stock_quote(data_client, symbol)
        .await
        .map(|(_, ask)| ask)
}

async fn build_mock_test_context(
    trade_client: Option<&Client>,
    data_client: Option<&DataClient>,
) -> OrdersTestContext {
    let market_snapshot = build_mock_market_snapshot(trade_client, data_client).await;
    let server = spawn_test_server_with_market_snapshot(market_snapshot.clone()).await;
    let client = Client::builder()
        .api_key("mock-api-key")
        .secret_key("mock-secret-key")
        .base_url(server.base_url.clone())
        .build()
        .expect("mock trade client should build");

    OrdersTestContext {
        runtime_mode: OrdersRuntimeMode::Mock,
        trade_client: client,
        data_client: data_client.cloned(),
        market_snapshot: Some(market_snapshot),
        mock_server: Some(server),
    }
}

async fn build_mock_market_snapshot(
    trade_client: Option<&Client>,
    data_client: Option<&DataClient>,
) -> OrdersMarketSnapshot {
    let mut snapshot = OrdersMarketSnapshot::default();

    if let Some(data_client) = data_client {
        if let Ok((bid, ask)) = latest_stock_quote(data_client, stock_test_symbol()).await {
            snapshot =
                snapshot.with_instrument(stock_test_symbol(), InstrumentSnapshot::equity(bid, ask));
        }
    }

    if let (Some(trade_client), Some(data_client)) = (trade_client, data_client) {
        if let Ok(contract) =
            discover_live_option_contract(trade_client, data_client, stock_test_symbol()).await
        {
            let ask = (contract.non_marketable_buy_limit_price * Decimal::new(2, 0))
                .max(Decimal::new(2, 2))
                .round_dp(2);
            snapshot = snapshot.with_instrument(
                contract.symbol,
                InstrumentSnapshot::option(contract.non_marketable_buy_limit_price, ask),
            );
        }
    }

    snapshot
}

fn best_option_candidate(
    contracts: Vec<alpaca_trade::options_contracts::OptionContract>,
    snapshots: HashMap<String, alpaca_data::options::Snapshot>,
) -> Option<OptionContractContext> {
    contracts
        .into_iter()
        .filter_map(|contract| {
            let snapshot = snapshots.get(&contract.symbol)?;
            let ask = snapshot
                .latestQuote
                .as_ref()
                .and_then(|quote| quote.ap)
                .or_else(|| snapshot.latestTrade.as_ref().and_then(|trade| trade.p))
                .and_then(|value| Decimal::from_str(&value.to_string()).ok())?;
            if ask <= MIN_PRICE {
                return None;
            }

            Some((contract.symbol, ask))
        })
        .min_by_key(|(_, ask)| *ask)
        .map(|(symbol, ask)| OptionContractContext {
            symbol,
            non_marketable_buy_limit_price: conservative_price_below_market(ask),
        })
}

async fn discover_quoted_contracts(
    trade_client: &Client,
    data_client: &DataClient,
    underlying_symbol: &str,
    contract_type: ContractType,
    spot: Decimal,
) -> Result<Vec<QuotedOptionContract>, String> {
    let contracts = list_active_tradable_contracts(
        trade_client,
        underlying_symbol,
        contract_type.clone(),
        strike_window(spot, -OPTION_DISCOVERY_STRIKE_WINDOW_BPS),
        strike_window(spot, OPTION_DISCOVERY_STRIKE_WINDOW_BPS),
    )
    .await?;
    let snapshots = option_snapshots_by_symbol(
        data_client,
        contracts
            .iter()
            .map(|contract| contract.symbol.clone())
            .collect::<Vec<_>>(),
    )
    .await?;

    let quoted = contracts
        .into_iter()
        .filter_map(|contract| quoted_contract(contract, &snapshots))
        .collect::<Vec<_>>();

    if quoted.is_empty() {
        return Err(format!(
            "no quoted {:?} contracts were available for {underlying_symbol}",
            contract_type
        ));
    }

    Ok(quoted)
}

async fn list_active_tradable_contracts(
    trade_client: &Client,
    underlying_symbol: &str,
    contract_type: ContractType,
    strike_price_gte: Decimal,
    strike_price_lte: Decimal,
) -> Result<Vec<OptionContract>, String> {
    let today = Utc::now().date_naive();
    let latest_expiration = today
        .checked_add_days(Days::new(OPTION_DISCOVERY_LOOKAHEAD_DAYS))
        .ok_or_else(|| "failed to compute option discovery lookahead window".to_owned())?;
    let mut page_token = None;
    let mut contracts = Vec::new();

    loop {
        let response = trade_client
            .options_contracts()
            .list(OptionsContractsListRequest {
                underlying_symbols: Some(vec![underlying_symbol.to_owned()]),
                status: Some(ContractStatus::Active),
                expiration_date_gte: Some(today.to_string()),
                expiration_date_lte: Some(latest_expiration.to_string()),
                r#type: Some(contract_type.clone()),
                strike_price_gte: Some(strike_price_gte),
                strike_price_lte: Some(strike_price_lte),
                limit: Some(1_000),
                page_token: page_token.clone(),
                ..OptionsContractsListRequest::default()
            })
            .await
            .map_err(|error| format!("options_contracts list failed: {error}"))?;

        contracts.extend(
            response
                .option_contracts
                .into_iter()
                .filter(|contract| contract.tradable),
        );
        page_token = response.next_page_token;

        if page_token.is_none() {
            break;
        }
    }

    if contracts.is_empty() {
        return Err(format!(
            "no active tradable {:?} contracts were returned for {underlying_symbol}",
            contract_type
        ));
    }

    Ok(contracts)
}

async fn option_snapshots_by_symbol(
    data_client: &DataClient,
    symbols: Vec<String>,
) -> Result<HashMap<String, options::Snapshot>, String> {
    let mut snapshots = HashMap::new();

    for batch in symbols.chunks(OPTION_SNAPSHOT_BATCH_SIZE) {
        let response = data_client
            .options()
            .snapshots(options::SnapshotsRequest {
                symbols: batch.to_vec(),
                ..options::SnapshotsRequest::default()
            })
            .await
            .map_err(|error| format!("options snapshots request failed: {error}"))?;
        snapshots.extend(response.snapshots);
    }

    Ok(snapshots)
}

fn quoted_contract(
    contract: OptionContract,
    snapshots: &HashMap<String, options::Snapshot>,
) -> Option<QuotedOptionContract> {
    let snapshot = snapshots.get(&contract.symbol)?;
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

    if bid <= MIN_PRICE || ask <= MIN_PRICE || ask < bid {
        return None;
    }

    Some(QuotedOptionContract { contract, bid, ask })
}

fn find_call_spread(
    underlying_symbol: &str,
    spot: Decimal,
    contracts: Vec<QuotedOptionContract>,
) -> Result<MultiLegOrderContext, String> {
    for (_, mut expiration_contracts) in group_by_expiration(contracts) {
        sort_by_strike(&mut expiration_contracts);

        let mut best_candidate = None;
        for window in expiration_contracts.windows(2) {
            let lower = &window[0];
            let higher = &window[1];
            if higher.contract.strike_price <= lower.contract.strike_price {
                continue;
            }

            let score = (lower.contract.strike_price - spot).abs();
            let candidate = build_debit_mleg_context(
                underlying_symbol,
                vec![
                    strategy_leg(lower.clone(), 1, OrderSide::Buy, PositionIntent::BuyToOpen),
                    strategy_leg(
                        higher.clone(),
                        1,
                        OrderSide::Sell,
                        PositionIntent::SellToOpen,
                    ),
                ],
            );
            if let Ok(context) = candidate {
                match &best_candidate {
                    Some((best_score, _)) if score >= *best_score => {}
                    _ => best_candidate = Some((score, context)),
                }
            }
        }

        if let Some((_, context)) = best_candidate {
            return Ok(context);
        }
    }

    Err(format!(
        "failed to discover a quoted debit call spread for {underlying_symbol}"
    ))
}

fn find_put_spread(
    underlying_symbol: &str,
    spot: Decimal,
    contracts: Vec<QuotedOptionContract>,
) -> Result<MultiLegOrderContext, String> {
    for (_, mut expiration_contracts) in group_by_expiration(contracts) {
        sort_by_strike(&mut expiration_contracts);

        let mut best_candidate = None;
        for window in expiration_contracts.windows(2) {
            let lower = &window[0];
            let higher = &window[1];
            if higher.contract.strike_price <= lower.contract.strike_price {
                continue;
            }

            let score = (higher.contract.strike_price - spot).abs();
            let candidate = build_debit_mleg_context(
                underlying_symbol,
                vec![
                    strategy_leg(higher.clone(), 1, OrderSide::Buy, PositionIntent::BuyToOpen),
                    strategy_leg(
                        lower.clone(),
                        1,
                        OrderSide::Sell,
                        PositionIntent::SellToOpen,
                    ),
                ],
            );
            if let Ok(context) = candidate {
                match &best_candidate {
                    Some((best_score, _)) if score >= *best_score => {}
                    _ => best_candidate = Some((score, context)),
                }
            }
        }

        if let Some((_, context)) = best_candidate {
            return Ok(context);
        }
    }

    Err(format!(
        "failed to discover a quoted debit put spread for {underlying_symbol}"
    ))
}

fn find_iron_condor(
    underlying_symbol: &str,
    spot: Decimal,
    puts: Vec<QuotedOptionContract>,
    calls: Vec<QuotedOptionContract>,
) -> Result<MultiLegOrderContext, String> {
    let put_groups = group_by_expiration(puts);
    let call_groups = group_by_expiration(calls)
        .into_iter()
        .collect::<HashMap<_, _>>();

    for (expiration, mut expiration_puts) in put_groups {
        let Some(mut expiration_calls) = call_groups.get(&expiration).cloned() else {
            continue;
        };

        sort_by_strike(&mut expiration_puts);
        sort_by_strike(&mut expiration_calls);

        let put_candidates = expiration_puts
            .iter()
            .filter(|contract| contract.contract.strike_price < spot)
            .cloned()
            .collect::<Vec<_>>();
        let call_candidates = expiration_calls
            .iter()
            .filter(|contract| contract.contract.strike_price > spot)
            .cloned()
            .collect::<Vec<_>>();

        if put_candidates.len() < 2 || call_candidates.len() < 2 {
            continue;
        }

        let mut best_candidate = None;
        for outer_put_index in 0..put_candidates.len() - 1 {
            for inner_put_index in outer_put_index + 1..put_candidates.len() {
                let outer_put = put_candidates[outer_put_index].clone();
                let inner_put = put_candidates[inner_put_index].clone();

                for inner_call_index in 0..call_candidates.len() - 1 {
                    for outer_call_index in inner_call_index + 1..call_candidates.len() {
                        let inner_call = call_candidates[inner_call_index].clone();
                        let outer_call = call_candidates[outer_call_index].clone();
                        let score = (spot - inner_put.contract.strike_price).abs()
                            + (inner_call.contract.strike_price - spot).abs()
                            + (inner_put.contract.strike_price - outer_put.contract.strike_price)
                                .abs()
                            + (outer_call.contract.strike_price - inner_call.contract.strike_price)
                                .abs();

                        let candidate = build_debit_mleg_context(
                            underlying_symbol,
                            vec![
                                strategy_leg(
                                    outer_put.clone(),
                                    1,
                                    OrderSide::Buy,
                                    PositionIntent::BuyToOpen,
                                ),
                                strategy_leg(
                                    inner_put.clone(),
                                    1,
                                    OrderSide::Sell,
                                    PositionIntent::SellToOpen,
                                ),
                                strategy_leg(
                                    inner_call.clone(),
                                    1,
                                    OrderSide::Sell,
                                    PositionIntent::SellToOpen,
                                ),
                                strategy_leg(
                                    outer_call.clone(),
                                    1,
                                    OrderSide::Buy,
                                    PositionIntent::BuyToOpen,
                                ),
                            ],
                        );

                        if let Ok(context) = candidate {
                            match &best_candidate {
                                Some((best_score, _)) if score >= *best_score => {}
                                _ => best_candidate = Some((score, context)),
                            }
                        }
                    }
                }
            }
        }

        if let Some((_, context)) = best_candidate {
            return Ok(context);
        }
    }

    Err(format!(
        "failed to discover a quoted debit iron condor for {underlying_symbol}"
    ))
}

#[derive(Debug, Clone, PartialEq)]
struct StrategyLeg {
    contract: QuotedOptionContract,
    ratio_qty: u32,
    side: OrderSide,
    position_intent: PositionIntent,
}

fn strategy_leg(
    contract: QuotedOptionContract,
    ratio_qty: u32,
    side: OrderSide,
    position_intent: PositionIntent,
) -> StrategyLeg {
    StrategyLeg {
        contract,
        ratio_qty,
        side,
        position_intent,
    }
}

fn build_debit_mleg_context(
    underlying_symbol: &str,
    legs: Vec<StrategyLeg>,
) -> Result<MultiLegOrderContext, String> {
    let best_debit = legs
        .iter()
        .map(best_case_debit_contribution)
        .sum::<Decimal>()
        .round_dp(2);
    let worst_debit = legs
        .iter()
        .map(worst_case_debit_contribution)
        .sum::<Decimal>()
        .round_dp(2);

    if worst_debit <= MIN_PRICE {
        return Err(format!(
            "discovered multi-leg strategy for {underlying_symbol} was not a net debit"
        ));
    }

    let non_marketable_limit_price =
        conservative_price_below_market(best_debit.max(Decimal::new(1, 2)));
    let more_conservative_limit_price = conservative_price_below_market(non_marketable_limit_price);
    let marketable_limit_price = (worst_debit + Decimal::new(10, 2)).round_dp(2);

    Ok(MultiLegOrderContext {
        underlying_symbol: underlying_symbol.to_owned(),
        legs: legs
            .into_iter()
            .map(|leg| OptionLegRequest {
                symbol: leg.contract.contract.symbol,
                ratio_qty: leg.ratio_qty,
                side: Some(leg.side),
                position_intent: Some(leg.position_intent),
            })
            .collect(),
        non_marketable_limit_price,
        more_conservative_limit_price,
        marketable_limit_price,
    })
}

fn best_case_debit_contribution(leg: &StrategyLeg) -> Decimal {
    let quantity = Decimal::from(leg.ratio_qty);
    match leg.side {
        OrderSide::Buy => leg.contract.bid * quantity,
        OrderSide::Sell => -(leg.contract.ask * quantity),
        _ => unreachable!("unexpected order side in mleg debit calculation"),
    }
}

fn worst_case_debit_contribution(leg: &StrategyLeg) -> Decimal {
    let quantity = Decimal::from(leg.ratio_qty);
    match leg.side {
        OrderSide::Buy => leg.contract.ask * quantity,
        OrderSide::Sell => -(leg.contract.bid * quantity),
        _ => unreachable!("unexpected order side in mleg debit calculation"),
    }
}

fn group_by_expiration(
    contracts: Vec<QuotedOptionContract>,
) -> Vec<(String, Vec<QuotedOptionContract>)> {
    let mut grouped = HashMap::<String, Vec<QuotedOptionContract>>::new();
    for contract in contracts {
        grouped
            .entry(contract.contract.expiration_date.clone())
            .or_default()
            .push(contract);
    }

    let mut grouped = grouped.into_iter().collect::<Vec<_>>();
    grouped.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
    grouped
}

fn sort_by_strike(contracts: &mut [QuotedOptionContract]) {
    contracts.sort_by(|lhs, rhs| {
        lhs.contract
            .strike_price
            .partial_cmp(&rhs.contract.strike_price)
            .expect("option strikes should always be comparable")
    });
}

fn strike_window(spot: Decimal, bps_offset: i64) -> Decimal {
    let multiplier = Decimal::new(10_000 + bps_offset, 4);
    (spot * multiplier).round_dp(2)
}

fn decimal_from_market_data(value: f64) -> Option<Decimal> {
    Decimal::from_str(&value.to_string()).ok()
}

fn is_terminal_status(status: OrderStatus) -> bool {
    matches!(
        status,
        OrderStatus::Filled
            | OrderStatus::Canceled
            | OrderStatus::Expired
            | OrderStatus::Rejected
            | OrderStatus::Replaced
    )
}

fn sanitize_client_order_component(value: &str) -> String {
    let mut sanitized = String::new();
    let mut last_was_dash = false;

    for ch in value.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch);
            last_was_dash = false;
            continue;
        }

        if !last_was_dash {
            sanitized.push('-');
            last_was_dash = true;
        }
    }

    sanitized.trim_matches('-').to_owned()
}
