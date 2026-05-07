# API Tools

[![Build status](https://github.com/fabienbellanger/api-tools/actions/workflows/CI.yml/badge.svg?branch=main)](https://github.com/fabienbellanger/api-tools/actions/workflows/CI.yml)
[![Crates.io](https://img.shields.io/crates/v/api-tools)](https://crates.io/crates/api-tools)
[![Documentation](https://docs.rs/api-tools/badge.svg)](https://docs.rs/api-tools)

> Toolkit for API in Rust

API Tools is a Rust library providing utilities for developing robust, consistent, and secure APIs.
It offers ready-to-use layers, extractors, error handling, and helpers designed to simplify API development, especially
with the Axum framework. The toolkit aims to standardize common API patterns and reduce boilerplate in your Rust
projects.

## Installation

For standard functionalities, no additional dependencies are required:

```toml
[dependencies]
api-tools = "*"
```

If you need all [features](#Features-list), you can use the `full` feature:

```toml
[dependencies]
api-tools = { version = "*", features = ["full"] }
```

Or you can use `cargo add` command:

```bash
cargo add api-tools
cargo add api-tools -F full
```

## Features list

| Name         | Description                       | Default |
| ------------ | --------------------------------- | :-----: |
| `axum`       | Enable Axum feature               |   ❌    |
| `prometheus` | Enable Prometheus metrics feature |   ❌    |
| `full`       | Enable all features               |   ❌    |

## Components

### Value objects

| Name          | Description                                                                                |
| ------------- | ------------------------------------------------------------------------------------------ |
| `UtcDateTime` | A wrapper around `chrono::DateTime` to handle date and time values in UTC                  |
| `Timezone`    | A wrapper around `chrono_tz::Tz` to handle time zones                                      |
| `Pagination`  | A struct to handle pagination parameters, including page number, page size and total count |
| `QuerySort`   | A struct to handle sorting query parameters, including field and direction                 |

### Axum

#### Security

| Name  | Description                              |
| ----- | ---------------------------------------- |
| `Jwt` | A wrapper for JWT generation and parsing |

#### Layers

| Name               | Description                                                                                                                              |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `BasicAuthLayer`   | Provides HTTP Basic Authentication middleware for protecting routes with username and password                                           |
| `CorsLayer`        | Adds Cross-Origin Resource Sharing (CORS) headers to responses, allowing or restricting resource sharing between different origins       |
| `HttpErrorsLayer`  | Middleware for intercepting and customizing HTTP error responses, enabling standardized error handling across your API                   |
| `LoggerLayer`      | Logs incoming requests and outgoing responses, useful for debugging and monitoring API activity                                          |
| `RequestId`        | Middleware that generates and attaches a unique request identifier (UUID) to each incoming request for traceability                      |
| `TimeLimiterLayer` | Middleware that restricts API usage to specific time slots. Outside of these allowed periods, it returns a 503 Service Unavailable error |
| `PrometheusLayer`  | Middleware that records per-request Prometheus metrics (`http_requests_total`, `http_requests_duration_seconds`). Host metrics (CPU, memory, swap, disks) are collected separately by `spawn_system_metrics_collector` to keep the request path free of blocking I/O |

##### Utility functions

| Name                              | Description                                                                                                                                                                          |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `body_from_parts`                 | Construct a response body from `Parts`, status code, message and headers                                                                                                             |
| `header_value_to_str`             | Convert `HeaderValue` to `&str`                                                                                                                                                      |
| `spawn_system_metrics_collector`  | Spawn a background Tokio task that periodically refreshes host metrics (CPU, memory, swap, disks) and publishes them as Prometheus gauges. Call once at app startup (`prometheus` feature) |

#### Extractors

| Name               | Description                                                            |
| ------------------ | ---------------------------------------------------------------------- |
| `ExtractRequestId` | Extracts the unique request identifier (UUID) from the request headers |
| `Path`             | Extracts and deserializes path parameters from the request URL         |
| `Query`            | Extracts and deserializes query string parameters from the request URL |

#### Response helpers

| Name               | Description                                                                                                 |
| ------------------ | ----------------------------------------------------------------------------------------------------------- |
| `ApiSuccess`       | Represents a successful API response (Status code and data in JSON). It implements the `IntoResponse` trait |
| `ApiError`         | Represents a list of HTTP errors                                                                            |
| `ApiErrorResponse` | Encapsulates the details of an API error response, including the status code and the error message          |

#### Handlers

| Name                | Description                                                                                                                                                |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `PrometheusHandler` | Installs the global Prometheus recorder. Use `get_handle()` for default histogram buckets or `get_handle_with_buckets(&[f64])` to provide custom buckets   |

## Code coverage

- [2026-05-07] `84.56% coverage, 460/544 lines covered`
- [2026-05-07] `56.88% coverage, 310/545 lines covered`

## To-Do list

- [ ] Add more documentation
