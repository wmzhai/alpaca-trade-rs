use axum::{routing::get, Router};

use crate::handlers;

pub fn build_router() -> Router {
    Router::new().route("/health", get(handlers::health))
}
