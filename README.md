# alpaca-trade-rs

`alpaca-trade-rs` is a Rust workspace for the non-crypto Alpaca Trading HTTP REST API.

## Current Status

- Phase 6 milestone: `options_contracts`
- Implemented resources: `account`, `clock`, `calendar`, `assets`, `options_contracts`
- Next resource phase: `orders` (Phase 7)
- API surface: non-crypto Alpaca Trading HTTP REST only
- Explicit exclusions: stream / websocket APIs, crypto trading APIs
- Published crate: `alpaca-trade`
- Internal workspace tool: `alpaca-trade-mock`
- Default client environment: Alpaca Paper
- Testing taxonomy: `live_readonly`, `paper_mutating_with_cleanup`, `mock_stateful`, `fault_injection_only`
- Default retry behavior: automatic retry is limited to `GET`
- Retry semantics: `max_get_attempts` counts total attempts, so `1` disables retry and `2` means one retry after the first failed `GET`
- Numeric model policy: high-precision financial fields in the public Rust API use `alpaca_trade::Decimal`, while request/response wire shapes still mirror the official Alpaca contract
- Benchmark note: Phase 5 does not add a dedicated benchmark because assets introduces two straightforward read-only `GET` endpoints without a new pagination or transport primitive

## Workspace

- `crates/alpaca-trade`: async Trading API client
- `crates/alpaca-trade-mock`: internal workspace helper for future market-hours-sensitive Trading API tests
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
- `account_model`, `account_transport`, `clock_model`, `clock_transport`, `calendar_model`, `calendar_transport`, `assets_model`, and `assets_transport` stay local/offline.
- `account_live`, `clock_live`, `calendar_live`, and `assets_live` are the current `live_readonly` credential-gated smoke paths against the official Alpaca Paper API.
- Future mutating families will follow the `paper_mutating_with_cleanup` or `mock_stateful` taxonomy instead of reusing the read-only smoke path.
- The live test helper accepts both the standard `APCA_*` names and the repo-local `ALPACA_TRADE_*` aliases.
- If `.env` credentials are missing, the live tests print skip messages and exit successfully, so a green local run may not include a real paper request.
