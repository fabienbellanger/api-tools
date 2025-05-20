//! Timezone value object representation

use chrono_tz::Tz;
use std::fmt::Display;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum TimezoneError {
    #[error("Invalid timezone: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timezone {
    value: Tz,
}

impl Timezone {
    /// Create a new timezone
    pub fn new(tz: Tz) -> Self {
        Self { value: tz }
    }
}

impl TryFrom<&str> for Timezone {
    type Error = TimezoneError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let tz = Tz::from_str(value).map_err(|e| TimezoneError::Invalid(e.to_string()))?;

        Ok(Self::new(tz))
    }
}

impl Display for Timezone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl Default for Timezone {
    fn default() -> Self {
        Self::new(Tz::Europe__Paris)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono_tz::Tz::Europe__Paris;

    #[test]
    fn test_try_from_str() {
        let tz = Timezone::try_from("Europe/Paris").unwrap();
        assert_eq!(tz.value, Europe__Paris);

        let tz = Timezone::try_from("Invalid");
        assert!(tz.is_err());
    }

    #[test]
    fn test_display() {
        let tz = Timezone::new(Europe__Paris);
        assert_eq!(tz.to_string(), "Europe/Paris");
    }

    #[test]
    fn test_default() {
        let tz = Timezone::default();
        assert_eq!(tz.value, Europe__Paris);
    }
}
