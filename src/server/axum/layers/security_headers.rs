//! Security layer (standard security headers: CSP, HSTS, etc.)

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderName, HeaderValue, header},
    response::Response,
};
use futures::future::BoxFuture;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Configuration for security headers
#[derive(Clone, Debug)]
pub struct SecurityHeadersConfig {
    pub content_security_policy: HeaderValue,
    pub strict_transport_security: HeaderValue,
    pub x_content_type_options: HeaderValue,
    pub x_frame_options: HeaderValue,
    pub x_xss_protection: HeaderValue,
    pub referrer_policy: HeaderValue,
    pub permissions_policy: HeaderValue,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        SecurityHeadersConfig {
            content_security_policy: HeaderValue::from_static("default-src 'self';"),
            strict_transport_security: HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
            x_content_type_options: HeaderValue::from_static("nosniff"),
            x_frame_options: HeaderValue::from_static("DENY"),
            x_xss_protection: HeaderValue::from_static("1; mode=block"),
            referrer_policy: HeaderValue::from_static("no-referrer"),
            permissions_policy: HeaderValue::from_static("geolocation=(self), microphone=(), camera=()"),
        }
    }
}

#[derive(Clone)]
pub struct SecurityHeadersLayer {
    pub config: SecurityHeadersConfig,
}

impl SecurityHeadersLayer {
    /// Create a new `SecurityLayer`
    pub fn new(config: SecurityHeadersConfig) -> Self {
        Self { config }
    }
}

impl<S> Layer<S> for SecurityHeadersLayer {
    type Service = SecurityHeadersMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityHeadersMiddleware {
            inner,
            config: self.config.clone(),
        }
    }
}

#[derive(Clone)]
pub struct SecurityHeadersMiddleware<S> {
    inner: S,
    config: SecurityHeadersConfig,
}

impl<S> Service<Request<Body>> for SecurityHeadersMiddleware<S>
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
        let config = self.config.clone();
        let future = self.inner.call(request);

        Box::pin(async move {
            let mut response: Response = future.await?;

            let headers = response.headers_mut();
            headers.insert(header::CONTENT_SECURITY_POLICY, config.content_security_policy);
            headers.insert(header::STRICT_TRANSPORT_SECURITY, config.strict_transport_security);
            headers.insert(header::X_CONTENT_TYPE_OPTIONS, config.x_content_type_options);
            headers.insert(header::X_FRAME_OPTIONS, config.x_frame_options);
            headers.insert(header::X_XSS_PROTECTION, config.x_xss_protection);
            headers.insert(header::REFERRER_POLICY, config.referrer_policy);
            headers.insert(HeaderName::from_static("permissions-policy"), config.permissions_policy);

            Ok(response)
        })
    }
}
