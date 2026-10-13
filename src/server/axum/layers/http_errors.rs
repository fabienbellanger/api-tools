//! Override some HTTP errors

use crate::server::axum::response::ApiError;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use futures::future::BoxFuture;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Configuration for the `HttpErrorsLayer`
#[derive(Clone, Debug)]
pub struct HttpErrorsConfig {
    /// Maximum size of the body in bytes
    pub body_max_size: usize,
}

/// Layer rewriting framework errors (`404`, `405`, `422`) into JSON API errors
#[derive(Clone)]
pub struct HttpErrorsLayer {
    /// HTTP errors configuration
    pub config: HttpErrorsConfig,
}

impl HttpErrorsLayer {
    /// Create a new `HttpErrorsLayer`
    #[must_use]
    pub fn new(config: &HttpErrorsConfig) -> Self {
        Self { config: config.clone() }
    }
}

impl<S> Layer<S> for HttpErrorsLayer {
    type Service = HttpErrorsMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HttpErrorsMiddleware {
            inner,
            config: self.config.clone(),
        }
    }
}

/// Middleware produced by [`HttpErrorsLayer`]
#[derive(Clone)]
pub struct HttpErrorsMiddleware<S> {
    inner: S,
    config: HttpErrorsConfig,
}

impl<S> Service<Request<Body>> for HttpErrorsMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Send + Clone + 'static,
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
        let mut inner = self.inner.clone();
        let config = self.config.clone();

        Box::pin(async move {
            let response: Response = inner.call(request).await?;

            // Check the content-type
            let headers = response.headers();
            if let Some(content_type) = headers.get("content-type") {
                let content_type = content_type.to_str().unwrap_or_default();
                if content_type.starts_with("image/")
                    || content_type.starts_with("audio/")
                    || content_type.starts_with("video/")
                {
                    return Ok(response);
                }
            }

            let (parts, body) = response.into_parts();

            // Only 404/405/422 are rewritten. Every other status (notably the 200s that make up
            // the bulk of the traffic) is returned untouched, without buffering: this preserves
            // streaming and avoids a full-body copy + UTF-8 scan, plus the spurious 413 a legitimate
            // large body used to trigger by overflowing `to_bytes`.
            match parts.status {
                // The 405 body is discarded, so there is no need to read it.
                StatusCode::METHOD_NOT_ALLOWED => Ok(ApiError::MethodNotAllowed.into_response()),
                // 422/404 are the only statuses whose body we still need to inspect.
                StatusCode::UNPROCESSABLE_ENTITY | StatusCode::NOT_FOUND => {
                    match axum::body::to_bytes(body, config.body_max_size).await {
                        Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                            Ok(body) => match parts.status {
                                StatusCode::UNPROCESSABLE_ENTITY => {
                                    Ok(ApiError::UnprocessableEntity(body).into_response())
                                }
                                StatusCode::NOT_FOUND if body.is_empty() => {
                                    Ok(ApiError::NotFound("Resource Not Found".to_owned()).into_response())
                                }
                                _ => Ok(Response::from_parts(parts, Body::from(body))),
                            },
                            Err(err) => Ok(ApiError::InternalServerError(err.to_string()).into_response()),
                        },
                        Err(_) => Ok(ApiError::PayloadTooLarge.into_response()),
                    }
                }
                _ => Ok(Response::from_parts(parts, body)),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header;
    use std::convert::Infallible;
    use tower::{ServiceBuilder, ServiceExt};

    fn layer() -> HttpErrorsLayer {
        HttpErrorsLayer::new(&HttpErrorsConfig { body_max_size: 1024 })
    }

    async fn read_body(response: Response) -> String {
        let body = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn ok_response_passes_through_unchanged() {
        let svc = ServiceBuilder::new()
            .layer(layer())
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(
                    Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from("hello"))
                        .unwrap(),
                )
            }));

        let response = svc
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(read_body(response).await, "hello");
    }

    #[tokio::test]
    async fn empty_404_is_rewritten_as_json_api_error() {
        let svc = ServiceBuilder::new()
            .layer(layer())
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::empty())
                        .unwrap(),
                )
            }));

        let response = svc
            .oneshot(Request::builder().uri("/missing").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json",
        );

        let body = read_body(response).await;
        assert!(body.contains("\"code\":404"), "body was: {body}");
        assert!(body.contains("Resource Not Found"), "body was: {body}");
    }

    /// 404 with a non-empty body must be left untouched — the inner service
    /// already provided a response body.
    #[tokio::test]
    async fn non_empty_404_is_passed_through() {
        let svc = ServiceBuilder::new()
            .layer(layer())
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::from("custom 404"))
                        .unwrap(),
                )
            }));

        let response = svc
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(read_body(response).await, "custom 404");
    }

    #[tokio::test]
    async fn method_not_allowed_is_rewritten_as_json_api_error() {
        let svc = ServiceBuilder::new()
            .layer(layer())
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(
                    Response::builder()
                        .status(StatusCode::METHOD_NOT_ALLOWED)
                        .body(Body::empty())
                        .unwrap(),
                )
            }));

        let response = svc
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json",
        );

        let body = read_body(response).await;
        assert!(body.contains("\"code\":405"), "body was: {body}");
        assert!(body.contains("Method not allowed"), "body was: {body}");
    }

    #[tokio::test]
    async fn unprocessable_entity_body_is_wrapped_into_json_message() {
        let svc = ServiceBuilder::new()
            .layer(layer())
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(
                    Response::builder()
                        .status(StatusCode::UNPROCESSABLE_ENTITY)
                        .body(Body::from("validation failed"))
                        .unwrap(),
                )
            }));

        let response = svc
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body = read_body(response).await;
        assert!(body.contains("\"code\":422"), "body was: {body}");
        assert!(body.contains("validation failed"), "body was: {body}");
    }

    /// Binary content-types (image/audio/video) must short-circuit before any
    /// body reading: rewriting them as JSON would corrupt the payload.
    #[tokio::test]
    async fn image_content_type_short_circuits_without_touching_body() {
        let payload = vec![0u8, 1, 2, 3, 4];
        let payload_clone = payload.clone();

        let svc = ServiceBuilder::new()
            .layer(layer())
            .service(tower::service_fn(move |_req: Request<Body>| {
                let payload = payload_clone.clone();
                async move {
                    Ok::<_, Infallible>(
                        Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header(header::CONTENT_TYPE, "image/png")
                            .body(Body::from(payload))
                            .unwrap(),
                    )
                }
            }));

        let response = svc
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
        assert_eq!(body.to_vec(), payload);
    }

    /// The size limit is still enforced on the statuses whose body is actually
    /// read (`422`): an oversized body there is turned into a `413`.
    #[tokio::test]
    async fn unprocessable_entity_over_limit_still_413() {
        let small_layer = HttpErrorsLayer::new(&HttpErrorsConfig { body_max_size: 4 });
        let svc = ServiceBuilder::new()
            .layer(small_layer)
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(
                    Response::builder()
                        .status(StatusCode::UNPROCESSABLE_ENTITY)
                        .body(Body::from("validation failed for many fields"))
                        .unwrap(),
                )
            }));

        let response = svc
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let body = read_body(response).await;
        assert!(body.contains("\"code\":413"), "body was: {body}");
    }

    /// A `200 OK` whose body exceeds `body_max_size` must pass through untouched:
    /// the body is never buffered for passthrough statuses, so it can no longer
    /// be turned into a spurious `413`.
    #[tokio::test]
    async fn large_ok_body_is_not_truncated_to_413() {
        let small_layer = HttpErrorsLayer::new(&HttpErrorsConfig { body_max_size: 4 });
        let svc = ServiceBuilder::new()
            .layer(small_layer)
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(
                    Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from("hello world"))
                        .unwrap(),
                )
            }));

        let response = svc
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(read_body(response).await, "hello world");
    }

    /// A non-rewritten status (`500`) carrying a body must be returned
    /// unchanged, without being buffered.
    #[tokio::test]
    async fn passthrough_status_body_is_not_buffered() {
        let svc = ServiceBuilder::new()
            .layer(layer())
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from("upstream failure"))
                        .unwrap(),
                )
            }));

        let response = svc
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(read_body(response).await, "upstream failure");
    }

    fn service_returning(
        status: StatusCode,
        content_type: Option<&'static str>,
        body: &'static [u8],
    ) -> impl Service<Request<Body>, Response = Response, Error = Infallible> + Clone {
        ServiceBuilder::new()
            .layer(layer())
            .service(tower::service_fn(move |_req: Request<Body>| async move {
                let mut builder = Response::builder().status(status);
                if let Some(content_type) = content_type {
                    builder = builder.header(header::CONTENT_TYPE, content_type);
                }
                Ok::<_, Infallible>(builder.body(Body::from(body)).unwrap())
            }))
    }

    async fn call(svc: impl Service<Request<Body>, Response = Response, Error = Infallible>) -> Response {
        svc.oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    /// A 422 body that is not valid UTF-8 cannot become a JSON message: it is
    /// reported as a 500.
    #[tokio::test]
    async fn unprocessable_entity_with_invalid_utf8_body_returns_500() {
        let response = call(service_returning(StatusCode::UNPROCESSABLE_ENTITY, None, b"\xff\xfe")).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = read_body(response).await;
        assert!(body.contains("\"code\":500"), "body was: {body}");
    }

    #[tokio::test]
    async fn not_found_with_invalid_utf8_body_returns_500() {
        let response = call(service_returning(StatusCode::NOT_FOUND, None, b"\xff\xfe")).await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn audio_and_video_content_types_short_circuit() {
        for content_type in ["audio/mpeg", "video/mp4"] {
            let response = call(service_returning(StatusCode::NOT_FOUND, Some(content_type), b"")).await;

            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert_eq!(response.headers().get(header::CONTENT_TYPE).unwrap(), content_type);
            assert_eq!(read_body(response).await, "", "{content_type} must not be rewritten");
        }
    }

    /// Other error statuses are not rewritten and keep their original body.
    #[tokio::test]
    async fn other_error_statuses_pass_through_unchanged() {
        for status in [
            StatusCode::BAD_REQUEST,
            StatusCode::UNAUTHORIZED,
            StatusCode::INTERNAL_SERVER_ERROR,
        ] {
            let response = call(service_returning(status, Some("text/plain"), b"original")).await;

            assert_eq!(response.status(), status);
            assert_eq!(response.headers().get(header::CONTENT_TYPE).unwrap(), "text/plain");
            assert_eq!(read_body(response).await, "original");
        }
    }

    /// A 405 body is discarded, whatever the inner service put in it.
    #[tokio::test]
    async fn method_not_allowed_body_is_replaced() {
        let response = call(service_returning(StatusCode::METHOD_NOT_ALLOWED, None, b"custom")).await;

        assert_eq!(
            read_body(response).await,
            r#"{"code":405,"message":"Method not allowed"}"#
        );
    }

    #[tokio::test]
    async fn inner_service_error_is_propagated() {
        let svc = ServiceBuilder::new()
            .layer(layer())
            .service(tower::service_fn(|_req: Request<Body>| async {
                Err::<Response, _>("boom")
            }));

        let err = svc
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap_err();
        assert_eq!(err, "boom");
    }
}
