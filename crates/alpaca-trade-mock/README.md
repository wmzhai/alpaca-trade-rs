# alpaca-trade-mock

Internal workspace helper for market-hours-sensitive Trading API tests in this workspace.

## Current Routes

- `GET /health`
- `GET /v2/account`
- `GET /v2/account/activities`
- `GET /v2/account/activities/{activity_type}`
- `GET /v2/orders`
- `POST /v2/orders`
- `DELETE /v2/orders`
- `GET /v2/orders/{order_id}`
- `PATCH /v2/orders/{order_id}`
- `DELETE /v2/orders/{order_id}`
- `GET /v2/orders:by_client_order_id`
- `GET /v2/positions`
- `DELETE /v2/positions`
- `GET /v2/positions/{symbol_or_asset_id}`
- `DELETE /v2/positions/{symbol_or_asset_id}`
- `POST /v2/positions/{symbol_or_contract_id}/exercise`
- `POST /v2/positions/{symbol_or_contract_id}/do-not-exercise`

## Trading State Behavior

- State is in-memory only; restarting the server clears all stored accounts, orders, positions, activities, and exercise overrides.
- Virtual accounts are lazy-created from `apca-api-key-id`; each new account starts with `cash = 1000000` and stays isolated from every other API key.
- `GET /v2/account` reflects order-driven cash changes from the shared trading state while unrelated account fields stay on stable defaults.
- Orders, positions, and activities are projected from one account-scoped truth source, so fills and position actions stay searchable across all linked routes.
- Order creation, replacement, and position valuation resolve live market data through `alpaca-data`; if live quotes or option snapshots are unavailable, the request fails instead of inventing fallback prices.
- Partial fills are not implemented in this mock.
- Single-leg `market` orders fill immediately at the current bid/ask midpoint.
- Single-leg `limit` buys fill when `limit_price >= midpoint`; single-leg `limit` sells fill when `limit_price <= midpoint`.
- Multi-leg option orders compute a net combo midpoint from the live midpoint of each leg and use that combo midpoint for the same marketable-vs-open decision.
- Non-marketable limit orders remain in `new` until `cancel`, `replace`, or `cancel_all`.
- Filled orders update cash, positions, and activities together; position closes, exercise, and DNE actions leave matching activity records in the same account.
- Route tests and workspace integration tests both depend on live `alpaca-data` quotes and `optionchain` discovery; missing market data is treated as a test failure.

## Usage

Start with the default bind address: `cargo run -p alpaca-trade-mock`.

The current default bind address is `127.0.0.1:9817`.

Use `--bind 127.0.0.1:9901` to override the default address.

This crate is kept in the workspace for local development only, is not a published release target, and exists to support `mock_stateful` fallback coverage for linked trading-state verification around `account`, `orders`, `positions`, and `activities`.
