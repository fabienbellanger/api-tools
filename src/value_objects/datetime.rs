//! Datetime represents a date and time value in the UTC timezone.

use jiff::{SignedDuration, Timestamp};
use serde::Deserialize;
use std::fmt::{Display, Formatter};
use std::ops::Add;
use thiserror::Error;

/// UTC Datetime possible errors
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UtcDateTimeError {
    /// The input is not a valid RFC3339 date time
    #[error("Invalid date time: {0}")]
    InvalidDateTime(String),
}

/// Date time with UTC timezone
#[derive(Debug, Default, Clone, PartialEq, PartialOrd, Eq, Deserialize)]
pub struct UtcDateTime {
    value: Timestamp,
}

impl UtcDateTime {
    /// Create a new date time for now
    #[must_use]
    pub fn now() -> Self {
        Self {
            value: Timestamp::now(),
        }
    }

    /// Create a new date time
    #[must_use]
    pub const fn new(value: Timestamp) -> Self {
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
    ///
    /// # Errors
    ///
    /// Returns [`UtcDateTimeError::InvalidDateTime`] if `value` is not a valid RFC3339 date time.
    pub fn from_rfc3339(value: &str) -> Result<Self, UtcDateTimeError> {
        let value = value
            .parse::<Timestamp>()
            .map_err(|e| UtcDateTimeError::InvalidDateTime(format!("{e}: {value}")))?;

        Ok(Self { value })
    }

    /// Get timestamp value
    #[must_use]
    pub fn timestamp(&self) -> i64 {
        self.value.as_second()
    }

    /// Get date time value
    #[must_use]
    pub const fn value(&self) -> Timestamp {
        self.value
    }

    /// Add a signed duration (negative to go back in time)
    ///
    /// # Panics
    ///
    /// Panics if the result is out of the range supported by [`Timestamp`].
    #[must_use]
    pub fn add(&self, rhs: SignedDuration) -> Self {
        Self {
            value: self.value.add(rhs),
        }
    }
}

impl From<Timestamp> for UtcDateTime {
    fn from(value: Timestamp) -> Self {
        Self { value }
    }
}

impl Display for UtcDateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // RFC 3339 with `Z`; a precision of 0 truncates the sub-seconds
        write!(f, "{:.0}", self.value)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_utc_date_time_display() {
        let dt: Timestamp = "2024-08-28T12:00:00Z".parse().unwrap();
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
        let dt: Timestamp = "2024-08-28T12:00:00Z".parse().unwrap();
        let datetime = UtcDateTime::from(dt);

        assert_eq!(datetime.value(), dt);
    }

    #[test]
    fn test_timestamp() {
        let dt: Timestamp = "2024-08-28T12:00:00Z".parse().unwrap();
        let datetime = UtcDateTime::from(dt);

        assert_eq!(datetime.timestamp(), 1_724_846_400);
    }

    #[test]
    fn new_wraps_existing_timestamp() {
        let dt: Timestamp = "2024-08-28T12:00:00Z".parse().unwrap();
        let datetime = UtcDateTime::new(dt);
        assert_eq!(datetime.value(), dt);
    }

    #[test]
    fn now_returns_a_value_between_before_and_after_calls() {
        let before = Timestamp::now();
        let dt = UtcDateTime::now();
        let after = Timestamp::now();
        assert!(dt.value() >= before);
        assert!(dt.value() <= after);
    }

    #[test]
    fn test_add() {
        let dt: Timestamp = "2024-08-28T12:00:00Z".parse().unwrap();
        let datetime = UtcDateTime::from(dt);
        let new_datetime = datetime.add(SignedDuration::from_hours(1));
        assert_eq!(new_datetime.to_string(), "2024-08-28T13:00:00Z".to_owned());
    }

    #[test]
    fn from_rfc3339_converts_offset_to_utc() {
        let datetime = UtcDateTime::from_rfc3339("2024-08-28T14:00:00+02:00").unwrap();
        assert_eq!(datetime.to_string(), "2024-08-28T12:00:00Z");
    }

    #[test]
    fn from_rfc3339_error_contains_the_invalid_input() {
        let err = UtcDateTime::from_rfc3339("not-a-date").unwrap_err();
        let UtcDateTimeError::InvalidDateTime(message) = &err;

        assert!(message.ends_with(": not-a-date"), "message was: {message}");
        assert!(err.to_string().starts_with("Invalid date time: "));
    }

    #[test]
    fn display_truncates_sub_seconds() {
        let datetime = UtcDateTime::from_rfc3339("2024-08-28T12:00:00.987654Z").unwrap();
        assert_eq!(datetime.to_string(), "2024-08-28T12:00:00Z");
    }

    #[test]
    fn add_negative_delta_goes_back_in_time() {
        let datetime = UtcDateTime::from_rfc3339("2024-08-28T00:30:00Z").unwrap();
        let earlier = datetime.add(SignedDuration::from_hours(-1));

        assert_eq!(earlier.to_string(), "2024-08-27T23:30:00Z");
        assert!(earlier < datetime);
    }

    #[test]
    fn deserialize_from_rfc3339_string() {
        let datetime: UtcDateTime = serde_json::from_str(r#"{"value": "2024-08-28T14:00:00+02:00"}"#).unwrap();
        assert_eq!(datetime.to_string(), "2024-08-28T12:00:00Z");

        assert!(serde_json::from_str::<UtcDateTime>(r#"{"value": "not-a-date"}"#).is_err());
    }

    #[test]
    fn default_is_unix_epoch() {
        assert_eq!(UtcDateTime::default().timestamp(), 0);
    }
}
