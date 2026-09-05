use kyuubiki_protocol::{
    SolveAcousticBar1dRequest, SolveAdvectionDiffusionBar1dRequest, SolveBarRequest,
    SolveBeam1dRequest, SolveBucklingBeam1dRequest, SolveBucklingFrame2dRequest,
    SolveCohesiveInterface1dRequest, SolveCohesiveInterface2dRequest,
    SolveCohesiveInterfaceMesh2dRequest, SolveCohesiveInterfaceMesh3dRequest,
    SolveContactGap1dRequest, SolveElectricConductionPlaneQuad2dRequest,
    SolveElectrostaticBar1dRequest, SolveElectrostaticPlaneQuad2dRequest,
    SolveElectrostaticPlaneTriangle2dRequest, SolveFrame2dMaterialPDeltaRequest,
    SolveFrame2dPDeltaRequest, SolveFrame2dRequest, SolveFrame3dRequest,
    SolveHarmonicSpring1dRequest, SolveHeatBar1dRequest, SolveHeatPlaneQuad2dRequest,
    SolveHeatPlaneTriangle2dRequest, SolveMagnetostaticBar1dRequest,
    SolveMagnetostaticPlaneQuad2dRequest, SolveMagnetostaticPlaneTriangle2dRequest,
    SolveModalFrame2dRequest, SolveModalFrame3dRequest, SolveNonlinearSpring1dRequest,
    SolvePlaneQuad2dRequest, SolvePlaneTriangle2dRequest, SolveSolidTetra3dRequest,
    SolveSpring1dRequest, SolveSpring2dRequest, SolveSpring3dRequest,
    SolveStokesFlowPlaneQuad2dRequest, SolveStokesFlowPlaneTriangle2dRequest,
    SolveThermalBar1dRequest, SolveThermalBeam1dRequest, SolveThermalFrame2dRequest,
    SolveThermalFrame3dRequest, SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest,
    SolveThermalTruss2dRequest, SolveThermalTruss3dRequest, SolveTorsion1dRequest,
    SolveTransientHeatBar1dRequest, SolveTransientSpring1dRequest, SolveTruss2dRequest,
    SolveTruss3dRequest, WorkflowGraphRunRequest,
};
use serde::{Deserialize, Serialize};

use crate::config::BenchmarkProfile;

pub(crate) const SHARED_PROCESS_RSS_SCOPE: &str = "shared_process_high_water_mark";
pub(crate) const ISOLATED_CASE_RSS_SCOPE: &str = "isolated_case_process_high_water_mark";

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
    TransientHeatBar1d(SolveTransientHeatBar1dRequest),
    ElectrostaticBar1d(SolveElectrostaticBar1dRequest),
    MagnetostaticBar1d(SolveMagnetostaticBar1dRequest),
    AdvectionDiffusionBar1d(SolveAdvectionDiffusionBar1dRequest),
    Torsion1d(SolveTorsion1dRequest),
    Spring1d(SolveSpring1dRequest),
    TransientSpring1d(SolveTransientSpring1dRequest),
    HarmonicSpring1d(SolveHarmonicSpring1dRequest),
    Spring2d(SolveSpring2dRequest),
    Spring3d(SolveSpring3dRequest),
    NonlinearSpring1d(SolveNonlinearSpring1dRequest),
    ContactGap1d(SolveContactGap1dRequest),
    CohesiveInterface1d(SolveCohesiveInterface1dRequest),
    CohesiveInterface2d(SolveCohesiveInterface2dRequest),
    CohesiveInterfaceMesh2d(SolveCohesiveInterfaceMesh2dRequest),
    CohesiveInterfaceMesh3d(SolveCohesiveInterfaceMesh3dRequest),
    Beam1d(SolveBeam1dRequest),
    ThermalBeam1d(SolveThermalBeam1dRequest),
    Frame2d(SolveFrame2dRequest),
    Frame3d(SolveFrame3dRequest),
    ThermalFrame2d(SolveThermalFrame2dRequest),
    ThermalFrame3d(SolveThermalFrame3dRequest),
    ModalFrame2d(SolveModalFrame2dRequest),
    BucklingBeam1d(SolveBucklingBeam1dRequest),
    BucklingFrame2d(SolveBucklingFrame2dRequest),
    Frame2dPDelta(SolveFrame2dPDeltaRequest),
    Frame2dMaterialPDelta(SolveFrame2dMaterialPDeltaRequest),
    ModalFrame3d(SolveModalFrame3dRequest),
    SolidTetra3d(SolveSolidTetra3dRequest),
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
    ElectricConductionPlaneQuad2d(SolveElectricConductionPlaneQuad2dRequest),
    StokesFlowPlaneTriangle2d(SolveStokesFlowPlaneTriangle2dRequest),
    StokesFlowPlaneQuad2d(SolveStokesFlowPlaneQuad2dRequest),
    HeadlessActionManifest,
    DirectFemManifest,
    ProtocolOperatorTaskPreview(serde_json::Value),
    ProtocolWorkflowRoundTrip(WorkflowGraphRunRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BenchmarkReport {
    pub(crate) repeat: usize,
    pub(crate) profile: BenchmarkProfile,
    pub(crate) matrix: String,
    pub(crate) generated_at_unix_s: u64,
    #[serde(default = "default_rss_scope")]
    pub(crate) rss_scope: String,
    pub(crate) cases: Vec<BenchmarkResult>,
    #[serde(default)]
    pub(crate) preconditioner_comparisons: Vec<BenchmarkPreconditionerComparison>,
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
    #[serde(default)]
    pub(crate) history_step_count: Option<usize>,
    pub(crate) peak_rss_kib: u64,
    #[serde(default)]
    pub(crate) memory_stages: Vec<BenchmarkMemoryStage>,
    #[serde(default)]
    pub(crate) solver_iterations: Option<usize>,
    #[serde(default)]
    pub(crate) solver_matrix_non_zero_count: Option<usize>,
    #[serde(default)]
    pub(crate) solver_residual_norm: Option<f64>,
    #[serde(default)]
    pub(crate) solver_preconditioner: Option<String>,
    #[serde(default)]
    pub(crate) solver_preconditioner_reason: Option<String>,
    #[serde(default)]
    pub(crate) hotspot_label: Option<String>,
    #[serde(default)]
    pub(crate) hotspot_elapsed_ms: Option<f64>,
    #[serde(default)]
    pub(crate) hotspot_share_pct: Option<f64>,
    #[serde(default)]
    pub(crate) hotspot_hint: Option<String>,
    pub(crate) max_displacement: f64,
    pub(crate) max_stress: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct BenchmarkPreconditionerComparison {
    pub(crate) base_case_id: String,
    pub(crate) winner_preconditioner: String,
    pub(crate) winner_median_ms: f64,
    pub(crate) winner_solver_iterations: Option<usize>,
    pub(crate) winner_speedup_ratio: f64,
    pub(crate) winner_iteration_reduction_pct: Option<f64>,
    pub(crate) compared: Vec<BenchmarkPreconditionerResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct BenchmarkPreconditionerResult {
    pub(crate) solver_preconditioner: String,
    pub(crate) median_ms: f64,
    pub(crate) solver_iterations: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BenchmarkComparison {
    pub(crate) baseline_generated_at_unix_s: u64,
    pub(crate) baseline_rss_scope: String,
    pub(crate) current_rss_scope: String,
    pub(crate) peak_rss_comparable: bool,
    pub(crate) cases: Vec<BenchmarkComparisonCase>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BenchmarkComparisonCase {
    pub(crate) id: String,
    pub(crate) baseline_median_ms: f64,
    pub(crate) median_delta_pct: f64,
    pub(crate) peak_rss_delta_pct: f64,
}

#[cfg(test)]
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

pub(crate) fn select_case_ids(
    case_ids: &[String],
    filter: Option<&str>,
    exact: Option<&str>,
) -> Vec<String> {
    case_ids
        .iter()
        .filter(|id| match (filter, exact) {
            (_, Some(exact)) => id.as_str() == exact,
            (Some(filter), None) => id.contains(filter),
            (None, None) => true,
        })
        .cloned()
        .collect()
}

fn default_rss_scope() -> String {
    SHARED_PROCESS_RSS_SCOPE.to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        BenchmarkCase, BenchmarkReport, BenchmarkWorkload, SHARED_PROCESS_RSS_SCOPE,
        select_case_ids, select_cases,
    };
    use kyuubiki_protocol::SolveBarRequest;

    #[test]
    fn case_filter_matches_documented_substrings() {
        let cases = [case("axial-bar-10k"), case("axial-bar-100k")];

        assert_eq!(select_cases(&cases, Some("axial-bar")).len(), 2);
        assert_eq!(select_cases(&cases, Some("100k"))[0].id, "axial-bar-100k");
        assert!(select_cases(&cases, Some("missing")).is_empty());
    }

    #[test]
    fn case_id_filter_can_require_exact_identity() {
        let ids = vec![
            "frame-2d-100k".to_string(),
            "thermal-frame-2d-100k".to_string(),
        ];

        assert_eq!(select_case_ids(&ids, Some("frame-2d"), None).len(), 2);
        assert_eq!(
            select_case_ids(&ids, None, Some("frame-2d-100k")),
            vec!["frame-2d-100k"]
        );
    }

    #[test]
    fn legacy_reports_default_to_shared_process_rss_scope() {
        let report = serde_json::from_value::<BenchmarkReport>(serde_json::json!({
            "repeat": 1,
            "profile": "medium",
            "matrix": "core",
            "generated_at_unix_s": 1,
            "cases": [],
        }))
        .expect("legacy benchmark report should remain readable");

        assert_eq!(report.rss_scope, SHARED_PROCESS_RSS_SCOPE);
    }

    fn case(id: &str) -> BenchmarkCase {
        BenchmarkCase {
            id: id.to_string(),
            family: "axial_bar_1d",
            workload: BenchmarkWorkload::AxialBar(SolveBarRequest {
                length: 1.0,
                area: 1.0,
                youngs_modulus: 1.0,
                elements: 1,
                tip_force: 1.0,
            }),
        }
    }
}
