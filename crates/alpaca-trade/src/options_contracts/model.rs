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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OptionContract {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OptionDeliverable {
    pub symbol: String,
}
