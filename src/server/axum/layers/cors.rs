//! CORS layer for Axum

use axum::http::{HeaderName, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

/// CORS configuration
///
/// # Example
///
/// ```rust
/// use axum::http::{header, HeaderName, HeaderValue, Method};
/// use api_tools::server::axum::layers::cors::CorsConfig;
///
/// let cors_config = CorsConfig {
///     allow_origin: "*",
///     allow_methods: vec![Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE],
///     allow_headers: vec![header::AUTHORIZATION, header::ACCEPT, header::CONTENT_TYPE, header::ORIGIN],
/// };
/// ```
pub struct CorsConfig<'a> {
    pub allow_origin: &'a str,
    pub allow_methods: Vec<Method>,
    pub allow_headers: Vec<HeaderName>,
}

/// CORS layer
///
/// This function creates a CORS layer for Axum with the specified configuration.
///
/// # Example
///
/// ```rust
/// use axum::http::{header, HeaderName, HeaderValue, Method};
/// use api_tools::server::axum::layers::cors::{cors, CorsConfig};
///
/// let cors_config = CorsConfig {
///     allow_origin: "*",
///     allow_methods: vec![Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE],
///     allow_headers: vec![header::AUTHORIZATION, header::ACCEPT, header::CONTENT_TYPE, header::ORIGIN],
/// };
///
/// let layer = cors(cors_config);
/// ```
pub fn cors(config: CorsConfig) -> CorsLayer {
    let allow_origin = config.allow_origin;

    let layer = CorsLayer::new()
        .allow_methods(config.allow_methods)
        .allow_headers(config.allow_headers);

    if allow_origin == "*" {
        layer.allow_origin(Any)
    } else {
        let origins = allow_origin
            .split(',')
            .filter(|url| *url != "*" && !url.is_empty())
            .filter_map(|url| url.parse().ok())
            .collect::<Vec<HeaderValue>>();

        if origins.is_empty() {
            layer.allow_origin(Any)
        } else {
            layer
                .allow_origin(AllowOrigin::predicate(move |origin: &HeaderValue, _| {
                    origins.contains(origin)
                }))
                .allow_credentials(true)
        }
    }
}
