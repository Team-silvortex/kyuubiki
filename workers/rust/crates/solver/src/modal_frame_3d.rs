use crate::frame_3d_math::{
    frame3d_dof_map, frame3d_local_stiffness, frame3d_rotation, frame3d_transform,
    transform_frame3d_stiffness,
};
use crate::modal_frame_validation::validate_modal_frame_3d_request;
use crate::modal_math::{
    ensure_dense_modal_size, expand_mode_shape, jacobi_eigenpairs, mass_normalized_stiffness,
};
use kyuubiki_protocol::{
    ModalFrame3dModeResult, SolveModalFrame3dRequest, SolveModalFrame3dResult,
};

pub fn solve_modal_frame_3d(
    request: &SolveModalFrame3dRequest,
) -> Result<SolveModalFrame3dResult, String> {
    validate_modal_frame_3d_request(request)?;

    let dof_count = request.nodes.len() * 6;
    ensure_dense_modal_size(dof_count, "modal frame 3d")?;
    let mut stiffness = vec![vec![0.0; dof_count]; dof_count];
    let mut mass = vec![0.0; dof_count];
    let mut total_mass = 0.0;

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let dz = node_j.z - node_i.z;
        let length = (dx * dx + dy * dy + dz * dz).sqrt();
        let rotation = frame3d_rotation(dx, dy, dz, length)?;
        let local_stiffness = frame3d_local_stiffness(
            element.area,
            element.youngs_modulus,
            element.shear_modulus,
            element.torsion_constant,
            element.moment_of_inertia_y,
            element.moment_of_inertia_z,
            length,
        );
        let transform = frame3d_transform(&rotation);
        let element_stiffness = transform_frame3d_stiffness(&local_stiffness, &transform);
        let map = frame3d_dof_map(element.node_i, element.node_j);

        for row in 0..12 {
            for column in 0..12 {
                stiffness[map[row]][map[column]] += element_stiffness[row][column];
            }
        }

        let element_mass = element.density * element.area * length;
        let translational_mass = element_mass / 2.0;
        let rotary_mass = element_mass * length * length / 24.0;
        total_mass += element_mass;
        for node_index in [element.node_i, element.node_j] {
            let offset = node_index * 6;
            mass[offset] += translational_mass;
            mass[offset + 1] += translational_mass;
            mass[offset + 2] += translational_mass;
            mass[offset + 3] += rotary_mass;
            mass[offset + 4] += rotary_mass;
            mass[offset + 5] += rotary_mass;
        }
    }

    let constrained = constrained_modal_frame_3d_dofs(request);
    let free_dofs = (0..dof_count)
        .filter(|dof| !constrained.contains(dof))
        .collect::<Vec<_>>();
    if free_dofs.is_empty() {
        return Err("modal frame 3d must leave at least one free degree of freedom".to_string());
    }

    let reduced = mass_normalized_stiffness(&stiffness, &mass, &free_dofs, "modal frame 3d")?;
    let eigenpairs = jacobi_eigenpairs(reduced);
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
            ModalFrame3dModeResult {
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
        return Err("modal frame 3d did not produce a positive finite mode".to_string());
    }

    Ok(SolveModalFrame3dResult {
        input: request.clone(),
        min_frequency_hz: modes
            .iter()
            .map(|mode| mode.natural_frequency_hz)
            .fold(f64::INFINITY, f64::min),
        max_frequency_hz: modes
            .iter()
            .map(|mode| mode.natural_frequency_hz)
            .fold(0.0_f64, f64::max),
        modes,
        free_dofs,
        total_mass,
    })
}

fn constrained_modal_frame_3d_dofs(request: &SolveModalFrame3dRequest) -> Vec<usize> {
    request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            [
                node.fix_x.then_some(index * 6),
                node.fix_y.then_some(index * 6 + 1),
                node.fix_z.then_some(index * 6 + 2),
                node.fix_rx.then_some(index * 6 + 3),
                node.fix_ry.then_some(index * 6 + 4),
                node.fix_rz.then_some(index * 6 + 5),
            ]
            .into_iter()
            .flatten()
        })
        .collect()
}
