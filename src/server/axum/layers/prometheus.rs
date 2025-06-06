//! Prometheus' metrics layer

use axum::body::Body;
use axum::extract::MatchedPath;
use axum::http::Request;
use axum::response::Response;
use futures::future::BoxFuture;
use metrics::{counter, histogram};
use std::task::{Context, Poll};
use std::time::Instant;
use tower::{Layer, Service};

/// Prometheus metrics layer for Axum
pub struct PrometheusLayer {
    /// Service name
    pub service_name: String,
}

impl<S> Layer<S> for PrometheusLayer {
    type Service = PrometheusMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        PrometheusMiddleware {
            inner,
            service_name: self.service_name.clone(),
        }
    }
}

#[derive(Clone)]
pub struct PrometheusMiddleware<S> {
    inner: S,
    service_name: String,
}

impl<S> Service<Request<Body>> for PrometheusMiddleware<S>
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
        let start = Instant::now();
        let path = if let Some(matched_path) = request.extensions().get::<MatchedPath>() {
            matched_path.as_str().to_owned()
        } else {
            request.uri().path().to_owned()
        };
        let method = request.method().to_string();
        let service_name = self.service_name.clone();

        let future = self.inner.call(request);
        Box::pin(async move {
            let response = future.await?;

            let latency = start.elapsed().as_secs_f64();
            let status = response.status().as_u16().to_string();
            let labels = [
                ("method", method),
                ("path", path),
                ("service", service_name),
                ("status", status),
            ];

            counter!("http_requests_total", &labels).increment(1);
            histogram!("http_requests_duration_seconds", &labels).record(latency);

            Ok(response)
        })
    }
}
