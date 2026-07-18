use crate::frame_2d_math::{frame_local_stiffness, frame_transform, transform_frame_stiffness};
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::modal_frame_validation::validate_modal_frame_2d_request;
use crate::modal_math::{ensure_dense_modal_size, expand_mode_shape, jacobi_eigenpairs};
use crate::modal_sparse::{
    InverseIterationOptions, inverse_power_iteration, reduce_sparse_modal_system,
};
use kyuubiki_protocol::{
    ModalFrame2dModeResult, SolveModalFrame2dRequest, SolveModalFrame2dResult,
};

pub fn solve_modal_frame_2d(
    request: &SolveModalFrame2dRequest,
) -> Result<SolveModalFrame2dResult, String> {
    validate_modal_frame_2d_request(request)?;

    let dof_count = request.nodes.len() * 3;
    let mut stiffness = SparseMatrix::new(dof_count);
    let mut mass = vec![0.0; dof_count];
    let mut total_mass = 0.0;

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = (dx * dx + dy * dy).sqrt();
        let c = dx / length;
        let s = dy / length;
        let local_stiffness = frame_local_stiffness(
            element.area,
            element.youngs_modulus,
            element.moment_of_inertia,
            length,
        );
        let transform = frame_transform(c, s);
        let element_stiffness = transform_frame_stiffness(&local_stiffness, &transform);
        let map = frame_dof_map(element.node_i, element.node_j);

        for row in 0..6 {
            for column in 0..6 {
                add_at(
                    &mut stiffness,
                    map[row],
                    map[column],
                    element_stiffness[row][column],
                );
            }
        }

        let element_mass = element.density * element.area * length;
        let translational_mass = element_mass / 2.0;
        let rotary_mass = element_mass * length * length / 24.0;
        total_mass += element_mass;
        for node_index in [element.node_i, element.node_j] {
            mass[node_index * 3] += translational_mass;
            mass[node_index * 3 + 1] += translational_mass;
            mass[node_index * 3 + 2] += rotary_mass;
        }
    }

    let constrained = constrained_dofs(request);
    let sparse_system = reduce_sparse_modal_system(&stiffness, &mass, &constrained)?;
    let free_dofs = sparse_system.free_dofs.clone();
    let eigenpairs = if request.mode_count == Some(1) {
        let options = InverseIterationOptions::default();
        let pair = sparse_system
            .operator
            .smallest_tridiagonal_eigenpair(options.tolerance)
            .unwrap_or_else(|| {
                inverse_power_iteration(
                    free_dofs.len(),
                    options,
                    |vector| sparse_system.operator.apply(vector),
                    |rhs| sparse_system.solve_normalized_inverse(rhs),
                )
            })?;
        vec![(pair.eigenvalue, pair.vector)]
    } else {
        ensure_dense_modal_size(dof_count, "modal frame 2d")?;
        jacobi_eigenpairs(sparse_system.operator.dense_fallback_matrix()?)
    };
    let mode_limit = request.mode_count.unwrap_or(6).max(1).min(eigenpairs.len());

    let modes = eigenpairs
        .into_iter()
        .filter(|(eigenvalue, _)| eigenvalue.is_finite() && *eigenvalue > 1.0e-9)
        .take(mode_limit)
        .enumerate()
        .map(|(index, (eigenvalue, vector))| {
            let natural_frequency_rad_s = eigenvalue.sqrt();
            let natural_frequency_hz = natural_frequency_rad_s / std::f64::consts::TAU;
            let shape = expand_mode_shape(&vector, &mass, &free_dofs, dof_count);
            ModalFrame2dModeResult {
                index,
                eigenvalue_rad_s_squared: eigenvalue,
                natural_frequency_rad_s,
                natural_frequency_hz,
                period_s: 1.0 / natural_frequency_hz,
                participation_norm: shape.iter().map(|value| value * value).sum::<f64>().sqrt(),
                shape,
            }
        })
        .collect::<Vec<_>>();

    if modes.is_empty() {
        return Err("modal frame 2d did not produce a positive finite mode".to_string());
    }

    let min_frequency_hz = modes
        .iter()
        .map(|mode| mode.natural_frequency_hz)
        .fold(f64::INFINITY, f64::min);
    let max_frequency_hz = modes
        .iter()
        .map(|mode| mode.natural_frequency_hz)
        .fold(0.0_f64, f64::max);

    Ok(SolveModalFrame2dResult {
        input: request.clone(),
        modes,
        free_dofs,
        total_mass,
        min_frequency_hz,
        max_frequency_hz,
    })
}

fn constrained_dofs(request: &SolveModalFrame2dRequest) -> Vec<usize> {
    request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            let mut dofs = Vec::new();
            if node.fix_x {
                dofs.push(index * 3);
            }
            if node.fix_y {
                dofs.push(index * 3 + 1);
            }
            if node.fix_rz {
                dofs.push(index * 3 + 2);
            }
            dofs
        })
        .collect()
}

fn frame_dof_map(node_i: usize, node_j: usize) -> [usize; 6] {
    [
        node_i * 3,
        node_i * 3 + 1,
        node_i * 3 + 2,
        node_j * 3,
        node_j * 3 + 1,
        node_j * 3 + 2,
    ]
}
