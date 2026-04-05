use axum::Router;

use crate::routes::build_router;

pub fn build_app() -> Router {
    build_router()
}
