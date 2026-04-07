use axum::{Extension, Router, routing::get};

use crate::handlers;
use crate::state::OrdersState;

pub fn build_router(state: OrdersState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
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
        .layer(Extension(state))
}
