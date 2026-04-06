use crate::common::decimal::{
    deserialize_decimal_from_string_or_number as deserialize_decimal,
    deserialize_option_decimal_from_string_or_number as deserialize_option_decimal,
    string_contract::{serialize_decimal, serialize_option_decimal},
};
use crate::common::integer::{
    deserialize_option_u32_from_string_or_number as deserialize_option_u32,
    string_contract::serialize_option_u32,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum QueryOrderStatus {
    Open,
    Closed,
    All,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OrderSide {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
    Sell,
    #[serde(rename = "")]
    Unspecified,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum OrderType {
    Market,
    Limit,
    Stop,
    StopLimit,
    TrailingStop,
    #[serde(rename = "")]
    Unspecified,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum TimeInForce {
    Day,
    Gtc,
    Opg,
    Cls,
    Ioc,
    Fok,
    Gtd,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PositionIntent {
    BuyToOpen,
    BuyToClose,
    SellToOpen,
    SellToClose,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OrderClass {
    #[serde(rename = "simple", alias = "")]
    Simple,
    #[serde(rename = "bracket")]
    Bracket,
    #[serde(rename = "oco")]
    Oco,
    #[serde(rename = "oto")]
    Oto,
    #[serde(rename = "mleg")]
    Mleg,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    DoneForDay,
    Canceled,
    Expired,
    Replaced,
    PendingCancel,
    PendingReplace,
    Accepted,
    PendingNew,
    AcceptedForBidding,
    Stopped,
    Rejected,
    Suspended,
    Calculated,
    Held,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TakeProfit {
    #[serde(
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub limit_price: Decimal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StopLoss {
    #[serde(
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub stop_price: Decimal,
    #[serde(
        default,
        deserialize_with = "deserialize_option_decimal",
        serialize_with = "serialize_option_decimal"
    )]
    pub limit_price: Option<Decimal>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Order {
    pub id: String,
    pub client_order_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub submitted_at: String,
    pub filled_at: Option<String>,
    pub expired_at: Option<String>,
    pub expires_at: Option<String>,
    pub canceled_at: Option<String>,
    pub failed_at: Option<String>,
    pub replaced_at: Option<String>,
    pub replaced_by: Option<String>,
    pub replaces: Option<String>,
    pub asset_id: String,
    pub symbol: String,
    pub asset_class: String,
    #[serde(
        default,
        deserialize_with = "deserialize_option_decimal",
        serialize_with = "serialize_option_decimal"
    )]
    pub notional: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_option_decimal",
        serialize_with = "serialize_option_decimal"
    )]
    pub qty: Option<Decimal>,
    #[serde(
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub filled_qty: Decimal,
    #[serde(
        default,
        deserialize_with = "deserialize_option_decimal",
        serialize_with = "serialize_option_decimal"
    )]
    pub filled_avg_price: Option<Decimal>,
    pub order_class: OrderClass,
    pub order_type: OrderType,
    #[serde(rename = "type")]
    pub r#type: OrderType,
    pub side: OrderSide,
    pub position_intent: Option<PositionIntent>,
    pub time_in_force: TimeInForce,
    #[serde(
        default,
        deserialize_with = "deserialize_option_decimal",
        serialize_with = "serialize_option_decimal"
    )]
    pub limit_price: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_option_decimal",
        serialize_with = "serialize_option_decimal"
    )]
    pub stop_price: Option<Decimal>,
    pub status: OrderStatus,
    pub extended_hours: bool,
    pub legs: Option<Vec<Order>>,
    #[serde(
        default,
        deserialize_with = "deserialize_option_decimal",
        serialize_with = "serialize_option_decimal"
    )]
    pub trail_percent: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_option_decimal",
        serialize_with = "serialize_option_decimal"
    )]
    pub trail_price: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_option_decimal",
        serialize_with = "serialize_option_decimal"
    )]
    pub hwm: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_option_u32",
        serialize_with = "serialize_option_u32"
    )]
    pub ratio_qty: Option<u32>,
    pub take_profit: Option<TakeProfit>,
    pub stop_loss: Option<StopLoss>,
    pub subtag: Option<String>,
    pub source: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CancelAllOrderResult {
    pub id: String,
    pub status: u16,
    pub body: Option<Order>,
}
