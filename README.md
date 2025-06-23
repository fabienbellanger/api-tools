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
|--------------|-----------------------------------|:-------:|
| `axum`       | Enable Axum feature               |    ❌    |
| `prometheus` | Enable Prometheus metrics feature |    ❌    |
| `full`       | Enable all features               |    ❌    |

## Components

### Value objects

| Name          | Description                                                                                |
|---------------|--------------------------------------------------------------------------------------------|
| `UtcDateTime` | A wrapper around `chrono::DateTime` to handle date and time values in UTC                  |
| `Timezone`    | A wrapper around `chrono_tz::Tz` to handle time zones                                      |
| `Pagination`  | A struct to handle pagination parameters, including page number, page size and total count |
| `QuerySort`   | A struct to handle sorting query parameters, including field and direction                 |

### Axum

#### Layers

| Name               | Description                                                                                                                              |
|--------------------|------------------------------------------------------------------------------------------------------------------------------------------|
| `BasicAuthLayer`   | Provides HTTP Basic Authentication middleware for protecting routes with username and password                                           |
| `CorsLayer`        | Adds Cross-Origin Resource Sharing (CORS) headers to responses, allowing or restricting resource sharing between different origins       |
| `HttpErrorsLayer`  | Middleware for intercepting and customizing HTTP error responses, enabling standardized error handling across your API                   |
| `LoggerLayer`      | Logs incoming requests and outgoing responses, useful for debugging and monitoring API activity                                          |
| `RequestId`        | Middleware that generates and attaches a unique request identifier (UUID) to each incoming request for traceability                      |
| `TimeLimiterLayer` | Middleware that restricts API usage to specific time slots. Outside of these allowed periods, it returns a 503 Service Unavailable error |
| `PrometheusLayer`  | Middleware that collects and exposes Prometheus-compatible metrics for monitoring API performance and usage                              |

##### Utility functions

| Name                  | Description                                                              |
|-----------------------|--------------------------------------------------------------------------|
| `body_from_parts`     | Construct a response body from `Parts`, status code, message and headers |
| `header_value_to_str` | Convert `HeaderValue` to `&str`                                          |

#### Extractors

| Name               | Description                                                            |
|--------------------|------------------------------------------------------------------------|
| `ExtractRequestId` | Extracts the unique request identifier (UUID) from the request headers |
| `Path`             | Extracts and deserializes path parameters from the request URL         |
| `Query`            | Extracts and deserializes query string parameters from the request URL |

#### Response helpers

| Name               | Description                                                                                                 |
|--------------------|-------------------------------------------------------------------------------------------------------------|
| `ApiSuccess`       | Represents a successful API response (Status code and data in JSON). It implements the `IntoResponse` trait |
| `ApiError`         | Represents a list of HTTP errors                                                                            |
| `ApiErrorResponse` | Encapsulates the details of an API error response, including the status code and the error message          |

#### Handlers

| Name                | Description                                                                                       |
|---------------------|---------------------------------------------------------------------------------------------------|
| `PrometheusHandler` | Handler that exposes Prometheus metrics endpoint, allowing metrics scraping by Prometheus servers |

## Code coverage

- [2025-06-23] `43.62% coverage, 205/470 lines covered`

## To-Do list

- [ ] Add more documentation
