# Changelog

## 0.1.4 - 2026-04-05

- Switch Phase 1 `account` happy-path verification from mock-backed tests to live-first Alpaca Paper coverage.
- Load live test credentials from a local root `.env` via `ALPACA_TRADE_API_KEY` and `ALPACA_TRADE_SECRET_KEY`.
- Remove the old mock-backed `account` black-box tests now that local transport coverage and live happy-path coverage are split cleanly.

## 0.1.3 - 2026-04-04

- Change the default `alpaca-trade-mock` bind address to `127.0.0.1:9817`.
- Simplify the public Phase 1 startup instructions to use `cargo run -p alpaca-trade-mock` by default.
- Keep the `--bind` override documented for custom local addresses.

## 0.1.2 - 2026-04-04

- Reorder the planned Trading HTTP REST phases to prioritize lower-complexity read-only families before mutation-heavy trading resources.
- Document how to start the Phase 1 `alpaca-trade-mock` server and list the currently available Phase 1 routes.
- Document the current Phase 1 test flow, including the workspace test command and the self-starting mock-backed black-box tests.

## 0.1.1 - 2026-04-04

- Add `tools/api-coverage/trading-api.json` to track major non-crypto Alpaca Trading HTTP REST resource families and operation status.
- Explicitly document that this project excludes stream / websocket APIs and crypto trading scope.
- Align local API sync and release-audit scope around Trading HTTP REST only.

## 0.1.0 - 2026-04-04

- Release the initial `account` phase for `alpaca-trade` with `paper` as the default environment.
- Release `alpaca-trade-mock` with in-memory `/v2/account`, `/health`, and `/__admin/*` test-control routes.
- Add mock-backed integration coverage for successful account reads, rate limiting, and malformed JSON failures.

## 0.0.5 - 2026-04-04

- Add Phase 1 public documentation for the workspace, client crate, and mock crate.
- Add a public API regression test and a runnable `account_get` example for `alpaca-trade`.
- Add publish metadata and packaging exclusions for both crates.

## 0.0.4 - 2026-04-04

- Add mock-backed black-box tests for successful account reads through `alpaca_trade::Client`.
- Add integration coverage for `429` retry-after propagation and malformed JSON deserialization failures.
- Wire `alpaca-trade` package tests to the in-repo `alpaca-trade-mock` server crate.

## 0.0.3 - 2026-04-04

- Add the in-memory `alpaca-trade-mock` account server with seeded `/v2/account` data.
- Add route-level coverage for `/health`, auth enforcement, and seeded account responses.
- Add admin fault injection and reset endpoints for later failure-path integration tests.

## 0.0.2 - 2026-04-04

- Add the shared account transport foundation to `alpaca-trade`.
- Add the first `/v2/account` model coverage and request-path tests.
- Tighten account deserialization rules and early client configuration validation.

## 0.0.1 - 2026-04-04

- Bootstrap the workspace for the non-crypto Alpaca Trading HTTP API.
- Add the initial `alpaca-trade` client skeleton for the `account` phase.
- Add the first builder contract tests and a minimal placeholder mock crate.
