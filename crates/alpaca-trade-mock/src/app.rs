use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::Request,
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use parking_lot::RwLock;

use crate::auth::{MockHttpError, extract_auth};
use crate::handlers;
use crate::state::{MockTradingState, OrdersMarketSnapshot, OrdersState};

pub fn build_app() -> Router {
    build_app_with_market_snapshot(OrdersMarketSnapshot::default())
}

pub fn build_app_with_market_snapshot(market_snapshot: OrdersMarketSnapshot) -> Router {
    let route_state = TradingRouteState {
        trading_state: MockTradingState::new(),
        orders_states: Arc::new(RwLock::new(HashMap::new())),
        market_snapshot,
    };
    let trading_router = Router::new()
        .route("/v2/account", get(handlers::account_get))
        .route("/v2/account/activities", get(handlers::activities_list))
        .route(
            "/v2/account/activities/{activity_type}",
            get(handlers::activities_by_type),
        )
        .route(
            "/v2/positions",
            get(handlers::positions_list).delete(handlers::positions_close_all),
        )
        .route(
            "/v2/positions/{symbol_or_asset_id}",
            get(handlers::positions_get).delete(handlers::positions_close),
        )
        .route(
            "/v2/positions/{symbol_or_contract_id}/exercise",
            axum::routing::post(handlers::positions_exercise),
        )
        .route(
            "/v2/positions/{symbol_or_contract_id}/do-not-exercise",
            axum::routing::post(handlers::positions_do_not_exercise),
        )
        .route(
            "/v2/orders",
            get(handlers::orders_list)
                .post(handlers::orders_create)
                .delete(handlers::orders_cancel_all),
        )
        .route(
            "/v2/orders/{order_id}",
            get(handlers::orders_get)
                .patch(handlers::orders_replace)
                .delete(handlers::orders_cancel),
        )
        .route(
            "/v2/orders:by_client_order_id",
            get(handlers::orders_get_by_client_order_id),
        )
        .route_layer(middleware::from_fn_with_state(
            route_state,
            require_trading_auth,
        ));

    Router::new()
        .route("/health", get(handlers::health))
        .merge(trading_router)
}

#[derive(Clone)]
struct TradingRouteState {
    trading_state: MockTradingState,
    orders_states: Arc<RwLock<HashMap<String, OrdersState>>>,
    market_snapshot: OrdersMarketSnapshot,
}

async fn require_trading_auth(
    State(state): State<TradingRouteState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, MockHttpError> {
    let auth = extract_auth(request.headers())?;
    let api_key = auth.api_key;
    let _secret_key = auth.secret_key;

    state.trading_state.ensure_account(&api_key);
    let orders_state = {
        let mut orders_states = state.orders_states.write();
        orders_states
            .entry(api_key.clone())
            .or_insert_with(|| {
                OrdersState::new(
                    state.trading_state.clone(),
                    api_key,
                    state.market_snapshot.clone(),
                )
            })
            .clone()
    };
    request.extensions_mut().insert(orders_state);

    Ok(next.run(request).await)
}
