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

## Phase 1 Mock Server

Start the local mock server with:

```bash
cargo run -p alpaca-trade-mock -- --bind 127.0.0.1:16803
```

Current Phase 1 routes:

- `GET /health`
- `GET /v2/account`
- `POST /__admin/reset`
- `POST /__admin/faults`
- `DELETE /__admin/faults`

## Phase 1 Testing

Run the full automated test suite with:

```bash
cargo test --workspace
```

If you only want the mock route contract tests, run:

```bash
cargo test -p alpaca-trade-mock --test app_routes -- --nocapture
```

The current black-box client tests start their own mock server via `spawn_test_server()`, so they do not require a manually started mock process.
