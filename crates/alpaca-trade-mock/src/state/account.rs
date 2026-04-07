use alpaca_trade::Decimal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountProfile {
    pub id: String,
    pub account_number: String,
    pub status: String,
    pub currency: String,
}

impl AccountProfile {
    pub fn new(api_key: &str) -> Self {
        Self {
            id: api_key.to_owned(),
            account_number: format!("mock-account-{api_key}"),
            status: "ACTIVE".to_owned(),
            currency: "USD".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashLedger {
    cash: Decimal,
}

impl CashLedger {
    pub fn seeded_default() -> Self {
        Self {
            cash: Decimal::new(1_000_000, 0),
        }
    }

    pub fn cash_balance(&self) -> Decimal {
        self.cash
    }

    pub fn apply_delta(&mut self, delta: Decimal) {
        self.cash += delta;
    }
}
