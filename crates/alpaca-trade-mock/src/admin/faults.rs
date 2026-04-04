use axum::{Json, extract::State};

use crate::state::{AppState, FaultRule};

pub async fn reset(State(state): State<AppState>) -> Json<serde_json::Value> {
    state.reset();
    Json(serde_json::json!({ "status": "ok" }))
}

pub async fn add_fault(
    State(state): State<AppState>,
    Json(rule): Json<FaultRule>,
) -> Json<serde_json::Value> {
    state.push_fault(rule);
    Json(serde_json::json!({ "status": "ok" }))
}

pub async fn clear_faults(State(state): State<AppState>) -> Json<serde_json::Value> {
    state.clear_faults();
    Json(serde_json::json!({ "status": "ok" }))
}
