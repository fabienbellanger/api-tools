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

impl LoggerMessage {
    /// Log the request: at `error` level for server errors (except `503`, the expected answer of
    /// the time limiter), at `info` level otherwise. Successful `/metrics` scrapes are not logged.
    fn log(&self) {
        macro_rules! log_request {
            ($level:ident) => {
                $level!(
                    status_code = %self.status_code,
                    method = %self.method,
                    path = %self.path,
                    uri = %self.uri,
                    host = %self.host,
                    request_id = %self.request_id,
                    user_agent = %self.user_agent,
                    version = %self.version,
                    latency = %format!("{:?}", self.latency),
                    body_size = %ByteSize::b(self.body_size),
                );
            };
        }

        if self.status_code >= StatusCode::INTERNAL_SERVER_ERROR.as_u16()
            && self.status_code != StatusCode::SERVICE_UNAVAILABLE.as_u16()
        {
            log_request!(error);
        } else if !self.path.starts_with("/metrics") {
            log_request!(info);
        }
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

        let mut message = LoggerMessage {
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

            message.status_code = response.status().as_u16();
            message.version = format!("{:?}", response.version());
            message.latency = now.elapsed();
            message.body_size = response.body().size_hint().lower();
            message.log();

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

    #[tokio::test]
    async fn middleware_propagates_inner_service_error() {
        let svc = ServiceBuilder::new()
            .layer(LoggerLayer)
            .service(tower::service_fn(|_req: Request<Body>| async {
                Err::<Response, _>("boom")
            }));

        let err = svc
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap_err();
        assert_eq!(err, "boom");
    }

    /// The logger must not consume or alter the response body.
    #[tokio::test]
    async fn middleware_preserves_response_headers_and_body() {
        let svc = ServiceBuilder::new()
            .layer(LoggerLayer)
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(
                    Response::builder()
                        .status(StatusCode::CREATED)
                        .header("x-custom", "value")
                        .body(Body::from("payload"))
                        .unwrap(),
                )
            }));

        let resp = svc
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        assert_eq!(resp.headers().get("x-custom").unwrap(), "value");
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        assert_eq!(&body[..], b"payload");
    }

    #[test]
    fn test_logger_message_default_fmt() {
        assert_eq!(
            LoggerMessage::default().to_string(),
            "status_code: 0, method: , path: , uri: , host: , request_id: , user_agent: , version: , latency: 0ns, body_size: 0 B",
        );
    }

    /// Tests that check the emitted log events with a capturing subscriber.
    /// Without a subscriber, `tracing` never evaluates the event fields.
    mod events {
        use super::*;
        use std::collections::HashMap;
        use std::sync::{Arc, Mutex};
        use tracing::field::{Field, Visit};
        use tracing::{Event, Level, Subscriber};
        use tracing_subscriber::layer::{Context, SubscriberExt};
        use tracing_subscriber::{Layer, registry};

        #[derive(Debug)]
        struct Captured {
            level: Level,
            fields: HashMap<String, String>,
        }

        #[derive(Clone, Default)]
        struct Capture(Arc<Mutex<Vec<Captured>>>);

        struct FieldsVisitor(HashMap<String, String>);

        impl Visit for FieldsVisitor {
            fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                self.0.insert(field.name().to_string(), format!("{value:?}"));
            }
        }

        impl<S: Subscriber> Layer<S> for Capture {
            fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
                let mut visitor = FieldsVisitor(HashMap::new());
                event.record(&mut visitor);
                self.0.lock().unwrap().push(Captured {
                    level: *event.metadata().level(),
                    fields: visitor.0,
                });
            }
        }

        /// Sends `request` through the logger and returns the captured events.
        async fn logged(status: StatusCode, request: Request<Body>) -> Vec<Captured> {
            let capture = Capture::default();
            let _guard = tracing::subscriber::set_default(registry().with(capture.clone()));

            make_svc(status).oneshot(request).await.unwrap();

            std::mem::take(&mut *capture.0.lock().unwrap())
        }

        fn get(uri: &str) -> Request<Body> {
            Request::builder().uri(uri).body(Body::empty()).unwrap()
        }

        #[tokio::test]
        async fn success_is_logged_at_info_with_request_fields() {
            let request = Request::builder()
                .method(Method::POST)
                .uri("/foo?x=1")
                .header("host", "localhost")
                .header("user-agent", "test/1.0")
                .header("x-request-id", "abc")
                .body(Body::empty())
                .unwrap();

            let events = logged(StatusCode::OK, request).await;

            assert_eq!(events.len(), 1, "{events:?}");
            let event = &events[0];
            assert_eq!(event.level, Level::INFO);
            let field = |name: &str| event.fields.get(name).map(String::as_str);
            assert_eq!(field("status_code"), Some("200"));
            assert_eq!(field("method"), Some("POST"));
            assert_eq!(field("path"), Some("/foo"));
            assert_eq!(field("uri"), Some("/foo?x=1"));
            assert_eq!(field("host"), Some("localhost"));
            assert_eq!(field("request_id"), Some("abc"));
            assert_eq!(field("user_agent"), Some("test/1.0"));
            assert_eq!(field("version"), Some("HTTP/1.1"));
            // The inner service answers "ok".
            assert_eq!(field("body_size"), Some("2 B"));
            assert!(event.fields.contains_key("latency"), "{event:?}");
        }

        #[tokio::test]
        async fn missing_headers_are_logged_as_empty_strings() {
            let events = logged(StatusCode::OK, get("/")).await;

            assert_eq!(events.len(), 1, "{events:?}");
            for name in ["host", "request_id", "user_agent"] {
                assert_eq!(events[0].fields.get(name).map(String::as_str), Some(""), "{name}");
            }
        }

        #[tokio::test]
        async fn client_error_is_logged_at_info() {
            let events = logged(StatusCode::NOT_FOUND, get("/missing")).await;

            assert_eq!(events.len(), 1, "{events:?}");
            assert_eq!(events[0].level, Level::INFO);
            assert_eq!(events[0].fields.get("status_code").map(String::as_str), Some("404"));
        }

        #[tokio::test]
        async fn server_error_is_logged_at_error() {
            let events = logged(StatusCode::INTERNAL_SERVER_ERROR, get("/boom")).await;

            assert_eq!(events.len(), 1, "{events:?}");
            assert_eq!(events[0].level, Level::ERROR);
            assert_eq!(events[0].fields.get("status_code").map(String::as_str), Some("500"));
        }

        /// 503 is the expected "outside time slot" answer, not a failure.
        #[tokio::test]
        async fn service_unavailable_is_logged_at_info() {
            let events = logged(StatusCode::SERVICE_UNAVAILABLE, get("/")).await;

            assert_eq!(events.len(), 1, "{events:?}");
            assert_eq!(events[0].level, Level::INFO);
        }

        #[tokio::test]
        async fn options_request_is_not_logged() {
            let request = Request::builder()
                .method(Method::OPTIONS)
                .uri("/foo")
                .body(Body::empty())
                .unwrap();

            assert!(logged(StatusCode::NO_CONTENT, request).await.is_empty());
        }

        #[tokio::test]
        async fn metrics_success_is_not_logged() {
            assert!(logged(StatusCode::OK, get("/metrics")).await.is_empty());
        }

        /// Server errors are always logged, even on `/metrics`.
        #[tokio::test]
        async fn metrics_server_error_is_still_logged() {
            let events = logged(StatusCode::INTERNAL_SERVER_ERROR, get("/metrics")).await;

            assert_eq!(events.len(), 1, "{events:?}");
            assert_eq!(events[0].level, Level::ERROR);
        }
    }
}
