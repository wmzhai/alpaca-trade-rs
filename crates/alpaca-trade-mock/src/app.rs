use axum::Router;

use crate::routes::build_router;
use crate::state::{OrdersMarketSnapshot, OrdersState};

pub fn build_app() -> Router {
    build_app_with_market_snapshot(OrdersMarketSnapshot::default())
}

pub fn build_app_with_market_snapshot(market_snapshot: OrdersMarketSnapshot) -> Router {
    build_router(OrdersState::new(market_snapshot))
}
