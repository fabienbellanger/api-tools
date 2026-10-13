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
    /// Allowed origins (comma-separated list, or `*`)
    pub allow_origin: &'a str,
    /// Allowed HTTP methods
    pub allow_methods: Vec<Method>,
    /// Allowed request headers
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

    fn service(
        allow_origin: &str,
    ) -> impl tower::Service<
        Request<Body>,
        Response = Response,
        Error = Infallible,
        Future = impl Future<Output = Result<Response, Infallible>> + Send,
    > + Clone
    + Send {
        ServiceBuilder::new()
            .layer(cors(CorsConfig {
                allow_origin,
                allow_methods: default_methods(),
                allow_headers: default_headers(),
            }))
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(ok_response())
            }))
    }

    async fn get_with_origin(allow_origin: &str, origin: &str) -> Response {
        let req = Request::builder()
            .method(Method::GET)
            .uri("/")
            .header(header::ORIGIN, origin)
            .body(Body::empty())
            .unwrap();
        service(allow_origin).oneshot(req).await.unwrap()
    }

    #[tokio::test]
    async fn cors_preflight_returns_allowed_methods_and_headers() {
        let req = Request::builder()
            .method(Method::OPTIONS)
            .uri("/")
            .header(header::ORIGIN, "https://example.com")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .body(Body::empty())
            .unwrap();
        let resp = service("*").oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(), "*");
        assert_eq!(
            resp.headers().get(header::ACCESS_CONTROL_ALLOW_METHODS).unwrap(),
            "GET,POST"
        );
        assert_eq!(
            resp.headers().get(header::ACCESS_CONTROL_ALLOW_HEADERS).unwrap(),
            "content-type"
        );
    }

    /// Wildcard origin never allows credentials (forbidden by the spec).
    #[tokio::test]
    async fn cors_wildcard_origin_does_not_allow_credentials() {
        let resp = get_with_origin("*", "https://example.com").await;

        assert!(resp.headers().get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS).is_none());
    }

    /// A `*` mixed into a whitelist is dropped: only the explicit origins are
    /// allowed.
    #[tokio::test]
    async fn cors_whitelist_ignores_wildcard_entry() {
        let resp = get_with_origin("*,https://allowed.com", "https://allowed.com").await;
        assert_eq!(
            resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(),
            "https://allowed.com"
        );

        let resp = get_with_origin("*,https://allowed.com", "https://other.com").await;
        assert!(resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
    }

    #[tokio::test]
    async fn cors_whitelist_skips_empty_entries() {
        let resp = get_with_origin(",https://allowed.com,,", "https://allowed.com").await;
        assert_eq!(
            resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(),
            "https://allowed.com"
        );
    }

    /// Origins are compared exactly: no trailing slash, port or scheme
    /// normalization.
    #[tokio::test]
    async fn cors_whitelist_matches_origin_exactly() {
        for origin in [
            "http://allowed.com",
            "https://allowed.com:8443",
            "https://sub.allowed.com",
        ] {
            let resp = get_with_origin("https://allowed.com", origin).await;
            assert!(
                resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none(),
                "{origin} must not be allowed"
            );
        }
    }

    #[tokio::test]
    async fn cors_only_commas_falls_back_to_any() {
        let resp = get_with_origin(",,", "https://anywhere.com").await;
        assert_eq!(resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(), "*");
    }
}
