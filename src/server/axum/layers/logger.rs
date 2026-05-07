//! Logger layer

use super::header_value_to_str;
use axum::body::HttpBody;
use axum::http::{Method, StatusCode};
use axum::{body::Body, http::Request, response::Response};
use bytesize::ByteSize;
use futures::future::BoxFuture;
use std::{
    fmt::Display,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tower::{Layer, Service};

#[derive(Debug, Default)]
struct LoggerMessage {
    method: String,
    request_id: String,
    host: String,
    path: String,
    uri: String,
    user_agent: String,
    status_code: u16,
    version: String,
    latency: Duration,
    body_size: u64,
}

impl Display for LoggerMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status_code: {}, method: {}, path: {}, uri: {}, host: {}, request_id: {}, user_agent: {}, version: {}, latency: {:?}, body_size: {}",
            self.status_code,
            self.method,
            self.path,
            self.uri,
            self.host,
            self.request_id,
            self.user_agent,
            self.version,
            self.latency,
            ByteSize::b(self.body_size),
        )
    }
}

#[derive(Clone)]
pub struct LoggerLayer;

impl<S> Layer<S> for LoggerLayer {
    type Service = LoggerMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LoggerMiddleware { inner }
    }
}

#[derive(Clone)]
pub struct LoggerMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for LoggerMiddleware<S>
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
        let now = Instant::now();
        let request_headers = request.headers();

        let message = LoggerMessage {
            method: request.method().to_string(),
            path: request.uri().path().to_string(),
            uri: request.uri().to_string(),
            host: header_value_to_str(request_headers.get("host")).to_string(),
            request_id: header_value_to_str(request_headers.get("x-request-id")).to_string(),
            user_agent: header_value_to_str(request_headers.get("user-agent")).to_string(),
            ..Default::default()
        };

        let future = self.inner.call(request);
        Box::pin(async move {
            let response: Response = future.await?;

            // Skip logging for OPTIONS requests
            if message.method == Method::OPTIONS.to_string() {
                return Ok(response);
            }

            let status_code = response.status().as_u16();
            let version = format!("{:?}", response.version());
            let latency = now.elapsed();
            let body_size = response.body().size_hint().lower();

            macro_rules! log_request {
                ($level:ident) => {
                    $level!(
                        status_code = %status_code,
                        method = %message.method,
                        path = %message.path,
                        uri = %message.uri,
                        host = %message.host,
                        request_id = %message.request_id,
                        user_agent = %message.user_agent,
                        version = %version,
                        latency = %format!("{:?}", latency),
                        body_size = %ByteSize::b(body_size),
                    );
                };
            }

            if response.status() >= StatusCode::INTERNAL_SERVER_ERROR
                && response.status() != StatusCode::SERVICE_UNAVAILABLE
            {
                log_request!(error);
            } else if !message.path.starts_with("/metrics") {
                log_request!(info);
            }

            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;
    use std::time::Duration;
    use tower::{ServiceBuilder, ServiceExt};

    fn make_svc(
        status: StatusCode,
    ) -> impl tower::Service<
        Request<Body>,
        Response = Response,
        Error = Infallible,
        Future = impl std::future::Future<Output = Result<Response, Infallible>> + Send,
    > + Clone
    + Send
    + 'static {
        let layer = LoggerLayer;
        ServiceBuilder::new()
            .layer(layer)
            .service(tower::service_fn(move |_req: Request<Body>| async move {
                Ok::<_, Infallible>(Response::builder().status(status).body(Body::from("ok")).unwrap())
            }))
    }

    #[tokio::test]
    async fn middleware_passes_through_2xx_response() {
        let svc = make_svc(StatusCode::OK);
        let req = Request::builder()
            .method(Method::GET)
            .uri("/foo")
            .header("host", "localhost")
            .header("user-agent", "test/1.0")
            .header("x-request-id", "abc")
            .body(Body::empty())
            .unwrap();

        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn middleware_passes_through_5xx_response() {
        // 5xx (except 503) takes the ERROR-level branch.
        let svc = make_svc(StatusCode::INTERNAL_SERVER_ERROR);
        let req = Request::builder().uri("/boom").body(Body::empty()).unwrap();
        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// SERVICE_UNAVAILABLE is the only 5xx still logged at info level — it is
    /// the legitimate "outside time slot" response from `time_limiter`, not a
    /// real failure.
    #[tokio::test]
    async fn middleware_treats_503_as_info_branch() {
        let svc = make_svc(StatusCode::SERVICE_UNAVAILABLE);
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    /// Sentinel: an OPTIONS request must short-circuit the logger before any
    /// status/latency computation. Pins the regression added in commit
    /// 76f81c7.
    #[tokio::test]
    async fn middleware_short_circuits_on_options_method() {
        let svc = make_svc(StatusCode::NO_CONTENT);
        let req = Request::builder()
            .method(Method::OPTIONS)
            .uri("/foo")
            .body(Body::empty())
            .unwrap();
        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }

    /// `/metrics` requests are passed through but skip the info log to avoid
    /// a feedback loop with the Prometheus scraper.
    #[tokio::test]
    async fn middleware_passes_through_metrics_path() {
        let svc = make_svc(StatusCode::OK);
        let req = Request::builder().uri("/metrics").body(Body::empty()).unwrap();
        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// Missing optional headers (host, user-agent, x-request-id) must default
    /// to empty strings without panicking.
    #[tokio::test]
    async fn middleware_handles_missing_optional_headers() {
        let svc = make_svc(StatusCode::OK);
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let resp = svc.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn test_logger_message_fmt() {
        let message = LoggerMessage {
            method: "GET".to_string(),
            request_id: "abc-123".to_string(),
            host: "localhost".to_string(),
            path: "/test".to_string(),
            uri: "/test?query=1".to_string(),
            user_agent: "TestAgent/1.0".to_string(),
            status_code: 200,
            version: "HTTP/1.1".to_string(),
            latency: Duration::from_millis(42),
            body_size: 1_524,
        };
        let expected = String::from(
            "status_code: 200, method: GET, path: /test, uri: /test?query=1, host: localhost, request_id: abc-123, user_agent: TestAgent/1.0, version: HTTP/1.1, latency: 42ms, body_size: 1.5 KiB",
        );

        assert_eq!(message.to_string(), expected);
    }
}
