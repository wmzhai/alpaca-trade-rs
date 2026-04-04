# Changelog

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
