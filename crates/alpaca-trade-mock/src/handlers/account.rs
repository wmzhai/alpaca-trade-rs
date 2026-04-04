use axum::{
    Json,
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};

use crate::state::AppState;

fn has_auth(headers: &HeaderMap) -> bool {
    headers.contains_key("APCA-API-KEY-ID") && headers.contains_key("APCA-API-SECRET-KEY")
}

pub async fn get_account(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !has_auth(&headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "message": "missing credentials" })),
        )
            .into_response();
    }

    if let Some(fault) = state.take_fault("GET", "/v2/account") {
        let status =
            StatusCode::from_u16(fault.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut response = (status, fault.body).into_response();

        for (key, value) in fault.headers {
            if let (Ok(key), Ok(value)) = (
                HeaderName::try_from(key.as_str()),
                HeaderValue::from_str(&value),
            ) {
                response.headers_mut().insert(key, value);
            }
        }

        return response;
    }

    Json(state.account()).into_response()
}
