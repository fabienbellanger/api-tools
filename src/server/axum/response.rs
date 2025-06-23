//! API response module

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use opentelemetry::TraceId;
use opentelemetry::trace::TraceContextExt;
use serde::Serialize;
use thiserror::Error;
use tracing_opentelemetry::OpenTelemetrySpanExt;

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
pub(crate) struct ApiErrorResponse<T: Serialize + PartialEq> {
    code: u16,
    message: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace_id: Option<String>,
}

impl<T: Serialize + PartialEq> ApiErrorResponse<T> {
    pub(crate) fn new(status_code: StatusCode, message: T, trace_id: Option<String>) -> Self {
        Self {
            code: status_code.as_u16(),
            message,
            trace_id,
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
        let ctx = tracing::Span::current().context();
        let trace_id = ctx.span().span_context().trace_id();
        let trace_id = if trace_id == TraceId::INVALID {
            None
        } else {
            Some(trace_id.to_string())
        };

        match code {
            StatusCode::REQUEST_TIMEOUT => (
                StatusCode::REQUEST_TIMEOUT,
                Json(ApiErrorResponse::new(StatusCode::REQUEST_TIMEOUT, message, trace_id)),
            ),
            StatusCode::TOO_MANY_REQUESTS => (
                StatusCode::TOO_MANY_REQUESTS,
                Json(ApiErrorResponse::new(StatusCode::TOO_MANY_REQUESTS, message, trace_id)),
            ),
            StatusCode::METHOD_NOT_ALLOWED => (
                StatusCode::METHOD_NOT_ALLOWED,
                Json(ApiErrorResponse::new(StatusCode::METHOD_NOT_ALLOWED, message, trace_id)),
            ),
            StatusCode::PAYLOAD_TOO_LARGE => (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(ApiErrorResponse::new(StatusCode::PAYLOAD_TOO_LARGE, message, trace_id)),
            ),
            StatusCode::BAD_REQUEST => (
                StatusCode::BAD_REQUEST,
                Json(ApiErrorResponse::new(StatusCode::BAD_REQUEST, message, trace_id)),
            ),
            StatusCode::UNAUTHORIZED => (
                StatusCode::UNAUTHORIZED,
                Json(ApiErrorResponse::new(StatusCode::UNAUTHORIZED, message, trace_id)),
            ),
            StatusCode::FORBIDDEN => (
                StatusCode::FORBIDDEN,
                Json(ApiErrorResponse::new(StatusCode::FORBIDDEN, message, trace_id)),
            ),
            StatusCode::NOT_FOUND => (
                StatusCode::NOT_FOUND,
                Json(ApiErrorResponse::new(StatusCode::NOT_FOUND, message, trace_id)),
            ),
            StatusCode::SERVICE_UNAVAILABLE => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiErrorResponse::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    message,
                    trace_id,
                )),
            ),
            StatusCode::UNPROCESSABLE_ENTITY => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ApiErrorResponse::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    message,
                    trace_id,
                )),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    message,
                    trace_id,
                )),
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
    fn test_api_success_partial_eq() {
        let success1 = ApiSuccess::new(StatusCode::OK, json!({"data": "test"}));
        let success2 = ApiSuccess::new(StatusCode::OK, json!({"data": "test"}));
        assert_eq!(success1, success2);

        let success3 = ApiSuccess::new(StatusCode::BAD_REQUEST, json!({"data": "test"}));
        assert_ne!(success1, success3);
    }

    #[tokio::test]
    async fn test_api_success_into_response() {
        let data = json!({"hello": "world"});
        let api_success = ApiSuccess::new(StatusCode::OK, data.clone());
        let response = api_success.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body();
        let body_bytes = axum::body::to_bytes(body, 1_024).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(body_str, data.to_string());
    }

    #[test]
    fn test_new_api_error_response() {
        let error = ApiErrorResponse::new(StatusCode::BAD_REQUEST, "Bad request", None);
        assert_eq!(error.code, 400);
        assert_eq!(error.message, "Bad request");
    }

    #[tokio::test]
    async fn test_api_error_into_response_bad_request() {
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
    async fn test_api_error_into_response_unauthorized() {
        let error = ApiError::Unauthorized("Not authorized".to_string());
        assert_eq!(error.to_string(), "Unauthorized: Not authorized");

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = response.into_body();
        let body_bytes = axum::body::to_bytes(body, 1_024).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(
            body_str,
            json!({ "code": 401, "message": "Not authorized" }).to_string()
        );
    }

    #[tokio::test]
    async fn test_api_error_into_response_forbidden() {
        let error = ApiError::Forbidden("Access denied".to_string());
        assert_eq!(error.to_string(), "Forbidden: Access denied");

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let body = response.into_body();
        let body_bytes = axum::body::to_bytes(body, 1_024).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(body_str, json!({ "code": 403, "message": "Access denied" }).to_string());
    }

    #[tokio::test]
    async fn test_api_error_into_response_not_found() {
        let error = ApiError::NotFound("Resource missing".to_string());
        assert_eq!(error.to_string(), "Not found: Resource missing");

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = response.into_body();
        let body_bytes = axum::body::to_bytes(body, 1_024).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(
            body_str,
            json!({ "code": 404, "message": "Resource missing" }).to_string()
        );
    }

    #[tokio::test]
    async fn test_api_error_into_response_unprocessable_entity() {
        let error = ApiError::UnprocessableEntity("Invalid data".to_string());
        assert_eq!(error.to_string(), "Unprocessable entity: Invalid data");

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body = response.into_body();
        let body_bytes = axum::body::to_bytes(body, 1_024).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(body_str, json!({ "code": 422, "message": "Invalid data" }).to_string());
    }

    #[tokio::test]
    async fn test_api_error_into_response_internal_server_error() {
        let error = ApiError::InternalServerError("Unexpected".to_string());
        assert_eq!(error.to_string(), "Internal server error: Unexpected");

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = response.into_body();
        let body_bytes = axum::body::to_bytes(body, 1_024).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(body_str, json!({ "code": 500, "message": "Unexpected" }).to_string());
    }

    #[tokio::test]
    async fn test_api_error_into_response_timeout() {
        let error = ApiError::Timeout;
        assert_eq!(error.to_string(), "Timeout");

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);

        let body = response.into_body();
        let body_bytes = axum::body::to_bytes(body, 1_024).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(
            body_str,
            json!({ "code": 408, "message": "Request timeout" }).to_string()
        );
    }

    #[tokio::test]
    async fn test_api_error_into_response_too_many_requests() {
        let error = ApiError::TooManyRequests;
        assert_eq!(error.to_string(), "Too many requests");

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        let body = response.into_body();
        let body_bytes = axum::body::to_bytes(body, 1_024).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(
            body_str,
            json!({ "code": 429, "message": "Too many requests" }).to_string()
        );
    }

    #[tokio::test]
    async fn test_api_error_into_response_method_not_allowed() {
        let error = ApiError::MethodNotAllowed;
        assert_eq!(error.to_string(), "Method not allowed");

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

        let body = response.into_body();
        let body_bytes = axum::body::to_bytes(body, 1_024).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(
            body_str,
            json!({ "code": 405, "message": "Method not allowed" }).to_string()
        );
    }

    #[tokio::test]
    async fn test_api_error_into_response_payload_too_large() {
        let error = ApiError::PayloadTooLarge;
        assert_eq!(error.to_string(), "Payload too large");

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);

        let body = response.into_body();
        let body_bytes = axum::body::to_bytes(body, 1_024).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(
            body_str,
            json!({ "code": 413, "message": "Payload too large" }).to_string()
        );
    }

    #[tokio::test]
    async fn test_api_error_into_response_service_unavailable() {
        let error = ApiError::ServiceUnavailable;
        assert_eq!(error.to_string(), "Service unavailable");

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = response.into_body();
        let body_bytes = axum::body::to_bytes(body, 1_024).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(
            body_str,
            json!({ "code": 503, "message": "Service unavailable" }).to_string()
        );
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
