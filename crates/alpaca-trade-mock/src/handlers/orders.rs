use std::str::FromStr;

use alpaca_trade::Decimal;
use alpaca_trade::orders::{
    OptionLegRequest, OrderClass, OrderSide, OrderType, PositionIntent, StopLoss, TakeProfit,
    TimeInForce,
};
use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::state::{
    CreateOrderInput, ListOrdersFilter, OrdersState, OrdersStateError, ReplaceOrderInput,
};

type RouteResult<T> = Result<T, MockHttpError>;

#[derive(Debug)]
pub struct MockHttpError {
    status: StatusCode,
    message: String,
}

impl MockHttpError {
    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl IntoResponse for MockHttpError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({
                "code": self.status.as_u16(),
                "message": self.message,
            })),
        )
            .into_response()
    }
}

impl From<OrdersStateError> for MockHttpError {
    fn from(error: OrdersStateError) -> Self {
        match error {
            OrdersStateError::NotFound(message) => Self::not_found(message),
            OrdersStateError::Conflict(message) => Self::conflict(message),
            OrdersStateError::MarketDataUnavailable(message) => Self::internal(message),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ByClientOrderIdQuery {
    client_order_id: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct ListOrdersQuery {
    status: Option<String>,
    symbols: Option<String>,
    side: Option<OrderSide>,
    asset_class: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderBody {
    symbol: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_decimal")]
    qty: Option<Decimal>,
    #[serde(default, deserialize_with = "deserialize_option_decimal")]
    notional: Option<Decimal>,
    side: Option<OrderSide>,
    #[serde(rename = "type")]
    r#type: Option<OrderType>,
    time_in_force: Option<TimeInForce>,
    #[serde(default, deserialize_with = "deserialize_option_decimal")]
    limit_price: Option<Decimal>,
    #[serde(default, deserialize_with = "deserialize_option_decimal")]
    stop_price: Option<Decimal>,
    #[serde(default, deserialize_with = "deserialize_option_decimal")]
    trail_price: Option<Decimal>,
    #[serde(default, deserialize_with = "deserialize_option_decimal")]
    trail_percent: Option<Decimal>,
    extended_hours: Option<bool>,
    client_order_id: Option<String>,
    order_class: Option<OrderClass>,
    take_profit: Option<TakeProfit>,
    stop_loss: Option<StopLoss>,
    legs: Option<Vec<OptionLegBody>>,
    position_intent: Option<PositionIntent>,
}

#[derive(Debug, Deserialize)]
pub struct OptionLegBody {
    symbol: String,
    #[serde(deserialize_with = "deserialize_u32")]
    ratio_qty: u32,
    side: Option<OrderSide>,
    position_intent: Option<PositionIntent>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ReplaceOrderBody {
    #[serde(default, deserialize_with = "deserialize_option_decimal")]
    qty: Option<Decimal>,
    time_in_force: Option<TimeInForce>,
    #[serde(default, deserialize_with = "deserialize_option_decimal")]
    limit_price: Option<Decimal>,
    #[serde(default, deserialize_with = "deserialize_option_decimal")]
    stop_price: Option<Decimal>,
    #[serde(default, deserialize_with = "deserialize_option_decimal")]
    trail: Option<Decimal>,
    client_order_id: Option<String>,
}

pub async fn orders_create(
    Extension(state): Extension<OrdersState>,
    Json(body): Json<CreateOrderBody>,
) -> RouteResult<Json<alpaca_trade::orders::Order>> {
    let order = state
        .create_order(CreateOrderInput {
            symbol: body.symbol,
            qty: body.qty,
            notional: body.notional,
            side: body.side,
            order_type: body.r#type,
            time_in_force: body.time_in_force,
            limit_price: body.limit_price,
            stop_price: body.stop_price,
            trail_price: body.trail_price,
            trail_percent: body.trail_percent,
            extended_hours: body.extended_hours,
            client_order_id: body.client_order_id,
            order_class: body.order_class,
            position_intent: body.position_intent,
            legs: body.legs.map(|legs| {
                legs.into_iter()
                    .map(|leg| OptionLegRequest {
                        symbol: leg.symbol,
                        ratio_qty: leg.ratio_qty,
                        side: leg.side,
                        position_intent: leg.position_intent,
                    })
                    .collect()
            }),
            take_profit: body.take_profit,
            stop_loss: body.stop_loss,
        })
        .await?;
    Ok(Json(order))
}

pub async fn orders_list(
    Extension(state): Extension<OrdersState>,
    Query(query): Query<ListOrdersQuery>,
) -> RouteResult<Json<Vec<alpaca_trade::orders::Order>>> {
    let symbols = query.symbols.map(|symbols| {
        symbols
            .split(',')
            .map(|symbol| symbol.trim().to_owned())
            .filter(|symbol| !symbol.is_empty())
            .collect::<Vec<_>>()
    });

    Ok(Json(state.list_orders(ListOrdersFilter {
        status: query.status,
        symbols,
        side: query.side,
        asset_class: query.asset_class,
    })))
}

pub async fn orders_get(
    Extension(state): Extension<OrdersState>,
    Path(order_id): Path<String>,
) -> RouteResult<Json<alpaca_trade::orders::Order>> {
    let order = state
        .get_order(&order_id)
        .ok_or_else(|| MockHttpError::not_found(format!("order {order_id} was not found")))?;
    Ok(Json(order))
}

pub async fn orders_get_by_client_order_id(
    Extension(state): Extension<OrdersState>,
    Query(query): Query<ByClientOrderIdQuery>,
) -> RouteResult<Json<alpaca_trade::orders::Order>> {
    let order = state
        .get_by_client_order_id(&query.client_order_id)
        .ok_or_else(|| {
            MockHttpError::not_found(format!(
                "client_order_id {} was not found",
                query.client_order_id
            ))
        })?;
    Ok(Json(order))
}

pub async fn orders_replace(
    Extension(state): Extension<OrdersState>,
    Path(order_id): Path<String>,
    Json(body): Json<ReplaceOrderBody>,
) -> RouteResult<Json<alpaca_trade::orders::Order>> {
    let order = state
        .replace_order(
            &order_id,
            ReplaceOrderInput {
                qty: body.qty,
                time_in_force: body.time_in_force,
                limit_price: body.limit_price,
                stop_price: body.stop_price,
                trail: body.trail,
                client_order_id: body.client_order_id,
            },
        )
        .await?;
    Ok(Json(order))
}

pub async fn orders_cancel(
    Extension(state): Extension<OrdersState>,
    Path(order_id): Path<String>,
) -> RouteResult<StatusCode> {
    state.cancel_order(&order_id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn orders_cancel_all(
    Extension(state): Extension<OrdersState>,
) -> RouteResult<Json<Vec<alpaca_trade::orders::CancelAllOrderResult>>> {
    Ok(Json(state.cancel_all_orders()))
}

fn deserialize_option_decimal<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Decimal::from_str(&value)
            .map(Some)
            .map_err(serde::de::Error::custom),
        Some(Value::Number(value)) => Decimal::from_str(&value.to_string())
            .map(Some)
            .map_err(serde::de::Error::custom),
        Some(other) => Err(serde::de::Error::custom(format!(
            "expected decimal string or number, got {other}"
        ))),
    }
}

fn deserialize_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(value) => value.parse::<u32>().map_err(serde::de::Error::custom),
        Value::Number(value) => value
            .to_string()
            .parse::<u32>()
            .map_err(serde::de::Error::custom),
        other => Err(serde::de::Error::custom(format!(
            "expected u32 string or number, got {other}"
        ))),
    }
}
