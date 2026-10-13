//! Axum layers

pub mod basic_auth;
pub mod body_limiter;
pub mod cors;
pub mod http_errors;
pub mod logger;
#[cfg(feature = "prometheus")]
pub mod prometheus;
pub mod request_id;
pub mod security_headers;
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
    let msg = serde_json::json!(ApiErrorResponse::new(status_code, message, None));

    Bytes::from(msg.to_string())
}

/// Convert `HeaderValue` to `&str`
#[must_use]
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

    #[test]
    fn test_header_value_to_str_with_non_utf8_value() {
        let header_value = HeaderValue::from_bytes(b"\xff\xfe").unwrap();
        assert_eq!(header_value_to_str(Some(&header_value)), "");
    }

    fn empty_parts() -> Parts {
        let (parts, ()) = axum::http::Response::new(()).into_parts();
        parts
    }

    #[test]
    fn test_body_from_parts_sets_status_content_type_and_json_body() {
        let mut parts = empty_parts();
        let body = body_from_parts(&mut parts, StatusCode::FORBIDDEN, "Access denied", None);

        assert_eq!(parts.status, StatusCode::FORBIDDEN);
        assert_eq!(parts.headers.get(CONTENT_TYPE).unwrap(), "application/json");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            serde_json::json!({ "code": 403, "message": "Access denied" })
        );
    }

    #[test]
    fn test_body_from_parts_adds_extra_headers() {
        let mut parts = empty_parts();
        body_from_parts(
            &mut parts,
            StatusCode::UNAUTHORIZED,
            "Unauthorized",
            Some(vec![
                (
                    axum::http::header::WWW_AUTHENTICATE,
                    HeaderValue::from_static("basic realm=RESTRICTED"),
                ),
                (HeaderName::from_static("x-custom"), HeaderValue::from_static("value")),
            ]),
        );

        assert_eq!(
            parts.headers.get(axum::http::header::WWW_AUTHENTICATE).unwrap(),
            "basic realm=RESTRICTED"
        );
        assert_eq!(parts.headers.get("x-custom").unwrap(), "value");
    }

    #[test]
    fn test_body_from_parts_overrides_existing_content_type() {
        let mut parts = empty_parts();
        parts
            .headers
            .insert(CONTENT_TYPE, HeaderValue::from_static("text/html"));

        body_from_parts(&mut parts, StatusCode::NOT_FOUND, "Not found", None);

        assert_eq!(parts.headers.get(CONTENT_TYPE).unwrap(), "application/json");
        assert_eq!(parts.headers.get_all(CONTENT_TYPE).iter().count(), 1);
    }
}
