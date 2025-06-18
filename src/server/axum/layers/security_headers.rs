//! Security layer (standard security headers: CSP, HSTS, etc.)

use axum::{
    Router,
    http::{HeaderName, HeaderValue, header},
};
use tower::{Layer, ServiceBuilder};
use tower_http::set_header::SetResponseHeaderLayer;

pub struct SecurityLayerConfig {
    pub content_security_policy: HeaderValue,
    pub strict_transport_security: HeaderValue,
    pub x_content_type_options: HeaderValue,
    pub x_frame_options: HeaderValue,
    pub x_xss_protection: HeaderValue,
    pub referrer_policy: HeaderValue,
    pub permissions_policy: HeaderValue,
}

impl Default for SecurityLayerConfig {
    fn default() -> Self {
        SecurityLayerConfig {
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

pub fn security_headers_layer(config: SecurityLayerConfig) -> ServiceBuilder<impl Layer<Router>> {
    ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::if_not_present(header::CONTENT_SECURITY_POLICY, config.content_security_policy))
        .layer(SetResponseHeaderLayer::if_not_present(header::STRICT_TRANSPORT_SECURITY, config.strict_transport_security))
        .layer(SetResponseHeaderLayer::if_not_present(header::X_CONTENT_TYPE_OPTIONS, config.x_content_type_options))
        .layer(SetResponseHeaderLayer::if_not_present(header::X_FRAME_OPTIONS, config.x_frame_options))
        .layer(SetResponseHeaderLayer::if_not_present(header::REFERRER_POLICY, config.referrer_policy))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("permissions-policy"),
            config.permissions_policy,
        ))
}
