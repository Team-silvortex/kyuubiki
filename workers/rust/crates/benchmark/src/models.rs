use kyuubiki_protocol::{
    SolveBarRequest, SolveHeatPlaneQuad2dRequest, SolvePlaneQuad2dRequest,
    SolvePlaneTriangle2dRequest, SolveTruss2dRequest, SolveTruss3dRequest,
};
use serde::{Deserialize, Serialize};

use crate::config::BenchmarkProfile;

#[derive(Debug, Clone)]
pub(crate) struct BenchmarkCase {
    pub(crate) id: String,
    pub(crate) family: &'static str,
    pub(crate) workload: BenchmarkWorkload,
}

#[derive(Debug, Clone)]
pub(crate) enum BenchmarkWorkload {
    AxialBar(SolveBarRequest),
    Truss2d(SolveTruss2dRequest),
    Truss3d(SolveTruss3dRequest),
    PlaneTriangle2d(SolvePlaneTriangle2dRequest),
    PlaneQuad2d(SolvePlaneQuad2dRequest),
    HeatPlaneQuad2d(SolveHeatPlaneQuad2dRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BenchmarkReport {
    pub(crate) repeat: usize,
    pub(crate) profile: BenchmarkProfile,
    pub(crate) matrix: String,
    pub(crate) generated_at_unix_s: u64,
    pub(crate) cases: Vec<BenchmarkResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BenchmarkResult {
    pub(crate) id: String,
    pub(crate) family: String,
    pub(crate) ok: bool,
    pub(crate) error: Option<String>,
    pub(crate) repeat: usize,
    pub(crate) min_ms: f64,
    pub(crate) median_ms: f64,
    pub(crate) mean_ms: f64,
    pub(crate) p95_ms: f64,
    pub(crate) max_ms: f64,
    pub(crate) dof_count: usize,
    pub(crate) node_count: usize,
    pub(crate) element_count: usize,
    pub(crate) peak_rss_kib: u64,
    pub(crate) max_displacement: f64,
    pub(crate) max_stress: f64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BenchmarkComparison {
    pub(crate) baseline_generated_at_unix_s: u64,
    pub(crate) cases: Vec<BenchmarkComparisonCase>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BenchmarkComparisonCase {
    pub(crate) id: String,
    pub(crate) baseline_median_ms: f64,
    pub(crate) median_delta_pct: f64,
    pub(crate) peak_rss_delta_pct: f64,
}

pub(crate) fn select_cases<'a>(
    cases: &'a [BenchmarkCase],
    filter: Option<&str>,
) -> Vec<&'a BenchmarkCase> {
    match filter {
        Some(filter) => cases
            .iter()
            .filter(|case| case.id.contains(filter))
            .collect(),
        None => cases.iter().collect(),
    }
}
