# alpaca-trade

Async Rust client for the non-crypto Alpaca Trading HTTP API.

## Current Milestone

- Phase 5 milestone: `assets`
- Implemented resources: `account`, `clock`, `calendar`, `assets`
- Next resource phase: `options_contracts` (Phase 6)
- Default retry behavior: automatic retry is limited to `GET`
- Retry semantics: `max_get_attempts` counts total attempts, so `1` disables retry and `2` means one retry after the first failed `GET`
- Benchmark note: no dedicated benchmark because Phase 5 only adds two straightforward read-only GET endpoints without a new local performance-sensitive loop

## Defaults

- Environment: `paper`
- Transport style: async HTTP with `reqwest`

## Example

```rust
use alpaca_trade::{Client, assets::ListRequest};

# async fn demo() -> Result<(), alpaca_trade::Error> {
let client = Client::builder()
    .api_key(std::env::var("APCA_API_KEY_ID").expect("APCA_API_KEY_ID is required"))
    .secret_key(std::env::var("APCA_API_SECRET_KEY").expect("APCA_API_SECRET_KEY is required"))
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
println!("{} {}", assets[0].symbol, assets[0].name);
# Ok(())
# }
```
