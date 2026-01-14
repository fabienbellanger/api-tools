//! Logger layer

use super::header_value_to_str;
use axum::body::HttpBody;
use axum::http::StatusCode;
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
    use std::time::Duration;

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
