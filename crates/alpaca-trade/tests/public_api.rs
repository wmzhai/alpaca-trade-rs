use alpaca_trade::{Client, account::Account};

#[test]
fn public_api_exposes_account_types_and_accessor() {
    let client = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .build()
        .expect("client should build");

    let _ = client.account();
    let _ = Account::default();
}
