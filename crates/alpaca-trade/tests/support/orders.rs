use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use alpaca_data::stocks::LatestQuoteRequest;
use alpaca_data::{Client as DataClient, options};
use alpaca_trade::Client;
use alpaca_trade::calendar::ListRequest as CalendarListRequest;
use alpaca_trade::options_contracts::{ContractStatus, ListRequest as OptionsContractsListRequest};
use alpaca_trade::orders::{Order, OrderStatus};
use alpaca_trade::{Decimal, Error};
use tokio::time::sleep;

use super::Credentials;

const DEDICATED_ORDERS_TEST_ACCOUNT_ENV: &str = "ALPACA_TRADE_ORDERS_TEST_ACCOUNT";
const MIN_PRICE: Decimal = Decimal::ZERO; // placeholder; use helper for 0.01
static CLIENT_ORDER_COUNTER: AtomicU64 = AtomicU64::new(1);

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

#[derive(Clone)]
pub(crate) struct PaperTestContext {
    pub(crate) trade_client: Client,
    pub(crate) data_client: DataClient,
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

impl PaperTestContext {
    pub(crate) fn next_client_order_id(&self, suite: &str, test_name: &str) -> String {
        let _ = &self.trade_client;
        next_client_order_id(suite, test_name)
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
        std::env::var(DEDICATED_ORDERS_TEST_ACCOUNT_ENV).ok().as_deref(),
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

pub(crate) async fn paper_test_context() -> Option<PaperTestContext> {
    let Some(credentials) = super::trade_credentials() else {
        eprintln!(
            "skipping orders mutating tests: missing ALPACA_TRADE_API_KEY / ALPACA_TRADE_SECRET_KEY or APCA_API_KEY_ID / APCA_API_SECRET_KEY"
        );
        return None;
    };

    let trade_client = Client::builder()
        .api_key(credentials.api_key.clone())
        .secret_key(credentials.secret_key.clone())
        .paper()
        .build()
        .expect("paper trade client should build");

    let runtime_mode = detect_orders_runtime_mode(&trade_client).await;
    if runtime_mode != OrdersRuntimeMode::Paper {
        eprintln!(
            "skipping orders mutating tests: runtime mode resolved to {:?}; set {}=1 on the dedicated Paper test account during market hours to enable real-path coverage",
            runtime_mode,
            DEDICATED_ORDERS_TEST_ACCOUNT_ENV,
        );
        return None;
    }

    Some(PaperTestContext {
        trade_client,
        data_client: data_client_from_trade_credentials(&credentials),
    })
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

    select_runtime_mode(true, clock.is_open, has_calendar_session, dedicated_account_marker)
}

pub(crate) async fn stock_price_context(
    data_client: &DataClient,
    symbol: &str,
) -> Result<StockPriceContext, String> {
    let quote = data_client
        .stocks()
        .latest_quote(LatestQuoteRequest {
            symbol: symbol.to_owned(),
            ..LatestQuoteRequest::default()
        })
        .await
        .map_err(|error| format!("latest stock quote request failed: {error}"))?;
    let ask = quote
        .quote
        .ap
        .or(quote.quote.bp)
        .and_then(|value| Decimal::from_str(&value.to_string()).ok())
        .ok_or_else(|| format!("latest stock quote for {symbol} is missing both ask and bid prices"))?;

    Ok(StockPriceContext {
        non_marketable_buy_limit_price: conservative_price_below_market(ask),
        more_conservative_buy_limit_price: conservative_price_below_market(
            conservative_price_below_market(ask),
        ),
    })
}

pub(crate) async fn discover_option_contract(
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

    best_option_candidate(tradable_contracts, snapshots.snapshots)
        .ok_or_else(|| format!("no quoted option contract snapshot was available for {underlying_symbol}"))
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

pub(crate) async fn cleanup_open_orders(client: &Client, cleanup: &CleanupTracker) {
    if cleanup.allow_cancel_all && is_dedicated_orders_test_account_marker_set() {
        let _ = client.orders().cancel_all().await;
    }

    for order_id in cleanup.created_order_ids.iter().rev() {
        let _ = client.orders().cancel(order_id).await;
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
