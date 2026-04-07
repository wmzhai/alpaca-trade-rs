use alpaca_trade::Decimal;
use alpaca_trade::orders::OrderStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActivityEventKind {
    New,
    Filled,
    Canceled,
    Replaced,
    PositionClosed,
    Exercised,
    DoNotExercise,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivityEvent {
    pub(crate) sequence: u64,
    pub(crate) kind: ActivityEventKind,
    pub(crate) order_id: String,
    pub(crate) client_order_id: String,
    pub(crate) related_order_id: Option<String>,
    pub(crate) status: Option<OrderStatus>,
    pub(crate) symbol: String,
    pub(crate) asset_class: String,
    pub(crate) occurred_at: String,
    pub(crate) cash_delta: Decimal,
}

impl ActivityEvent {
    pub(crate) fn new(
        sequence: u64,
        kind: ActivityEventKind,
        order_id: String,
        client_order_id: String,
        related_order_id: Option<String>,
        status: Option<OrderStatus>,
        symbol: String,
        asset_class: String,
        occurred_at: String,
        cash_delta: Decimal,
    ) -> Self {
        Self {
            sequence,
            kind,
            order_id,
            client_order_id,
            related_order_id,
            status,
            symbol,
            asset_class,
            occurred_at,
            cash_delta,
        }
    }
}
