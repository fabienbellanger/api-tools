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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use axum::response::Response;
    use std::convert::Infallible;
    use tower::{ServiceBuilder, ServiceExt};

    fn ok_response() -> Response {
        Response::builder().status(StatusCode::OK).body(Body::empty()).unwrap()
    }

    fn default_methods() -> Vec<Method> {
        vec![Method::GET, Method::POST]
    }

    fn default_headers() -> Vec<HeaderName> {
        vec![header::CONTENT_TYPE]
    }

    #[tokio::test]
    async fn cors_with_wildcard_origin_allows_any() {
        let layer = cors(CorsConfig {
            allow_origin: "*",
            allow_methods: default_methods(),
            allow_headers: default_headers(),
        });
        let svc = ServiceBuilder::new()
            .layer(layer)
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(ok_response())
            }));

        let req = Request::builder()
            .method(Method::GET)
            .uri("/")
            .header(header::ORIGIN, "https://example.com")
            .body(Body::empty())
            .unwrap();
        let resp = svc.oneshot(req).await.unwrap();

        assert_eq!(resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(), "*");
    }

    #[tokio::test]
    async fn cors_whitelist_allows_authorized_origin() {
        let layer = cors(CorsConfig {
            allow_origin: "https://allowed.com,https://other.com",
            allow_methods: default_methods(),
            allow_headers: default_headers(),
        });
        let svc = ServiceBuilder::new()
            .layer(layer)
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(ok_response())
            }));

        let req = Request::builder()
            .method(Method::GET)
            .uri("/")
            .header(header::ORIGIN, "https://allowed.com")
            .body(Body::empty())
            .unwrap();
        let resp = svc.oneshot(req).await.unwrap();

        assert_eq!(
            resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(),
            "https://allowed.com",
        );
        assert_eq!(
            resp.headers().get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS).unwrap(),
            "true",
        );
    }

    #[tokio::test]
    async fn cors_whitelist_omits_allow_origin_for_unauthorized_origin() {
        let layer = cors(CorsConfig {
            allow_origin: "https://allowed.com",
            allow_methods: default_methods(),
            allow_headers: default_headers(),
        });
        let svc = ServiceBuilder::new()
            .layer(layer)
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(ok_response())
            }));

        let req = Request::builder()
            .method(Method::GET)
            .uri("/")
            .header(header::ORIGIN, "https://forbidden.com")
            .body(Body::empty())
            .unwrap();
        let resp = svc.oneshot(req).await.unwrap();

        // tower-http does not block the request — it simply omits the
        // Access-Control-Allow-Origin header, leaving the browser to enforce.
        assert!(resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
    }

    /// An empty `allow_origin` produces no parseable origins, so we fall back
    /// to `Any` rather than rejecting everything (existing behavior).
    #[tokio::test]
    async fn cors_empty_origin_string_falls_back_to_any() {
        let layer = cors(CorsConfig {
            allow_origin: "",
            allow_methods: default_methods(),
            allow_headers: default_headers(),
        });
        let svc = ServiceBuilder::new()
            .layer(layer)
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(ok_response())
            }));

        let req = Request::builder()
            .method(Method::GET)
            .uri("/")
            .header(header::ORIGIN, "https://anywhere.com")
            .body(Body::empty())
            .unwrap();
        let resp = svc.oneshot(req).await.unwrap();

        assert_eq!(resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(), "*");
    }
}
