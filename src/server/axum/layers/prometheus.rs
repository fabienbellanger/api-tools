//! Prometheus' metrics layer
//!
//! This module provides:
//!
//! 1. [`PrometheusLayer`] — a tower [`Layer`] that records per-request HTTP
//!    metrics (`http_requests_total`, `http_requests_duration_seconds`).
//!    The middleware overhead is in the microsecond range.
//!
//! 2. [`spawn_system_metrics_collector`] — a helper that spawns a background
//!    Tokio task to collect host-level metrics (CPU, memory, swap, disk
//!    usage). System metrics are intentionally **not** collected from the
//!    request path: they do not change at request granularity, and collecting
//!    them inline would add hundreds of milliseconds to every response (see
//!    `sysinfo::MINIMUM_CPU_UPDATE_INTERVAL`).
//!
//! # Example
//!
//! ```ignore
//! use std::path::PathBuf;
//! use std::time::Duration;
//! use api_tools::server::axum::layers::prometheus::{
//!     PrometheusLayer, spawn_system_metrics_collector,
//! };
//!
//! let layer = PrometheusLayer { service_name: "myapp".into() };
//!
//! // Once, at application startup:
//! let _collector = spawn_system_metrics_collector(
//!     "myapp".into(),
//!     vec![PathBuf::from("/")],
//!     Duration::from_secs(10),
//! );
//! ```

use axum::body::Body;
use axum::extract::MatchedPath;
use axum::http::{Method, Request};
use axum::response::Response;
use futures::future::BoxFuture;
use metrics::{SharedString, counter, gauge, histogram};
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use tokio::task::JoinHandle;
use tower::{Layer, Service};

/// Prometheus metrics layer for Axum.
///
/// Records `http_requests_total` (counter) and
/// `http_requests_duration_seconds` (histogram) for every request, labeled
/// by `method`, `path` (the matched route — bounded cardinality), `service`
/// and `status`. Requests to `/metrics` are excluded.
///
/// System metrics (CPU, memory, swap, disks) are **not** collected here.
/// Use [`spawn_system_metrics_collector`] at startup instead.
#[derive(Clone)]
pub struct PrometheusLayer {
    /// Service name used as a label on every metric.
    pub service_name: String,
}

impl<S> Layer<S> for PrometheusLayer {
    type Service = PrometheusMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        PrometheusMiddleware {
            inner,
            // One-time conversion: subsequent per-request clones bump the
            // refcount only.
            service_name: Arc::from(self.service_name.as_str()),
        }
    }
}

#[derive(Clone)]
pub struct PrometheusMiddleware<S> {
    inner: S,
    service_name: Arc<str>,
}

/// Map standard HTTP methods to a `&'static str` to avoid an allocation on
/// every request. Falls back to an owned string for non-standard methods.
fn method_label(method: &Method) -> Cow<'static, str> {
    match *method {
        Method::GET => Cow::Borrowed("GET"),
        Method::POST => Cow::Borrowed("POST"),
        Method::PUT => Cow::Borrowed("PUT"),
        Method::DELETE => Cow::Borrowed("DELETE"),
        Method::PATCH => Cow::Borrowed("PATCH"),
        Method::HEAD => Cow::Borrowed("HEAD"),
        Method::OPTIONS => Cow::Borrowed("OPTIONS"),
        Method::CONNECT => Cow::Borrowed("CONNECT"),
        Method::TRACE => Cow::Borrowed("TRACE"),
        _ => Cow::Owned(method.as_str().to_owned()),
    }
}

impl<S> Service<Request<Body>> for PrometheusMiddleware<S>
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
        let path = if let Some(matched_path) = request.extensions().get::<MatchedPath>() {
            matched_path.as_str().to_owned()
        } else {
            request.uri().path().to_owned()
        };
        let method = method_label(request.method());
        let service_name = Arc::clone(&self.service_name);

        let start = Instant::now();
        let future = self.inner.call(request);
        Box::pin(async move {
            let response = future.await?;

            // Exclude metrics endpoint
            if path != "/metrics" {
                let latency = start.elapsed().as_secs_f64();
                let status = status_label(response.status().as_u16());
                let labels: [(&'static str, SharedString); 4] = [
                    ("method", method.into()),
                    ("path", path.into()),
                    ("service", service_name.into()),
                    ("status", status.into()),
                ];

                counter!("http_requests_total", &labels).increment(1);
                histogram!("http_requests_duration_seconds", &labels).record(latency);
            }

            Ok(response)
        })
    }
}

/// Map common HTTP status codes to a `&'static str` to avoid formatting an
/// integer on every response. Falls back to an owned string for uncommon
/// codes.
fn status_label(code: u16) -> Cow<'static, str> {
    match code {
        200 => Cow::Borrowed("200"),
        201 => Cow::Borrowed("201"),
        204 => Cow::Borrowed("204"),
        301 => Cow::Borrowed("301"),
        302 => Cow::Borrowed("302"),
        304 => Cow::Borrowed("304"),
        400 => Cow::Borrowed("400"),
        401 => Cow::Borrowed("401"),
        403 => Cow::Borrowed("403"),
        404 => Cow::Borrowed("404"),
        409 => Cow::Borrowed("409"),
        422 => Cow::Borrowed("422"),
        500 => Cow::Borrowed("500"),
        502 => Cow::Borrowed("502"),
        503 => Cow::Borrowed("503"),
        504 => Cow::Borrowed("504"),
        _ => Cow::Owned(code.to_string()),
    }
}

/// Spawn a background task that periodically refreshes host metrics and
/// publishes them as Prometheus gauges.
///
/// Emitted gauges (all labeled by `service`):
///
/// - `system_cpu_usage` — average global CPU usage in percent
/// - `system_total_memory` / `system_used_memory` — bytes
/// - `system_total_swap` / `system_used_swap` — bytes
/// - `system_total_disks_space` / `system_used_disks_space` — bytes,
///   summed over `disk_mount_points`
///
/// The first tick reports `system_cpu_usage = 0.0` because `sysinfo` needs
/// two snapshots to compute a delta. Subsequent ticks report the real value.
///
/// The returned [`JoinHandle`] can be aborted at shutdown; if dropped, the
/// task continues running for the lifetime of the Tokio runtime.
pub fn spawn_system_metrics_collector(
    service_name: String,
    disk_mount_points: Vec<PathBuf>,
    interval: Duration,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let refresh_kind = RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(MemoryRefreshKind::everything());
        let mut sys = System::new_with_specifics(refresh_kind);

        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;

            sys.refresh_cpu_usage();
            sys.refresh_memory();

            let cpu_usage = sys.global_cpu_usage();
            let total_memory = sys.total_memory();
            let used_memory = sys.used_memory();
            let total_swap = sys.total_swap();
            let used_swap = sys.used_swap();

            let disks = Disks::new_with_refreshed_list();
            let mut total_disks_space: u64 = 0;
            let mut used_disks_space: u64 = 0;
            for disk in &disks {
                if disk_mount_points.contains(&disk.mount_point().to_path_buf()) {
                    total_disks_space += disk.total_space();
                    used_disks_space += disk.total_space().saturating_sub(disk.available_space());
                }
            }

            gauge!("system_cpu_usage", "service" => service_name.clone()).set(cpu_usage);
            gauge!("system_total_memory", "service" => service_name.clone()).set(total_memory as f64);
            gauge!("system_used_memory", "service" => service_name.clone()).set(used_memory as f64);
            gauge!("system_total_swap", "service" => service_name.clone()).set(total_swap as f64);
            gauge!("system_used_swap", "service" => service_name.clone()).set(used_swap as f64);
            gauge!("system_total_disks_space", "service" => service_name.clone()).set(total_disks_space as f64);
            gauge!("system_used_disks_space", "service" => service_name.clone()).set(used_disks_space as f64);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, Response, StatusCode};
    use std::convert::Infallible;
    use tower::{ServiceBuilder, ServiceExt};

    /// Sentinel test: the middleware must not block on host-metrics collection.
    /// In 0.7.x, this call took ~200 ms because of an in-path
    /// `tokio::time::sleep(MINIMUM_CPU_UPDATE_INTERVAL)`. After the refactor,
    /// it should complete in well under 10 ms.
    #[tokio::test]
    async fn middleware_does_not_block_on_system_metrics() {
        let svc = ServiceBuilder::new()
            .layer(PrometheusLayer {
                service_name: "test".into(),
            })
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(Response::builder().status(StatusCode::OK).body(Body::empty()).unwrap())
            }));

        let start = Instant::now();
        let response = svc
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let elapsed = start.elapsed();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            elapsed < Duration::from_millis(50),
            "middleware took {elapsed:?}, expected < 50 ms",
        );
    }

    /// Smoke test: the collector must start, tick at least twice without
    /// panicking, and remain alive until aborted.
    #[tokio::test]
    async fn collector_ticks_without_panicking() {
        let handle = spawn_system_metrics_collector("test".into(), vec![PathBuf::from("/")], Duration::from_millis(50));

        tokio::time::sleep(Duration::from_millis(150)).await;

        assert!(!handle.is_finished(), "collector ended prematurely");
        handle.abort();
    }

    #[test]
    fn test_method_label_standard_methods_are_borrowed() {
        for (method, expected) in [
            (Method::GET, "GET"),
            (Method::POST, "POST"),
            (Method::PUT, "PUT"),
            (Method::DELETE, "DELETE"),
            (Method::PATCH, "PATCH"),
            (Method::HEAD, "HEAD"),
            (Method::OPTIONS, "OPTIONS"),
            (Method::CONNECT, "CONNECT"),
            (Method::TRACE, "TRACE"),
        ] {
            let label = method_label(&method);
            assert_eq!(label, expected);
            assert!(
                matches!(label, Cow::Borrowed(_)),
                "{method} should be Borrowed, got Owned (allocation in hot path)",
            );
        }
    }

    #[test]
    fn test_method_label_custom_method_is_owned() {
        let custom = Method::from_bytes(b"PURGE").unwrap();
        let label = method_label(&custom);
        assert_eq!(label, "PURGE");
        assert!(matches!(label, Cow::Owned(_)));
    }

    #[test]
    fn test_status_label_common_codes_are_borrowed() {
        for code in [200, 201, 204, 301, 302, 304, 400, 401, 403, 404, 409, 422, 500, 502, 503, 504] {
            let label = status_label(code);
            assert_eq!(label, code.to_string());
            assert!(
                matches!(label, Cow::Borrowed(_)),
                "{code} should be Borrowed, got Owned (allocation in hot path)",
            );
        }
    }

    #[test]
    fn test_status_label_uncommon_code_is_owned() {
        let label = status_label(418);
        assert_eq!(label, "418");
        assert!(matches!(label, Cow::Owned(_)));
    }

    /// The middleware must short-circuit on `/metrics` requests (avoiding
    /// observation loops). We can't easily inspect the global recorder, but
    /// we can at least verify the path is exercised without panicking.
    #[tokio::test]
    async fn middleware_handles_metrics_path() {
        let svc = ServiceBuilder::new()
            .layer(PrometheusLayer {
                service_name: "test".into(),
            })
            .service(tower::service_fn(|_req: Request<Body>| async {
                Ok::<_, Infallible>(Response::builder().status(StatusCode::OK).body(Body::empty()).unwrap())
            }));

        let response = svc
            .oneshot(Request::builder().uri("/metrics").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
