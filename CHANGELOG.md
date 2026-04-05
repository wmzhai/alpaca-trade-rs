# Changelog

## 0.5.0 - 2026-04-05

- Finish Phase 5 by shipping the live-first `assets` Trading HTTP REST resource family.
- Preserve the official `status`, `asset_class`, `exchange`, and `attributes` list query words together with the official `symbol_or_asset_id` single-asset path behavior.
- Keep `assets` outside `alpaca-trade-mock` and validate it through the official Alpaca Paper smoke path.
- No dedicated benchmark was added because Phase 5 only introduces straightforward read-only `GET` endpoints without a new pagination or transport primitive.

## 0.4.5 - 2026-04-05

- Add the credential-gated `assets_live` Alpaca Paper smoke test for both `list` and `get`.
- Update the workspace and crate READMEs to list `assets` as implemented and move `options_contracts` to the next resource phase.
- Mark both `assets` operations as implemented, live-first, and mock-free in the Trading API coverage manifest.

## 0.4.4 - 2026-04-05

- Add `AssetsClient::get()` for the official `GET /v2/assets/{symbol_or_asset_id}` path without local symbol, UUID, or CUSIP guessing.
- Expand local `assets_transport` coverage to include `list`, `get`, malformed JSON, `429`, `5xx`, and invalid path-segment guardrails.
- Keep Phase 5 transport behavior on the existing enriched GET-only retry semantics from the shared foundation.

## 0.4.3 - 2026-04-05

- Implement the `Asset` mirror model for the official `assets` resource, including optional `cusip`, margin requirement, and `attributes` fields.
- Add `AssetsClient::list()` and wire `GET /v2/assets` through the shared authenticated transport.
- Add local model coverage plus list-path/auth/query coverage for the Phase 5 `assets` family.

## 0.4.2 - 2026-04-05

- Add the Phase 5 `assets` public API skeleton with `Client::assets()`, `AssetsClient`, `Asset`, and `ListRequest`.
- Add local request-shape coverage for the official `status`, `asset_class`, `exchange`, and `attributes` query words.
- Keep the initial Phase 5 scaffold thin while model, transport, live coverage, and docs land in later tasks.

## 0.4.1 - 2026-04-05

- Fail fast during `Client::builder().build()` when `api_key` or `secret_key` cannot be encoded as HTTP header values, so invalid credentials no longer survive until the first authenticated request.
- Clarify that `RetryPolicy::max_get_attempts` counts total `GET` attempts, with `1` disabling retry and `2` allowing one retry after the initial failed `GET`.
- Re-run the Phase 4 foundation verification suite on top of the tightened credential validation and retry semantics documentation.

## 0.4.0 - 2026-04-05

- Complete Phase 4 by shipping the shared Trading HTTP REST foundation before `assets`.
- Add multi-method transport support, enriched error metadata, Trading-safe retry defaults, builder ergonomics, and shared request primitives.
- Re-run the existing live-first `account`, `clock`, and `calendar` coverage on top of the new transport foundation.
- Realign the project docs and coverage manifest so `assets` becomes Phase 5 and later phases follow the new order.

## 0.3.6 - 2026-04-05

- Realign the public docs and local design docs around the Phase 4 `foundation` milestone and the new Phase 5 `assets` follow-up.
- Insert the shared `foundation` milestone and revised post-foundation phase order into `tools/api-coverage/trading-api.json`.
- Document the Phase 4 benchmark rationale: no dedicated benchmark was added because foundation changes shared transport semantics rather than introducing a new public high-volume endpoint.

## 0.3.5 - 2026-04-05

- Move the existing `account`, `clock`, and `calendar` clients onto the new foundation transport pipeline.
- Rework public transport tests around a shared scripted TCP server and enriched error metadata assertions.
- Keep the public observer and retry configuration surface covered for future foundation-aware resources.

## 0.3.4 - 2026-04-05

- Expand `ClientBuilder` so it can load credentials from the official `APCA_API_KEY_ID` / `APCA_API_SECRET_KEY` names or from caller-supplied custom env var names.
- Allow callers to inject a preconfigured `reqwest::Client` while preserving its default headers and other transport settings on real Trading REST requests, and reject mixing a custom client with any explicit builder `timeout()`.
- Wire builder-level observer hooks and Trading-safe retry policy customization into the authenticated HTTP transport path, with `NoopObserver` and `RetryPolicy::trading_safe()` as the defaults, and redact URL userinfo from observer start events.
- Add regression coverage for env credential precedence, subprocess-isolated env loading, injected transport clients, timeout conflict handling, observer lifecycle callbacks, and the public builder retry/observer surface.

## 0.3.3 - 2026-04-05

- Replace the single GET-only transport helper with a unified HTTP pipeline that can shape query/body requests, accept `204 No Content`, and emit richer request metadata.
- Add public Trading-safe retry and observer configuration types for future builder wiring, with observer success hooks reserved for validated client-level success.
- Upgrade transport failures to include endpoint name, method, status, request id, retry-after, and a bounded body snippet.

## 0.3.2 - 2026-04-05

- Add shared request guardrails for trimmed and reserved-character-safe path validation, ordered query writing, empty-aware CSV query encoding, and an initial pagination `collect_all()` landing point.
- Replace the static endpoint enum with metadata-backed endpoints that carry stable operation names, HTTP methods, auth requirements, and dynamic path support for asset lookups.
- Introduce `InvalidRequest` errors so request-shaping failures surface before any network call is attempted.

## 0.3.1 - 2026-04-05

- Redact the full public trading-client `Debug` surface, including `Client`, `ClientBuilder`, `account()`, `clock()`, `calendar()`, and shared auth state, so credentials do not appear in public debug strings.
- Stop exposing raw `base_url` values in `Client` debug output, which prevents leaks from custom URLs that embed secrets.
- Keep the existing non-exhaustive debug redaction behavior for `clock()` and `calendar()` intact.
- Extend public regression coverage to assert debug redaction for the builder, root client, custom base URLs, and `account()` resource client.

## 0.3.0 - 2026-04-05

- Finish Phase 3 by shipping the live-first `calendar` Trading HTTP REST resource.
- Keep `calendar` outside `alpaca-trade-mock` and validate it through the official Alpaca Paper smoke path.
- Roll the workspace forward from the Phase 2 patch series to the Phase 3 MINOR release.

## 0.2.3 - 2026-04-05

- Add the credential-gated `calendar_live` Alpaca Paper smoke test for the Phase 3 read-only market calendar resource.
- Update the public workspace docs to include `calendar` in the supported Trading HTTP REST scope and example flow.
- Mark `calendar` as implemented, live-first, and mock-free in the Trading API coverage manifest.

## 0.2.2 - 2026-04-05

- Implement `GET /v2/calendar` through `CalendarClient::list()` using the shared authenticated HTTP transport.
- Add local model coverage for the official `calendar` array response and required fields.
- Add local transport coverage for the `/v2/calendar` path, auth headers, and ordered `start` / `end` query words.

## 0.2.1 - 2026-04-05

- Add the Phase 3 `calendar` public API skeleton with `Client::calendar()`, `CalendarClient`, `Calendar`, and `ListRequest`.
- Add local request-shape coverage for the official `start` and `end` query words.
- Keep the initial Phase 3 scaffold thin while transport and live coverage land in later tasks.

## 0.2.0 - 2026-04-05

- Finish Phase 2 by shipping the live-first `clock` Trading HTTP REST resource.
- Keep `clock` outside `alpaca-trade-mock` and validate it through the official Alpaca Paper smoke path.
- Roll the workspace forward from the Phase 1 patch series to the Phase 2 MINOR release.

## 0.1.10 - 2026-04-05

- Add the credential-gated `clock_live` Alpaca Paper smoke test for Phase 2.
- Update the public workspace and crate docs to include `clock` in the supported Trading HTTP REST scope.
- Mark `clock` as implemented and live-first in the Trading API coverage manifest.

## 0.1.9 - 2026-04-05

- Implement `GET /v2/clock` in `ClockClient` using the existing authenticated HTTP transport.
- Add local model coverage for the official `clock` response shape and required fields.
- Add local transport coverage for the `/v2/clock` path and Alpaca auth headers.

## 0.1.8 - 2026-04-05

- Add the Phase 2 `clock` public API skeleton with `Client::clock()`, `ClockClient`, and `Clock`.
- Export the new `clock` module from `alpaca-trade` without changing the existing `account` API.
- Keep Phase 2 scoped to the public resource skeleton before transport and live coverage land.

## 0.1.7 - 2026-04-05

- Accept both the standard `APCA_*` credential names and the repo-local `ALPACA_TRADE_*` aliases for Phase 1 live account test loading.
- Switch public examples back to the official Alpaca environment variable names while keeping the local live-test helper compatible with both naming conventions.
- Clarify that Phase 1 live account verification is credential-gated Alpaca Paper smoke coverage, so green local runs can still skip the real paper request when credentials are absent.

## 0.1.6 - 2026-04-05

- Realign public Phase 1 documentation around live-first, credential-gated `account` testing against Alpaca Paper.
- Mark `alpaca-trade-mock` as an internal workspace-only tool and disable future publishing with `publish = false`.
- Update the public coverage manifest and package docs to match the internal-mock release boundary.

## 0.1.5 - 2026-04-05

- Remove `/v2/account` and all Phase 1 business state from `alpaca-trade-mock`.
- Keep `alpaca-trade-mock` runnable as a minimal internal scaffold with `/health`, `build_app()`, and `spawn_test_server()`.
- Drop the no-longer-needed account/admin/state dependencies from the mock crate.

## 0.1.4 - 2026-04-05

- Switch Phase 1 `account` happy-path verification from local mock-server tests to live-first Alpaca Paper coverage.
- Load live test credentials from a local root `.env` via `ALPACA_TRADE_API_KEY` and `ALPACA_TRADE_SECRET_KEY`.
- Remove the old `account` black-box tests against the local mock server now that local transport coverage and live happy-path coverage are split cleanly.

## 0.1.3 - 2026-04-04

- Change the default `alpaca-trade-mock` bind address to `127.0.0.1:9817`.
- Simplify the public Phase 1 startup instructions to use `cargo run -p alpaca-trade-mock` by default.
- Keep the `--bind` override documented for custom local addresses.

## 0.1.2 - 2026-04-04

- Reorder the planned Trading HTTP REST phases to prioritize lower-complexity read-only families before mutation-heavy trading resources.
- Document how to start the Phase 1 `alpaca-trade-mock` server and list the currently available Phase 1 routes.
- Document the current Phase 1 test flow, including the workspace test command and the self-starting local mock-server black-box tests.

## 0.1.1 - 2026-04-04

- Add `tools/api-coverage/trading-api.json` to track major non-crypto Alpaca Trading HTTP REST resource families and operation status.
- Explicitly document that this project excludes stream / websocket APIs and crypto trading scope.
- Align local API sync and release-audit scope around Trading HTTP REST only.

## 0.1.0 - 2026-04-04

- Release the initial `account` phase for `alpaca-trade` with `paper` as the default environment.
- Release `alpaca-trade-mock` with in-memory `/v2/account`, `/health`, and `/__admin/*` test-control routes.
- Add local mock-server integration coverage for successful account reads, rate limiting, and malformed JSON failures.

## 0.0.5 - 2026-04-04

- Add Phase 1 public documentation for the workspace, client crate, and mock crate.
- Add a public API regression test and a runnable `account_get` example for `alpaca-trade`.
- Add publish metadata and packaging exclusions for both crates.

## 0.0.4 - 2026-04-04

- Add local mock-server black-box tests for successful account reads through `alpaca_trade::Client`.
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
