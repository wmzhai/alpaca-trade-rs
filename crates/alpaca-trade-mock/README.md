# alpaca-trade-mock

Minimal in-memory mock server for testing `alpaca-trade`.

## Current Routes

- `GET /health`
- `GET /v2/account`
- `POST /__admin/reset`
- `POST /__admin/faults`
- `DELETE /__admin/faults`

## Usage

```bash
cargo run -p alpaca-trade-mock -- --bind 127.0.0.1:16803
```
