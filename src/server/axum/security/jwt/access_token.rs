//! Access token entity

use crate::{server::axum::response::ApiError, value_objects::datetime::UtcDateTime};
use axum::{extract::FromRequestParts, http::request::Parts};
use hyper::{HeaderMap, header};
use serde::Deserialize;

/// Access Token Value represents the value of the access token
pub type AccessTokenValue = String;

/// Access Token
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AccessToken {
    /// Token
    pub token: AccessTokenValue,

    /// Expiration time
    pub expired_at: UtcDateTime,
}

impl AccessToken {
    /// Create a new access token
    ///
    /// # Example
    ///
    /// ```
    /// use api_tools::server::axum::security::jwt::access_token::AccessToken;
    /// use api_tools::value_objects::datetime::UtcDateTime;
    ///
    /// let token = "my_access_token".to_string();
    /// let expired_at = UtcDateTime::now();
    /// let access_token = AccessToken::new(token, expired_at.clone());
    ///
    /// assert_eq!(access_token.token, "my_access_token".to_string());
    /// assert_eq!(access_token.expired_at, expired_at);
    /// ```
    #[must_use]
    pub const fn new(token: String, expired_at: UtcDateTime) -> Self {
        Self { token, expired_at }
    }

    /// Extract bearer token from headers
    ///
    /// The header must be `Bearer <token>`. The scheme is case-insensitive (RFC 6750) and an empty
    /// token is rejected.
    pub fn extract_bearer_token_from_headers(headers: &HeaderMap) -> Option<Self> {
        headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.trim_start().split_once(' '))
            .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("Bearer"))
            .map(|(_, token)| token.trim())
            .filter(|token| !token.is_empty())
            .map(|token| Self::new(token.to_string(), UtcDateTime::now()))
    }
}

/// JWT extractor from HTTP headers
impl<S> FromRequestParts<S> for AccessToken
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Self::extract_bearer_token_from_headers(&parts.headers)
            .ok_or_else(|| ApiError::Unauthorized("Missing or invalid token".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_bearer_token_from_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer my_token"));

        let token = AccessToken::extract_bearer_token_from_headers(&headers);
        assert!(token.is_some());
        assert_eq!(token.unwrap().token, "my_token");
    }

    #[test]
    fn test_extract_bearer_token_from_headers_invalid() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Invalid my_token"));

        let token = AccessToken::extract_bearer_token_from_headers(&headers);
        assert!(token.is_none());
    }

    #[tokio::test]
    async fn from_request_parts_succeeds_when_bearer_header_is_present() {
        let req = axum::http::Request::builder()
            .header(header::AUTHORIZATION, "Bearer valid.jwt.token")
            .body(())
            .unwrap();
        let (mut parts, ()) = req.into_parts();

        let token = AccessToken::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(token.token, "valid.jwt.token");
    }

    #[tokio::test]
    async fn from_request_parts_returns_unauthorized_when_header_is_missing() {
        let req = axum::http::Request::builder().body(()).unwrap();
        let (mut parts, ()) = req.into_parts();

        let err = AccessToken::from_request_parts(&mut parts, &()).await.unwrap_err();
        assert!(matches!(err, ApiError::Unauthorized(_)));
    }

    #[test]
    fn test_extract_bearer_token_from_headers_missing_header() {
        let headers = HeaderMap::new();
        assert!(AccessToken::extract_bearer_token_from_headers(&headers).is_none());
    }

    #[test]
    fn test_extract_bearer_token_from_headers_trims_whitespace() {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer    my_token   "));

        let token = AccessToken::extract_bearer_token_from_headers(&headers).unwrap();
        assert_eq!(token.token, "my_token");
    }

    /// A header value with opaque (non visible-ASCII) bytes cannot be turned
    /// into a `&str` and must be ignored rather than panicking.
    #[test]
    fn test_extract_bearer_token_from_headers_non_ascii_value() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_bytes(b"Bearer \xfftoken").unwrap(),
        );

        assert!(AccessToken::extract_bearer_token_from_headers(&headers).is_none());
    }

    #[tokio::test]
    async fn from_request_parts_returns_unauthorized_for_non_bearer_scheme() {
        let req = axum::http::Request::builder()
            .header(header::AUTHORIZATION, "Basic dXNlcjpwYXNz")
            .body(())
            .unwrap();
        let (mut parts, ()) = req.into_parts();

        let err = AccessToken::from_request_parts(&mut parts, &()).await.unwrap_err();
        assert_eq!(err, ApiError::Unauthorized("Missing or invalid token".to_string()));
    }

    fn extract(value: &'static str) -> Option<String> {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static(value));
        AccessToken::extract_bearer_token_from_headers(&headers).map(|t| t.token)
    }

    #[test]
    fn test_extract_bearer_token_rejects_empty_token() {
        assert_eq!(extract("Bearer"), None);
        assert_eq!(extract("Bearer "), None);
        assert_eq!(extract("Bearer     "), None);
    }

    /// RFC 6750: the auth scheme is case-insensitive.
    #[test]
    fn test_extract_bearer_token_scheme_is_case_insensitive() {
        assert_eq!(extract("bearer tok"), Some("tok".to_string()));
        assert_eq!(extract("BEARER tok"), Some("tok".to_string()));
    }

    /// `Bearer` must be the scheme itself, not a substring of the header.
    #[test]
    fn test_extract_bearer_token_rejects_bearer_not_used_as_scheme() {
        assert_eq!(extract("xBearer tok"), None);
        assert_eq!(extract("Token Bearer tok"), None);
        assert_eq!(extract("Bearertok"), None);
        assert_eq!(extract("tok"), None);
    }

    #[test]
    fn test_extract_bearer_token_ignores_leading_whitespace() {
        assert_eq!(extract("  Bearer tok"), Some("tok".to_string()));
    }

    #[test]
    fn test_extract_bearer_token_keeps_token_containing_bearer() {
        assert_eq!(extract("Bearer aBearerb"), Some("aBearerb".to_string()));
    }

    /// An empty token is a client error (401), not a server one.
    #[tokio::test]
    async fn from_request_parts_returns_unauthorized_for_empty_bearer_token() {
        let req = axum::http::Request::builder()
            .header(header::AUTHORIZATION, "Bearer ")
            .body(())
            .unwrap();
        let (mut parts, ()) = req.into_parts();

        let err = AccessToken::from_request_parts(&mut parts, &()).await.unwrap_err();
        assert!(matches!(err, ApiError::Unauthorized(_)));
    }
}
