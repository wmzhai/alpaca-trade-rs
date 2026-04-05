# alpaca-trade

Async Rust client for the non-crypto Alpaca Trading HTTP API.

## Current Coverage

- `account`
- `clock`
- `calendar`

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
