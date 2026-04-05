use alpaca_trade::{
    Client,
    account::Account,
    calendar::{Calendar, ListRequest},
    clock::Clock,
};

#[test]
fn public_api_exposes_account_calendar_and_clock_types_and_accessors() {
    let client = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .build()
        .expect("client should build");

    let _ = client.account();
    let _ = client.calendar();
    let _ = client.clock();
    let _ = Account::default();
    let _ = Calendar::default();
    let _ = ListRequest::default();
    let _ = Clock::default();
}

#[test]
fn clock_client_debug_does_not_expose_credentials() {
    let client = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .build()
        .expect("client should build");

    let debug = format!("{:?}", client.clock());

    assert!(
        !debug.contains("key"),
        "debug output leaked api key: {debug}"
    );
    assert!(
        !debug.contains("secret"),
        "debug output leaked secret key: {debug}"
    );
}
