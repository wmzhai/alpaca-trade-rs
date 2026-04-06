use alpaca_trade::{
    Client, Decimal, NoopObserver, RetryPolicy,
    account::Account,
    assets::{Asset, ListRequest as AssetsListRequest},
    calendar::{Calendar, ListRequest as CalendarListRequest},
    clock::Clock,
    orders::{
        CancelAllOrderResult, CreateRequest as OrdersCreateRequest, ListRequest as OrdersListRequest,
        OptionLegRequest, Order, OrderClass, OrderSide, OrderStatus, OrderType, OrdersClient,
        PositionIntent, QueryOrderStatus, ReplaceRequest as OrdersReplaceRequest, TimeInForce,
    },
    options_contracts::{
        ContractStatus, ContractStyle, ContractType, DeliverableSettlementMethod,
        DeliverableSettlementType, DeliverableType, ListRequest as OptionsContractsListRequest,
        ListResponse, OptionContract, OptionDeliverable,
    },
};
use std::fs;

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
fn public_api_exposes_account_assets_calendar_clock_options_contracts_and_orders_types_and_accessors() {
    let client = Client::builder()
        .api_key(API_KEY_SENTINEL)
        .secret_key(SECRET_KEY_SENTINEL)
        .build()
        .expect("client should build");

    let _ = client.account();
    let _ = client.assets();
    let _ = client.calendar();
    let _ = client.clock();
    let _ = client.orders();
    let _ = client.options_contracts();
    let account = Account::default();
    let _: Option<Decimal> = account.cash.clone();
    let _: Option<Decimal> = account.buying_power.clone();
    let _: fn(Asset) -> Option<Decimal> = |asset| asset.maintenance_margin_requirement.clone();
    let _: fn(Asset) -> Option<Decimal> = |asset| asset.margin_requirement_long.clone();
    let _: fn(Asset) -> Option<Decimal> = |asset| asset.margin_requirement_short.clone();
    let _: Option<Asset> = None;
    let _: Option<CancelAllOrderResult> = None;
    let _: Option<ListResponse> = None;
    let _: Option<Order> = None;
    let _: Option<OrdersClient> = None;
    let _: Option<OptionContract> = None;
    let _: Option<OptionDeliverable> = None;
    let _: fn(ListResponse) -> Option<String> = |response| response.next_page_token;
    let _: fn(OptionContract) -> Decimal = |contract| contract.strike_price;
    let _: fn(OptionContract) -> Option<Vec<OptionDeliverable>> = |contract| contract.deliverables;
    let _: fn(OptionDeliverable) -> DeliverableType = |deliverable| deliverable.r#type;
    let _ = AssetsListRequest::default();
    let _ = Calendar::default();
    let _ = CalendarListRequest::default();
    let _ = Clock::default();
    let _ = OrdersCreateRequest::default();
    let _ = OrdersListRequest::default();
    let _ = OptionLegRequest::default();
    let _ = OrdersReplaceRequest::default();
    let _ = OptionsContractsListRequest::default();
    let _ = OrderClass::Simple;
    let _ = OrderSide::Buy;
    let _ = OrderStatus::Accepted;
    let _ = OrderType::Market;
    let _ = PositionIntent::BuyToOpen;
    let _ = QueryOrderStatus::Open;
    let _ = TimeInForce::Day;
    let _ = ContractStatus::Active;
    let _ = ContractType::Call;
    let _ = ContractStyle::American;
    let _ = DeliverableType::Equity;
    let _ = DeliverableSettlementType::TPlus2;
    let _ = DeliverableSettlementMethod::Ccc;
    let _ = Decimal::new(12345, 2);
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
fn options_contracts_client_debug_does_not_expose_credentials() {
    let client = Client::builder()
        .api_key(API_KEY_SENTINEL)
        .secret_key(SECRET_KEY_SENTINEL)
        .build()
        .expect("client should build");

    let debug = format!("{:?}", client.options_contracts());

    assert_debug_redacts(&debug);
}

#[test]
fn orders_client_debug_does_not_expose_credentials() {
    let client = Client::builder()
        .api_key(API_KEY_SENTINEL)
        .secret_key(SECRET_KEY_SENTINEL)
        .build()
        .expect("client should build");

    let debug = format!("{:?}", client.orders());

    assert_debug_redacts(&debug);
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

#[test]
fn options_contracts_wire_enums_are_non_exhaustive() {
    let model = fs::read_to_string(format!(
        "{}/src/options_contracts/model.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("options_contracts model source should be readable");

    for enum_name in [
        "ContractStatus",
        "ContractType",
        "ContractStyle",
        "DeliverableType",
        "DeliverableSettlementType",
        "DeliverableSettlementMethod",
    ] {
        let marker = format!("#[non_exhaustive]\npub enum {enum_name}");
        assert!(
            model.contains(&marker),
            "{enum_name} should remain non_exhaustive"
        );
    }
}
