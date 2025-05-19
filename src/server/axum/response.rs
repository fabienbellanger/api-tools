//! API response module

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

/// API response success
#[derive(Debug, Clone)]
pub struct ApiSuccess<T: Serialize + PartialEq>(StatusCode, Json<T>);

impl<T> PartialEq for ApiSuccess<T>
where
    T: Serialize + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1.0 == other.1.0
    }
}

impl<T: Serialize + PartialEq> ApiSuccess<T> {
    pub fn new(status: StatusCode, data: T) -> Self {
        ApiSuccess(status, Json(data))
    }
}

impl<T: Serialize + PartialEq> IntoResponse for ApiSuccess<T> {
    fn into_response(self) -> Response {
        (self.0, self.1).into_response()
    }
}

/// Generic response structure shared by all API responses.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ApiErrorResponse<T: Serialize + PartialEq> {
    code: u16,
    message: T,
}

impl<T: Serialize + PartialEq> ApiErrorResponse<T> {
    pub fn new(status_code: StatusCode, message: T) -> Self {
        Self {
            code: status_code.as_u16(),
            message,
        }
    }
}

/// API error
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ApiError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unprocessable entity: {0}")]
    UnprocessableEntity(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("Timeout")]
    Timeout,

    #[error("Too many requests")]
    TooManyRequests,

    #[error("Method not allowed")]
    MethodNotAllowed,

    #[error("Payload too large")]
    PayloadTooLarge,

    #[error("Service unavailable")]
    ServiceUnavailable,
}

impl ApiError {
    fn response(code: StatusCode, message: &str) -> impl IntoResponse + '_ {
        match code {
            StatusCode::REQUEST_TIMEOUT => (
                StatusCode::REQUEST_TIMEOUT,
                Json(ApiErrorResponse::new(StatusCode::REQUEST_TIMEOUT, message)),
            ),
            StatusCode::TOO_MANY_REQUESTS => (
                StatusCode::TOO_MANY_REQUESTS,
                Json(ApiErrorResponse::new(StatusCode::TOO_MANY_REQUESTS, message)),
            ),
            StatusCode::METHOD_NOT_ALLOWED => (
                StatusCode::METHOD_NOT_ALLOWED,
                Json(ApiErrorResponse::new(StatusCode::METHOD_NOT_ALLOWED, message)),
            ),
            StatusCode::PAYLOAD_TOO_LARGE => (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(ApiErrorResponse::new(StatusCode::PAYLOAD_TOO_LARGE, message)),
            ),
            StatusCode::BAD_REQUEST => (
                StatusCode::BAD_REQUEST,
                Json(ApiErrorResponse::new(StatusCode::BAD_REQUEST, message)),
            ),
            StatusCode::UNAUTHORIZED => (
                StatusCode::UNAUTHORIZED,
                Json(ApiErrorResponse::new(StatusCode::UNAUTHORIZED, message)),
            ),
            StatusCode::FORBIDDEN => (
                StatusCode::FORBIDDEN,
                Json(ApiErrorResponse::new(StatusCode::FORBIDDEN, message)),
            ),
            StatusCode::NOT_FOUND => (
                StatusCode::NOT_FOUND,
                Json(ApiErrorResponse::new(StatusCode::NOT_FOUND, message)),
            ),
            StatusCode::SERVICE_UNAVAILABLE => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiErrorResponse::new(StatusCode::SERVICE_UNAVAILABLE, message)),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse::new(StatusCode::INTERNAL_SERVER_ERROR, message)),
            ),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Timeout => Self::response(StatusCode::REQUEST_TIMEOUT, "Request timeout").into_response(),
            ApiError::TooManyRequests => {
                Self::response(StatusCode::TOO_MANY_REQUESTS, "Too many requests").into_response()
            }
            ApiError::MethodNotAllowed => {
                Self::response(StatusCode::METHOD_NOT_ALLOWED, "Method not allowed").into_response()
            }
            ApiError::PayloadTooLarge => {
                Self::response(StatusCode::PAYLOAD_TOO_LARGE, "Payload too large").into_response()
            }
            ApiError::ServiceUnavailable => {
                Self::response(StatusCode::SERVICE_UNAVAILABLE, "Service unavailable").into_response()
            }
            ApiError::BadRequest(message) => Self::response(StatusCode::BAD_REQUEST, &message).into_response(),
            ApiError::Unauthorized(message) => Self::response(StatusCode::UNAUTHORIZED, &message).into_response(),
            ApiError::Forbidden(message) => Self::response(StatusCode::FORBIDDEN, &message).into_response(),
            ApiError::NotFound(message) => Self::response(StatusCode::NOT_FOUND, &message).into_response(),
            ApiError::UnprocessableEntity(message) => {
                Self::response(StatusCode::UNPROCESSABLE_ENTITY, &message).into_response()
            }
            ApiError::InternalServerError(message) => {
                Self::response(StatusCode::INTERNAL_SERVER_ERROR, &message).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_new_api_error_response() {
        let error = ApiErrorResponse::new(StatusCode::BAD_REQUEST, "Bad request");
        assert_eq!(error.code, 400);
        assert_eq!(error.message, "Bad request");
    }

    #[tokio::test]
    async fn test_api_error_into_response() {
        let error = ApiError::BadRequest("Invalid input".to_string());
        assert_eq!(error.to_string(), "Bad request: Invalid input");

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = response.into_body();
        let body_bytes = axum::body::to_bytes(body, 1_024).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(body_str, json!({ "code": 400, "message": "Invalid input" }).to_string());
    }

    #[tokio::test]
    async fn test_api_error_response() {
        let response = ApiError::response(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error");
        let response = response.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = response.into_body();
        let body_bytes = axum::body::to_bytes(body, 1_024).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(
            body_str,
            json!({ "code": 500, "message": "Internal server error" }).to_string()
        );
    }
}
