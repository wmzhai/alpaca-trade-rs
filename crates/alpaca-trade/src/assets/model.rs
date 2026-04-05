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
    pub maintenance_margin_requirement: Option<f64>,
    pub margin_requirement_long: Option<String>,
    pub margin_requirement_short: Option<String>,
    pub attributes: Option<Vec<String>>,
}
