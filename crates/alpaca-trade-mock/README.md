# alpaca-trade-mock

Internal workspace helper for future market-hours-sensitive Trading API tests.

## Current Routes

- `GET /health`

## Usage

Start with the default bind address: `cargo run -p alpaca-trade-mock`.

The current default bind address is `127.0.0.1:9817`.

Use `--bind 127.0.0.1:9901` to override the default address.

This crate is kept in the workspace for local development only and is not a published release target.
