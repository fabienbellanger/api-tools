//! JWT Payload module

use crate::security::jwt::Jwt;
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

// /// JWT payload
// #[derive(Debug, Serialize, Deserialize)]
// pub struct Payload {
//     /// Subject
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub sub: Option<String>,

//     /// Issued at
//     pub iat: i64,

//     /// Expiration time
//     pub exp: i64,

//     /// Not before
//     pub nbf: i64,

//     /// Additional data
//     #[serde(flatten, skip_serializing_if = "Option::is_none")]
//     pub data: Option<HashMap<String, String>>,
// }
