use alpaca_trade::{
    Client, NoopObserver, RetryPolicy,
    account::Account,
    assets::{Asset, ListRequest as AssetsListRequest},
    calendar::{Calendar, ListRequest as CalendarListRequest},
    clock::Clock,
};

const API_KEY_SENTINEL: &str = "api-key-sentinel-7f4d0c1a";
const SECRET_KEY_SENTINEL: &str = "secret-key-sentinel-9b82e6f3";
const URL_SECRET_SENTINEL: &str = "url-secret-sentinel-5c11aa2d";

fn assert_debug_redacts(debug: &str) {
    assert!(
        !debug.contains(API_KEY_SENTINEL),
        "debug output leaked api key: {debug}"
    );
    assert!(
        !debug.contains(SECRET_KEY_SENTINEL),
        "debug output leaked secret key: {debug}"
    );
    assert!(
        !debug.contains(URL_SECRET_SENTINEL),
        "debug output leaked secret-bearing url fragment: {debug}"
    );
}

#[test]
fn public_api_exposes_account_assets_calendar_and_clock_types_and_accessors() {
    let client = Client::builder()
        .api_key(API_KEY_SENTINEL)
        .secret_key(SECRET_KEY_SENTINEL)
        .build()
        .expect("client should build");

    let _ = client.account();
    let _ = client.assets();
    let _ = client.calendar();
    let _ = client.clock();
    let _ = Account::default();
    let _ = Asset::default();
    let _ = AssetsListRequest::default();
    let _ = Calendar::default();
    let _ = CalendarListRequest::default();
    let _ = Clock::default();
}

#[test]
fn public_api_exposes_builder_retry_and_observer_surface() {
    #[derive(Debug, Default)]
    struct TestObserver;

    impl alpaca_trade::Observer for TestObserver {}

    let client = Client::builder()
        .api_key(API_KEY_SENTINEL)
        .secret_key(SECRET_KEY_SENTINEL)
        .observer(TestObserver)
        .retry_policy(RetryPolicy::trading_safe())
        .build()
        .expect("client should build");

    let _ = client.account();
    let _ = NoopObserver;
}

#[test]
fn clock_client_debug_does_not_expose_credentials() {
    let client = Client::builder()
        .api_key(API_KEY_SENTINEL)
        .secret_key(SECRET_KEY_SENTINEL)
        .build()
        .expect("client should build");

    let debug = format!("{:?}", client.clock());

    assert_debug_redacts(&debug);
}

#[test]
fn builder_debug_does_not_expose_credentials() {
    let builder = Client::builder()
        .api_key(API_KEY_SENTINEL)
        .secret_key(SECRET_KEY_SENTINEL);

    let debug = format!("{:?}", builder);

    assert_debug_redacts(&debug);
}

#[test]
fn client_debug_does_not_expose_credentials() {
    let client = Client::builder()
        .api_key(API_KEY_SENTINEL)
        .secret_key(SECRET_KEY_SENTINEL)
        .build()
        .expect("client should build");

    let debug = format!("{:?}", client);

    assert_debug_redacts(&debug);
}

#[test]
fn client_debug_does_not_expose_custom_base_url_secrets() {
    let client = Client::builder()
        .api_key(API_KEY_SENTINEL)
        .secret_key(SECRET_KEY_SENTINEL)
        .base_url(format!("https://user:{URL_SECRET_SENTINEL}@example.com"))
        .build()
        .expect("client should build");

    let debug = format!("{:?}", client);

    assert_debug_redacts(&debug);
}

#[test]
fn account_client_debug_does_not_expose_credentials() {
    let client = Client::builder()
        .api_key(API_KEY_SENTINEL)
        .secret_key(SECRET_KEY_SENTINEL)
        .build()
        .expect("client should build");

    let debug = format!("{:?}", client.account());

    assert_debug_redacts(&debug);
}

#[test]
fn calendar_client_debug_does_not_expose_credentials() {
    let client = Client::builder()
        .api_key(API_KEY_SENTINEL)
        .secret_key(SECRET_KEY_SENTINEL)
        .build()
        .expect("client should build");

    let debug = format!("{:?}", client.calendar());

    assert_debug_redacts(&debug);
}

#[test]
fn assets_client_debug_does_not_expose_credentials() {
    let client = Client::builder()
        .api_key(API_KEY_SENTINEL)
        .secret_key(SECRET_KEY_SENTINEL)
        .build()
        .expect("client should build");

    let debug = format!("{:?}", client.assets());

    assert_debug_redacts(&debug);
}
