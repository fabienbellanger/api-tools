# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!--
## [Unreleased]

## `x.y.z` (YYYY-MM-DD) [CURRENT | YANKED]

### Added (for new features)
### Changed (for changes in existing functionality)
### Deprecated (for soon-to-be removed features)
### Removed (for now removed features)
### Fixed (for any bug fixes)
### Security
-->

## `0.10.0` (2026-10-14) [CURRENT]

### Changed

- [Breaking] Update MSRV to Rust 1.89 (required by `uuid` `1.28`).
- Update `jsonwebtoken` (`10.4.0` → `11.1.0`). Its `Algorithm` enum is now
  `#[non_exhaustive]`: `Jwt::set_encoding_key` and `Jwt::set_decoding_key` return
  `JwtError::InvalidAlgorithm` for an algorithm this crate does not support yet.
  No `jsonwebtoken` type is exposed in this crate's public API.
- Pure constructors and getters are now `#[must_use]` (and `const fn` where
  possible). Ignoring their return value now raises a warning in caller code.
- Enable strict lints in `Cargo.toml` (clippy `pedantic`, `nursery`, `cargo` and a
  few `restriction` lints, `missing_docs`, `unsafe_code = "forbid"`). The whole
  public API is now documented, including `# Errors` sections. `make lint` also
  lints tests and doctests (`--all-targets`).
- `QuerySorts::from` now trims each part and reads a field **without prefix as
  ascending** (`id` = `+id`); it used to be silently ignored. In a query string a
  raw `+` is decoded as a space, so `?sort=+id,-name` used to drop `id`; it now
  sorts on both fields. Empty fields (`+`, `-`) are ignored.
- `AccessToken::extract_bearer_token_from_headers` (and the `AccessToken`
  extractor) now require the `Bearer` scheme followed by a non-empty token. The
  scheme is case-insensitive (RFC 6750). `Authorization: Bearer` with no token
  used to produce an empty token, which ended up as a `500` once parsed; it is now
  rejected with a `401`. Headers where `Bearer` is not the scheme
  (`Token Bearer x`) are rejected.
- `Pagination::new` now clamps `max_limit` between `PAGINATION_MIN_LIMIT` and
  `PAGINATION_MAX_LIMIT`, as documented. A `max_limit` below the minimum used to
  give a `limit` below `PAGINATION_MIN_LIMIT`.
- `Pagination::set_max_limit` now lowers the current `limit` when it exceeds the
  new max limit. It used to only store the value.
- The `Path` extractor now handles axum's `ErrorKind::DeserializeError` (a custom
  `Deserialize` error on a named param) explicitly. The `400` message is now
  ``Cannot parse `key` with value `value`: message`` instead of being prefixed
  with `Unhandled deserialization error:`.
- Update dependencies: `bytes` (`1.12.0` → `1.12.1`), `bytesize` (`2.4.2` →
  `2.7.0`), `futures` (`0.3.32` → `0.3.34`), `hyper` (`1.10.1` → `1.12.0`),
  `serde` (`1.0.228` → `1.0.229`), `serde_json` (`1.0.150` → `1.0.154`),
  `thiserror` (`2.0.18` → `2.0.21`), `tokio` (`1.52.3` → `1.53.2`), `tower-http`
  (`0.7.0` → `0.7.1`), `uuid` (`1.23.4` → `1.28.0`), `opentelemetry` (`0.32.0` →
  `0.33.1`), `tracing-opentelemetry` (`0.33.0` → `0.34.0`), `base64` (dev,
  `0.22.1` → `0.23.1`).
- Add `tracing-subscriber` (`0.3.23`) as a dev dependency, to test the logged
  events and the `trace_id` in error responses.

### Fixed

- `BodyLimiterLayer` can now be stacked over `RequestBodyLimitLayer` in a single
  `ServiceBuilder`, as shown in the module docs. It used to fail to compile once
  applied to a `Router`, because `RequestBodyLimit` returns a
  `Response<ResponseBody<Body>>` and `BodyLimiterMiddleware` only accepted an
  inner `Response<Body>`. The middleware now accepts any inner response body
  and turns it back into an axum `Body`.

## `0.9.1` (2026-07-04)

### Added

- `server::axum::layers::body_limiter::BodyLimiterLayer` — a reusable tower
  layer that rejects incoming requests whose `Content-Length` exceeds a
  configured maximum, before the body is read, with a JSON `413 Payload Too
  Large`. The check is O(1) and preserves streaming; requests without a
  `Content-Length` are not capped by this layer.
- `server::axum::layers::body_limiter::RequestBodyLimitLayer` — re-export of
  `tower_http::limit::RequestBodyLimitLayer` (the `limit` feature of `tower-http`
  is now enabled). Stack it under `BodyLimiterLayer` for a hard cap that also
  covers chunked bodies without a `Content-Length`; see the module docs for the
  two-layer example.

### Fixed

- `HttpErrorsLayer` no longer buffers the response body for statuses it does not
  rewrite. The status is now matched first, so only `422`/`404` responses have
  their body read; `200 OK` (the bulk of the traffic), `405`, `500`, `3xx`, etc.
  pass through without a full-body copy + UTF-8 scan, preserving streaming.
- A legitimate large body on a passthrough status (e.g. `200 OK`) is no longer
  turned into a spurious `413 Payload Too Large`. The `body_max_size` limit is
  still enforced on `422`/`404`, where the body is actually read.

## `0.9.0` (2026-06-16)

### Changed

- [BREAKING] Bump `tower-http` from `0.6` to `0.7`. Since `cors()` returns a
  `tower_http::cors::CorsLayer` and `MakeRequestUuid` implements `tower_http::request_id::MakeRequestId`,
  downstream crates must upgrade their own `tower-http` to `0.7` as well.
- Reduce the enabled `tower-http` features to only those actually used (`cors`,
  `request-id`); the unused `fs`, `util`, and `set-header` features are no longer pulled in,
  shrinking the dependency footprint.
- Update other dependencies (`tokio`, `jsonwebtoken`, `opentelemetry`,
  `tracing-opentelemetry`, `hyper`, `chrono`, `uuid`, `serde_json`, `metrics`, `bytesize`).

## `0.8.0` (2026-05-07)

### Fixed

- [CRITICAL] `PrometheusLayer` no longer blocks every HTTP request for ~210 ms.
  The middleware previously called `SystemMetrics::new()` inline, which contained a
  `tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL)` (200 ms) plus a full
  `System::new_all()` scan. Host metrics are now collected by a background task; the
  middleware only records per-request counter and histogram (~µs).
- Fix typo in metric name: `system_used_disks_usage` → `system_used_disks_space`
  (now consistent with `system_total_disks_space`).

### Added

- `spawn_system_metrics_collector(service_name, disk_mount_points, interval) -> JoinHandle<()>`:
  spawns a Tokio background task that periodically refreshes host metrics (CPU, memory,
  swap, disks) and publishes them as Prometheus gauges. Call once at app startup.
- `PrometheusHandler::get_handle_with_buckets(&[f64])`: install the recorder with custom
  histogram buckets for `http_requests_duration_seconds`.
- `pub const DEFAULT_DURATION_BUCKETS: &[f64]` exported from
  `server::axum::handlers::prometheus`.
- Tests for `PrometheusLayer` (latency sentinel) and `spawn_system_metrics_collector`
  (smoke tick).

### Changed

- [BREAKING] `PrometheusLayer` no longer carries `disk_mount_points`. Pass the mount
  points to `spawn_system_metrics_collector` instead.
- [BREAKING] Metric `system_used_disks_usage` renamed to `system_used_disks_space`
  (dashboards must be updated).
- Internal label allocations reduced: `service_name` is converted once to `Arc<str>`,
  standard HTTP methods and common status codes are mapped to `&'static str` to avoid
  per-request allocations.
- Fix `clippy::field-reassign-with-default` regression in `jwt/mod.rs` tests.
- Update `LICENSE` (copyright 2025-2026, full author name).
- Update `README.md` and `CLAUDE.md` to document the new Prometheus pattern.

## `0.7.0` (2026-02-12)

### Changed

- Skip logging for OPTIONS requests
- Update dependencies

## `0.6.6` (2026-01-14)

### Changed

- Fix duplication code in `logger.rs` (Clippy error)
- Update dependencies

### Added

- Add `CLAUDE.md` file

## `0.6.5` (2025-11-04)

### Changed

- [BREAKING] Change `PAGINATION_MIN_LIMIT` from `50` to `10`

## `0.6.4` (2025-10-23)

### Changed

- Improve 404 HTTP code from `HttpErrorsLayer`

## `0.6.3` (2025-10-23)

### Changed

- Revert 404 HTTP code from `HttpErrorsLayer`
- Update dependencies

## `0.6.2` (2025-10-14)

### Changed

- [Breaking] Update MSRV to Rust 1.88
- Update dependencies
- Lint

## `0.6.1` (2025-06-26)

### Changed

- [Breaking] Move `secrity` module into `axum` module

## `0.6.0` (2025-06-26)

### Added

- Add `Jwt` in security module

## `0.5.0` (2025-06-23)

### Added

- Add the trace ID in the `ApiError` body (`trace_id` field)

## `0.4.0` (2025-06-18)

### Added

- Add security headers layer `SecurityHeadersLayer`

## `0.3.0` (2025-06-11)

### Added

- Add system metrics to Prometheus like CPU, memory, swap and disk usage.

## `0.2.2` (2025-06-09)

### Fixed

- Export prometheus handler module only if the `prometheus` feature is enable

## `0.2.1` (2025-06-06)

### Added

- Add `PrometheusHandler`

### Fixed

- Fix `PrometheusLayer`

## `0.2.0` (2025-06-06)

### Added

- Add `prometheus` layer

## `0.1.2` (2025-06-03)

### Fixed

- Fix docs.rs documentation build

## `0.1.1` (2025-06-03)

### Fixed

- Fix docs.rs documentation build

## `0.1.0` (2025-06-03)

### Added

- Add `TimeLimiterLayer` layer

### Changed

- Complete the `README.md` file
- Move MSRV and Coverage information from `README.md` to `HELP.md` file
- Bump `tower-http` to `0.6.5`
- [BREAKING] Rename `ExtractRequestId` extractor to `RequestId`

## `0.0.4` (2025-05-28)

### Added

- Add `http_errors` layer

### Changed

- Bump `tokio` to `1.45.1`
- Bump `uuid` to `1.17.0`

## `0.0.3` (2025-05-20)

### Changed

- Add all features in [docs.rs](https://docs.rs)

## `0.0.2` (2025-05-20)

### Fixed

- Fix feature `default`

## `0.0.1` (2025-05-20)

### Added

- Initialize the project
