//! JWT Payload module

use super::Jwt;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use thiserror::Error;

/// Payload errors
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PayloadError {
    /// No token found in the request headers
    #[error("Missing token")]
    MissingToken,

    /// The token could not be parsed or validated
    #[error("Invalid token: {0}")]
    ParseTokenError(String),

    /// The request headers are malformed
    #[error("Invalid headers")]
    InvalidHeaders,
}

/// Extracts a JWT payload of type `P` from request headers of type `H`
pub trait PayloadExtractor<H, P: Debug + Serialize + for<'de> Deserialize<'de>> {
    /// Extract payload from request headers
    ///
    /// # Errors
    ///
    /// Returns a [`PayloadError`] if the token is missing, malformed or invalid.
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
