use std::time::Instant;

use crate::linear_algebra::{
    SparseMatrix, reduce_sparse_system, solve_spd_system_profile_with_options,
};
use crate::linear_solver_profile::SpdSolveOptions;
use crate::thermal_plane_2d_profile::{ThermalPlaneProfileStage, push_thermal_plane_stage};
use kyuubiki_protocol::SolveThermalPlaneTriangle2dRequest;

pub(crate) struct ThermalPlaneSolvedDisplacements {
    pub(crate) displacements: Vec<f64>,
    pub(crate) solver_iterations: usize,
    pub(crate) solver_matrix_non_zero_count: usize,
    pub(crate) solver_residual_norm: f64,
}

pub(crate) fn solve_thermal_plane_displacements(
    request: &SolveThermalPlaneTriangle2dRequest,
    global_stiffness: &SparseMatrix,
    force_vector: &[f64],
    solve_options: SpdSolveOptions,
    stages: &mut Vec<ThermalPlaneProfileStage>,
    collect_stages: bool,
) -> Result<ThermalPlaneSolvedDisplacements, String> {
    let mut stage_started = Instant::now();
    let constrained = request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            [
                node.fix_x.then_some(index * 2),
                node.fix_y.then_some(index * 2 + 1),
            ]
        })
        .flatten()
        .collect::<Vec<_>>();
    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(global_stiffness, force_vector, &constrained);
    push_thermal_plane_stage(
        stages,
        collect_stages,
        "reduce_system",
        stage_started.elapsed(),
    );

    stage_started = Instant::now();
    let solve_profile =
        solve_spd_system_profile_with_options(&reduced_stiffness, &reduced_force, solve_options)?;
    let solver_iterations = solve_profile.iterations;
    let solver_matrix_non_zero_count = solve_profile.matrix_non_zero_count;
    let solver_residual_norm = solve_profile.residual_norm;
    let reduced_displacements = solve_profile.solution;
    push_thermal_plane_stage(
        stages,
        collect_stages,
        "solve_system",
        stage_started.elapsed(),
    );

    let mut displacements = vec![0.0; request.nodes.len() * 2];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }
    Ok(ThermalPlaneSolvedDisplacements {
        displacements,
        solver_iterations,
        solver_matrix_non_zero_count,
        solver_residual_norm,
    })
}
