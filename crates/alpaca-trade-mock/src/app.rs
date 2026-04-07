use axum::{
    Router,
    body::Body,
    extract::State,
    http::Request,
    middleware::{self, Next},
    response::Response,
    routing::get,
};

use crate::auth::{MockHttpError, extract_auth};
use crate::handlers;
use crate::state::{MockTradingState, OrdersMarketSnapshot, OrdersState};

pub fn build_app() -> Router {
    build_app_with_market_snapshot(OrdersMarketSnapshot::default())
}

pub fn build_app_with_market_snapshot(market_snapshot: OrdersMarketSnapshot) -> Router {
    let trading_state = MockTradingState::new();
    let orders_router = Router::new()
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
        .with_state(OrdersState::new(market_snapshot))
        .route_layer(middleware::from_fn_with_state(
            trading_state,
            require_trading_auth,
        ));

    Router::new()
        .route("/health", get(handlers::health))
        .merge(orders_router)
}

async fn require_trading_auth(
    State(trading_state): State<MockTradingState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, MockHttpError> {
    let auth = extract_auth(request.headers())?;
    let api_key = auth.api_key;
    let _secret_key = auth.secret_key;

    trading_state.ensure_account(&api_key);

    Ok(next.run(request).await)
}
