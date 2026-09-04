use kyuubiki_solver::{
    SpdSolveOptions, solve_electric_conduction_plane_quad_2d_with_options,
    solve_electrostatic_plane_quad_2d_with_options,
    solve_electrostatic_plane_triangle_2d_with_options,
    solve_magnetostatic_plane_quad_2d_with_options,
    solve_magnetostatic_plane_triangle_2d_with_options,
};

use crate::models::BenchmarkWorkload;
use crate::runner_preconditioner::parse_preconditioner;
use crate::runner_structural::WorkloadMetrics;

pub(crate) fn run_electromagnetic_workload(
    workload: &BenchmarkWorkload,
    solver_preconditioner: &str,
    progress: bool,
) -> Option<Result<WorkloadMetrics, String>> {
    let options = SpdSolveOptions {
        preconditioner: parse_preconditioner(solver_preconditioner),
        progress_interval: progress.then_some(256),
    };
    let result = match workload {
        BenchmarkWorkload::ElectrostaticPlaneTriangle2d(request) => {
            solve_electrostatic_plane_triangle_2d_with_options(request, options).map(|result| {
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len(),
                    result.max_potential,
                    result.max_electric_field,
                )
                .with_preconditioner(solver_preconditioner)
            })
        }
        BenchmarkWorkload::ElectrostaticPlaneQuad2d(request) => {
            solve_electrostatic_plane_quad_2d_with_options(request, options).map(|result| {
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len(),
                    result.max_potential,
                    result.max_electric_field,
                )
                .with_preconditioner(solver_preconditioner)
            })
        }
        BenchmarkWorkload::MagnetostaticPlaneTriangle2d(request) => {
            solve_magnetostatic_plane_triangle_2d_with_options(request, options).map(|result| {
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len(),
                    result.max_vector_potential,
                    result.max_magnetic_field_strength,
                )
                .with_preconditioner(solver_preconditioner)
            })
        }
        BenchmarkWorkload::MagnetostaticPlaneQuad2d(request) => {
            solve_magnetostatic_plane_quad_2d_with_options(request, options).map(|result| {
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len(),
                    result.max_vector_potential,
                    result.max_magnetic_field_strength,
                )
                .with_preconditioner(solver_preconditioner)
            })
        }
        BenchmarkWorkload::ElectricConductionPlaneQuad2d(request) => {
            solve_electric_conduction_plane_quad_2d_with_options(request, options).map(|result| {
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len(),
                    result.max_electric_potential_v,
                    result.max_current_density_a_m2,
                )
                .with_preconditioner(solver_preconditioner)
            })
        }
        _ => return None,
    };

    Some(result)
}
