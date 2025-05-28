//! Override some HTTP errors

use crate::server::axum::response::ApiError;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use futures::future::BoxFuture;
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[derive(Clone, Debug)]
pub struct HttpErrorsConfig {
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

            // Vérifie le content-type
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
                        _ => Ok(Response::from_parts(parts, Body::from(body))),
                    },
                    Err(err) => Ok(ApiError::InternalServerError(err.to_string()).into_response()),
                },
                Err(_) => Ok(ApiError::PayloadTooLarge.into_response()),
            }
        })
    }
}
