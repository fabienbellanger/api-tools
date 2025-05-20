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
