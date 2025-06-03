//! Axum layers

pub mod basic_auth;
pub mod cors;
pub mod http_errors;
pub mod logger;
pub mod request_id;
pub mod time_limiter;

use crate::server::axum::response::ApiErrorResponse;
use axum::http::header::CONTENT_TYPE;
use axum::http::response::Parts;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use bytes::Bytes;
use std::str::from_utf8;

/// Construct a response body from `Parts`, status code, message and headers
pub fn body_from_parts(
    parts: &mut Parts,
    status_code: StatusCode,
    message: &str,
    headers: Option<Vec<(HeaderName, HeaderValue)>>,
) -> Bytes {
    // Status
    parts.status = status_code;

    // Headers
    parts
        .headers
        .insert(CONTENT_TYPE, HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()));
    if let Some(headers) = headers {
        for header in headers {
            parts.headers.insert(header.0, header.1);
        }
    }

    // Body
    let msg = serde_json::json!(ApiErrorResponse::new(status_code, message));

    Bytes::from(msg.to_string())
}

/// Convert `HeaderValue` to `&str`
pub fn header_value_to_str(value: Option<&HeaderValue>) -> &str {
    match value {
        Some(value) => from_utf8(value.as_bytes()).unwrap_or_default(),
        None => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_value_to_str() {
        let header_value = HeaderValue::from_static("test_value");
        let result = header_value_to_str(Some(&header_value));
        assert_eq!(result, "test_value");

        let none_result = header_value_to_str(None);
        assert_eq!(none_result, "");
    }
}
