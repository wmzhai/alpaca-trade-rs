# alpaca-trade-rs

`alpaca-trade-rs` is a Rust workspace for the non-crypto Alpaca Trading HTTP REST API.

## Current Status

- Phase 3 scope: `account`, `clock`, `calendar`
- API surface: non-crypto Alpaca Trading HTTP REST only
- Explicit exclusions: stream / websocket APIs, crypto trading APIs
- Published crate: `alpaca-trade`
- Internal workspace tool: `alpaca-trade-mock`
- Default client environment: Alpaca Paper Trading
- Phase 3 happy-path testing: live-first, with credential-gated Alpaca Paper smoke coverage when credentials are available

## Workspace

- `crates/alpaca-trade`: async Trading API client
- `crates/alpaca-trade-mock`: internal workspace helper for future market-hours-sensitive Trading API tests
- `tools/api-coverage/trading-api.json`: family-level coverage manifest for Trading HTTP REST audit work

## Phase 3 API

```rust
use alpaca_trade::{Client, calendar::ListRequest};

# async fn demo() -> Result<(), alpaca_trade::Error> {
let client = Client::builder()
    .api_key(std::env::var("APCA_API_KEY_ID").expect("APCA_API_KEY_ID is required"))
    .secret_key(std::env::var("APCA_API_SECRET_KEY").expect("APCA_API_SECRET_KEY is required"))
    .build()?;

let calendar = client
    .calendar()
    .list(ListRequest {
        start: Some("2026-04-01".into()),
        end: Some("2026-04-03".into()),
    })
    .await?;
println!("{} {} {}", calendar[0].date, calendar[0].open, calendar[0].close);
# Ok(())
# }
```

## Phase 3 Testing

Create a local root `.env` file with either:

- `APCA_API_KEY_ID=...` and `APCA_API_SECRET_KEY=...`
- `ALPACA_TRADE_API_KEY=...` and `ALPACA_TRADE_SECRET_KEY=...`

Run the full automated test suite with `cargo test --workspace -- --nocapture`.

Notes:
- `account_model`, `account_transport`, `clock_model`, `clock_transport`, `calendar_model`, and `calendar_transport` stay local/offline.
- `account_live`, `clock_live`, and `calendar_live` are the credential-gated live smoke paths against the official Alpaca Paper API.
- The live test helper accepts both the standard `APCA_*` names and the repo-local `ALPACA_TRADE_*` aliases.
- If `.env` credentials are missing, the live test prints a skip message and exits successfully, so a green local run may not include a real paper request.
