//! Access token entity

use crate::{server::axum::response::ApiError, value_objects::datetime::UtcDateTime};
use axum::{extract::FromRequestParts, http::request::Parts};
use hyper::{HeaderMap, header};
use serde::Deserialize;

/// Access Token Value represents the value of the access token
pub type AccessTokenValue = String;

/// Access Token
#[derive(Debug, Clone, PartialEq, Deserialize)]
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
    /// use api_tools::security::jwt::access_token::AccessToken;
    /// use api_tools::value_objects::datetime::UtcDateTime;
    ///
    /// let token = "my_access_token".to_string();
    /// let expired_at = UtcDateTime::now();
    /// let access_token = AccessToken::new(token, expired_at.clone());
    ///
    /// assert_eq!(access_token.token, "my_access_token".to_string());
    /// assert_eq!(access_token.expired_at, expired_at);
    /// ```
    pub fn new(token: String, expired_at: UtcDateTime) -> Self {
        Self { token, expired_at }
    }

    /// Extract bearer token from headers
    pub fn extract_bearer_token_from_headers(headers: &HeaderMap) -> Option<Self> {
        headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| {
                let words = h.split("Bearer").collect::<Vec<&str>>();
                words.get(1).map(|w| w.trim())
            })
            .map(|token| AccessToken::new(token.to_string(), UtcDateTime::now()))
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
            .ok_or(ApiError::Unauthorized("Missing or invalid token".to_string()))
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
}
