# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**api-tools** is a Rust library providing utilities for developing APIs, especially with the Axum framework. It offers
reusable layers, extractors, error handling, and helpers to standardize common API patterns and reduce boilerplate.

## Development Commands

### Building and Testing

```bash
# Run tests with all features enabled
cargo test --all-features -- --nocapture

# Run a specific test
cargo test --all-features test_name -- --nocapture

# Build in release mode (includes linting, audit, and tests)
make build

# Build without audit (faster)
make build-no-audit

# Check everything (lint, audit, and test)
make check
```

### Code Quality

```bash
# Format and lint
make lint

# Lint with audit (includes clippy, rustfmt, and cargo-audit)
make lint-audit

# Fix audit issues
make audit-fix
```

### Code Coverage

```bash
# Run coverage tests (requires cargo-tarpaulin)
make coverage

# Generate HTML coverage report
cargo tarpaulin --all-features --ignore-tests --line --count --include-files src/**/* --out Html
```

### Documentation

```bash
# Open documentation for this crate only (no dependencies)
make doc

# Open documentation including private items
make doc-public

# Watch documentation changes
make watch-doc
```

### MSRV

```bash
# Find minimum supported Rust version
make find-msrv

# Verify MSRV (currently 1.88)
make verify-msrv
```

## Architecture

**IMPORTANT:** The crate follows the hexagonal architecture, with a clean separation between domain logic and
infrastructure.

### Module Structure

- **`value_objects/`**: Reusable value objects
    - `datetime`: UtcDateTime wrapper around chrono::DateTime
    - `timezone`: Timezone wrapper around chrono_tz::Tz
    - `pagination`: Pagination handling (page number, size, total count)
    - `query_sort`: Query sorting parameters (field, direction)

- **`server/axum/`**: Axum-specific components (behind `axum` feature)
    - **`layers/`**: Middleware layers
        - `basic_auth`: HTTP Basic Authentication
        - `cors`: CORS headers management
        - `http_errors`: HTTP error response customization
        - `logger`: Request/response logging
        - `request_id`: UUID-based request tracing
        - `time_limiter`: Time-slot based API access control
        - `prometheus`: Prometheus metrics collection (behind `prometheus` feature)
        - `security_headers`: Security headers (CSP, etc.)

    - **`extractors`**: Request data extraction
        - `ExtractRequestId`: Extract request UUID from headers
        - `Path`: Deserialize path parameters
        - `Query`: Deserialize query string parameters

    - **`response`**: Response helpers
        - `ApiSuccess`: Successful API responses with IntoResponse trait
        - `ApiError`: HTTP error list representation
        - `ApiErrorResponse`: Error response encapsulation

    - **`security/jwt/`**: JWT authentication
        - `access_token`: JWT generation and parsing
        - `payload`: JWT payload structures

    - **`handlers/`**: Request handlers
        - `prometheus`: Prometheus metrics endpoint

### Features

The crate uses Cargo features for optional dependencies:

- `axum`: Enables Axum-specific components (disabled by default)
- `prometheus`: Enables Prometheus metrics (disabled by default)
- `full`: Enables all features

When adding new functionality, consider whether it should be behind a feature flag to keep the default minimal.

## Testing

- Tests are located alongside the code they test
- All tests should pass with `--all-features` flag
- Use `-- --nocapture` to see println output during tests
- Coverage target is tracked in README.md (current: ~41%)

## Dependencies Management

```bash
# Upgrade dependencies (compatible versions)
make upgrade

# Upgrade with incompatible changes
make upgrade-force
```

## Release Preparation

Before releasing a new version:

```bash
make prepare  # Runs lint, test, and verify-msrv
```
