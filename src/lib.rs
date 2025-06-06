//! # Api Tools - A toolkit for API in Rust
//!
//! Toolkit for API in Rust
//!
//! API Tools is a Rust library providing utilities for developing robust, consistent, and secure APIs.
//! It offers ready-to-use layers, extractors, error handling, and helpers designed to simplify API development,
//! especially with the Axum framework.
//! The toolkit aims to standardize common API patterns and reduce boilerplate in your Rust projects.
//!
//! ## Features list
//!
//! | Name         | Description                       | Default |
//! | ------------ | --------------------------------- | :-----: |
//! | `axum`       | Enable Axum feature               |   ❌    |
//! | `prometheus` | Enable Prometheus metrics feature |   ❌    |
//! | `full`       | Enable all features               |   ❌    |
//!
//! ## Components
//!
//! ### Value objects
//!
//! | Name          | Description                                                                                |
//! | ------------- | ------------------------------------------------------------------------------------------ |
//! | `UtcDateTime` | A wrapper around `chrono::DateTime` to handle date and time values in UTC                  |
//! | `Timezone`    | A wrapper around `chrono_tz::Tz` to handle time zones                                      |
//! | `Pagination`  | A struct to handle pagination parameters, including page number, page size and total count |
//! | `QuerySort`   | A struct to handle sorting query parameters, including field and direction                 |
//!
//! ### Axum
//!
//! #### Layers
//!
//! | Name               | Description                                                                                                                              |
//! | ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------- |
//! | `BasicAuthLayer`   | Provides HTTP Basic Authentication middleware for protecting routes with username and password                                           |
//! | `CorsLayer`        | Adds Cross-Origin Resource Sharing (CORS) headers to responses, allowing or restricting resource sharing between different origins       |
//! | `HttpErrorsLayer`  | Middleware for intercepting and customizing HTTP error responses, enabling standardized error handling across your API                   |
//! | `LoggerLayer`      | Logs incoming requests and outgoing responses, useful for debugging and monitoring API activity                                          |
//! | `RequestId`        | Middleware that generates and attaches a unique request identifier (UUID) to each incoming request for traceability                      |
//! | `TimeLimiterLayer` | Middleware that restricts API usage to specific time slots. Outside of these allowed periods, it returns a 503 Service Unavailable error |
//! | `PrometheusLayer`  | Middleware that collects and exposes Prometheus-compatible metrics for monitoring API performance and usage                              |
//!
//! ##### Utility functions
//!
//! | Name                  | Description                                                              |
//! | --------------------- | ------------------------------------------------------------------------ |
//! | `body_from_parts`     | Construct a response body from `Parts`, status code, message and headers |
//! | `header_value_to_str` | Convert `HeaderValue` to `&str`                                          |
//!
//! #### Extractors
//!
//! | Name               | Description                                                            |
//! | ------------------ | ---------------------------------------------------------------------- |
//! | `ExtractRequestId` | Extracts the unique request identifier (UUID) from the request headers |
//! | `Path`             | Extracts and deserializes path parameters from the request URL         |
//! | `Query`            | Extracts and deserializes query string parameters from the request URL |
//!
//! #### Response helpers
//!
//! | Name               | Description                                                                                                 |
//! | ------------------ | ----------------------------------------------------------------------------------------------------------- |
//! | `ApiSuccess`       | Represents a successful API response (Status code and data in JSON). It implements the `IntoResponse` trait |
//! | `ApiError`         | Represents a list of HTTP errors                                                                            |
//! | `ApiErrorResponse` | Encapsulates the details of an API error response, including the status code and the error message          |
//! 
//! #### Handlers
//! 
//! | Name                | Description                                                                                       |
//! | ------------------- | ------------------------------------------------------------------------------------------------- |
//! | `PrometheusHandler` | Handler that exposes Prometheus metrics endpoint, allowing metrics scraping by Prometheus servers |

#[macro_use]
extern crate tracing;

pub mod security;
pub mod server;
pub mod value_objects;
