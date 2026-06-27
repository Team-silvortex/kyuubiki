use crate::frame_2d_math::{frame_local_stiffness, frame_transform, transform_frame_stiffness};
use crate::modal_math::{expand_mode_shape, jacobi_eigenpairs, mass_normalized_stiffness};
use kyuubiki_protocol::{
    ModalFrame2dModeResult, SolveModalFrame2dRequest, SolveModalFrame2dResult,
};

pub fn solve_modal_frame_2d(
    request: &SolveModalFrame2dRequest,
) -> Result<SolveModalFrame2dResult, String> {
    validate_modal_frame_2d_request(request)?;

    let dof_count = request.nodes.len() * 3;
    let mut stiffness = vec![vec![0.0; dof_count]; dof_count];
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
                stiffness[map[row]][map[column]] += element_stiffness[row][column];
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
    let free_dofs = (0..dof_count)
        .filter(|dof| !constrained.contains(dof))
        .collect::<Vec<_>>();
    if free_dofs.is_empty() {
        return Err("modal frame 2d must leave at least one free degree of freedom".to_string());
    }

    let reduced = mass_normalized_stiffness(&stiffness, &mass, &free_dofs, "modal frame 2d")?;
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

fn validate_modal_frame_2d_request(request: &SolveModalFrame2dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("modal frame 2d must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("modal frame 2d must define at least one element".to_string());
    }
    if constrained_dofs(request).len() < 3 {
        return Err("modal frame 2d must restrain at least three degrees of freedom".to_string());
    }
    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("modal frame 2d element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("modal frame 2d element must connect two distinct nodes".to_string());
        }
        for (label, value) in [
            ("area", element.area),
            ("youngs_modulus", element.youngs_modulus),
            ("moment_of_inertia", element.moment_of_inertia),
            ("section_modulus", element.section_modulus),
            ("density", element.density),
        ] {
            if !(value.is_finite() && value > 0.0) {
                return Err(format!("modal frame 2d element {label} must be positive"));
            }
        }
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = ((node_j.x - node_i.x).powi(2) + (node_j.y - node_i.y).powi(2)).sqrt();
        if !(length.is_finite() && length > 1.0e-12) {
            return Err("modal frame 2d element length must be positive".to_string());
        }
    }
    Ok(())
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
