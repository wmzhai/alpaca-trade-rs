# alpaca-trade-mock

Internal workspace helper for market-hours-sensitive Trading API tests in this workspace.

## Current Routes

- `GET /health`
- `GET /v2/orders`
- `POST /v2/orders`
- `DELETE /v2/orders`
- `GET /v2/orders/{order_id}`
- `PATCH /v2/orders/{order_id}`
- `DELETE /v2/orders/{order_id}`
- `GET /v2/orders:by_client_order_id`

## Orders Behavior

- State is in-memory only; restarting the server clears all stored orders.
- `market` orders fill immediately.
- `limit` buys fill only when `limit_price >= ask`; `limit` sells fill only when `limit_price <= bid`.
- Non-marketable limit orders remain open until `cancel`, `replace`, or `cancel_all`.
- Route tests use fixed in-process market data, while workspace integration tests may seed the mock from `alpaca-data` before startup.

## Usage

Start with the default bind address: `cargo run -p alpaca-trade-mock`.

The current default bind address is `127.0.0.1:9817`.

Use `--bind 127.0.0.1:9901` to override the default address.

This crate is kept in the workspace for local development only, is not a published release target, and exists to support `mock_stateful` fallback coverage for resources such as `orders`.
