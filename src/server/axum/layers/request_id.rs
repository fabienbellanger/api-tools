//! Request ID middleware

use axum::http::{HeaderName, Request};
use std::sync::LazyLock;
use tower_http::request_id::{MakeRequestId, RequestId};
use uuid::Uuid;

/// Request ID generator producing a random UUID v4
#[derive(Clone, Copy)]
pub struct MakeRequestUuid;

/// Request ID header
pub static REQUEST_ID_HEADER: LazyLock<HeaderName> = LazyLock::new(|| HeaderName::from_static("x-request-id"));

impl MakeRequestId for MakeRequestUuid {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        Uuid::new_v4().to_string().parse().ok().map(RequestId::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    #[test]
    fn test_request_id_header_name() {
        assert_eq!(REQUEST_ID_HEADER.as_str(), "x-request-id");
    }

    #[test]
    fn test_make_request_uuid_returns_valid_uuid() {
        let mut maker = MakeRequestUuid;
        let request = Request::builder().body(Body::empty()).unwrap();

        let id = maker.make_request_id(&request).expect("must produce a request id");
        let value = id.header_value().to_str().expect("ascii");

        assert!(
            Uuid::parse_str(value).is_ok(),
            "expected a parseable UUID, got {value:?}"
        );
    }

    #[test]
    fn test_make_request_uuid_is_unique_across_calls() {
        let mut maker = MakeRequestUuid;
        let request = Request::builder().body(Body::empty()).unwrap();

        let id_a = maker.make_request_id(&request).unwrap();
        let id_b = maker.make_request_id(&request).unwrap();

        assert_ne!(
            id_a.header_value().to_str().unwrap(),
            id_b.header_value().to_str().unwrap(),
        );
    }

    /// End-to-end wiring with tower-http: the generated id is set on the
    /// request and propagated to the response.
    #[tokio::test]
    async fn test_make_request_uuid_with_tower_http_layers() {
        use std::convert::Infallible;
        use tower::{ServiceBuilder, ServiceExt};
        use tower_http::request_id::{PropagateRequestIdLayer, SetRequestIdLayer};

        let svc = ServiceBuilder::new()
            .layer(SetRequestIdLayer::new(REQUEST_ID_HEADER.clone(), MakeRequestUuid))
            .layer(PropagateRequestIdLayer::new(REQUEST_ID_HEADER.clone()))
            .service(tower::service_fn(|req: Request<Body>| async move {
                // The handler sees the id too.
                assert!(req.headers().contains_key(REQUEST_ID_HEADER.clone()));
                Ok::<_, Infallible>(axum::response::Response::new(Body::empty()))
            }));

        let response = svc
            .clone()
            .oneshot(Request::builder().body(Body::empty()).unwrap())
            .await
            .unwrap();
        let value = response.headers().get(REQUEST_ID_HEADER.clone()).unwrap();
        assert!(Uuid::parse_str(value.to_str().unwrap()).is_ok());

        // An incoming id is kept as is.
        let response = svc
            .oneshot(
                Request::builder()
                    .header(REQUEST_ID_HEADER.clone(), "my-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.headers().get(REQUEST_ID_HEADER.clone()).unwrap(), "my-id");
    }
}
