//! Timezone value object representation

use jiff::tz::TimeZone;
use std::fmt::Display;
use thiserror::Error;

/// Timezone possible errors
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TimezoneError {
    /// The input is not a known IANA timezone
    #[error("Invalid timezone: {0}")]
    Invalid(String),
}

/// IANA timezone (e.g. `Europe/Paris`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timezone {
    value: TimeZone,
}

impl Timezone {
    /// Create a new timezone
    #[must_use]
    pub const fn new(tz: TimeZone) -> Self {
        Self { value: tz }
    }

    /// Get the timezone value
    #[must_use]
    pub const fn value(&self) -> &TimeZone {
        &self.value
    }
}

impl TryFrom<&str> for Timezone {
    type Error = TimezoneError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // Look-up in the system Time Zone Database (case-insensitive, canonicalized name)
        let tz = TimeZone::get(value).map_err(|e| TimezoneError::Invalid(e.to_string()))?;

        Ok(Self::new(tz))
    }
}

impl Display for Timezone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.value.iana_name() {
            Some(name) => f.write_str(name),
            // Time zone without IANA name (e.g. a fixed offset)
            None => match self.value.to_fixed_offset() {
                Ok(offset) => write!(f, "{offset}"),
                Err(_) => f.write_str("Etc/Unknown"),
            },
        }
    }
}

impl Default for Timezone {
    fn default() -> Self {
        // Embedded at compile time: does not depend on the system Time Zone Database
        Self::new(jiff::tz::get!("Europe/Paris"))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_try_from_str() {
        let tz = Timezone::try_from("Europe/Paris").unwrap();
        assert_eq!(tz.value.iana_name(), Some("Europe/Paris"));

        let tz = Timezone::try_from("Invalid");
        assert!(tz.is_err());
    }

    #[test]
    fn test_display() {
        let tz = Timezone::new(TimeZone::get("Europe/Paris").unwrap());
        assert_eq!(tz.to_string(), "Europe/Paris");
    }

    #[test]
    fn test_default() {
        let tz = Timezone::default();
        assert_eq!(tz.to_string(), "Europe/Paris");
    }

    #[test]
    fn try_from_str_accepts_other_zones() {
        let tz = Timezone::try_from("America/New_York").unwrap();
        assert_eq!(tz.to_string(), "America/New_York");

        let tz = Timezone::try_from("UTC").unwrap();
        assert_eq!(tz.to_string(), "UTC");
    }

    #[test]
    fn try_from_str_is_case_insensitive_and_canonicalized() {
        let tz = Timezone::try_from("europe/PARIS").unwrap();
        assert_eq!(tz.to_string(), "Europe/Paris");
    }

    #[test]
    fn display_fixed_offset_without_iana_name() {
        let tz = Timezone::new(TimeZone::fixed(jiff::tz::offset(2)));
        assert_eq!(tz.to_string(), "+02");
    }

    #[test]
    fn try_from_str_error_variant() {
        let err = Timezone::try_from("Mars/Olympus").unwrap_err();
        let TimezoneError::Invalid(message) = &err;

        assert_ne!(message, "");
        assert!(err.to_string().starts_with("Invalid timezone: "));
    }

    #[test]
    fn try_from_empty_str_fails() {
        assert!(Timezone::try_from("").is_err());
    }
}
