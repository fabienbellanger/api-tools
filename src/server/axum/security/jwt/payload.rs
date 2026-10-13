//! JWT Payload module

use super::Jwt;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use thiserror::Error;

/// Payload errors
#[derive(Debug, Clone, PartialEq, Error)]
pub enum PayloadError {
    #[error("Missing token")]
    MissingToken,

    #[error("Invalid token: {0}")]
    ParseTokenError(String),

    #[error("Invalid headers")]
    InvalidHeaders,
}

pub trait PayloadExtractor<H, P: Debug + Serialize + for<'de> Deserialize<'de>> {
    /// Extract payload from request headers
    fn try_from_headers(headers: &H, jwt: &Jwt) -> Result<P, PayloadError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_error_display() {
        assert_eq!(PayloadError::MissingToken.to_string(), "Missing token");
        assert_eq!(
            PayloadError::ParseTokenError("expired".to_string()).to_string(),
            "Invalid token: expired"
        );
        assert_eq!(PayloadError::InvalidHeaders.to_string(), "Invalid headers");
    }
}
