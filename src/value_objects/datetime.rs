//! Datetime represents a date and time value in the UTC timezone.

use chrono::{DateTime, Utc};
use std::fmt::{Display, Formatter};
use thiserror::Error;

/// UTC Datetime possible errors
#[derive(Debug, Clone, PartialEq, Error)]
pub enum UtcDateTimeError {
    #[error("Invalid date time: {0}")]
    InvalidDateTime(String),
}

/// Date time with UTC timezone
#[derive(Debug, Default, Clone, PartialEq, PartialOrd, Eq)]
pub struct UtcDateTime {
    value: DateTime<Utc>,
}

impl UtcDateTime {
    /// Create a new date time for now
    pub fn now() -> Self {
        Self { value: Utc::now() }
    }

    /// Create a new date time
    pub fn new(value: DateTime<Utc>) -> Self {
        Self { value }
    }

    /// Create a new date time from RFC3339 string
    ///
    /// # Example
    /// ```rust
    /// use api_tools::value_objects::datetime::UtcDateTime;
    ///
    /// let datetime = UtcDateTime::from_rfc3339("2024-08-28T12:00:00Z");
    /// assert_eq!(datetime.unwrap().to_string(), "2024-08-28T12:00:00Z".to_owned());
    ///
    /// let invalid_datetime = UtcDateTime::from_rfc3339("2024-08-T12:00:00Z");
    /// ```
    pub fn from_rfc3339(value: &str) -> Result<Self, UtcDateTimeError> {
        let dt = DateTime::parse_from_rfc3339(value)
            .map_err(|e| UtcDateTimeError::InvalidDateTime(format!("{e}: {value}")))?;

        Ok(Self {
            value: dt.with_timezone(&Utc),
        })
    }

    /// Get timestamp value
    pub fn timestamp(&self) -> i64 {
        self.value.timestamp()
    }

    /// Get date time value
    pub fn value(&self) -> DateTime<Utc> {
        self.value
    }
}

impl From<DateTime<Utc>> for UtcDateTime {
    fn from(value: DateTime<Utc>) -> Self {
        Self { value }
    }
}

impl Display for UtcDateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_utc_date_time_display() {
        let dt = DateTime::parse_from_rfc3339("2024-08-28T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let datetime = UtcDateTime::from(dt);

        assert_eq!(datetime.to_string(), "2024-08-28T12:00:00Z");
    }

    #[test]
    fn test_from_rfc3339() {
        let datetime = UtcDateTime::from_rfc3339("2024-08-28T12:00:00Z");
        assert!(datetime.is_ok());
        assert_eq!(datetime.unwrap().to_string(), "2024-08-28T12:00:00Z".to_owned());

        let invalid_datetime = UtcDateTime::from_rfc3339("2024-08-T12:00:00Z");
        assert!(invalid_datetime.is_err());
    }

    #[test]
    fn test_value() {
        let dt = DateTime::parse_from_rfc3339("2024-08-28T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let datetime = UtcDateTime::from(dt);

        assert_eq!(datetime.value(), dt);
    }

    #[test]
    fn test_timestamp() {
        let dt = DateTime::parse_from_rfc3339("2024-08-28T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let datetime = UtcDateTime::from(dt);

        assert_eq!(datetime.timestamp(), 1724846400);
    }
}
