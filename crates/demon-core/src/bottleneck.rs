//! Bottleneck detection + capacity planning (pure core).
//!
//! Detection is **rules + confidence tiers, not ML**: each breached threshold is a
//! corroborating signal, and more corroboration raises confidence. Scaling computes a
//! target node count **from a capacity model — never a blind `+1`**: `N = ceil(load /
//! (per-node-capacity × target-utilisation))`, clamped by guardrails. The guarded
//! scale-out action itself runs through the normal mutation pipeline.

use serde::{Deserialize, Serialize};

/// Observed load for one service (normalised percentages + workload signals).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ServiceLoad {
    /// CPU utilisation (0–100).
    pub cpu_pct: f64,
    /// Memory utilisation (0–100).
    pub mem_pct: f64,
    /// Disk utilisation (0–100).
    pub disk_pct: f64,
    /// 99th-percentile latency (ms).
    pub p99_latency_ms: f64,
    /// Pending work queue depth.
    pub queue_depth: u64,
}

/// Confidence tier for a finding — rises with corroborating signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

/// A detected pressure signal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Which signal tripped (`cpu`, `mem`, `disk`, `latency`, `queue`).
    pub signal: String,
    /// Confidence this is a real bottleneck.
    pub confidence: Confidence,
    /// Human-readable detail.
    pub detail: String,
}

/// Per-node capacity assumptions for a service kind.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CapacityModel {
    /// Work units one node can sustain at 100% (same unit as `observed_load`).
    pub per_node_capacity: f64,
    /// Target utilisation fraction to plan to (e.g. 0.70 leaves 30% headroom).
    pub target_utilisation: f64,
    /// Hard cap on node count (guardrail).
    pub max_nodes: u32,
}

/// A computed scaling recommendation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScaleRecommendation {
    /// Current node count.
    pub current_nodes: u32,
    /// Recommended node count (from the capacity model, clamped to guardrails).
    pub desired_nodes: u32,
    /// Why.
    pub reason: String,
}

/// The result of analysing a service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BottleneckReport {
    /// Corroborating pressure findings.
    pub findings: Vec<Finding>,
    /// Overall confidence (the strongest finding), if any.
    pub confidence: Option<Confidence>,
    /// Scaling recommendation, if scaling out would help and is within guardrails.
    pub recommendation: Option<ScaleRecommendation>,
}

fn finding(signal: &str, confidence: Confidence, detail: String) -> Finding {
    Finding {
        signal: signal.to_owned(),
        confidence,
        detail,
    }
}

/// Rule-based detection: each breached threshold is a finding. Severe breaches are
/// `High`, moderate ones `Medium`/`Low`.
#[must_use]
pub fn detect(load: &ServiceLoad) -> Vec<Finding> {
    let mut out = Vec::new();
    if load.cpu_pct >= 90.0 {
        out.push(finding("cpu", Confidence::High, format!("cpu at {:.0}%", load.cpu_pct)));
    } else if load.cpu_pct >= 80.0 {
        out.push(finding("cpu", Confidence::Medium, format!("cpu at {:.0}%", load.cpu_pct)));
    }
    if load.mem_pct >= 90.0 {
        out.push(finding("mem", Confidence::High, format!("memory at {:.0}%", load.mem_pct)));
    } else if load.mem_pct >= 80.0 {
        out.push(finding("mem", Confidence::Medium, format!("memory at {:.0}%", load.mem_pct)));
    }
    if load.disk_pct >= 85.0 {
        out.push(finding("disk", Confidence::High, format!("disk at {:.0}%", load.disk_pct)));
    } else if load.disk_pct >= 75.0 {
        out.push(finding("disk", Confidence::Low, format!("disk at {:.0}%", load.disk_pct)));
    }
    if load.p99_latency_ms >= 1000.0 {
        out.push(finding(
            "latency",
            Confidence::High,
            format!("p99 latency {:.0}ms", load.p99_latency_ms),
        ));
    } else if load.p99_latency_ms >= 500.0 {
        out.push(finding(
            "latency",
            Confidence::Medium,
            format!("p99 latency {:.0}ms", load.p99_latency_ms),
        ));
    }
    if load.queue_depth >= 1000 {
        out.push(finding("queue", Confidence::High, format!("queue depth {}", load.queue_depth)));
    } else if load.queue_depth >= 200 {
        out.push(finding("queue", Confidence::Medium, format!("queue depth {}", load.queue_depth)));
    }
    out
}

/// The strongest confidence among findings.
#[must_use]
pub fn overall_confidence(findings: &[Finding]) -> Option<Confidence> {
    findings.iter().map(|f| f.confidence).max()
}

/// Convert a clamped, non-negative `f64` count to `u32` (deliberate, bounded).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn nodes_to_u32(n: f64) -> u32 {
    n.max(1.0).round() as u32
}

/// Compute the desired node count from the capacity model. Never a blind `+1`:
/// `ceil(observed_load / (per_node_capacity × target_utilisation))`, clamped to
/// `[1, max_nodes]`.
#[must_use]
pub fn desired_nodes(observed_load: f64, model: &CapacityModel) -> u32 {
    let util = model.target_utilisation.clamp(0.05, 1.0);
    let effective = (model.per_node_capacity * util).max(f64::MIN_POSITIVE);
    let raw = (observed_load / effective).ceil();
    nodes_to_u32(raw).min(model.max_nodes.max(1))
}

/// Analyse a service: detect pressure, and if it is real (≥ `Medium`) and the capacity
/// model says more nodes are needed, recommend scaling out.
#[must_use]
pub fn analyze(
    load: &ServiceLoad,
    current_nodes: u32,
    observed_load: f64,
    model: &CapacityModel,
) -> BottleneckReport {
    let findings = detect(load);
    let confidence = overall_confidence(&findings);
    let desired = desired_nodes(observed_load, model);
    let recommendation = match confidence {
        Some(c) if c >= Confidence::Medium && desired > current_nodes => Some(ScaleRecommendation {
            current_nodes,
            desired_nodes: desired,
            reason: format!(
                "{} corroborating signal(s); capacity model wants {desired} node(s) (have {current_nodes})",
                findings.len()
            ),
        }),
        _ => None,
    };
    BottleneckReport {
        findings,
        confidence,
        recommendation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calm() -> ServiceLoad {
        ServiceLoad {
            cpu_pct: 30.0,
            mem_pct: 40.0,
            disk_pct: 50.0,
            p99_latency_ms: 80.0,
            queue_depth: 5,
        }
    }

    #[test]
    fn calm_service_has_no_findings() {
        assert!(detect(&calm()).is_empty());
        assert_eq!(overall_confidence(&[]), None);
    }

    #[test]
    fn corroborating_breaches_raise_confidence() {
        let hot = ServiceLoad {
            cpu_pct: 95.0,
            mem_pct: 92.0,
            disk_pct: 60.0,
            p99_latency_ms: 1200.0,
            queue_depth: 1500,
        };
        let f = detect(&hot);
        assert_eq!(f.len(), 4); // cpu, mem, latency, queue
        assert_eq!(overall_confidence(&f), Some(Confidence::High));
    }

    #[test]
    fn capacity_model_computes_n_not_blind_increment() {
        // load 1000 units, 300/node at 70% target => effective 210/node => ceil(1000/210)=5
        let model = CapacityModel {
            per_node_capacity: 300.0,
            target_utilisation: 0.70,
            max_nodes: 10,
        };
        assert_eq!(desired_nodes(1000.0, &model), 5);
        // guardrail clamps
        assert_eq!(desired_nodes(100_000.0, &model), 10);
        // never below 1
        assert_eq!(desired_nodes(0.0, &model), 1);
    }

    #[test]
    fn analyze_recommends_scale_when_pressured_and_undersized() {
        let model = CapacityModel {
            per_node_capacity: 300.0,
            target_utilisation: 0.70,
            max_nodes: 10,
        };
        let hot = ServiceLoad {
            cpu_pct: 95.0,
            mem_pct: 50.0,
            disk_pct: 50.0,
            p99_latency_ms: 1200.0,
            queue_depth: 50,
        };
        let report = analyze(&hot, 2, 1000.0, &model);
        let rec = report.recommendation.expect("should recommend scaling");
        assert_eq!(rec.current_nodes, 2);
        assert_eq!(rec.desired_nodes, 5);
        assert_eq!(report.confidence, Some(Confidence::High));
    }

    #[test]
    fn no_recommendation_when_capacity_sufficient() {
        let model = CapacityModel {
            per_node_capacity: 300.0,
            target_utilisation: 0.70,
            max_nodes: 10,
        };
        // pressured but already have enough nodes for the load
        let hot = ServiceLoad {
            cpu_pct: 95.0,
            mem_pct: 50.0,
            disk_pct: 50.0,
            p99_latency_ms: 1200.0,
            queue_depth: 50,
        };
        let report = analyze(&hot, 8, 1000.0, &model); // desired 5 <= 8
        assert!(report.recommendation.is_none());
    }

    #[test]
    fn low_confidence_alone_does_not_recommend() {
        let model = CapacityModel {
            per_node_capacity: 100.0,
            target_utilisation: 0.70,
            max_nodes: 10,
        };
        // only a Low disk finding; even if undersized, Low confidence won't auto-recommend
        let mild = ServiceLoad {
            cpu_pct: 40.0,
            mem_pct: 40.0,
            disk_pct: 78.0,
            p99_latency_ms: 100.0,
            queue_depth: 10,
        };
        let report = analyze(&mild, 1, 1000.0, &model);
        assert_eq!(report.confidence, Some(Confidence::Low));
        assert!(report.recommendation.is_none());
    }
}
