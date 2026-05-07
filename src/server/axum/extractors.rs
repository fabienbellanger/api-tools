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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::get;
    use serde::Deserialize;
    use tower::ServiceExt;

    async fn read_body(response: axum::response::Response) -> String {
        let body = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    }

    // ---------------- RequestId ----------------

    #[tokio::test]
    async fn request_id_returns_header_value_when_present() {
        let app: Router = Router::new().route(
            "/",
            get(|RequestId(value): RequestId| async move { value.to_str().unwrap_or_default().to_owned() }),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-request-id", "abc-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(read_body(response).await, "abc-123");
    }

    #[tokio::test]
    async fn request_id_returns_empty_value_when_header_absent() {
        let app: Router = Router::new().route(
            "/",
            get(|RequestId(value): RequestId| async move { value.to_str().unwrap_or_default().to_owned() }),
        );

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(read_body(response).await, "");
    }

    // ---------------- Path ----------------

    #[derive(Deserialize)]
    struct PathArgs {
        id: u64,
    }

    async fn path_handler(Path(args): Path<PathArgs>) -> String {
        args.id.to_string()
    }

    #[tokio::test]
    async fn path_extractor_deserializes_valid_param() {
        let app: Router = Router::new().route("/users/{id}", get(path_handler));

        let response = app
            .oneshot(Request::builder().uri("/users/42").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(read_body(response).await, "42");
    }

    #[tokio::test]
    async fn path_extractor_returns_400_with_api_error_on_type_mismatch() {
        let app: Router = Router::new().route("/users/{id}", get(path_handler));

        let response = app
            .oneshot(Request::builder().uri("/users/abc").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = read_body(response).await;
        assert!(body.contains("\"code\":400"), "body was: {body}");
    }

    // ---------------- Query ----------------

    #[derive(Deserialize)]
    struct QueryArgs {
        page: u32,
        limit: u32,
    }

    async fn query_handler(Query(args): Query<QueryArgs>) -> String {
        format!("{}-{}", args.page, args.limit)
    }

    #[tokio::test]
    async fn query_extractor_deserializes_valid_query_string() {
        let app: Router = Router::new().route("/", get(query_handler));

        let response = app
            .oneshot(Request::builder().uri("/?page=1&limit=20").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(read_body(response).await, "1-20");
    }

    #[tokio::test]
    async fn query_extractor_returns_400_when_required_keys_missing() {
        let app: Router = Router::new().route("/", get(query_handler));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = read_body(response).await;
        assert!(body.contains("\"code\":400"), "body was: {body}");
    }

    #[tokio::test]
    async fn query_extractor_returns_400_on_type_mismatch() {
        let app: Router = Router::new().route("/", get(query_handler));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/?page=abc&limit=20")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
