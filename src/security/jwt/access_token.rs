//! Access token entity

use crate::value_objects::datetime::UtcDateTime;

/// Access Token Value represents the value of the access token
pub type AccessTokenValue = String;

/// Access Token
#[derive(Debug, Clone, PartialEq)]
pub struct AccessToken {
    /// Token
    pub token: AccessTokenValue,

    /// Expiration time
    pub expired_at: UtcDateTime,
}

impl AccessToken {
    /// Create a new access token
    pub fn new(token: String, expired_at: UtcDateTime) -> Self {
        Self { token, expired_at }
    }
}
