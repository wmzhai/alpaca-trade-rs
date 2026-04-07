use std::collections::HashMap;

use alpaca_trade::Decimal;

pub const DEFAULT_STOCK_SYMBOL: &str = "SPY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrumentSnapshot {
    pub asset_class: String,
    pub bid: Decimal,
    pub ask: Decimal,
}

impl InstrumentSnapshot {
    pub fn equity(bid: Decimal, ask: Decimal) -> Self {
        Self {
            asset_class: "us_equity".to_owned(),
            bid,
            ask,
        }
    }

    pub fn option(bid: Decimal, ask: Decimal) -> Self {
        Self {
            asset_class: "us_option".to_owned(),
            bid,
            ask,
        }
    }

    pub fn mid_price(&self) -> Decimal {
        mid_price(self.bid, self.ask)
    }
}

#[derive(Debug, Clone, Default)]
pub struct OrdersMarketSnapshot {
    instruments: HashMap<String, InstrumentSnapshot>,
}

impl OrdersMarketSnapshot {
    pub fn with_instrument(
        mut self,
        symbol: impl Into<String>,
        instrument: InstrumentSnapshot,
    ) -> Self {
        self.instruments.insert(symbol.into(), instrument);
        self
    }

    pub fn instrument(&self, symbol: &str) -> Option<InstrumentSnapshot> {
        self.instruments.get(symbol).cloned()
    }

    pub fn default_option_symbol(&self) -> Option<&str> {
        self.instruments.iter().find_map(|(symbol, instrument)| {
            if instrument.asset_class == "us_option" {
                Some(symbol.as_str())
            } else {
                None
            }
        })
    }
}

pub fn mid_price(bid: Decimal, ask: Decimal) -> Decimal {
    ((bid + ask) / Decimal::new(2, 0)).round_dp(2)
}
