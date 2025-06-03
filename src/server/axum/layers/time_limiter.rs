//! Time limiter layer

use crate::server::axum::{layers::body_from_parts, response::ApiError};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use chrono::Local;
use futures::future::BoxFuture;
use std::fmt::Display;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// TimeSlots represents a collection of time intervals
/// where each interval is defined by a start and end time.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeSlots(Vec<TimeSlot>);

impl TimeSlots {
    /// Get time slots vector
    ///
    /// # Example
    /// ```
    /// use api_tools::server::axum::layers::time_limiter::TimeSlots;
    ///
    /// let time_slots: TimeSlots = "08:00-12:00,13:00-17:00".into();
    /// assert_eq!(time_slots.values().len(), 2);
    /// assert_eq!(time_slots.values()[0].start, "08:00");
    /// assert_eq!(time_slots.values()[0].end, "12:00");
    /// assert_eq!(time_slots.values()[1].start, "13:00");
    /// assert_eq!(time_slots.values()[1].end, "17:00");
    /// ```
    pub fn values(&self) -> &Vec<TimeSlot> {
        &self.0
    }

    /// Check if a time is in the time slots list
    ///
    /// # Example
    /// ```
    /// use api_tools::server::axum::layers::time_limiter::TimeSlots;
    ///
    /// let time_slots: TimeSlots = "08:00-12:00,13:00-17:00".into();
    /// let now = "09:00";
    /// assert_eq!(time_slots.contains(now), true);
    ///
    /// let now = "08:00";
    /// assert_eq!(time_slots.contains(now), true);
    ///
    /// let now = "17:00";
    /// assert_eq!(time_slots.contains(now), true);
    ///
    /// let now = "12:30";
    /// assert_eq!(time_slots.contains(now), false);
    ///
    /// let time_slots: TimeSlots = "".into();
    /// let now = "09:00";
    /// assert_eq!(time_slots.contains(now), false);
    /// ```
    pub fn contains(&self, time: &str) -> bool {
        self.0.iter().any(|slot| *slot.start <= *time && *time <= *slot.end)
    }
}

impl Display for TimeSlots {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut slots = String::new();
        for (i, slot) in self.0.iter().enumerate() {
            slots.push_str(&format!("{} - {}", slot.start, slot.end));

            if i < self.0.len() - 1 {
                slots.push_str(", ");
            }
        }
        write!(f, "{}", slots)
    }
}

impl From<&str> for TimeSlots {
    fn from(value: &str) -> Self {
        Self(
            value
                .split(',')
                .filter_map(|part| part.try_into().ok())
                .collect::<Vec<_>>(),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimeSlot {
    pub start: String,
    pub end: String,
}

impl TryFrom<&str> for TimeSlot {
    type Error = ApiError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let (start, end) = value.split_once('-').ok_or(ApiError::InternalServerError(
            "Time slots configuration error".to_string(),
        ))?;

        if start.len() != 5 || end.len() != 5 {
            return Err(ApiError::InternalServerError(
                "Time slots configuration error".to_string(),
            ));
        }

        Ok(Self {
            start: start.to_string(),
            end: end.to_string(),
        })
    }
}

#[derive(Clone)]
pub struct TimeLimiterLayer {
    pub time_slots: TimeSlots,
}

impl TimeLimiterLayer {
    /// Create a new `TimeLimiterLayer`
    pub fn new(time_slots: TimeSlots) -> Self {
        Self { time_slots }
    }
}

impl<S> Layer<S> for TimeLimiterLayer {
    type Service = TimeLimiterMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TimeLimiterMiddleware {
            inner,
            time_slots: self.time_slots.clone(),
        }
    }
}

#[derive(Clone)]
pub struct TimeLimiterMiddleware<S> {
    inner: S,
    time_slots: TimeSlots,
}

impl<S> Service<Request<Body>> for TimeLimiterMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    // `BoxFuture` is a type alias for `Pin<Box<dyn Future + Send + 'a>>`
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let now = Local::now().format("%H:%M").to_string();
        let is_authorized = !self.time_slots.contains(&now);
        let time_slots = self.time_slots.clone();

        let future = self.inner.call(request);
        Box::pin(async move {
            let mut response = Response::default();

            response = match is_authorized {
                true => future.await?,
                false => {
                    let (mut parts, _body) = response.into_parts();
                    let msg = body_from_parts(
                        &mut parts,
                        StatusCode::SERVICE_UNAVAILABLE,
                        format!("Service unavailable during these times: {}", time_slots).as_str(),
                        None,
                    );
                    Response::from_parts(parts, Body::from(msg))
                }
            };

            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeslots_from_str() {
        let time_slots: TimeSlots = "08:00-12:00,13:00-17:00".into();
        assert_eq!(time_slots.values().len(), 2);
        assert_eq!(time_slots.values()[0].start, "08:00");
        assert_eq!(time_slots.values()[0].end, "12:00");
        assert_eq!(time_slots.values()[1].start, "13:00");
        assert_eq!(time_slots.values()[1].end, "17:00");
    }

    #[test]
    fn test_timeslot_try_from_valid() {
        let slot: TimeSlot = "10:00-11:00".try_into().unwrap();
        assert_eq!(slot.start, "10:00");
        assert_eq!(slot.end, "11:00");
    }

    #[test]
    fn test_timeslot_try_from_invalid_format() {
        let slot = TimeSlot::try_from("1000-1100");
        assert!(slot.is_err());
        let slot = TimeSlot::try_from("10:00/11:00");
        assert!(slot.is_err());
    }

    #[test]
    fn test_timeslots_display() {
        let time_slots: TimeSlots = "08:00-12:00,13:00-17:00".into();
        let display = format!("{}", time_slots);
        assert_eq!(display, "08:00 - 12:00, 13:00 - 17:00");
    }

    #[test]
    fn test_timeslots_empty_display() {
        let time_slots: TimeSlots = "".into();
        let display = format!("{}", time_slots);
        assert_eq!(display, "");
    }
}
