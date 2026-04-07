use alpaca_trade::Decimal;
use alpaca_trade::orders::{OrderSide, PositionIntent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionFact {
    pub sequence: u64,
    pub order_id: String,
    pub parent_order_id: Option<String>,
    pub asset_id: String,
    pub symbol: String,
    pub asset_class: String,
    pub side: OrderSide,
    pub position_intent: Option<PositionIntent>,
    pub qty: Decimal,
    pub price: Decimal,
    pub occurred_at: String,
}

impl ExecutionFact {
    pub fn new(
        sequence: u64,
        order_id: String,
        parent_order_id: Option<String>,
        asset_id: String,
        symbol: String,
        asset_class: String,
        side: OrderSide,
        position_intent: Option<PositionIntent>,
        qty: Decimal,
        price: Decimal,
        occurred_at: String,
    ) -> Self {
        Self {
            sequence,
            order_id,
            parent_order_id,
            asset_id,
            symbol,
            asset_class,
            side,
            position_intent,
            qty,
            price,
            occurred_at,
        }
    }
}
