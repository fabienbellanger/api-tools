//! Security layer (standard security headers: CSP, HSTS, etc.)

use std::task::{Context, Poll};
use axum::{
    body::Body, extract::Request, http::{header, HeaderName, HeaderValue}, response::Response
};
use futures::future::BoxFuture;
use tower::{Layer, Service};

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
    pub fn new(config: &SecurityHeadersConfig) -> Self {
        Self { config: config.clone() }
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

// pub fn security_headers_layer(config: SecurityHeadersConfig) -> ServiceBuilder<impl Layer<Router> + Clone> {
//     ServiceBuilder::new()
//         .layer(SetResponseHeaderLayer::if_not_present(header::CONTENT_SECURITY_POLICY, config.content_security_policy))
//         .layer(SetResponseHeaderLayer::if_not_present(header::STRICT_TRANSPORT_SECURITY, config.strict_transport_security))
//         .layer(SetResponseHeaderLayer::if_not_present(header::X_CONTENT_TYPE_OPTIONS, config.x_content_type_options))
//         .layer(SetResponseHeaderLayer::if_not_present(header::X_FRAME_OPTIONS, config.x_frame_options))
//         .layer(SetResponseHeaderLayer::if_not_present(header::REFERRER_POLICY, config.referrer_policy))
//         .layer(SetResponseHeaderLayer::if_not_present(
//             HeaderName::from_static("permissions-policy"),
//             config.permissions_policy,
//         ))
// }

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

    fn call(&mut self, mut request: Request<Body>) -> Self::Future {
        // Add security headers to the request
        request.headers_mut().insert(
            header::CONTENT_SECURITY_POLICY,
            self.config.content_security_policy.clone(),
        );
        request.headers_mut().insert(
            header::STRICT_TRANSPORT_SECURITY,
            self.config.strict_transport_security.clone(),
        );
        request.headers_mut().insert(
            header::X_CONTENT_TYPE_OPTIONS,
            self.config.x_content_type_options.clone(),
        );
        request.headers_mut().insert(
            header::X_FRAME_OPTIONS,
            self.config.x_frame_options.clone(),
        );
        request.headers_mut().insert(
            header::REFERRER_POLICY,
            self.config.referrer_policy.clone(),
        );
        request.headers_mut().insert(
            HeaderName::from_static("permissions-policy"),
            self.config.permissions_policy.clone(),
        );

        let future = self.inner.call(request);
        Box::pin(async move {
            let response: Response = future.await?;
            Ok(response)
        })
    }
}
