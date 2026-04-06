# alpaca-trade

Async Rust client for the non-crypto Alpaca Trading HTTP API.

## Current Milestone

- Phase 6 milestone: `options_contracts`
- Implemented resources: `account`, `clock`, `calendar`, `assets`, `options_contracts`
- Next resource phase: `orders` (Phase 7)
- Default retry behavior: automatic retry is limited to `GET`
- Retry semantics: `max_get_attempts` counts total attempts, so `1` disables retry and `2` means one retry after the first failed `GET`
- Numeric model policy: high-precision financial fields in the public Rust API use `alpaca_trade::Decimal`, while request/response wire shapes still mirror the official Alpaca contract
- Benchmark note: Phase 6 reuses the existing read-only `GET` and pagination foundation, so the current milestone does not add a dedicated benchmark track

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

## Examples

Set `APCA_API_KEY_ID` and `APCA_API_SECRET_KEY`, then run one of:

```sh
cargo run -p alpaca-trade --example client_builder
cargo run -p alpaca-trade --example account_get
cargo run -p alpaca-trade --example assets_list
```
