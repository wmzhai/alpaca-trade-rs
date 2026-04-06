use crate::common::decimal::{
    deserialize_decimal_from_string_or_number as deserialize_decimal,
    deserialize_option_decimal_from_string_or_number as deserialize_option_decimal,
    string_contract::{serialize_decimal, serialize_option_decimal},
};
use crate::common::pagination::PaginatedResponse;
use crate::error::Error;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContractStatus {
    Active,
    Inactive,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContractType {
    Call,
    Put,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContractStyle {
    American,
    European,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeliverableType {
    Cash,
    Equity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliverableSettlementType {
    #[serde(rename = "T+0")]
    TPlus0,
    #[serde(rename = "T+1")]
    TPlus1,
    #[serde(rename = "T+2")]
    TPlus2,
    #[serde(rename = "T+3")]
    TPlus3,
    #[serde(rename = "T+4")]
    TPlus4,
    #[serde(rename = "T+5")]
    TPlus5,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliverableSettlementMethod {
    #[serde(rename = "BTOB")]
    Btob,
    #[serde(rename = "CADF")]
    Cadf,
    #[serde(rename = "CAFX")]
    Cafx,
    #[serde(rename = "CCC")]
    Ccc,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ListResponse {
    pub option_contracts: Vec<OptionContract>,
    pub next_page_token: Option<String>,
}

impl PaginatedResponse for ListResponse {
    fn next_page_token(&self) -> Option<&str> {
        self.next_page_token.as_deref()
    }

    fn merge_page(&mut self, next: Self) -> Result<(), Error> {
        self.option_contracts.extend(next.option_contracts);
        self.next_page_token = next.next_page_token;
        Ok(())
    }

    fn clear_next_page_token(&mut self) {
        self.next_page_token = None;
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OptionContract {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub status: ContractStatus,
    pub tradable: bool,
    pub expiration_date: String,
    pub root_symbol: Option<String>,
    pub underlying_symbol: String,
    pub underlying_asset_id: String,
    #[serde(rename = "type")]
    pub r#type: ContractType,
    pub style: ContractStyle,
    #[serde(
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub strike_price: Decimal,
    #[serde(
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub multiplier: Decimal,
    #[serde(
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub size: Decimal,
    #[serde(
        default,
        deserialize_with = "deserialize_option_decimal",
        serialize_with = "serialize_option_decimal"
    )]
    pub open_interest: Option<Decimal>,
    pub open_interest_date: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_option_decimal",
        serialize_with = "serialize_option_decimal"
    )]
    pub close_price: Option<Decimal>,
    pub close_price_date: Option<String>,
    pub deliverables: Option<Vec<OptionDeliverable>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OptionDeliverable {
    #[serde(rename = "type")]
    pub r#type: DeliverableType,
    pub symbol: String,
    pub asset_id: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_option_decimal",
        serialize_with = "serialize_option_decimal"
    )]
    pub amount: Option<Decimal>,
    #[serde(
        deserialize_with = "deserialize_decimal",
        serialize_with = "serialize_decimal"
    )]
    pub allocation_percentage: Decimal,
    pub settlement_type: DeliverableSettlementType,
    pub settlement_method: DeliverableSettlementMethod,
    pub delayed_settlement: bool,
}

#[cfg(test)]
mod tests {
    use super::{ContractStatus, ContractStyle, ContractType, ListResponse, OptionContract};
    use crate::common::pagination::PaginatedResponse;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn contract(symbol: &str) -> OptionContract {
        OptionContract {
            id: format!("id-{symbol}"),
            symbol: symbol.to_owned(),
            name: format!("{symbol} Jun 20 2025 100 Call"),
            status: ContractStatus::Active,
            tradable: true,
            expiration_date: "2025-06-20".to_owned(),
            root_symbol: Some(symbol.to_owned()),
            underlying_symbol: symbol.to_owned(),
            underlying_asset_id: format!("asset-{symbol}"),
            r#type: ContractType::Call,
            style: ContractStyle::American,
            strike_price: Decimal::from_str("100").unwrap(),
            multiplier: Decimal::from_str("100").unwrap(),
            size: Decimal::from_str("100").unwrap(),
            open_interest: None,
            open_interest_date: None,
            close_price: None,
            close_price_date: None,
            deliverables: None,
        }
    }

    #[test]
    fn list_response_merge_page_appends_contracts_and_updates_next_page_token() {
        let mut combined = ListResponse {
            option_contracts: vec![contract("AAPL")],
            next_page_token: Some("cursor-2".into()),
        };

        combined
            .merge_page(ListResponse {
                option_contracts: vec![contract("SPY")],
                next_page_token: Some("cursor-3".into()),
            })
            .expect("merge should succeed");

        assert_eq!(combined.option_contracts.len(), 2);
        assert_eq!(combined.option_contracts[1].symbol, "SPY");
        assert_eq!(combined.next_page_token.as_deref(), Some("cursor-3"));
    }

    #[test]
    fn list_response_clear_next_page_token_removes_cursor() {
        let mut response = ListResponse {
            option_contracts: vec![contract("AAPL")],
            next_page_token: Some("cursor-2".into()),
        };

        response.clear_next_page_token();

        assert_eq!(response.next_page_token, None);
    }
}
