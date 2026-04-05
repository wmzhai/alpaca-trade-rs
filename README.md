# alpaca-trade-rs

`alpaca-trade-rs` is a Rust workspace for the non-crypto Alpaca Trading HTTP REST API.

## Current Status

- Phase 1 scope: `account`
- API surface: non-crypto Alpaca Trading HTTP REST only
- Explicit exclusions: stream / websocket APIs, crypto trading APIs
- Published crate: `alpaca-trade`
- Internal workspace tool: `alpaca-trade-mock`
- Default client environment: Alpaca Paper Trading
- Phase 1 happy-path testing: live-first against the official paper endpoint

## Workspace

- `crates/alpaca-trade`: async Trading API client
- `crates/alpaca-trade-mock`: internal workspace helper for future market-hours-sensitive Trading API tests
- `tools/api-coverage/trading-api.json`: family-level coverage manifest for Trading HTTP REST audit work

## Phase 1 API

```rust
use alpaca_trade::Client;

# async fn demo() -> Result<(), alpaca_trade::Error> {
let client = Client::builder()
    .api_key(std::env::var("ALPACA_TRADE_API_KEY").expect("ALPACA_TRADE_API_KEY is required"))
    .secret_key(std::env::var("ALPACA_TRADE_SECRET_KEY").expect("ALPACA_TRADE_SECRET_KEY is required"))
    .build()?;

let account = client.account().get().await?;
println!("{}", account.status);
# Ok(())
# }
```

## Phase 1 Testing

Create a local root `.env` file with `ALPACA_TRADE_API_KEY=...` and `ALPACA_TRADE_SECRET_KEY=...`.

Run the full automated test suite with `cargo test --workspace -- --nocapture`.

Notes:
- `account_model` and `account_transport` stay local/offline.
- `account_live` talks to the official Alpaca Paper API.
- If `.env` credentials are missing, the live test prints a skip message and exits successfully.
