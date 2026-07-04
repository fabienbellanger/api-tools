# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**api-tools** is a Rust library of reusable building blocks (layers, extractors, response helpers,
value objects, JWT, Prometheus integration) for HTTP APIs built on Axum. Optional features keep the
default dependency set minimal.

## Common Commands

```bash
# Tests (always with --all-features; some modules are gated)
cargo test --all-features -- --nocapture
cargo test --all-features <test_name> -- --nocapture     # single test
cargo test --all-features prometheus -- --nocapture      # filter by module

# Lint + format (lint enforces -D warnings on clippy --all-features)
make lint
make lint-audit         # adds cargo-audit
make check              # lint-audit + test

# MSRV (currently 1.88, declared in Cargo.toml)
make verify-msrv

# Release prep
make prepare            # lint + test + verify-msrv
make build              # lint-audit + test + cargo build --release

# Docs
make doc                # cargo doc --open --no-deps --all-features
```

`make help` lists all targets.

## Feature Flags

| Feature      | Enables                                             |
| ------------ | --------------------------------------------------- |
| `axum`       | Everything under `server::axum::*`                  |
| `prometheus` | `metrics`, `metrics-exporter-prometheus`, `sysinfo` |
| `full`       | `axum` + `prometheus`                               |

`default = []` — the bare crate compiles with only the value objects. New optional integrations
should be gated behind a feature, not added to the default set.

## Architecture Notes

The crate is a flat library, not an application — no domain layer, no DI container. Each module
exposes one focused primitive that downstream services compose into their own Axum router.

- `value_objects/` — pure, framework-agnostic types (datetime, timezone, pagination, query_sort).
  No `axum` dependency; safe to use without any feature.
- `server/axum/` — gated behind `axum`. Sub-modules (`layers/`, `extractors/`, `response/`,
  `handlers/`, `security/jwt/`) are independent — pick what you need.

### Prometheus module (non-obvious pattern, since 0.8)

`server::axum::layers::prometheus` is split into two pieces that **must** both be wired by the
caller:

1. **`PrometheusLayer`** — tower middleware. Records per-request metrics only
   (`http_requests_total`, `http_requests_duration_seconds`). Microsecond overhead. **Do not** put
   any blocking I/O or sysinfo refresh in this hot path — that mistake (a 200 ms `tokio::sleep`)
   was the reason for the 0.8 rewrite.
2. **`spawn_system_metrics_collector(service_name, disk_mount_points, interval)`** — call **once at
   app startup** to publish host gauges (`system_cpu_usage`, `system_*_memory`, `system_*_swap`,
   `system_*_disks_space`) on a background Tokio task. Returns a `JoinHandle<()>` for shutdown
   control.

`PrometheusHandler::get_handle()` installs the global recorder with default histogram buckets;
`get_handle_with_buckets(&[f64])` lets callers override them for low-latency services.

### `HttpErrorsLayer` — status matched before buffering (since 0.9.1)

`server::axum::layers::http_errors` matches `parts.status` **first** and only buffers the body for
the statuses it actually rewrites (`422`, `404`). `405` is rewritten without reading its (discarded)
body; every other status — notably the `200`s that make up the bulk of the traffic — is a pure
passthrough with no full-body copy or UTF-8 scan, which preserves streaming. Consequence: a
legitimate large body on a passthrough status is no longer turned into a spurious `413` (the
`body_max_size` limit only applies where the body is read, i.e. `422`/`404`). Do **not** reintroduce
a blanket `to_bytes` before the status match — a sentinel-free regression there would re-add the copy
and the false `413`.

### `BodyLimiterLayer` — two-layer request size cap (since 0.9.1)

`server::axum::layers::body_limiter` caps **incoming request** size. It is deliberately split so
callers can compose the level of guarantee they need:

1. **`BodyLimiterLayer`** — checks the `Content-Length` header and rejects early with a JSON `413`
   (`ApiError::PayloadTooLarge`), without reading the body. O(1), streaming-preserving, but blind to
   chunked requests that omit `Content-Length`.
2. **`RequestBodyLimitLayer`** — re-export of `tower_http::limit::RequestBodyLimitLayer` (the `limit`
   feature of `tower-http` is enabled). Stack it **under** `BodyLimiterLayer` for a hard cap that
   also covers chunked bodies (enforced as the body is read; tower-http's plain `413`).

## Testing Conventions

- Tests live next to the code (`#[cfg(test)] mod tests` in the same file). No separate `tests/`
  directory.
- All tests must pass with `--all-features` — feature-gated code without tests is not acceptable.
- The middleware in `layers/prometheus.rs` has a sentinel test
  (`middleware_does_not_block_on_system_metrics`) that asserts < 50 ms latency. Any future change
  that re-introduces blocking I/O in the request path will fail it.
