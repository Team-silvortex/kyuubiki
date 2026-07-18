use kyuubiki_engine::{EngineSolveRequest, solve};
use kyuubiki_protocol::AnalysisResult;
use kyuubiki_solver::{
    PlaneQuadProfileStage, SpdSolveOptions, ThermalPlaneProfileStage,
    profile_plane_quad_2d_with_options, profile_plane_triangle_2d_with_options,
    profile_thermal_plane_quad_2d_with_options, profile_thermal_plane_triangle_2d_with_options,
};

use crate::models::{BenchmarkMemoryStage, BenchmarkWorkload};
use crate::runner_preconditioner::parse_preconditioner;

pub(crate) struct WorkloadMetrics {
    pub(crate) node_count: usize,
    pub(crate) element_count: usize,
    pub(crate) dof_count: usize,
    pub(crate) max_displacement: f64,
    pub(crate) max_stress: f64,
    pub(crate) memory_stages: Vec<BenchmarkMemoryStage>,
    pub(crate) solver_iterations: Option<usize>,
    pub(crate) solver_matrix_non_zero_count: Option<usize>,
    pub(crate) solver_residual_norm: Option<f64>,
    pub(crate) solver_preconditioner: Option<String>,
}

pub(crate) fn run_thermal_structural_workload(
    workload: &BenchmarkWorkload,
    solver_preconditioner: &str,
    progress: bool,
) -> Option<Result<WorkloadMetrics, String>> {
    let solve_options = SpdSolveOptions {
        preconditioner: parse_preconditioner(solver_preconditioner),
        progress_interval: progress.then_some(256),
    };
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
            profile_thermal_plane_triangle_2d_with_options(request, solve_options.clone()).map(
                |profile| {
                    let result = profile.result;
                    WorkloadMetrics::from_counts(
                        result.nodes.len(),
                        result.elements.len(),
                        result.nodes.len() * 2,
                        result.max_displacement,
                        result.max_stress,
                    )
                    .with_profile(
                        benchmark_stages_from_thermal(profile.stages),
                        profile.solver_iterations,
                        profile.solver_matrix_non_zero_count,
                        profile.solver_residual_norm,
                        solver_preconditioner,
                    )
                },
            )
        }
        BenchmarkWorkload::ThermalPlaneQuad2d(request) => {
            profile_thermal_plane_quad_2d_with_options(request, solve_options).map(|profile| {
                let result = profile.result;
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len() * 2,
                    result.max_displacement,
                    result.max_stress,
                )
                .with_profile(
                    benchmark_stages_from_thermal(profile.stages),
                    profile.solver_iterations,
                    profile.solver_matrix_non_zero_count,
                    profile.solver_residual_norm,
                    solver_preconditioner,
                )
            })
        }
        BenchmarkWorkload::PlaneTriangle2d(request) => {
            profile_plane_triangle_2d_with_options(request, solve_options.clone()).map(|profile| {
                let result = profile.result;
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len() * 2,
                    result.max_displacement,
                    result.max_stress,
                )
                .with_profile(
                    benchmark_stages_from_plane(profile.stages),
                    profile.solver_iterations,
                    profile.solver_matrix_non_zero_count,
                    profile.solver_residual_norm,
                    solver_preconditioner,
                )
            })
        }
        BenchmarkWorkload::PlaneQuad2d(request) => {
            profile_plane_quad_2d_with_options(request, solve_options).map(|profile| {
                let result = profile.result;
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len() * 2,
                    result.max_displacement,
                    result.max_stress,
                )
                .with_profile(
                    benchmark_stages_from_plane(profile.stages),
                    profile.solver_iterations,
                    profile.solver_matrix_non_zero_count,
                    profile.solver_residual_norm,
                    solver_preconditioner,
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
            memory_stages: Vec::new(),
            solver_iterations: None,
            solver_matrix_non_zero_count: None,
            solver_residual_norm: None,
            solver_preconditioner: None,
        }
    }

    fn with_profile(
        mut self,
        memory_stages: Vec<BenchmarkMemoryStage>,
        solver_iterations: usize,
        solver_matrix_non_zero_count: usize,
        solver_residual_norm: f64,
        solver_preconditioner: &str,
    ) -> Self {
        self.memory_stages = memory_stages;
        self.solver_iterations = Some(solver_iterations);
        self.solver_matrix_non_zero_count = Some(solver_matrix_non_zero_count);
        self.solver_residual_norm = Some(solver_residual_norm);
        self.solver_preconditioner = Some(solver_preconditioner.to_string());
        self
    }
}

fn benchmark_stages_from_plane(stages: Vec<PlaneQuadProfileStage>) -> Vec<BenchmarkMemoryStage> {
    stages
        .into_iter()
        .map(|stage| BenchmarkMemoryStage {
            label: stage.label.to_string(),
            rss_kib: stage.rss_kib,
            elapsed_ms: Some(stage.elapsed_ms),
        })
        .collect()
}

fn benchmark_stages_from_thermal(
    stages: Vec<ThermalPlaneProfileStage>,
) -> Vec<BenchmarkMemoryStage> {
    stages
        .into_iter()
        .map(|stage| BenchmarkMemoryStage {
            label: stage.label.to_string(),
            rss_kib: stage.rss_kib,
            elapsed_ms: Some(stage.elapsed_ms),
        })
        .collect()
}
