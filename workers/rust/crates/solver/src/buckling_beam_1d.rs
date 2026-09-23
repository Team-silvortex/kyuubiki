use crate::buckling_math::{checked_mode_shape, mode_direction_diagnostics};
use crate::buckling_sparse::hybrid_generalized_eigenpairs;
use crate::linear_algebra::{SparseMatrix, add_at, reduce_sparse_system};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};
use crate::solver_postprocess::try_collect_results;
use kyuubiki_protocol::{
    BUCKLING_MODE_CLUSTER_RELATIVE_TOLERANCE, BucklingBeam1dModeResult, SolveBucklingBeam1dRequest,
    SolveBucklingBeam1dResult,
};
use std::{borrow::Cow, collections::HashSet};

pub fn solve_buckling_beam_1d(
    request: &SolveBucklingBeam1dRequest,
) -> Result<SolveBucklingBeam1dResult, String> {
    solve_buckling_beam_1d_internal(Cow::Borrowed(request))
}

pub fn solve_buckling_beam_1d_owned(
    request: SolveBucklingBeam1dRequest,
) -> Result<SolveBucklingBeam1dResult, String> {
    solve_buckling_beam_1d_internal(Cow::Owned(request))
}

fn solve_buckling_beam_1d_internal(
    request: Cow<'_, SolveBucklingBeam1dRequest>,
) -> Result<SolveBucklingBeam1dResult, String> {
    validate(request.as_ref())?;
    checkpoint(SolverStage::ElementAssembly, 0)?;
    let dof_count = request.nodes.len() * 2;
    let mut elastic = SparseMatrix::new(dof_count);
    let mut geometric = SparseMatrix::new(dof_count);

    for (index, element) in request.elements.iter().enumerate() {
        // Beam rotations are derivatives along global +x, not the input edge order.
        let (left, right) = if request.nodes[element.node_i].x < request.nodes[element.node_j].x {
            (element.node_i, element.node_j)
        } else {
            (element.node_j, element.node_i)
        };
        let length = request.nodes[right].x - request.nodes[left].x;
        let local_elastic =
            elastic_stiffness(element.youngs_modulus * element.moment_of_inertia, length);
        let local_geometric = geometric_stiffness(element.reference_compressive_force, length);
        if local_elastic
            .iter()
            .chain(&local_geometric)
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err(format!(
                "buckling beam 1d element {index} stiffness exceeds the finite numeric range"
            ));
        }
        let map = [left * 2, left * 2 + 1, right * 2, right * 2 + 1];
        for row in 0..4 {
            for column in 0..4 {
                add_at(
                    &mut elastic,
                    map[row],
                    map[column],
                    local_elastic[row][column],
                );
                add_at(
                    &mut geometric,
                    map[row],
                    map[column],
                    local_geometric[row][column],
                );
            }
        }
        checkpoint_chunk(
            SolverStage::ElementAssembly,
            index + 1,
            request.elements.len(),
        )?;
    }

    let constrained = constrained_dofs(request.as_ref());
    let zero_rhs = vec![0.0; dof_count];
    let (reduced_elastic, _, free_dofs) = reduce_sparse_system(&elastic, &zero_rhs, &constrained)?;
    let (reduced_geometric, _, geometric_free_dofs) =
        reduce_sparse_system(&geometric, &zero_rhs, &constrained)?;
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
    let modes =
        try_collect_results(
            SolverStage::ResultTotals,
            eigenpairs.into_iter().zip(diagnostics).enumerate().map(
                |(index, (pair, diagnostic))| {
                    let shape = checked_mode_shape(&pair, &free_dofs, dof_count)?;
                    Ok(BucklingBeam1dModeResult {
                        index,
                        load_factor: pair.eigenvalue,
                        residual_norm: pair.residual_norm,
                        relative_gap_to_next: diagnostic.relative_gap_to_next,
                        direction_assessment: diagnostic.assessment,
                        shape,
                    })
                },
            ),
        )?;
    if modes.is_empty() {
        return Err("buckling beam 1d did not produce a positive finite mode".to_string());
    }
    Ok(SolveBucklingBeam1dResult {
        input: request.into_owned(),
        minimum_load_factor: modes[0].load_factor,
        modes,
        free_dofs,
        mode_cluster_relative_tolerance: BUCKLING_MODE_CLUSTER_RELATIVE_TOLERANCE,
    })
}

fn validate(request: &SolveBucklingBeam1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 || request.elements.is_empty() {
        return Err("buckling beam 1d requires at least two nodes and one element".to_string());
    }
    if request.nodes.iter().any(|node| !node.x.is_finite()) {
        return Err("buckling beam 1d node coordinates must be finite".to_string());
    }
    let mut node_ids = HashSet::new();
    if request
        .nodes
        .iter()
        .any(|node| node.id.is_empty() || !node_ids.insert(node.id.as_str()))
    {
        return Err("buckling beam 1d node ids must be non-empty and unique".to_string());
    }
    let mut element_ids = HashSet::new();
    for element in &request.elements {
        if element.id.is_empty() || !element_ids.insert(element.id.as_str()) {
            return Err("buckling beam 1d element ids must be non-empty and unique".to_string());
        }
        if element.node_i >= request.nodes.len()
            || element.node_j >= request.nodes.len()
            || element.node_i == element.node_j
        {
            return Err("buckling beam 1d element topology is invalid".to_string());
        }
        let length = (request.nodes[element.node_j].x - request.nodes[element.node_i].x).abs();
        if !(length.is_finite() && length > 1.0e-12) {
            return Err("buckling beam 1d element length must be positive".to_string());
        }
        for (label, value) in [
            ("youngs_modulus", element.youngs_modulus),
            ("moment_of_inertia", element.moment_of_inertia),
            (
                "reference_compressive_force",
                element.reference_compressive_force,
            ),
        ] {
            if !(value.is_finite() && value > 0.0) {
                return Err(format!("buckling beam 1d {label} must be positive"));
            }
        }
    }
    if constrained_dofs(request).len() < 2 {
        return Err("buckling beam 1d must restrain at least two degrees of freedom".to_string());
    }
    Ok(())
}

fn elastic_stiffness(ei: f64, length: f64) -> [[f64; 4]; 4] {
    let l2 = length * length;
    let factor = ei / length.powi(3);
    [
        [12.0, 6.0 * length, -12.0, 6.0 * length],
        [6.0 * length, 4.0 * l2, -6.0 * length, 2.0 * l2],
        [-12.0, -6.0 * length, 12.0, -6.0 * length],
        [6.0 * length, 2.0 * l2, -6.0 * length, 4.0 * l2],
    ]
    .map(|row| row.map(|value| value * factor))
}

fn geometric_stiffness(force: f64, length: f64) -> [[f64; 4]; 4] {
    let l2 = length * length;
    let factor = force / (30.0 * length);
    [
        [36.0, 3.0 * length, -36.0, 3.0 * length],
        [3.0 * length, 4.0 * l2, -3.0 * length, -l2],
        [-36.0, -3.0 * length, 36.0, -3.0 * length],
        [3.0 * length, -l2, -3.0 * length, 4.0 * l2],
    ]
    .map(|row| row.map(|value| value * factor))
}

fn constrained_dofs(request: &SolveBucklingBeam1dRequest) -> Vec<usize> {
    request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            [
                node.fix_y.then_some(index * 2),
                node.fix_rz.then_some(index * 2 + 1),
            ]
            .into_iter()
            .flatten()
        })
        .collect()
}
