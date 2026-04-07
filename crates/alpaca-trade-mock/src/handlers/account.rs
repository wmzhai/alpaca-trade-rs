use alpaca_trade::account::Account;
use axum::Json;
use axum::extract::Extension;

use crate::state::OrdersState;

pub async fn account_get(Extension(state): Extension<OrdersState>) -> Json<Account> {
    Json(state.project_account())
}
