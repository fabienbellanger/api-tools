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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_duration_buckets_are_monotonically_increasing() {
        assert!(!DEFAULT_DURATION_BUCKETS.is_empty());
        for pair in DEFAULT_DURATION_BUCKETS.windows(2) {
            assert!(pair[0] < pair[1], "buckets must be strictly increasing: {pair:?}");
        }
    }

    /// Single combined test for the whole handler lifecycle. `install_recorder`
    /// mutates a process-wide global, so we cannot run multiple tests against
    /// it in parallel — tarpaulin / `cargo test` would race. Keeping the
    /// scenario in one `#[test]` keeps the order deterministic without
    /// pulling in `serial_test`.
    #[test]
    fn handler_install_recorder_lifecycle() {
        // First successful install with custom buckets.
        let handle =
            PrometheusHandler::get_handle_with_buckets(&[0.001, 0.01, 0.1]).expect("first install should succeed");
        // Sanity: rendering an empty registry yields a (possibly empty) string.
        let _rendered = handle.render();

        // Subsequent installs must fail because the global recorder is already
        // set. Both flavours of the API exercise the same install path.
        let err = PrometheusHandler::get_handle().expect_err("second install must fail");
        assert!(matches!(err, ApiError::InternalServerError(_)));

        let err = PrometheusHandler::get_handle_with_buckets(&[0.5, 1.0]).expect_err("third install must fail");
        assert!(matches!(err, ApiError::InternalServerError(_)));
    }
}
