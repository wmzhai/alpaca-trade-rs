mod support;

use alpaca_trade::Client;
use alpaca_trade::options_contracts::{ContractStatus, ListRequest};

#[tokio::test]
async fn options_contracts_live_lists_and_gets_active_contract() {
    let Some(credentials) = support::trade_credentials() else {
        eprintln!(
            "skipping live options_contracts test: missing ALPACA_TRADE_API_KEY / ALPACA_TRADE_SECRET_KEY or APCA_API_KEY_ID / APCA_API_SECRET_KEY"
        );
        return;
    };

    let client = Client::builder()
        .api_key(credentials.api_key)
        .secret_key(credentials.secret_key)
        .paper()
        .build()
        .expect("client should build");

    let contracts = client
        .options_contracts()
        .list(ListRequest {
            underlying_symbols: Some(vec!["SPY".into()]),
            status: Some(ContractStatus::Active),
            limit: Some(1),
            ..ListRequest::default()
        })
        .await
        .expect("live options_contracts list request should succeed");

    let first = contracts
        .option_contracts
        .first()
        .expect("live options_contracts list should return at least one contract");

    assert!(!first.symbol.is_empty());
    assert_eq!(first.status, ContractStatus::Active);
    assert_eq!(first.underlying_symbol, "SPY");

    let contract = client
        .options_contracts()
        .get(&first.symbol)
        .await
        .expect("live options_contracts get request should succeed");

    assert_eq!(contract.symbol, first.symbol);
    assert_eq!(contract.underlying_symbol, first.underlying_symbol);
}
