use crate::buckling_math::mode_direction_diagnostics;
use crate::buckling_sparse::hybrid_generalized_eigenpairs;
use crate::frame_2d_stability::assemble_frame_2d_stability;
use crate::linear_algebra::reduce_sparse_system;
use kyuubiki_protocol::{
    BUCKLING_MODE_CLUSTER_RELATIVE_TOLERANCE, BucklingFrame2dModeResult,
    SolveBucklingFrame2dRequest, SolveBucklingFrame2dResult,
};

pub fn solve_buckling_frame_2d(
    request: &SolveBucklingFrame2dRequest,
) -> Result<SolveBucklingFrame2dResult, String> {
    let system = assemble_frame_2d_stability(request)?;
    let dof_count = request.frame.nodes.len() * 3;
    let zero_rhs = vec![0.0; dof_count];
    let (reduced_elastic, _, free_dofs) =
        reduce_sparse_system(&system.elastic, &zero_rhs, &system.constrained_dofs);
    let (reduced_geometric, _, geometric_free_dofs) =
        reduce_sparse_system(&system.geometric, &zero_rhs, &system.constrained_dofs);
    debug_assert_eq!(free_dofs, geometric_free_dofs);
    let mode_limit = request.mode_count.unwrap_or(3).max(1).min(free_dofs.len());
    let eigenpairs =
        hybrid_generalized_eigenpairs(&reduced_elastic, &reduced_geometric, mode_limit)?;
    let diagnostics = mode_direction_diagnostics(
        &eigenpairs
            .iter()
            .map(|pair| pair.eigenvalue)
            .collect::<Vec<_>>(),
    );
    let modes = eigenpairs
        .into_iter()
        .zip(diagnostics)
        .enumerate()
        .map(|(index, (pair, diagnostic))| BucklingFrame2dModeResult {
            index,
            load_factor: pair.eigenvalue,
            residual_norm: pair.residual_norm,
            relative_gap_to_next: diagnostic.relative_gap_to_next,
            direction_assessment: diagnostic.assessment,
            shape: expand_and_normalize(&pair.vector, &free_dofs, dof_count),
        })
        .collect::<Vec<_>>();

    Ok(SolveBucklingFrame2dResult {
        input: request.clone(),
        minimum_load_factor: modes[0].load_factor,
        static_result: system.static_result,
        element_preloads: system.element_preloads,
        modes,
        free_dofs,
        mode_cluster_relative_tolerance: BUCKLING_MODE_CLUSTER_RELATIVE_TOLERANCE,
    })
}

fn expand_and_normalize(reduced: &[f64], free: &[usize], size: usize) -> Vec<f64> {
    let mut shape = vec![0.0; size];
    for (index, &dof) in free.iter().enumerate() {
        shape[dof] = reduced[index];
    }
    let norm = shape.iter().map(|value| value * value).sum::<f64>().sqrt();
    if norm > 0.0 {
        shape.iter_mut().for_each(|value| *value /= norm);
    }
    shape
}
