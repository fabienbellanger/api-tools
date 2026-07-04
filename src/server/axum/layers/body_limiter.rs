//! Request body size limiter layer
//!
//! Rejects incoming requests whose advertised `Content-Length` exceeds a
//! configured maximum, **before** the body is read. The check is O(1) and does
//! not buffer anything, so it preserves streaming for legitimate requests.
//!
//! # Scope
//!
//! The limit is enforced from the `Content-Length` header. A chunked request
//! that omits `Content-Length` is **not** capped by this layer.
//!
//! # Hard guarantee on chunked bodies
//!
//! For a hard cap that also covers requests without a `Content-Length` (chunked
//! / streamed uploads), stack this layer on top of the re-exported
//! [`RequestBodyLimitLayer`]. `BodyLimiterLayer` rejects the common
//! honest-client case early with a JSON `413`, while `RequestBodyLimitLayer`
//! wraps the body and enforces the same limit as it is read, backstopping the
//! chunked case (with tower-http's plain `413`):
//!
//! ```
//! use api_tools::server::axum::layers::body_limiter::{
//!     BodyLimiterConfig, BodyLimiterLayer, RequestBodyLimitLayer,
//! };
//! use tower::ServiceBuilder;
//!
//! const MAX: usize = 2 * 1024 * 1024; // 2 MiB
//!
//! let _layers = ServiceBuilder::new()
//!     .layer(BodyLimiterLayer::new(&BodyLimiterConfig { body_max_size: MAX }))
//!     .layer(RequestBodyLimitLayer::new(MAX));
//! ```

use crate::server::axum::response::ApiError;
use axum::body::Body;
use axum::http::Request;
use axum::http::header::CONTENT_LENGTH;
use axum::response::{IntoResponse, Response};
use futures::future::BoxFuture;
use std::task::{Context, Poll};
use tower::{Layer, Service};
#[doc(inline)]
pub use tower_http::limit::RequestBodyLimitLayer;

/// Configuration for the `BodyLimiterLayer`
#[derive(Clone, Debug)]
pub struct BodyLimiterConfig {
    /// Maximum size of the request body in bytes
    pub body_max_size: usize,
}

#[derive(Clone)]
pub struct BodyLimiterLayer {
    pub config: BodyLimiterConfig,
}

impl BodyLimiterLayer {
    /// Create a new `BodyLimiterLayer`
    pub fn new(config: &BodyLimiterConfig) -> Self {
        Self { config: config.clone() }
    }
}

impl<S> Layer<S> for BodyLimiterLayer {
    type Service = BodyLimiterMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        BodyLimiterMiddleware {
            inner,
            config: self.config.clone(),
        }
    }
}

#[derive(Clone)]
pub struct BodyLimiterMiddleware<S> {
    inner: S,
    config: BodyLimiterConfig,
}

impl<S> Service<Request<Body>> for BodyLimiterMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    // `BoxFuture` is a type alias for `Pin<Box<dyn Future + Send + 'a>>`
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        // Reject early, without reading the body, when the advertised
        // `Content-Length` exceeds the limit. This is O(1) and preserves
        // streaming for legitimate requests.
        let content_length = request
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<usize>().ok());

        if let Some(length) = content_length
            && length > self.config.body_max_size
        {
            return Box::pin(async move { Ok(ApiError::PayloadTooLarge.into_response()) });
        }

        Box::pin(self.inner.call(request))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use std::convert::Infallible;
    use tower::{ServiceBuilder, ServiceExt};

    fn layer() -> BodyLimiterLayer {
        BodyLimiterLayer::new(&BodyLimiterConfig { body_max_size: 8 })
    }

    fn ok_service() -> impl Service<Request<Body>, Response = Response, Error = Infallible> + Clone {
        ServiceBuilder::new()
            .layer(layer())
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(
                    Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from("ok"))
                        .unwrap(),
                )
            }))
    }

    async fn read_body(response: Response) -> String {
        let body = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    }

    /// A `Content-Length` above the limit is rejected with a JSON `413` and the
    /// inner handler is never invoked.
    #[tokio::test]
    async fn content_length_over_limit_is_rejected_with_413() {
        let svc = ServiceBuilder::new()
            .layer(layer())
            .service(tower::service_fn(|_req: Request<Body>| async {
                // Should never be reached.
                Ok::<_, Infallible>(
                    Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from("ok"))
                        .unwrap(),
                )
            }));

        let response = svc
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(CONTENT_LENGTH, "9")
                    .body(Body::from("123456789"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let body = read_body(response).await;
        assert!(body.contains("\"code\":413"), "body was: {body}");
    }

    /// A `Content-Length` at the limit passes through.
    #[tokio::test]
    async fn content_length_at_limit_passes_through() {
        let response = ok_service()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(CONTENT_LENGTH, "8")
                    .body(Body::from("12345678"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(read_body(response).await, "ok");
    }

    /// A `Content-Length` below the limit passes through.
    #[tokio::test]
    async fn content_length_under_limit_passes_through() {
        let response = ok_service()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(CONTENT_LENGTH, "3")
                    .body(Body::from("abc"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(read_body(response).await, "ok");
    }

    /// Without `Content-Length` the request is not capped by this layer
    /// (documented limitation) and passes through.
    #[tokio::test]
    async fn missing_content_length_passes_through() {
        let response = ok_service()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(read_body(response).await, "ok");
    }
}
