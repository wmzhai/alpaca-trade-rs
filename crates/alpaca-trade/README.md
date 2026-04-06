# alpaca-trade

Async Rust client for the non-crypto Alpaca Trading HTTP API.

## Current Milestone

- Phase 7 milestone: `orders`
- Implemented resources: `account`, `clock`, `calendar`, `assets`, `options_contracts`, `orders`
- Next resource phase: `positions` (Phase 8)
- Default retry behavior: automatic retry is limited to `GET`
- Retry semantics: `max_get_attempts` counts total attempts, so `1` disables retry and `2` means one retry after the first failed `GET`
- Numeric model policy: high-precision financial fields in the public Rust API use `alpaca_trade::Decimal`, while request/response wire shapes still mirror the official Alpaca contract
- Benchmark note: Phase 7 continues to reuse the shared transport foundation, so the current milestone does not add a dedicated benchmark track

## Defaults

- Environment: `paper`
- Transport style: async HTTP with `reqwest`

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
use alpaca_trade::orders::{ListRequest, QueryOrderStatus};

# async fn demo() -> Result<(), alpaca_trade::Error> {
let client = Client::builder()
    .api_key(std::env::var("APCA_API_KEY_ID").expect("APCA_API_KEY_ID is required"))
    .secret_key(std::env::var("APCA_API_SECRET_KEY").expect("APCA_API_SECRET_KEY is required"))
    .paper()
    .build()?;

let orders = client
    .orders()
    .list(ListRequest {
        status: Some(QueryOrderStatus::Open),
        limit: Some(10),
        ..ListRequest::default()
    })
    .await?;

println!("{}", orders.len());
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

## Testing Notes

- `orders_mutating` uses the shared `orders()` public API against dedicated Paper trading when `ALPACA_TRADE_ORDERS_TEST_ACCOUNT=1` is set during market hours.
- The dedicated Paper path dynamically discovers real single-leg and multi-leg option contracts through `alpaca-data`, including call spreads, put spreads, and iron condors.
- When that dedicated Paper path is unavailable, the same mutating flow falls back to the internal `alpaca-trade-mock` stateful orders server.
- Both the dedicated Paper path and the mock fallback require live `alpaca-data` quotes and `optionchain` discovery; missing live market data fails the run instead of falling back to seeded snapshots.
- The test support accepts both the standard `APCA_*` credential names and the repo-local `ALPACA_TRADE_*` aliases.
