use axum::{Router, routing::get};

use crate::handlers;

pub fn build_router() -> Router {
    Router::new().route("/health", get(handlers::health))
}
