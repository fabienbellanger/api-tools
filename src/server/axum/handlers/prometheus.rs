//! Prometheus metrics handler for Axum

use crate::server::axum::response::ApiError;
#[cfg(feature = "prometheus")]
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};

/// Buckets for HTTP request duration in seconds
const SECONDS_DURATION_BUCKETS: &[f64; 11] = &[0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];

/// Prometheus metrics handler for Axum
pub struct PrometheusHandler {}

impl PrometheusHandler {
    /// Return a new `PrometheusHandle`
    pub fn get_handle() -> Result<PrometheusHandle, ApiError> {
        PrometheusBuilder::new()
            .set_buckets_for_metric(
                Matcher::Full("http_requests_duration_seconds".to_string()),
                SECONDS_DURATION_BUCKETS,
            )
            .map_err(|err| ApiError::InternalServerError(err.to_string()))?
            .install_recorder()
            .map_err(|err| ApiError::InternalServerError(err.to_string()))
    }
}
