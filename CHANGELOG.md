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

## `0.4.0` (2025-06-18) [CURRENT]

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
