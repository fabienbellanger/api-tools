//! Access token entity

use crate::{server::axum::response::ApiError, value_objects::datetime::UtcDateTime};
use axum::{extract::FromRequestParts, http::request::Parts};
use hyper::{HeaderMap, header};

/// Access Token Value represents the value of the access token
pub type AccessTokenValue = String;

/// Access Token
#[derive(Debug, Clone, PartialEq)]
pub struct AccessToken {
    /// Token
    pub token: AccessTokenValue,

    /// Expiration time
    pub expired_at: UtcDateTime,
}

impl AccessToken {
    /// Create a new access token
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
