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

/// `TimeSlots` represents a collection of time intervals
/// where each interval is defined by a start and end time.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    #[must_use]
    pub const fn values(&self) -> &Vec<TimeSlot> {
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
    #[must_use]
    pub fn contains(&self, time: &str) -> bool {
        self.0.iter().any(|slot| *slot.start <= *time && *time <= *slot.end)
    }
}

impl Display for TimeSlots {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, slot) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{} - {}", slot.start, slot.end)?;
        }
        Ok(())
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

/// Time slot with `HH:MM` start and end times
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeSlot {
    /// Start time (`HH:MM`)
    pub start: String,
    /// End time (`HH:MM`)
    pub end: String,
}

impl TryFrom<&str> for TimeSlot {
    type Error = ApiError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let (start, end) = value
            .split_once('-')
            .ok_or_else(|| ApiError::InternalServerError("Time slots configuration error".to_string()))?;

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

/// Layer answering `503 Service Unavailable` during the configured time slots
#[derive(Clone)]
pub struct TimeLimiterLayer {
    /// Time slots during which the service is unavailable
    pub time_slots: TimeSlots,
}

impl TimeLimiterLayer {
    /// Create a new `TimeLimiterLayer`
    #[must_use]
    pub const fn new(time_slots: TimeSlots) -> Self {
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

/// Middleware produced by [`TimeLimiterLayer`]
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
        let unavailable = self
            .time_slots
            .contains(&now)
            .then(|| unavailable_response(&self.time_slots));

        let future = self.inner.call(request);
        Box::pin(async move {
            match unavailable {
                None => future.await,
                Some(response) => Ok(response),
            }
        })
    }
}

/// `503 Service Unavailable` JSON response listing the time slots
fn unavailable_response(time_slots: &TimeSlots) -> Response {
    let (mut parts, _body) = Response::new(Body::empty()).into_parts();
    let msg = body_from_parts(
        &mut parts,
        StatusCode::SERVICE_UNAVAILABLE,
        format!("Service unavailable during these times: {time_slots}").as_str(),
        None,
    );
    Response::from_parts(parts, Body::from(msg))
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
        let display = format!("{time_slots}");
        assert_eq!(display, "08:00 - 12:00, 13:00 - 17:00");
    }

    #[test]
    fn test_timeslots_empty_display() {
        let time_slots: TimeSlots = "".into();
        let display = format!("{time_slots}");
        assert_eq!(display, "");
    }

    #[test]
    fn timeslots_contains_at_exact_borders() {
        let slots: TimeSlots = "08:00-12:00".into();
        assert!(slots.contains("08:00"), "lower bound is inclusive");
        assert!(slots.contains("12:00"), "upper bound is inclusive");
        assert!(!slots.contains("07:59"));
        assert!(!slots.contains("12:01"));
    }

    #[test]
    fn timeslots_empty_never_contains() {
        let slots: TimeSlots = "".into();
        assert!(!slots.contains("00:00"));
        assert!(!slots.contains("12:00"));
        assert!(!slots.contains("23:59"));
    }
}

#[cfg(test)]
mod middleware_tests {
    use super::*;
    use axum::http::header;
    use std::convert::Infallible;
    use tower::{ServiceBuilder, ServiceExt};

    fn ok_response() -> Response {
        Response::builder()
            .status(StatusCode::OK)
            .body(Body::from("ok"))
            .unwrap()
    }

    /// Empty time slots → `contains` is always false → `is_authorized` is
    /// always true → the inner handler is invoked.
    #[tokio::test]
    async fn middleware_invokes_handler_when_no_slot_matches() {
        let layer = TimeLimiterLayer::new(TimeSlots::from(""));
        let svc = ServiceBuilder::new()
            .layer(layer)
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(ok_response())
            }));

        let resp = svc
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        assert_eq!(&body[..], b"ok");
    }

    /// A slot covering the entire day always contains the current time, so
    /// every request must be rejected with 503 + a JSON `ApiErrorResponse`.
    /// This pins the deterministic branch without depending on the wall
    /// clock.
    #[tokio::test]
    async fn middleware_returns_503_when_request_falls_inside_a_slot() {
        let layer = TimeLimiterLayer::new(TimeSlots::from("00:00-23:59"));
        let svc = ServiceBuilder::new()
            .layer(layer)
            .service(tower::service_fn(|_req: Request<Body>| async {
                // Should never be reached.
                Ok::<_, Infallible>(ok_response())
            }));

        let resp = svc
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(resp.headers().get(header::CONTENT_TYPE).unwrap(), "application/json",);

        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let body = std::str::from_utf8(&body).unwrap();
        assert!(body.contains("\"code\":503"), "body was: {body}");
        assert!(body.contains("Service unavailable"), "body was: {body}");
    }

    #[tokio::test]
    async fn middleware_503_message_lists_the_time_slots() {
        let layer = TimeLimiterLayer::new(TimeSlots::from("00:00-23:59"));
        let svc = ServiceBuilder::new()
            .layer(layer)
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(ok_response())
            }));

        let resp = svc
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        assert_eq!(
            std::str::from_utf8(&body).unwrap(),
            r#"{"code":503,"message":"Service unavailable during these times: 00:00 - 23:59"}"#
        );
    }

    #[tokio::test]
    async fn middleware_propagates_inner_service_error_when_authorized() {
        let svc = ServiceBuilder::new()
            .layer(TimeLimiterLayer::new(TimeSlots::from("")))
            .service(tower::service_fn(|_req: Request<Body>| async {
                Err::<Response, _>("boom")
            }));

        let err = svc
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap_err();
        assert_eq!(err, "boom");
    }

    #[test]
    fn timeslots_from_str_skips_invalid_slots() {
        let slots: TimeSlots = "08:00-12:00,invalid,9:00-10:00,13:00-17:00".into();
        assert_eq!(
            slots.values(),
            &vec![
                TimeSlot {
                    start: "08:00".to_string(),
                    end: "12:00".to_string(),
                },
                TimeSlot {
                    start: "13:00".to_string(),
                    end: "17:00".to_string(),
                },
            ]
        );
    }

    #[test]
    fn timeslots_contains_checks_every_slot() {
        let slots: TimeSlots = "08:00-09:00,13:00-14:00".into();
        assert!(slots.contains("08:30"));
        assert!(slots.contains("13:30"));
        assert!(!slots.contains("10:00"));
        assert!(!slots.contains("15:00"));
    }

    #[test]
    fn timeslot_try_from_rejects_wrong_lengths() {
        for value in ["8:00-12:00", "08:00-1200", "08:00-", "-12:00", "08:00:00-12:00"] {
            let err = TimeSlot::try_from(value).unwrap_err();
            assert_eq!(
                err,
                ApiError::InternalServerError("Time slots configuration error".to_string()),
                "{value:?} must be rejected"
            );
        }
    }

    #[test]
    fn timeslots_single_display() {
        let slots: TimeSlots = "22:00-23:00".into();
        assert_eq!(slots.to_string(), "22:00 - 23:00");
    }
}
