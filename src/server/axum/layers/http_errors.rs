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

#[derive(Clone)]
pub struct HttpErrorsLayer {
    pub config: HttpErrorsConfig,
}

impl HttpErrorsLayer {
    /// Create a new `HttpErrorsLayer`
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
            match axum::body::to_bytes(body, config.body_max_size).await {
                Ok(body) => match String::from_utf8(body.to_vec()) {
                    Ok(body) => match parts.status {
                        StatusCode::METHOD_NOT_ALLOWED => Ok(ApiError::MethodNotAllowed.into_response()),
                        StatusCode::UNPROCESSABLE_ENTITY => Ok(ApiError::UnprocessableEntity(body).into_response()),
                        StatusCode::NOT_FOUND if body.is_empty() => {
                            Ok(ApiError::NotFound("Resource Not Found".to_owned()).into_response())
                        }
                        _ => Ok(Response::from_parts(parts, Body::from(body))),
                    },
                    Err(err) => Ok(ApiError::InternalServerError(err.to_string()).into_response()),
                },
                Err(_) => Ok(ApiError::PayloadTooLarge.into_response()),
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

    #[tokio::test]
    async fn body_exceeding_max_size_returns_payload_too_large() {
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

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let body = read_body(response).await;
        assert!(body.contains("\"code\":413"), "body was: {body}");
    }
}
