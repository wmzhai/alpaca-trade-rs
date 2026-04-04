use std::{collections::BTreeMap, sync::Arc};

use alpaca_trade::account::Account;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FaultRule {
    pub method: String,
    pub path: String,
    pub status: u16,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub body: String,
}

#[derive(Clone, Debug)]
pub struct SeedState {
    account: Account,
}

#[derive(Clone, Debug)]
struct RuntimeState {
    account: Account,
    faults: Vec<FaultRule>,
}

#[derive(Clone, Debug)]
pub struct AppState {
    seed: SeedState,
    runtime: Arc<RwLock<RuntimeState>>,
}

impl AppState {
    pub fn new() -> Self {
        let account = Account {
            id: "e6fe16f3-64a4-4921-8928-cadf02f92f98".into(),
            account_number: "010203ABCD".into(),
            status: "ACTIVE".into(),
            currency: Some("USD".into()),
            cash: Some("100000.00".into()),
            portfolio_value: Some("100000.00".into()),
            non_marginable_buying_power: Some("100000.00".into()),
            accrued_fees: Some("0".into()),
            pending_transfer_in: Some("0".into()),
            pending_transfer_out: Some("0".into()),
            pattern_day_trader: Some(false),
            trade_suspended_by_user: Some(false),
            trading_blocked: Some(false),
            transfers_blocked: Some(false),
            account_blocked: Some(false),
            created_at: Some("2019-06-12T22:47:07.99658Z".into()),
            shorting_enabled: Some(true),
            long_market_value: Some("0".into()),
            short_market_value: Some("0".into()),
            equity: Some("100000.00".into()),
            last_equity: Some("100000.00".into()),
            multiplier: Some("1".into()),
            buying_power: Some("100000.00".into()),
            initial_margin: Some("0".into()),
            maintenance_margin: Some("0".into()),
            sma: Some("0".into()),
            daytrade_count: Some(0),
            balance_asof: Some("2026-04-04".into()),
            last_maintenance_margin: Some("0".into()),
            daytrading_buying_power: Some("100000.00".into()),
            regt_buying_power: Some("100000.00".into()),
            options_buying_power: Some("100000.00".into()),
            options_approved_level: Some(0),
            options_trading_level: Some(0),
            intraday_adjustments: Some("0".into()),
            pending_reg_taf_fees: Some("0".into()),
        };

        Self {
            seed: SeedState {
                account: account.clone(),
            },
            runtime: Arc::new(RwLock::new(RuntimeState {
                account,
                faults: Vec::new(),
            })),
        }
    }

    pub fn account(&self) -> Account {
        self.runtime.read().account.clone()
    }

    pub fn clear_faults(&self) {
        self.runtime.write().faults.clear();
    }

    pub fn push_fault(&self, fault: FaultRule) {
        self.runtime.write().faults.push(fault);
    }

    pub fn reset(&self) {
        let mut runtime = self.runtime.write();
        runtime.account = self.seed.account.clone();
        runtime.faults.clear();
    }

    pub fn take_fault(&self, method: &str, path: &str) -> Option<FaultRule> {
        let mut runtime = self.runtime.write();
        let index = runtime
            .faults
            .iter()
            .position(|fault| fault.method == method && fault.path == path)?;
        Some(runtime.faults.remove(index))
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
