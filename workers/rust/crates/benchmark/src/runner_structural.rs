use kyuubiki_engine::{EngineSolveRequest, solve};
use kyuubiki_protocol::AnalysisResult;

use crate::models::BenchmarkWorkload;

pub(crate) struct WorkloadMetrics {
    pub(crate) node_count: usize,
    pub(crate) element_count: usize,
    pub(crate) dof_count: usize,
    pub(crate) max_displacement: f64,
    pub(crate) max_stress: f64,
}

pub(crate) fn run_thermal_structural_workload(
    workload: &BenchmarkWorkload,
) -> Option<Result<WorkloadMetrics, String>> {
    let result = match workload {
        BenchmarkWorkload::ThermalBar1d(request) => {
            solve(EngineSolveRequest::ThermalBar1d(request.clone())).map(|result| {
                let AnalysisResult::ThermalBar1d(result) = result else {
                    unreachable!("thermal bar solve should return thermal bar result")
                };
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len(),
                    result.max_displacement,
                    result.max_stress,
                )
            })
        }
        BenchmarkWorkload::ThermalTruss2d(request) => {
            solve(EngineSolveRequest::ThermalTruss2d(request.clone())).map(|result| {
                let AnalysisResult::ThermalTruss2d(result) = result else {
                    unreachable!("thermal truss 2d solve should return thermal result")
                };
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len() * 2,
                    result.max_displacement,
                    result.max_stress,
                )
            })
        }
        BenchmarkWorkload::ThermalTruss3d(request) => {
            solve(EngineSolveRequest::ThermalTruss3d(request.clone())).map(|result| {
                let AnalysisResult::ThermalTruss3d(result) = result else {
                    unreachable!("thermal truss 3d solve should return thermal result")
                };
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len() * 3,
                    result.max_displacement,
                    result.max_stress,
                )
            })
        }
        BenchmarkWorkload::ThermalPlaneTriangle2d(request) => {
            solve(EngineSolveRequest::ThermalPlaneTriangle2d(request.clone())).map(|result| {
                let AnalysisResult::ThermalPlaneTriangle2d(result) = result else {
                    unreachable!("thermal triangle solve should return thermal plane result")
                };
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len() * 2,
                    result.max_displacement,
                    result.max_stress,
                )
            })
        }
        BenchmarkWorkload::ThermalPlaneQuad2d(request) => {
            solve(EngineSolveRequest::ThermalPlaneQuad2d(request.clone())).map(|result| {
                let AnalysisResult::ThermalPlaneQuad2d(result) = result else {
                    unreachable!("thermal quad solve should return thermal plane result")
                };
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len() * 2,
                    result.max_displacement,
                    result.max_stress,
                )
            })
        }
        BenchmarkWorkload::Frame2d(request) => solve(EngineSolveRequest::Frame2d(request.clone()))
            .map(|result| {
                let AnalysisResult::Frame2d(result) = result else {
                    unreachable!("frame 2d solve should return frame result")
                };
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len() * 3,
                    result.max_displacement,
                    result.max_stress,
                )
            }),
        BenchmarkWorkload::Frame3d(request) => solve(EngineSolveRequest::Frame3d(request.clone()))
            .map(|result| {
                let AnalysisResult::Frame3d(result) = result else {
                    unreachable!("frame 3d solve should return frame result")
                };
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len() * 6,
                    result.max_displacement,
                    result.max_stress,
                )
            }),
        BenchmarkWorkload::ThermalFrame2d(request) => {
            solve(EngineSolveRequest::ThermalFrame2d(request.clone())).map(|result| {
                let AnalysisResult::ThermalFrame2d(result) = result else {
                    unreachable!("thermal frame 2d solve should return thermal frame result")
                };
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len() * 3,
                    result.max_displacement,
                    result.max_stress,
                )
            })
        }
        BenchmarkWorkload::ThermalFrame3d(request) => {
            solve(EngineSolveRequest::ThermalFrame3d(request.clone())).map(|result| {
                let AnalysisResult::ThermalFrame3d(result) = result else {
                    unreachable!("thermal frame 3d solve should return thermal frame result")
                };
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len() * 6,
                    result.max_displacement,
                    result.max_stress,
                )
            })
        }
        _ => return None,
    };

    Some(result)
}

impl WorkloadMetrics {
    fn from_counts(
        node_count: usize,
        element_count: usize,
        dof_count: usize,
        max_displacement: f64,
        max_stress: f64,
    ) -> Self {
        Self {
            node_count,
            element_count,
            dof_count,
            max_displacement,
            max_stress,
        }
    }
}
