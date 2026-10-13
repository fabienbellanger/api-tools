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
        axum::extract::Path::<T>::from_request_parts(parts, state)
            .await
            .map(|value| Self(value.0))
            .map_err(path_rejection_to_api_error)
    }
}

/// Convert a `Path` rejection into a status code and an `ApiError`
///
/// Not generic, so it is compiled once and not for every `Path<T>`.
fn path_rejection_to_api_error(rejection: PathRejection) -> (StatusCode, ApiError) {
    match rejection {
        PathRejection::FailedToDeserializePathParams(inner) => {
            let mut status = StatusCode::BAD_REQUEST;

            let kind = inner.into_kind();
            let body = match &kind {
                ErrorKind::WrongNumberOfParameters { .. } => ApiError::BadRequest(kind.to_string()),
                ErrorKind::ParseErrorAtKey { .. } => ApiError::BadRequest(kind.to_string()),
                ErrorKind::ParseErrorAtIndex { .. } => ApiError::BadRequest(kind.to_string()),
                ErrorKind::ParseError { .. } => ApiError::BadRequest(kind.to_string()),
                ErrorKind::InvalidUtf8InPathParam { .. } => ApiError::BadRequest(kind.to_string()),
                ErrorKind::DeserializeError { .. } => ApiError::BadRequest(kind.to_string()),
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

    #[tokio::test]
    async fn path_extractor_returns_400_on_wrong_number_of_parameters() {
        // Two captures but a single-value extractor.
        let app: Router = Router::new().route(
            "/users/{id}/{name}",
            get(|Path(id): Path<u64>| async move { id.to_string() }),
        );

        let response = app
            .oneshot(Request::builder().uri("/users/1/alice").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = read_body(response).await;
        assert!(body.contains("\"code\":400"), "body was: {body}");
        assert!(body.contains("Wrong number of path arguments"), "body was: {body}");
    }

    #[tokio::test]
    async fn path_extractor_deserializes_tuple() {
        let app: Router = Router::new().route(
            "/users/{id}/posts/{post_id}",
            get(|Path((id, post_id)): Path<(u64, u64)>| async move { format!("{id}-{post_id}") }),
        );

        let response = app
            .oneshot(Request::builder().uri("/users/1/posts/2").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(read_body(response).await, "1-2");
    }

    #[tokio::test]
    async fn path_extractor_returns_400_on_tuple_type_mismatch() {
        let app: Router = Router::new().route(
            "/users/{id}/posts/{post_id}",
            get(|Path((id, post_id)): Path<(u64, u64)>| async move { format!("{id}-{post_id}") }),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/users/1/posts/abc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = read_body(response).await;
        assert!(body.contains("\"code\":400"), "body was: {body}");
    }

    #[tokio::test]
    async fn path_extractor_returns_400_with_custom_deserialize_message() {
        #[derive(Deserialize)]
        #[serde(try_from = "String")]
        struct Slug(#[allow(dead_code)] String);

        impl TryFrom<String> for Slug {
            type Error = String;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.chars().all(|c| c.is_ascii_lowercase()) {
                    Ok(Self(value))
                } else {
                    Err("slug must be lowercase".to_string())
                }
            }
        }

        let app: Router = Router::new().route("/posts/{slug}", get(|_: Path<Slug>| async { "ok" }));

        let response = app
            .oneshot(Request::builder().uri("/posts/Hello").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = read_body(response).await;
        assert!(body.contains("slug must be lowercase"), "body was: {body}");
    }

    /// Nested maps are not supported by the path deserializer: this is a
    /// programmer error, reported as a 500.
    #[tokio::test]
    async fn path_extractor_returns_500_on_unsupported_type() {
        let app: Router = Router::new().route(
            "/users/{id}",
            get(|_: Path<std::collections::HashMap<String, std::collections::HashMap<String, String>>>| async { "ok" }),
        );

        let response = app
            .oneshot(Request::builder().uri("/users/1").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = read_body(response).await;
        assert!(body.contains("\"code\":500"), "body was: {body}");
    }

    /// On a route without any capture, axum provides an empty parameter list,
    /// which surfaces as a wrong number of parameters (400).
    #[tokio::test]
    async fn path_extractor_returns_400_when_route_has_no_params() {
        let app: Router = Router::new().route("/users", get(path_handler));

        let response = app
            .oneshot(Request::builder().uri("/users").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    /// Outside a router there are no path params at all: this is a
    /// programmer error, reported as a 500.
    #[tokio::test]
    async fn path_extractor_returns_500_when_used_outside_a_router() {
        let (mut parts, _) = Request::builder().uri("/users/1").body(()).unwrap().into_parts();

        let Err((status, error)) = Path::<u64>::from_request_parts(&mut parts, &()).await else {
            panic!("extraction must fail without path params");
        };

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(matches!(error, ApiError::InternalServerError(_)));
    }

    #[tokio::test]
    async fn query_extractor_accepts_missing_query_for_optional_fields() {
        #[derive(Deserialize)]
        struct OptionalArgs {
            page: Option<u32>,
        }

        let app: Router = Router::new().route(
            "/",
            get(|Query(args): Query<OptionalArgs>| async move { format!("{:?}", args.page) }),
        );

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(read_body(response).await, "None");
    }

    #[tokio::test]
    async fn query_extractor_decodes_url_encoded_values() {
        #[derive(Deserialize)]
        struct SearchArgs {
            q: String,
        }

        let app: Router = Router::new().route("/", get(|Query(args): Query<SearchArgs>| async move { args.q }));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/?q=hello%20world%26co")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(read_body(response).await, "hello world&co");
    }

    #[tokio::test]
    async fn path_extractor_returns_400_on_single_value_type_mismatch() {
        let app: Router = Router::new().route("/users/{id}", get(|Path(id): Path<u64>| async move { id.to_string() }));

        let response = app
            .oneshot(Request::builder().uri("/users/abc").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = read_body(response).await;
        assert!(body.contains("Cannot parse `abc` to a `u64`"), "body was: {body}");
    }

    #[tokio::test]
    async fn path_extractor_returns_400_on_invalid_utf8() {
        let app: Router = Router::new().route("/users/{name}", get(|Path(name): Path<String>| async move { name }));

        let response = app
            .oneshot(Request::builder().uri("/users/%FF").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = read_body(response).await;
        assert!(body.contains("Invalid UTF-8 in `name`"), "body was: {body}");
    }

    /// A custom deserialization error on a named param keeps its message and
    /// is not reported as an unhandled error.
    #[tokio::test]
    async fn path_extractor_returns_400_on_custom_deserialize_error_at_key() {
        #[derive(Deserialize)]
        #[serde(try_from = "u64")]
        struct Even(#[allow(dead_code)] u64);

        impl TryFrom<u64> for Even {
            type Error = String;

            fn try_from(value: u64) -> Result<Self, Self::Error> {
                if value % 2 == 0 {
                    Ok(Self(value))
                } else {
                    Err("must be even".to_string())
                }
            }
        }

        #[derive(Deserialize)]
        struct Params {
            #[allow(dead_code)]
            id: Even,
        }

        let app: Router = Router::new().route("/users/{id}", get(|_: Path<Params>| async { "ok" }));

        let response = app
            .clone()
            .oneshot(Request::builder().uri("/users/3").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = read_body(response).await;
        assert!(body.contains("must be even"), "body was: {body}");
        assert!(!body.contains("Unhandled"), "body was: {body}");

        let response = app
            .oneshot(Request::builder().uri("/users/4").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// A string param whose custom `Deserialize` rejects the value reports
    /// the key, the value and the message.
    #[tokio::test]
    async fn path_extractor_returns_400_on_deserialize_error_at_key() {
        struct Lowercase;

        impl<'de> Deserialize<'de> for Lowercase {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct LowercaseVisitor;

                impl serde::de::Visitor<'_> for LowercaseVisitor {
                    type Value = Lowercase;

                    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                        f.write_str("a lowercase string")
                    }

                    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Lowercase, E> {
                        if value.chars().all(|c| c.is_ascii_lowercase()) {
                            Ok(Lowercase)
                        } else {
                            Err(E::custom("must be lowercase"))
                        }
                    }
                }

                deserializer.deserialize_str(LowercaseVisitor)
            }
        }

        #[derive(Deserialize)]
        struct Params {
            #[allow(dead_code)]
            slug: Lowercase,
        }

        let app: Router = Router::new().route("/posts/{slug}", get(|_: Path<Params>| async { "ok" }));

        let response = app
            .oneshot(Request::builder().uri("/posts/Hello").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = read_body(response).await;
        assert!(
            body.contains("Cannot parse `slug` with value `Hello`: must be lowercase"),
            "body was: {body}"
        );
        assert!(!body.contains("Unhandled"), "body was: {body}");
    }
}
