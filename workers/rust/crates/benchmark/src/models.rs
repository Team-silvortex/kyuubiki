use kyuubiki_protocol::{
    SolveAcousticBar1dRequest, SolveBarRequest, SolveBeam1dRequest, SolveContactGap1dRequest,
    SolveElectrostaticBar1dRequest, SolveElectrostaticPlaneQuad2dRequest,
    SolveElectrostaticPlaneTriangle2dRequest, SolveFrame2dRequest, SolveFrame3dRequest,
    SolveHeatBar1dRequest, SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest,
    SolveMagnetostaticBar1dRequest, SolveMagnetostaticPlaneQuad2dRequest,
    SolveMagnetostaticPlaneTriangle2dRequest, SolveModalFrame2dRequest, SolveModalFrame3dRequest,
    SolveNonlinearSpring1dRequest, SolvePlaneQuad2dRequest, SolvePlaneTriangle2dRequest,
    SolveSpring1dRequest, SolveSpring2dRequest, SolveSpring3dRequest,
    SolveStokesFlowPlaneQuad2dRequest, SolveThermalBar1dRequest, SolveThermalBeam1dRequest,
    SolveThermalFrame2dRequest, SolveThermalFrame3dRequest, SolveThermalPlaneQuad2dRequest,
    SolveThermalPlaneTriangle2dRequest, SolveThermalTruss2dRequest, SolveThermalTruss3dRequest,
    SolveTorsion1dRequest, SolveTruss2dRequest, SolveTruss3dRequest,
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
    ThermalBar1d(SolveThermalBar1dRequest),
    AcousticBar1d(SolveAcousticBar1dRequest),
    HeatBar1d(SolveHeatBar1dRequest),
    ElectrostaticBar1d(SolveElectrostaticBar1dRequest),
    MagnetostaticBar1d(SolveMagnetostaticBar1dRequest),
    Torsion1d(SolveTorsion1dRequest),
    Spring1d(SolveSpring1dRequest),
    Spring2d(SolveSpring2dRequest),
    Spring3d(SolveSpring3dRequest),
    NonlinearSpring1d(SolveNonlinearSpring1dRequest),
    ContactGap1d(SolveContactGap1dRequest),
    Beam1d(SolveBeam1dRequest),
    ThermalBeam1d(SolveThermalBeam1dRequest),
    Frame2d(SolveFrame2dRequest),
    Frame3d(SolveFrame3dRequest),
    ThermalFrame2d(SolveThermalFrame2dRequest),
    ThermalFrame3d(SolveThermalFrame3dRequest),
    ModalFrame2d(SolveModalFrame2dRequest),
    ModalFrame3d(SolveModalFrame3dRequest),
    Truss2d(SolveTruss2dRequest),
    Truss3d(SolveTruss3dRequest),
    ThermalTruss2d(SolveThermalTruss2dRequest),
    ThermalTruss3d(SolveThermalTruss3dRequest),
    PlaneTriangle2d(SolvePlaneTriangle2dRequest),
    PlaneQuad2d(SolvePlaneQuad2dRequest),
    ThermalPlaneTriangle2d(SolveThermalPlaneTriangle2dRequest),
    ThermalPlaneQuad2d(SolveThermalPlaneQuad2dRequest),
    HeatPlaneTriangle2d(SolveHeatPlaneTriangle2dRequest),
    HeatPlaneQuad2d(SolveHeatPlaneQuad2dRequest),
    ElectrostaticPlaneTriangle2d(SolveElectrostaticPlaneTriangle2dRequest),
    ElectrostaticPlaneQuad2d(SolveElectrostaticPlaneQuad2dRequest),
    MagnetostaticPlaneTriangle2d(SolveMagnetostaticPlaneTriangle2dRequest),
    MagnetostaticPlaneQuad2d(SolveMagnetostaticPlaneQuad2dRequest),
    StokesFlowPlaneQuad2d(SolveStokesFlowPlaneQuad2dRequest),
    HeadlessActionManifest,
    DirectFemManifest,
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
pub(crate) struct BenchmarkMemoryStage {
    pub(crate) label: String,
    pub(crate) rss_kib: u64,
    #[serde(default)]
    pub(crate) elapsed_ms: Option<f64>,
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
    #[serde(default)]
    pub(crate) memory_stages: Vec<BenchmarkMemoryStage>,
    #[serde(default)]
    pub(crate) solver_iterations: Option<usize>,
    #[serde(default)]
    pub(crate) solver_residual_norm: Option<f64>,
    #[serde(default)]
    pub(crate) solver_preconditioner: Option<String>,
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
