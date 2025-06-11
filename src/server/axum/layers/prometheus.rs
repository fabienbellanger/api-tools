//! Prometheus' metrics layer

use axum::body::Body;
use axum::extract::MatchedPath;
use axum::http::Request;
use axum::response::Response;
use bytesize::ByteSize;
use futures::future::BoxFuture;
use metrics::{counter, gauge, histogram};
use std::fmt;
use std::path::PathBuf;
use std::task::{Context, Poll};
use std::time::Instant;
use sysinfo::{Disks, System};
use tower::{Layer, Service};

/// Prometheus metrics layer for Axum
#[derive(Clone)]
pub struct PrometheusLayer {
    /// Service name
    pub service_name: String,

    /// Disk mount points to monitor
    pub disk_mount_points: Vec<PathBuf>,
}

impl<S> Layer<S> for PrometheusLayer {
    type Service = PrometheusMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        PrometheusMiddleware {
            inner,
            service_name: self.service_name.clone(),
            disk_mount_points: self.disk_mount_points.clone(),
        }
    }
}

#[derive(Clone)]
pub struct PrometheusMiddleware<S> {
    inner: S,
    service_name: String,
    disk_mount_points: Vec<PathBuf>,
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
        let method = request.method().to_string();
        let service_name = self.service_name.clone();
        let disk_mount_points = self.disk_mount_points.clone();

        let start = Instant::now();
        let future = self.inner.call(request);
        Box::pin(async move {
            let response = future.await?;

            // Exclude metrics endpoint
            if path != "/metrics" {
                let latency = start.elapsed().as_secs_f64();
                let status = response.status().as_u16().to_string();
                let labels = [
                    ("method", method),
                    ("path", path),
                    ("service", service_name.clone()),
                    ("status", status),
                ];

                counter!("http_requests_total", &labels).increment(1);
                histogram!("http_requests_duration_seconds", &labels).record(latency);
            }

            // System metrics
            let system_metrics = SystemMetrics::new(&disk_mount_points).await;
            system_metrics.add_metrics(service_name);

            Ok(response)
        })
    }
}

#[derive(Debug, Clone)]
struct SystemMetrics {
    /// Average CPU usage in percent
    cpu_usage: f32,

    /// Total memory in bytes
    total_memory: u64,

    /// Used memory in bytes
    used_memory: u64,

    /// Total swap space in bytes
    total_swap: u64,

    /// Used swap space in bytes
    used_swap: u64,

    /// Total disk space in bytes for a specified mount point
    total_disks_space: u64,

    /// Used disk space in bytes for a specified mount point
    used_disks_space: u64,
}

impl SystemMetrics {
    /// Creates a new `SystemMetrics` instance, refreshing the system information
    async fn new(disk_mount_points: &[PathBuf]) -> Self {
        let mut sys = System::new_all();

        // CPU
        sys.refresh_cpu_usage();
        let mut cpu_usage = sys.global_cpu_usage();
        tokio::time::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL).await;
        sys.refresh_cpu_usage();
        cpu_usage += sys.global_cpu_usage();
        cpu_usage /= 2.0;

        // Memory
        sys.refresh_memory();
        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();

        // Swap
        let total_swap = sys.total_swap();
        let used_swap = sys.used_swap();

        // Disks
        let disks = Disks::new_with_refreshed_list();
        let mut total_disks_space = 0;
        let mut used_disks_space = 0;
        for disk in &disks {
            if disk_mount_points.contains(&disk.mount_point().to_path_buf()) {
                total_disks_space += disk.total_space();
                used_disks_space += disk.total_space() - disk.available_space();
            }
        }

        Self {
            cpu_usage,
            total_memory,
            used_memory,
            total_swap,
            used_swap,
            total_disks_space,
            used_disks_space,
        }
    }

    /// Adds the system metrics to Prometheus gauges
    fn add_metrics(&self, service_name: String) {
        gauge!("system_cpu_usage", "service" => service_name.clone()).set(self.cpu_usage);
        gauge!("system_total_memory", "service" => service_name.clone()).set(self.total_memory as f64);
        gauge!("system_used_memory", "service" => service_name.clone()).set(self.used_memory as f64);
        gauge!("system_total_swap", "service" => service_name.clone()).set(self.total_swap as f64);
        gauge!("system_used_swap", "service" => service_name.clone()).set(self.used_swap as f64);
        gauge!("system_total_disks_space", "service" => service_name.clone()).set(self.total_disks_space as f64);
        gauge!("system_used_disks_usage", "service" => service_name).set(self.used_disks_space as f64);
    }
}

impl fmt::Display for SystemMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CPUs:       {:.1}%\n\
             Memory:     {} / {}\n\
             Swap:       {} / {}\n\
             Disk usage: {} / {}",
            self.cpu_usage,
            ByteSize::b(self.used_memory),
            ByteSize::b(self.total_memory),
            ByteSize::b(self.used_swap),
            ByteSize::b(self.total_swap),
            ByteSize::b(self.used_disks_space),
            ByteSize::b(self.total_disks_space),
        )
    }
}
