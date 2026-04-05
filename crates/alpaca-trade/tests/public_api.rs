use alpaca_trade::{Client, account::Account, clock::Clock};

#[test]
fn public_api_exposes_account_and_clock_types_and_accessors() {
    let client = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .build()
        .expect("client should build");

    let _ = client.account();
    let _ = client.clock();
    let _ = Account::default();
    let _ = Clock::default();
}
