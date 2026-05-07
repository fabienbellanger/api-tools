//! Request ID middleware

use axum::http::{HeaderName, Request};
use std::sync::LazyLock;
use tower_http::request_id::{MakeRequestId, RequestId};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct MakeRequestUuid;

/// Request ID header
pub static REQUEST_ID_HEADER: LazyLock<HeaderName> = LazyLock::new(|| HeaderName::from_static("x-request-id"));

impl MakeRequestId for MakeRequestUuid {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let id = Uuid::new_v4().to_string().parse();
        match id {
            Ok(id) => Some(RequestId::new(id)),
            _ => None,
        }
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
}
