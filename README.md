# alpaca-trade-rs

`alpaca-trade-rs` is a Rust workspace for the non-crypto Alpaca Trading HTTP REST API.

## Current Status

- Phase 4 milestone: `foundation`
- Implemented resources: `account`, `clock`, `calendar`
- Next resource phase: `assets` (Phase 5)
- API surface: non-crypto Alpaca Trading HTTP REST only
- Explicit exclusions: stream / websocket APIs, crypto trading APIs
- Published crate: `alpaca-trade`
- Internal workspace tool: `alpaca-trade-mock`
- Default client environment: Alpaca Paper
- Testing taxonomy: `live_readonly`, `paper_mutating_with_cleanup`, `mock_stateful`, `fault_injection_only`
- Default retry behavior: automatic retry is limited to `GET`
- Retry semantics: `max_get_attempts` counts total attempts, so `1` disables retry and `2` means one retry after the first failed `GET`
- Benchmark note: foundation does not add a dedicated benchmark because it changes shared transport semantics rather than introducing a new high-volume endpoint

## Workspace

- `crates/alpaca-trade`: async Trading API client
- `crates/alpaca-trade-mock`: internal workspace helper for future market-hours-sensitive Trading API tests
- `tools/api-coverage/trading-api.json`: family-level coverage manifest for Trading HTTP REST audit work

## Implemented API

```rust
use alpaca_trade::{Client, calendar::ListRequest};

# async fn demo() -> Result<(), alpaca_trade::Error> {
let client = Client::builder()
    .api_key(std::env::var("APCA_API_KEY_ID").expect("APCA_API_KEY_ID is required"))
    .secret_key(std::env::var("APCA_API_SECRET_KEY").expect("APCA_API_SECRET_KEY is required"))
    .build()?;

let rows = client
    .calendar()
    .list(ListRequest {
        start: Some("2026-04-01".into()),
        end: Some("2026-04-03".into()),
    })
    .await?;
println!("{} {}", rows[0].date, rows[0].open);
# Ok(())
# }
```

## Testing

Create a local root `.env` file with either:

- `APCA_API_KEY_ID=...` and `APCA_API_SECRET_KEY=...`
- `ALPACA_TRADE_API_KEY=...` and `ALPACA_TRADE_SECRET_KEY=...`

Run the full automated test suite with `cargo test --workspace -- --nocapture`.

Notes:
- `account_model`, `account_transport`, `clock_model`, `clock_transport`, `calendar_model`, and `calendar_transport` stay local/offline.
- `account_live`, `clock_live`, and `calendar_live` are the current `live_readonly` credential-gated smoke paths against the official Alpaca Paper API.
- Future mutating families will follow the `paper_mutating_with_cleanup` or `mock_stateful` taxonomy instead of reusing the read-only smoke path.
- The live test helper accepts both the standard `APCA_*` names and the repo-local `ALPACA_TRADE_*` aliases.
- If `.env` credentials are missing, the live tests print skip messages and exit successfully, so a green local run may not include a real paper request.
