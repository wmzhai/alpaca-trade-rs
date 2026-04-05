# alpaca-trade

Async Rust client for the non-crypto Alpaca Trading HTTP API.

## Current Milestone

- Phase 4 milestone: `foundation`
- Implemented resources: `account`, `clock`, `calendar`
- Next resource phase: `assets` (Phase 5)
- Default retry behavior: automatic retry is limited to `GET`
- Retry semantics: `max_get_attempts` counts total attempts, so `1` disables retry and `2` means one retry after the first failed `GET`

## Defaults

- Environment: `paper`
- Transport style: async HTTP with `reqwest`

## Example

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
