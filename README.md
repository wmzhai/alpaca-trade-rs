# alpaca-trade-rs

`alpaca-trade-rs` is a Rust workspace for the non-crypto Alpaca Trading HTTP REST API.

## Current Status

- Phase 7 milestone: `orders`
- Implemented resources: `account`, `clock`, `calendar`, `assets`, `options_contracts`, `orders`
- Next resource phase: `positions` (Phase 8)
- API surface: non-crypto Alpaca Trading HTTP REST only
- Explicit exclusions: stream / websocket APIs, crypto trading APIs
- Published crate: `alpaca-trade`
- Internal workspace tool: `alpaca-trade-mock`
- Default client environment: Alpaca Paper
- Testing taxonomy: `live_readonly`, `paper_mutating_with_cleanup`, `mock_stateful`, `fault_injection_only`
- Default retry behavior: automatic retry is limited to `GET`
- Retry semantics: `max_get_attempts` counts total attempts, so `1` disables retry and `2` means one retry after the first failed `GET`
- Numeric model policy: high-precision financial fields in the public Rust API use `alpaca_trade::Decimal`, while request/response wire shapes still mirror the official Alpaca contract
- Benchmark note: Phase 6 reuses the existing read-only `GET` and pagination foundation, so the current milestone does not add a dedicated benchmark track

## Workspace

- `crates/alpaca-trade`: async Trading API client
- `crates/alpaca-trade-mock`: internal workspace helper for stateful `orders` fallback tests outside dedicated Paper mutating windows
- `tools/api-coverage/trading-api.json`: family-level coverage manifest for Trading HTTP REST audit work

## Implemented API

```rust
use alpaca_trade::{Client, assets::ListRequest};

# async fn demo() -> Result<(), alpaca_trade::Error> {
let client = Client::builder()
    .api_key(std::env::var("APCA_API_KEY_ID").expect("APCA_API_KEY_ID is required"))
    .secret_key(std::env::var("APCA_API_SECRET_KEY").expect("APCA_API_SECRET_KEY is required"))
    .paper()
    .build()?;

let assets = client
    .assets()
    .list(ListRequest {
        status: Some("active".into()),
        asset_class: Some("us_equity".into()),
        exchange: Some("NASDAQ".into()),
        attributes: Some(vec!["has_options".into()]),
    })
    .await?;
println!("{} {}", assets[0].symbol, assets[0].status);
# Ok(())
# }
```

```rust
use alpaca_trade::Client;
use alpaca_trade::options_contracts::{ContractStatus, ListRequest};

# async fn demo() -> Result<(), alpaca_trade::Error> {
let client = Client::builder()
    .api_key(std::env::var("APCA_API_KEY_ID").expect("APCA_API_KEY_ID is required"))
    .secret_key(std::env::var("APCA_API_SECRET_KEY").expect("APCA_API_SECRET_KEY is required"))
    .paper()
    .build()?;

let response = client
    .options_contracts()
    .list(ListRequest {
        underlying_symbols: Some(vec!["SPY".into()]),
        status: Some(ContractStatus::Active),
        limit: Some(1),
        ..ListRequest::default()
    })
    .await?;

println!("{}", response.option_contracts[0].symbol);
# Ok(())
# }
```

## Examples

Set `APCA_API_KEY_ID` and `APCA_API_SECRET_KEY`, then run one of:

```sh
cargo run -p alpaca-trade --example client_builder
cargo run -p alpaca-trade --example account_get
cargo run -p alpaca-trade --example assets_list
```

## Testing

Create a local root `.env` file with either:

- `APCA_API_KEY_ID=...` and `APCA_API_SECRET_KEY=...`
- `ALPACA_TRADE_API_KEY=...` and `ALPACA_TRADE_SECRET_KEY=...`

Run the full automated test suite with `cargo test --workspace -- --nocapture`.

Notes:
- `account_model`, `account_transport`, `clock_model`, `clock_transport`, `calendar_model`, `calendar_transport`, `assets_model`, `assets_transport`, `options_contracts_model`, and `options_contracts_transport` stay local/offline.
- `account_live`, `clock_live`, `calendar_live`, `assets_live`, and `options_contracts_live` are the current `live_readonly` credential-gated smoke paths against the official Alpaca Paper API.
- `orders_mutating` automatically uses dedicated-account Paper mutating coverage during market hours and falls back to `alpaca-trade-mock` outside that window or when the dedicated-account marker is unavailable.
- Set `ALPACA_TRADE_ORDERS_TEST_ACCOUNT=1` on the dedicated Paper test account to enable the real `paper_mutating_with_cleanup` path, including the guarded `cancel_all()` coverage.
- The live test helper accepts both the standard `APCA_*` names and the repo-local `ALPACA_TRADE_*` aliases.
- If `.env` credentials are missing, the live tests print skip messages and exit successfully, so a green local run may not include a real paper request.
