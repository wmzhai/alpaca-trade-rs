use axum::Json;
use axum::extract::{Extension, Path};

use super::orders::MockHttpError;
use crate::state::OrdersState;

type RouteResult<T> = Result<T, MockHttpError>;

pub async fn positions_list(
    Extension(state): Extension<OrdersState>,
) -> RouteResult<Json<Vec<serde_json::Value>>> {
    let positions = state
        .list_positions()
        .await?
        .into_iter()
        .map(|position| {
            serde_json::to_value(position).expect("position projection should serialize")
        })
        .collect::<Vec<_>>();
    Ok(Json(positions))
}

pub async fn positions_get(
    Extension(state): Extension<OrdersState>,
    Path(symbol_or_asset_id): Path<String>,
) -> RouteResult<Json<serde_json::Value>> {
    let position = state.get_position(&symbol_or_asset_id).await?;
    Ok(Json(
        serde_json::to_value(position).expect("position projection should serialize"),
    ))
}
