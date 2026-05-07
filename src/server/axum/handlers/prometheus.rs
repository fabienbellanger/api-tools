//! Prometheus metrics handler for Axum

use crate::server::axum::response::ApiError;
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};

/// Default buckets for the `http_requests_duration_seconds` histogram, in
/// seconds. Suitable for typical HTTP API latency distributions.
pub const DEFAULT_DURATION_BUCKETS: &[f64] = &[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];

/// Prometheus metrics handler for Axum
pub struct PrometheusHandler {}

impl PrometheusHandler {
    /// Install the global Prometheus recorder using
    /// [`DEFAULT_DURATION_BUCKETS`] for the request-duration histogram.
    pub fn get_handle() -> Result<PrometheusHandle, ApiError> {
        Self::get_handle_with_buckets(DEFAULT_DURATION_BUCKETS)
    }

    /// Install the global Prometheus recorder with custom histogram buckets
    /// (in seconds) for `http_requests_duration_seconds`. Use this when the
    /// default bucket distribution does not match your service's latency
    /// profile.
    pub fn get_handle_with_buckets(buckets: &[f64]) -> Result<PrometheusHandle, ApiError> {
        PrometheusBuilder::new()
            .set_buckets_for_metric(Matcher::Full("http_requests_duration_seconds".to_string()), buckets)
            .map_err(|err| ApiError::InternalServerError(err.to_string()))?
            .install_recorder()
            .map_err(|err| ApiError::InternalServerError(err.to_string()))
    }
}
