# alpaca-trade-rs

`alpaca-trade-rs` is a Rust workspace for the non-crypto Alpaca Trading HTTP REST API.

## Current Status

- Phase 1 scope: `account`
- API surface: non-crypto Alpaca Trading HTTP REST only
- Explicit exclusions: stream / websocket APIs, crypto trading APIs
- Workspace crates:
  - `alpaca-trade`
  - `alpaca-trade-mock`
- Default client environment: Alpaca Paper Trading
- Primary test backend: local in-memory mock server

## Workspace

- `crates/alpaca-trade`: async Trading API client
- `crates/alpaca-trade-mock`: minimal contract-oriented mock server
- `tools/api-coverage/trading-api.json`: family-level coverage manifest for Trading HTTP REST audit work

## Phase 1 API

```rust
use alpaca_trade::Client;

# async fn demo() -> Result<(), alpaca_trade::Error> {
let client = Client::builder()
    .api_key(std::env::var("APCA_API_KEY_ID").expect("APCA_API_KEY_ID is required"))
    .secret_key(std::env::var("APCA_API_SECRET_KEY").expect("APCA_API_SECRET_KEY is required"))
    .build()?;

let account = client.account().get().await?;
println!("{}", account.status);
# Ok(())
# }
```
