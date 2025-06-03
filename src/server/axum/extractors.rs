//! Extractor modules for Axum

use crate::server::axum::layers::request_id::REQUEST_ID_HEADER;
use crate::server::axum::response::ApiError;
use axum::extract::FromRequestParts;
use axum::extract::path::ErrorKind;
use axum::extract::rejection::PathRejection;
use axum::http::request::Parts;
use axum::http::{HeaderValue, StatusCode};
use serde::de::DeserializeOwned;

/// Request ID extractor from HTTP headers
pub struct RequestId(pub HeaderValue);

impl<S> FromRequestParts<S> for RequestId
where
    S: Send + Sync,
{
    type Rejection = ();

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        match parts.headers.get(REQUEST_ID_HEADER.clone()) {
            Some(id) => Ok(RequestId(id.clone())),
            _ => Ok(RequestId(HeaderValue::from_static(""))),
        }
    }
}

/// `Path` extractor customizes the error from `axum::extract::Path`
pub struct Path<T>(pub T);

impl<S, T> FromRequestParts<S> for Path<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = (StatusCode, ApiError);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match axum::extract::Path::<T>::from_request_parts(parts, state).await {
            Ok(value) => Ok(Self(value.0)),
            Err(rejection) => {
                let (status, body) = match rejection {
                    PathRejection::FailedToDeserializePathParams(inner) => {
                        let mut status = StatusCode::BAD_REQUEST;

                        let kind = inner.into_kind();
                        let body = match &kind {
                            ErrorKind::WrongNumberOfParameters { .. } => ApiError::BadRequest(kind.to_string()),
                            ErrorKind::ParseErrorAtKey { .. } => ApiError::BadRequest(kind.to_string()),
                            ErrorKind::ParseErrorAtIndex { .. } => ApiError::BadRequest(kind.to_string()),
                            ErrorKind::ParseError { .. } => ApiError::BadRequest(kind.to_string()),
                            ErrorKind::InvalidUtf8InPathParam { .. } => ApiError::BadRequest(kind.to_string()),
                            ErrorKind::UnsupportedType { .. } => {
                                // this error is caused by the programmer using an unsupported type
                                // (such as nested maps) so respond with `500` instead
                                status = StatusCode::INTERNAL_SERVER_ERROR;
                                ApiError::InternalServerError(kind.to_string())
                            }
                            ErrorKind::Message(msg) => ApiError::BadRequest(msg.clone()),
                            _ => ApiError::BadRequest(format!("Unhandled deserialization error: {kind}")),
                        };

                        (status, body)
                    }
                    PathRejection::MissingPathParams(error) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        ApiError::InternalServerError(error.to_string()),
                    ),
                    _ => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        ApiError::InternalServerError(format!("Unhandled path rejection: {rejection}")),
                    ),
                };

                Err((status, body))
            }
        }
    }
}

/// `Query` extractor customizes the error from `axum::extract::Query`
pub struct Query<T>(pub T);

impl<T, S> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = (StatusCode, ApiError);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or_default();
        let value = serde_urlencoded::from_str(query)
            .map_err(|err| (StatusCode::BAD_REQUEST, ApiError::BadRequest(err.to_string())))?;

        Ok(Query(value))
    }
}
