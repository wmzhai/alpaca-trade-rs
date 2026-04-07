use axum::Json;
use axum::extract::{Extension, Path};

use crate::state::OrdersState;

pub async fn activities_list(
    Extension(state): Extension<OrdersState>,
) -> Json<Vec<serde_json::Value>> {
    Json(
        state
            .list_activities(None)
            .into_iter()
            .map(|activity| {
                serde_json::to_value(activity).expect("activity projection should serialize")
            })
            .collect(),
    )
}

pub async fn activities_by_type(
    Extension(state): Extension<OrdersState>,
    Path(activity_type): Path<String>,
) -> Json<Vec<serde_json::Value>> {
    Json(
        state
            .list_activities(Some(&activity_type))
            .into_iter()
            .map(|activity| {
                serde_json::to_value(activity).expect("activity projection should serialize")
            })
            .collect(),
    )
}
