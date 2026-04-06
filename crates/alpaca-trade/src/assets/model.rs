use crate::common::decimal::{
    deserialize_option_decimal_from_string_or_number as deserialize_decimal,
    number_contract::serialize_option_decimal as serialize_decimal_number,
    string_contract::serialize_option_decimal as serialize_decimal_string,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub class: String,
    pub exchange: String,
    pub symbol: String,
    pub name: String,
    pub status: String,
    pub tradable: bool,
    pub marginable: bool,
    pub shortable: bool,
    pub easy_to_borrow: bool,
    pub fractionable: bool,
    pub cusip: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal_number"
    )]
    pub maintenance_margin_requirement: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal_string"
    )]
    pub margin_requirement_long: Option<Decimal>,
    #[serde(
        default,
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal_string"
    )]
    pub margin_requirement_short: Option<Decimal>,
    pub attributes: Option<Vec<String>>,
}
