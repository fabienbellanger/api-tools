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

## `0.8.0` (2026-05-07) [CURRENT]

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
