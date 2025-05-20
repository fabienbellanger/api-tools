//! Extractor modules for Axum

use crate::server::axum::layers::request_id::REQUEST_ID_HEADER;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::HeaderValue;

/// Request ID extractor from HTTP headers
pub struct ExtractRequestId(pub HeaderValue);

impl<S> FromRequestParts<S> for ExtractRequestId
where
    S: Send + Sync,
{
    type Rejection = ();

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        match parts.headers.get(REQUEST_ID_HEADER.clone()) {
            Some(id) => Ok(ExtractRequestId(id.clone())),
            _ => Ok(ExtractRequestId(HeaderValue::from_static(""))),
        }
    }
}
