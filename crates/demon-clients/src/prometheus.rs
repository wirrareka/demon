//! Prometheus PromQL client — the bottleneck engine's load-signal feed.
//!
//! Per `coordination/conventions/metrics.md`: time-series metrics live in a per-group
//! Prometheus (`monitor-<group>:9090`), queried over the HTTP API, WG-only, never
//! cross-group. This client runs the canonical PromQL signals and assembles a
//! [`ServiceLoad`] for `demon-core::bottleneck`. Missing signals degrade to `0`
//! (graceful — the analyzer lowers confidence rather than inventing pressure).

use demon_core::ServiceLoad;

/// Errors querying Prometheus.
#[derive(Debug, thiserror::Error)]
pub enum PromError {
    /// HTTP/transport error.
    #[error("prometheus http error: {0}")]
    Http(#[from] reqwest::Error),
}

/// A per-residency-group Prometheus client.
#[derive(Debug, Clone)]
pub struct PrometheusClient {
    base: String,
    http: reqwest::Client,
}

/// Extract the first scalar/vector sample value from a Prometheus query response.
#[must_use]
fn extract_scalar(v: &serde_json::Value) -> Option<f64> {
    let data = v.get("data")?;
    match data.get("resultType")?.as_str()? {
        "scalar" => data.get("result")?.get(1)?.as_str()?.parse().ok(),
        "vector" => {
            let first = data.get("result")?.as_array()?.first()?;
            first.get("value")?.get(1)?.as_str()?.parse().ok()
        }
        _ => None,
    }
}

impl PrometheusClient {
    /// Build a client for a group-local Prometheus base URL (e.g. `http://10.200.0.5:9090`).
    #[must_use]
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_owned(),
            http: reqwest::Client::new(),
        }
    }

    /// Run an instant PromQL query and return the first sample value, if any.
    ///
    /// # Errors
    /// [`PromError::Http`] on transport/HTTP failure.
    pub async fn query_scalar(&self, promql: &str) -> Result<Option<f64>, PromError> {
        let url = format!("{}/api/v1/query", self.base);
        let v: serde_json::Value = self
            .http
            .get(url)
            .query(&[("query", promql)])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(extract_scalar(&v))
    }

    /// Fetch host-level load (cpu/mem/disk from `node_exporter`) for a PromQL label
    /// selector such as `instance=~"core-1.*"`. p99/queue are service-specific and left
    /// at 0 for a host view.
    ///
    /// # Errors
    /// [`PromError::Http`] on transport/HTTP failure.
    pub async fn host_load(&self, selector: &str) -> Result<ServiceLoad, PromError> {
        let cpu = self
            .query_scalar(&format!(
                "100 - (avg by (instance) (rate(node_cpu_seconds_total{{mode=\"idle\",{selector}}}[5m])) * 100)"
            ))
            .await?
            .unwrap_or(0.0);
        let mem = self
            .query_scalar(&format!(
                "(1 - node_memory_MemAvailable_bytes{{{selector}}} / node_memory_MemTotal_bytes{{{selector}}}) * 100"
            ))
            .await?
            .unwrap_or(0.0);
        let disk = self
            .query_scalar(&format!(
                "(1 - node_filesystem_avail_bytes{{mountpoint=\"/\",{selector}}} / node_filesystem_size_bytes{{mountpoint=\"/\",{selector}}}) * 100"
            ))
            .await?
            .unwrap_or(0.0);
        Ok(ServiceLoad {
            cpu_pct: cpu,
            mem_pct: mem,
            disk_pct: disk,
            p99_latency_ms: 0.0,
            queue_depth: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_vector_result() {
        let v = json!({
            "status": "success",
            "data": { "resultType": "vector",
                "result": [{ "metric": { "instance": "core-1:9100" }, "value": [1_716_000_000.0, "87.5"] }] }
        });
        assert_eq!(extract_scalar(&v), Some(87.5));
    }

    #[test]
    fn parses_scalar_result() {
        let v = json!({ "status": "success", "data": { "resultType": "scalar", "result": [1_716_000_000.0, "42"] } });
        assert_eq!(extract_scalar(&v), Some(42.0));
    }

    #[test]
    fn empty_vector_is_none() {
        let v = json!({ "status": "success", "data": { "resultType": "vector", "result": [] } });
        assert_eq!(extract_scalar(&v), None);
    }
}
