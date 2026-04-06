use crate::common::decimal::{
    deserialize_option_decimal_from_string_or_number as deserialize_decimal,
    string_contract::serialize_option_decimal as serialize_decimal,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub account_number: String,
    pub status: String,
    pub currency: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub cash: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub portfolio_value: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub non_marginable_buying_power: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub accrued_fees: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub pending_transfer_in: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub pending_transfer_out: Option<Decimal>,
    pub pattern_day_trader: Option<bool>,
    pub trade_suspended_by_user: Option<bool>,
    pub trading_blocked: Option<bool>,
    pub transfers_blocked: Option<bool>,
    pub account_blocked: Option<bool>,
    pub created_at: Option<String>,
    pub shorting_enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub long_market_value: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub short_market_value: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub equity: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub last_equity: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub multiplier: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub buying_power: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub initial_margin: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub maintenance_margin: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub sma: Option<Decimal>,
    pub daytrade_count: Option<i64>,
    pub balance_asof: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub last_maintenance_margin: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub daytrading_buying_power: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub regt_buying_power: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub options_buying_power: Option<Decimal>,
    pub options_approved_level: Option<i64>,
    pub options_trading_level: Option<i64>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub intraday_adjustments: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub pending_reg_taf_fees: Option<Decimal>,
}
