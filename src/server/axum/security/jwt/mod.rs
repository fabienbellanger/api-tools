//! JWT module

pub mod access_token;
pub mod payload;

use crate::server::axum::response::ApiError;
use crate::server::axum::security::jwt::access_token::AccessToken;
use crate::value_objects::datetime::UtcDateTime;
use jsonwebtoken::errors::ErrorKind::ExpiredSignature;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use thiserror::Error;

const JWT_ACCESS_LIFETIME_IN_MINUTES: i64 = 15; // 15 minutes
const JWT_REFRESH_LIFETIME_IN_HOURS: i64 = 7 * 24; // 7 days

/// JWT errors
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum JwtError {
    /// The token could not be decoded
    #[error("Parse token error: {0}")]
    ParseError(String),

    /// The token could not be generated
    #[error("Generate token error: {0}")]
    GenerateError(String),

    /// The algorithm name is unknown or unsupported
    #[error("Invalid or unsupported algorithm: {0}")]
    InvalidAlgorithm(String),

    /// The encoding key is missing or invalid
    #[error("Encoding key error: {0}")]
    EncodingKeyError(String),

    /// The decoding key is missing or invalid, or the token failed validation
    #[error("Decoding key error: {0}")]
    DecodingKeyError(String),

    /// The token has expired
    #[error("Expired token")]
    ExpiredToken,
}

/// JWT error
impl From<JwtError> for ApiError {
    fn from(value: JwtError) -> Self {
        Self::InternalServerError(value.to_string())
    }
}

/// JWT representation
#[derive(Clone)]
pub struct Jwt {
    /// The algorithm supported for signing/verifying JWT
    algorithm: Algorithm,

    /// Access Token lifetime (in minute)
    /// The default value is 15 minutes.
    access_lifetime: i64,

    /// Refresh Token lifetime (in hour)
    /// The default value is 7 days.
    refresh_lifetime: i64,

    /// Encoding key
    encoding_key: Option<EncodingKey>,

    /// Decoding key
    decoding_key: Option<DecodingKey>,
}

impl Default for Jwt {
    fn default() -> Self {
        Self {
            algorithm: Algorithm::HS512,
            access_lifetime: JWT_ACCESS_LIFETIME_IN_MINUTES,
            refresh_lifetime: JWT_REFRESH_LIFETIME_IN_HOURS,
            encoding_key: None,
            decoding_key: None,
        }
    }
}

impl Debug for Jwt {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "JWT => algo: {:?}, access_lifetime: {}, refresh_lifetime: {}",
            self.algorithm, self.access_lifetime, self.refresh_lifetime
        )
    }
}

impl Jwt {
    /// Initialize a new `Jwt`
    ///
    /// # Errors
    ///
    /// Returns [`JwtError::InvalidAlgorithm`] for an unknown algorithm, and
    /// [`JwtError::EncodingKeyError`] / [`JwtError::DecodingKeyError`] if the keys required by the
    /// algorithm are missing or invalid.
    pub fn init(
        algorithm: &str,
        access_lifetime: i64,
        refresh_lifetime: i64,
        secret: Option<&str>,
        private_key: Option<&str>,
        public_key: Option<&str>,
    ) -> Result<Self, JwtError> {
        let mut jwt = Self {
            algorithm: Self::algorithm_from_str(algorithm)?,
            access_lifetime,
            refresh_lifetime,
            ..Default::default()
        };

        // Encoding key
        match (secret, private_key, jwt.use_secret()) {
            (Some(secret), _, true) => jwt.set_encoding_key(secret.trim())?,
            (_, Some(private_key), false) => jwt.set_encoding_key(private_key.trim())?,
            _ => return Err(JwtError::EncodingKeyError("invalid JWT encoding key".to_owned())),
        }

        // Decoding key
        match (secret, public_key, jwt.use_secret()) {
            (Some(secret), _, true) => jwt.set_decoding_key(secret.trim())?,
            (_, Some(public_key), false) => jwt.set_decoding_key(public_key.trim())?,
            _ => return Err(JwtError::DecodingKeyError("invalid JWT decoding key".to_owned())),
        }

        Ok(jwt)
    }

    /// Get access token lifetime
    #[must_use]
    pub const fn access_lifetime(&self) -> i64 {
        self.access_lifetime
    }

    /// Get refresh token lifetime
    #[must_use]
    pub const fn refresh_lifetime(&self) -> i64 {
        self.refresh_lifetime
    }

    /// Update access token lifetime (in minute)
    pub const fn set_access_lifetime(&mut self, duration: i64) {
        self.access_lifetime = duration;
    }

    /// Update refresh token lifetime (in day)
    pub const fn set_refresh_lifetime(&mut self, duration: i64) {
        self.refresh_lifetime = duration;
    }

    /// Update encoding key
    ///
    /// # Errors
    ///
    /// Returns [`JwtError::EncodingKeyError`] if the key cannot be parsed for the current algorithm.
    pub fn set_encoding_key(&mut self, secret: &str) -> Result<(), JwtError> {
        let key = match self.algorithm {
            Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => EncodingKey::from_secret(secret.as_bytes()),
            Algorithm::ES256 | Algorithm::ES384 => EncodingKey::from_ec_pem(secret.as_bytes())
                .map_err(|err| JwtError::EncodingKeyError(err.to_string()))?,
            Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => EncodingKey::from_rsa_pem(secret.as_bytes())
                .map_err(|err| JwtError::EncodingKeyError(err.to_string()))?,
            Algorithm::PS256 | Algorithm::PS384 | Algorithm::PS512 => EncodingKey::from_rsa_pem(secret.as_bytes())
                .map_err(|err| JwtError::EncodingKeyError(err.to_string()))?,
            Algorithm::EdDSA => EncodingKey::from_ed_pem(secret.as_bytes())
                .map_err(|err| JwtError::EncodingKeyError(err.to_string()))?,
        };

        self.encoding_key = Some(key);

        Ok(())
    }

    /// Update decoding key
    ///
    /// # Errors
    ///
    /// Returns [`JwtError::DecodingKeyError`] if the key cannot be parsed for the current algorithm.
    pub fn set_decoding_key(&mut self, secret: &str) -> Result<(), JwtError> {
        let key = match self.algorithm {
            Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => DecodingKey::from_secret(secret.as_bytes()),
            Algorithm::ES256 | Algorithm::ES384 => DecodingKey::from_ec_pem(secret.as_bytes())
                .map_err(|err| JwtError::DecodingKeyError(err.to_string()))?,
            Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => DecodingKey::from_rsa_pem(secret.as_bytes())
                .map_err(|err| JwtError::DecodingKeyError(err.to_string()))?,
            Algorithm::PS256 | Algorithm::PS384 | Algorithm::PS512 => DecodingKey::from_rsa_pem(secret.as_bytes())
                .map_err(|err| JwtError::DecodingKeyError(err.to_string()))?,
            Algorithm::EdDSA => DecodingKey::from_ed_pem(secret.as_bytes())
                .map_err(|err| JwtError::DecodingKeyError(err.to_string()))?,
        };

        self.decoding_key = Some(key);

        Ok(())
    }

    /// Generate JWT
    ///
    /// # Errors
    ///
    /// Returns [`JwtError::EncodingKeyError`] if no encoding key is set or encoding fails.
    pub fn generate<P: Debug + Serialize>(&self, payload: P, expired_at: UtcDateTime) -> Result<AccessToken, JwtError> {
        let header = jsonwebtoken::Header::new(self.algorithm);

        match self.encoding_key.clone() {
            Some(encoding_key) => {
                let token = encode(&header, &payload, &encoding_key)
                    .map_err(|err| JwtError::EncodingKeyError(err.to_string()))?;

                Ok(AccessToken { token, expired_at })
            }
            _ => Err(JwtError::EncodingKeyError("empty key".to_owned())),
        }
    }

    /// Parse JWT
    ///
    /// # Errors
    ///
    /// Returns [`JwtError::ExpiredToken`] if the token has expired, and
    /// [`JwtError::DecodingKeyError`] if no decoding key is set or the token is invalid.
    pub fn parse<P: Clone + Debug + for<'de> Deserialize<'de>>(&self, token: &AccessToken) -> Result<P, JwtError> {
        let validation = Validation::new(self.algorithm);

        match self.decoding_key.clone() {
            Some(decoding_key) => {
                let token = decode::<P>(&token.token, &decoding_key, &validation).map_err(|err| match err.kind() {
                    ExpiredSignature => JwtError::ExpiredToken,
                    _ => JwtError::DecodingKeyError(err.to_string()),
                })?;

                Ok(token.claims)
            }
            _ => Err(JwtError::DecodingKeyError("empty key".to_owned())),
        }
    }

    /// Return true if a secret key is used instead of a pair of keys
    fn use_secret(&self) -> bool {
        self.algorithm == Algorithm::HS256 || self.algorithm == Algorithm::HS384 || self.algorithm == Algorithm::HS512
    }

    /// Convert `&str` to `Algorithm`
    fn algorithm_from_str(algo: &str) -> Result<Algorithm, JwtError> {
        Ok(match algo {
            "HS256" => Algorithm::HS256,
            "HS384" => Algorithm::HS384,
            "HS512" => Algorithm::HS512,
            "ES256" => Algorithm::ES256,
            "ES384" => Algorithm::ES384,
            _ => {
                return Err(JwtError::InvalidAlgorithm(algo.to_string()));
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_use_secret() {
        let jwt = Jwt::default();
        assert!(jwt.use_secret());

        let mut jwt = Jwt {
            algorithm: Algorithm::ES256,
            ..Default::default()
        };
        assert!(!jwt.use_secret());

        jwt.algorithm = Algorithm::HS256;
        assert!(jwt.use_secret());
    }

    #[test]
    fn test_jwt_algorithm_from_str() {
        assert_eq!(Jwt::algorithm_from_str("HS256").unwrap(), Algorithm::HS256);
        assert_eq!(Jwt::algorithm_from_str("HS384").unwrap(), Algorithm::HS384);
        assert_eq!(Jwt::algorithm_from_str("HS512").unwrap(), Algorithm::HS512);
        assert_eq!(Jwt::algorithm_from_str("ES256").unwrap(), Algorithm::ES256);
        assert_eq!(Jwt::algorithm_from_str("ES384").unwrap(), Algorithm::ES384);

        let invalid_algo = Jwt::algorithm_from_str("ES512");
        assert!(invalid_algo.is_err());
        if let Err(e) = invalid_algo {
            assert_eq!(e, JwtError::InvalidAlgorithm("ES512".to_string()));
        }
    }

    #[test]
    fn test_jwt_default() {
        let jwt = Jwt::default();
        assert_eq!(jwt.algorithm, Algorithm::HS512);
        assert_eq!(jwt.access_lifetime, JWT_ACCESS_LIFETIME_IN_MINUTES);
        assert_eq!(jwt.refresh_lifetime, JWT_REFRESH_LIFETIME_IN_HOURS);
        assert!(jwt.encoding_key.is_none());
        assert!(jwt.decoding_key.is_none());
    }

    #[test]
    fn test_jwt_debug() {
        let jwt = Jwt::default();
        let debug_str = format!("{jwt:?}");

        assert_eq!(
            debug_str,
            format!("JWT => algo: HS512, access_lifetime: 15, refresh_lifetime: {}", 7 * 24)
        );
    }

    #[test]
    fn test_jwt_use_secret_all_algorithms() {
        for algo in [Algorithm::HS256, Algorithm::HS384, Algorithm::HS512] {
            let jwt = Jwt {
                algorithm: algo,
                ..Default::default()
            };
            assert!(jwt.use_secret(), "{algo:?} must use secret");
        }
        for algo in [
            Algorithm::ES256,
            Algorithm::ES384,
            Algorithm::RS256,
            Algorithm::RS384,
            Algorithm::RS512,
            Algorithm::PS256,
            Algorithm::PS384,
            Algorithm::PS512,
            Algorithm::EdDSA,
        ] {
            let jwt = Jwt {
                algorithm: algo,
                ..Default::default()
            };
            assert!(!jwt.use_secret(), "{algo:?} must use a key pair");
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestClaims {
        sub: String,
        exp: i64,
    }

    fn future_exp(seconds: i64) -> i64 {
        chrono::Utc::now().timestamp() + seconds
    }

    #[test]
    fn test_jwt_init_and_round_trip_hs512() {
        let jwt = Jwt::init("HS512", 15, 7 * 24, Some("super-secret"), None, None).expect("init");
        let claims = TestClaims {
            sub: "user-42".to_string(),
            exp: future_exp(60),
        };
        let token = jwt.generate(claims.clone(), UtcDateTime::now()).expect("generate");
        assert_ne!(token.token, "");

        let parsed: TestClaims = jwt.parse(&token).expect("parse");
        assert_eq!(parsed, claims);
    }

    #[test]
    fn test_jwt_parse_expired_token() {
        let jwt = Jwt::init("HS256", 15, 7 * 24, Some("secret"), None, None).expect("init");
        let claims = TestClaims {
            sub: "user".to_string(),
            // The default `Validation` leeway is 60 s, so push well past it.
            exp: future_exp(-300),
        };
        let token = jwt.generate(claims, UtcDateTime::now()).expect("generate");

        let err = jwt.parse::<TestClaims>(&token).unwrap_err();
        assert_eq!(err, JwtError::ExpiredToken);
    }

    #[test]
    fn test_jwt_init_invalid_algorithm() {
        let err = Jwt::init("FOO", 15, 7 * 24, Some("secret"), None, None).unwrap_err();
        assert_eq!(err, JwtError::InvalidAlgorithm("FOO".to_string()));
    }

    #[test]
    fn test_jwt_init_hs256_missing_secret() {
        let err = Jwt::init("HS256", 15, 7 * 24, None, None, None).unwrap_err();
        assert!(matches!(err, JwtError::EncodingKeyError(_)));
    }

    #[test]
    fn test_jwt_init_es256_with_secret_only_fails() {
        // ES256 requires a key pair; passing only a secret must fail at the
        // encoding-key stage.
        let err = Jwt::init("ES256", 15, 7 * 24, Some("secret"), None, None).unwrap_err();
        assert!(matches!(err, JwtError::EncodingKeyError(_)));
    }

    #[test]
    fn test_jwt_generate_without_encoding_key() {
        // Bypass `init` to construct a Jwt with no encoding key and check
        // `generate` reports it cleanly rather than panicking.
        let jwt = Jwt::default();
        let claims = TestClaims {
            sub: "user".to_string(),
            exp: future_exp(60),
        };
        let err = jwt.generate(claims, UtcDateTime::now()).unwrap_err();
        assert!(matches!(err, JwtError::EncodingKeyError(_)));
    }

    #[test]
    fn jwt_error_into_api_error_maps_to_internal_server_error() {
        let api_err: ApiError = JwtError::ExpiredToken.into();
        assert!(matches!(api_err, ApiError::InternalServerError(_)));
        // The Display payload of the JwtError is preserved.
        let ApiError::InternalServerError(msg) = api_err else {
            unreachable!()
        };
        assert_eq!(msg, "Expired token");
    }

    #[test]
    fn lifetime_getters_and_setters_round_trip() {
        let mut jwt = Jwt::default();
        assert_eq!(jwt.access_lifetime(), JWT_ACCESS_LIFETIME_IN_MINUTES);
        assert_eq!(jwt.refresh_lifetime(), JWT_REFRESH_LIFETIME_IN_HOURS);

        jwt.set_access_lifetime(30);
        jwt.set_refresh_lifetime(48);
        assert_eq!(jwt.access_lifetime(), 30);
        assert_eq!(jwt.refresh_lifetime(), 48);
    }

    /// Walks every non-HMAC algorithm to exercise each PEM-loading match arm
    /// in `set_encoding_key` / `set_decoding_key`. Each call must surface a
    /// typed error rather than panicking on garbage input.
    #[test]
    fn set_encoding_and_decoding_keys_reject_invalid_pem_for_each_algorithm() {
        for algo in [
            Algorithm::ES256,
            Algorithm::ES384,
            Algorithm::RS256,
            Algorithm::RS384,
            Algorithm::RS512,
            Algorithm::PS256,
            Algorithm::PS384,
            Algorithm::PS512,
            Algorithm::EdDSA,
        ] {
            let mut jwt = Jwt {
                algorithm: algo,
                ..Default::default()
            };
            assert!(
                matches!(
                    jwt.set_encoding_key("not-a-real-pem").unwrap_err(),
                    JwtError::EncodingKeyError(_),
                ),
                "set_encoding_key({algo:?}) should report EncodingKeyError",
            );
            assert!(
                matches!(
                    jwt.set_decoding_key("not-a-real-pem").unwrap_err(),
                    JwtError::DecodingKeyError(_),
                ),
                "set_decoding_key({algo:?}) should report DecodingKeyError",
            );
        }
    }

    #[test]
    fn init_es256_with_invalid_private_key_fails_at_encoding_stage() {
        // Drives the `(_, Some(private_key), false)` arm of the encoding-key
        // match; the inner `set_encoding_key` then surfaces the PEM error.
        let err = Jwt::init("ES256", 15, 7 * 24, None, Some("not-a-pem"), None).unwrap_err();
        assert!(matches!(err, JwtError::EncodingKeyError(_)));
    }

    #[test]
    fn parse_without_decoding_key_returns_decoding_error() {
        // `Jwt::default()` has no decoding key; parse must fail cleanly.
        let jwt = Jwt::default();
        let dummy = AccessToken {
            token: "irrelevant".to_string(),
            expired_at: UtcDateTime::now(),
        };
        let err = jwt.parse::<TestClaims>(&dummy).unwrap_err();
        assert!(matches!(err, JwtError::DecodingKeyError(_)));
    }

    #[test]
    fn test_jwt_parse_with_wrong_secret_fails() {
        let issuer = Jwt::init("HS256", 15, 7 * 24, Some("secret-A"), None, None).expect("init");
        let verifier = Jwt::init("HS256", 15, 7 * 24, Some("secret-B"), None, None).expect("init");
        let claims = TestClaims {
            sub: "user".to_string(),
            exp: future_exp(60),
        };
        let token = issuer.generate(claims, UtcDateTime::now()).expect("generate");

        let err = verifier.parse::<TestClaims>(&token).unwrap_err();
        assert!(matches!(err, JwtError::DecodingKeyError(_)));
    }

    // Throwaway key pairs generated for these tests only (openssl genpkey).
    const ES256_PRIVATE_KEY: &str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgD1kuNNc3Y10m9eSq
u6atDpKwxcZqm7/iRU3v/qgy04WhRANCAATspDj53OxcTLwvL811a+wGOwflYv+P
yCYkd+SedmeQGo7VsUjziFzBR9lBbhHNiBFD0Fao4Yg+4I0+0YsvvLSi
-----END PRIVATE KEY-----";

    const ES256_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE7KQ4+dzsXEy8Ly/NdWvsBjsH5WL/
j8gmJHfknnZnkBqO1bFI84hcwUfZQW4RzYgRQ9BWqOGIPuCNPtGLL7y0og==
-----END PUBLIC KEY-----";

    const ES384_PRIVATE_KEY: &str = "-----BEGIN PRIVATE KEY-----
MIG2AgEAMBAGByqGSM49AgEGBSuBBAAiBIGeMIGbAgEBBDAGNhTZ9AuSwCQr/92X
veoSSJLBt373/yVEqjbnlz/ZU3E2q6tre0wkbDnYYGOAmUOhZANiAASlHNP53R0U
eOxhrQMPIL7ipSVmE7Oynk1/MBT2ozXqMnecqkBP5EfgzTah1r1KI9qxb7t5o54E
Zaeo2LLT6g6H2qd3EmzmxmkWYcsY47Ff1+XCJdZVsHIXRfdFtyAQJQY=
-----END PRIVATE KEY-----";

    const ES384_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----
MHYwEAYHKoZIzj0CAQYFK4EEACIDYgAEpRzT+d0dFHjsYa0DDyC+4qUlZhOzsp5N
fzAU9qM16jJ3nKpAT+RH4M02oda9SiPasW+7eaOeBGWnqNiy0+oOh9qndxJs5sZp
FmHLGOOxX9flwiXWVbByF0X3RbcgECUG
-----END PUBLIC KEY-----";

    fn claims() -> TestClaims {
        TestClaims {
            sub: "user-42".to_string(),
            exp: future_exp(60),
        }
    }

    #[test]
    fn test_jwt_init_and_round_trip_es256() {
        let jwt = Jwt::init(
            "ES256",
            15,
            7 * 24,
            None,
            Some(ES256_PRIVATE_KEY),
            Some(ES256_PUBLIC_KEY),
        )
        .expect("init");
        let token = jwt.generate(claims(), UtcDateTime::now()).expect("generate");

        let parsed: TestClaims = jwt.parse(&token).expect("parse");
        assert_eq!(parsed.sub, "user-42");
    }

    #[test]
    fn test_jwt_init_and_round_trip_es384() {
        let jwt = Jwt::init(
            "ES384",
            15,
            7 * 24,
            None,
            Some(ES384_PRIVATE_KEY),
            Some(ES384_PUBLIC_KEY),
        )
        .expect("init");
        let token = jwt.generate(claims(), UtcDateTime::now()).expect("generate");

        let parsed: TestClaims = jwt.parse(&token).expect("parse");
        assert_eq!(parsed.sub, "user-42");
    }

    /// The keys are trimmed by `init`, so PEMs read from a file with a
    /// trailing newline (or indented in a config) must still load.
    #[test]
    fn test_jwt_init_es256_trims_surrounding_whitespace_from_keys() {
        let private_key = format!("\n  {ES256_PRIVATE_KEY}\n\n");
        let public_key = format!("  {ES256_PUBLIC_KEY}\n");
        let jwt = Jwt::init("ES256", 15, 7 * 24, None, Some(&private_key), Some(&public_key)).expect("init");

        let token = jwt.generate(claims(), UtcDateTime::now()).expect("generate");
        assert!(jwt.parse::<TestClaims>(&token).is_ok());
    }

    /// Drives the fallback arm of the decoding-key match: a valid private key
    /// but no public key.
    #[test]
    fn test_jwt_init_es256_without_public_key_fails_at_decoding_stage() {
        let err = Jwt::init("ES256", 15, 7 * 24, None, Some(ES256_PRIVATE_KEY), None).unwrap_err();
        assert!(matches!(err, JwtError::DecodingKeyError(_)));
    }

    #[test]
    fn test_jwt_init_es256_with_invalid_public_key_fails_at_decoding_stage() {
        let err = Jwt::init("ES256", 15, 7 * 24, None, Some(ES256_PRIVATE_KEY), Some("not-a-pem")).unwrap_err();
        assert!(matches!(err, JwtError::DecodingKeyError(_)));
    }

    #[test]
    fn test_jwt_es256_token_is_rejected_by_another_public_key() {
        let issuer = Jwt::init(
            "ES256",
            15,
            7 * 24,
            None,
            Some(ES256_PRIVATE_KEY),
            Some(ES256_PUBLIC_KEY),
        )
        .expect("init");
        let mut verifier = issuer.clone();
        // A P-384 public key cannot verify a P-256 signature.
        verifier.set_decoding_key(ES384_PUBLIC_KEY).ok();

        let token = issuer.generate(claims(), UtcDateTime::now()).expect("generate");
        assert!(verifier.parse::<TestClaims>(&token).is_err());
    }

    #[test]
    fn test_jwt_init_hs256_trims_surrounding_whitespace_from_secret() {
        let issuer = Jwt::init("HS256", 15, 7 * 24, Some("  secret\n"), None, None).expect("init");
        let verifier = Jwt::init("HS256", 15, 7 * 24, Some("secret"), None, None).expect("init");

        let token = issuer.generate(claims(), UtcDateTime::now()).expect("generate");
        assert!(verifier.parse::<TestClaims>(&token).is_ok());
    }

    #[test]
    fn test_jwt_init_sets_lifetimes() {
        let jwt = Jwt::init("HS384", 30, 48, Some("secret"), None, None).expect("init");
        assert_eq!(jwt.access_lifetime(), 30);
        assert_eq!(jwt.refresh_lifetime(), 48);
    }

    #[test]
    fn test_jwt_generate_keeps_expiration_date() {
        let jwt = Jwt::init("HS256", 15, 7 * 24, Some("secret"), None, None).expect("init");
        let expired_at = UtcDateTime::from_rfc3339("2030-01-01T00:00:00Z").unwrap();

        let token = jwt.generate(claims(), expired_at.clone()).expect("generate");
        assert_eq!(token.expired_at, expired_at);
    }

    #[test]
    fn test_jwt_parse_malformed_token_returns_decoding_error() {
        let jwt = Jwt::init("HS256", 15, 7 * 24, Some("secret"), None, None).expect("init");
        let token = AccessToken::new("not.a.jwt".to_string(), UtcDateTime::now());

        let err = jwt.parse::<TestClaims>(&token).unwrap_err();
        assert!(matches!(err, JwtError::DecodingKeyError(_)));
    }

    /// `Validation::new(self.algorithm)` pins the accepted algorithm: a token
    /// signed with HS256 must not be accepted by an HS512 verifier, even with
    /// the same secret.
    #[test]
    fn test_jwt_parse_rejects_token_signed_with_another_algorithm() {
        let issuer = Jwt::init("HS256", 15, 7 * 24, Some("secret"), None, None).expect("init");
        let verifier = Jwt::init("HS512", 15, 7 * 24, Some("secret"), None, None).expect("init");

        let token = issuer.generate(claims(), UtcDateTime::now()).expect("generate");
        let err = verifier.parse::<TestClaims>(&token).unwrap_err();
        assert!(matches!(err, JwtError::DecodingKeyError(_)));
    }

    /// The default `Validation` requires an `exp` claim.
    #[test]
    fn test_jwt_parse_rejects_token_without_exp_claim() {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct NoExp {
            sub: String,
        }

        let jwt = Jwt::init("HS256", 15, 7 * 24, Some("secret"), None, None).expect("init");
        let token = jwt
            .generate(
                NoExp {
                    sub: "user".to_string(),
                },
                UtcDateTime::now(),
            )
            .expect("generate");

        assert!(jwt.parse::<NoExp>(&token).is_err());
    }

    #[test]
    fn test_jwt_error_display() {
        assert_eq!(JwtError::ParseError("x".into()).to_string(), "Parse token error: x");
        assert_eq!(
            JwtError::GenerateError("x".into()).to_string(),
            "Generate token error: x"
        );
        assert_eq!(
            JwtError::InvalidAlgorithm("x".into()).to_string(),
            "Invalid or unsupported algorithm: x"
        );
        assert_eq!(
            JwtError::EncodingKeyError("x".into()).to_string(),
            "Encoding key error: x"
        );
        assert_eq!(
            JwtError::DecodingKeyError("x".into()).to_string(),
            "Decoding key error: x"
        );
        assert_eq!(JwtError::ExpiredToken.to_string(), "Expired token");
    }
}
