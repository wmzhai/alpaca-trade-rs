use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use alpaca_trade::Client;
use alpaca_trade::calendar::ListRequest as CalendarListRequest;

use super::Credentials;

const DEDICATED_ORDERS_TEST_ACCOUNT_ENV: &str = "ALPACA_TRADE_ORDERS_TEST_ACCOUNT";
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

