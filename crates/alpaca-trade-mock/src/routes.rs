use axum::{
    Router,
    routing::{get, post},
};

use crate::{admin, handlers, state::AppState};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/v2/account", get(handlers::get_account))
        .route("/__admin/reset", post(admin::reset))
        .route(
            "/__admin/faults",
            post(admin::add_fault).delete(admin::clear_faults),
        )
        .with_state(state)
}
