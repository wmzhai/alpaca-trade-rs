use axum::Router;

use crate::{routes::build_router, state::AppState};

pub fn build_app() -> Router {
    build_router(AppState::new())
}
