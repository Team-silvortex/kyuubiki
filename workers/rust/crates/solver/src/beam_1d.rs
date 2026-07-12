use crate::linear_algebra::{SparseMatrix, add_at, reduce_sparse_system, solve_spd_system};
use kyuubiki_protocol::{
    Beam1dElementResult, Beam1dNodeResult, SolveBeam1dRequest, SolveBeam1dResult,
    SolveThermalBeam1dRequest, SolveThermalBeam1dResult, ThermalBeam1dElementResult,
    ThermalBeam1dNodeResult,
};

pub fn solve_beam_1d(request: &SolveBeam1dRequest) -> Result<SolveBeam1dResult, String> {
    validate_beam_1d_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_y;
        force_vector[index * 2 + 1] = node.moment_z;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        let local_stiffness =
            beam_local_stiffness(element.youngs_modulus, element.moment_of_inertia, length);
        let equivalent_load = beam_uniform_load_vector(length, element.distributed_load_y);
        let map = [
            element.node_i * 2,
            element.node_i * 2 + 1,
            element.node_j * 2,
            element.node_j * 2 + 1,
        ];

        for row in 0..4 {
            force_vector[map[row]] += equivalent_load[row];
        }

        for row in 0..4 {
            for column in 0..4 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    local_stiffness[row][column],
                );
            }
        }
    }

    let constrained = request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            let mut dofs = Vec::new();
            if node.fix_y {
                dofs.push(index * 2);
            }
            if node.fix_rz {
                dofs.push(index * 2 + 1);
            }
            dofs
        })
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(&global_stiffness, &force_vector, &constrained);
    let reduced_displacements = solve_spd_system(&reduced_stiffness, &reduced_force)?;

    let mut displacements = vec![0.0; dof_count];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let uy = displacements[index * 2];
            let rz = displacements[index * 2 + 1];

            Beam1dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                uy,
                rz,
                displacement_magnitude: uy.abs(),
            }
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let length = (node_j.x - node_i.x).abs();
            let local_stiffness =
                beam_local_stiffness(element.youngs_modulus, element.moment_of_inertia, length);
            let local_displacements = [
                displacements[element.node_i * 2],
                displacements[element.node_i * 2 + 1],
                displacements[element.node_j * 2],
                displacements[element.node_j * 2 + 1],
            ];
            let equivalent_load = beam_uniform_load_vector(length, element.distributed_load_y);
            let local_forces = subtract_vector_4(
                &multiply_matrix_vector_4x4(&local_stiffness, &local_displacements),
                &equivalent_load,
            );
            let strain_energy = beam_strain_energy(&local_forces, &local_displacements);
            let max_bending_stress =
                local_forces[1].abs().max(local_forces[3].abs()) / element.section_modulus;

            Beam1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                shear_force_i: local_forces[0],
                moment_i: local_forces[1],
                shear_force_j: local_forces[2],
                moment_j: local_forces[3],
                max_bending_stress,
                strain_energy,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| node.displacement_magnitude)
        .fold(0.0_f64, f64::max);
    let max_rotation = nodes
        .iter()
        .map(|node| node.rz.abs())
        .fold(0.0_f64, f64::max);
    let max_moment = elements
        .iter()
        .flat_map(|element| [element.moment_i.abs(), element.moment_j.abs()])
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.max_bending_stress)
        .fold(0.0_f64, f64::max);
    let total_strain_energy = elements.iter().map(|element| element.strain_energy).sum();

    Ok(SolveBeam1dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_rotation,
        max_moment,
        max_stress,
        total_strain_energy,
    })
}

pub fn solve_thermal_beam_1d(
    request: &SolveThermalBeam1dRequest,
) -> Result<SolveThermalBeam1dResult, String> {
    validate_thermal_beam_1d_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_y;
        force_vector[index * 2 + 1] = node.moment_z;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        let local_stiffness =
            beam_local_stiffness(element.youngs_modulus, element.moment_of_inertia, length);
        let equivalent_load = add_vector_4(
            &beam_uniform_load_vector(length, element.distributed_load_y),
            &beam_thermal_gradient_vector(
                element.youngs_modulus,
                element.moment_of_inertia,
                element.thermal_expansion,
                element.section_depth,
                element.temperature_gradient_y,
            ),
        );
        let map = [
            element.node_i * 2,
            element.node_i * 2 + 1,
            element.node_j * 2,
            element.node_j * 2 + 1,
        ];

        for row in 0..4 {
            force_vector[map[row]] += equivalent_load[row];
        }

        for row in 0..4 {
            for column in 0..4 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    local_stiffness[row][column],
                );
            }
        }
    }

    let constrained = request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            let mut dofs = Vec::new();
            if node.fix_y {
                dofs.push(index * 2);
            }
            if node.fix_rz {
                dofs.push(index * 2 + 1);
            }
            dofs
        })
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(&global_stiffness, &force_vector, &constrained);
    let reduced_displacements = solve_spd_system(&reduced_stiffness, &reduced_force)?;

    let mut displacements = vec![0.0; dof_count];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let uy = displacements[index * 2];
            let rz = displacements[index * 2 + 1];

            ThermalBeam1dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                uy,
                rz,
                displacement_magnitude: uy.abs(),
            }
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let length = (node_j.x - node_i.x).abs();
            let local_stiffness =
                beam_local_stiffness(element.youngs_modulus, element.moment_of_inertia, length);
            let local_displacements = [
                displacements[element.node_i * 2],
                displacements[element.node_i * 2 + 1],
                displacements[element.node_j * 2],
                displacements[element.node_j * 2 + 1],
            ];
            let equivalent_load = add_vector_4(
                &beam_uniform_load_vector(length, element.distributed_load_y),
                &beam_thermal_gradient_vector(
                    element.youngs_modulus,
                    element.moment_of_inertia,
                    element.thermal_expansion,
                    element.section_depth,
                    element.temperature_gradient_y,
                ),
            );
            let local_forces = subtract_vector_4(
                &multiply_matrix_vector_4x4(&local_stiffness, &local_displacements),
                &equivalent_load,
            );
            let strain_energy = beam_strain_energy(&local_forces, &local_displacements);
            let max_bending_stress =
                local_forces[1].abs().max(local_forces[3].abs()) / element.section_modulus;

            ThermalBeam1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                temperature_gradient_y: element.temperature_gradient_y,
                thermal_curvature: element.thermal_expansion * element.temperature_gradient_y
                    / element.section_depth,
                shear_force_i: local_forces[0],
                moment_i: local_forces[1],
                shear_force_j: local_forces[2],
                moment_j: local_forces[3],
                max_bending_stress,
                strain_energy,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| node.displacement_magnitude)
        .fold(0.0_f64, f64::max);
    let max_rotation = nodes
        .iter()
        .map(|node| node.rz.abs())
        .fold(0.0_f64, f64::max);
    let max_moment = elements
        .iter()
        .flat_map(|element| [element.moment_i.abs(), element.moment_j.abs()])
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.max_bending_stress)
        .fold(0.0_f64, f64::max);
    let max_temperature_gradient = elements
        .iter()
        .map(|element| element.temperature_gradient_y.abs())
        .fold(0.0_f64, f64::max);
    let total_strain_energy = elements.iter().map(|element| element.strain_energy).sum();

    Ok(SolveThermalBeam1dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_rotation,
        max_moment,
        max_stress,
        max_temperature_gradient,
        total_strain_energy,
    })
}

fn validate_beam_1d_request(request: &SolveBeam1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d beam must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("1d beam must define at least one element".to_string());
    }

    let constrained_dofs = request.nodes.iter().fold(0, |sum, node| {
        sum + usize::from(node.fix_y) + usize::from(node.fix_rz)
    });
    if constrained_dofs < 2 {
        return Err("1d beam must restrain at least two degrees of freedom".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() {
            return Err(format!("1d beam node {index} has invalid x"));
        }
        if !node.load_y.is_finite() || !node.moment_z.is_finite() {
            return Err(format!("1d beam node {index} has invalid load"));
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("1d beam element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("1d beam element must connect two distinct nodes".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("1d beam element youngs_modulus must be positive".to_string());
        }
        if !(element.moment_of_inertia.is_finite() && element.moment_of_inertia > 0.0) {
            return Err("1d beam element moment_of_inertia must be positive".to_string());
        }
        if !(element.section_modulus.is_finite() && element.section_modulus > 0.0) {
            return Err("1d beam element section_modulus must be positive".to_string());
        }
        if !element.distributed_load_y.is_finite() {
            return Err("1d beam element distributed_load_y must be finite".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        if !length.is_finite() || length <= 1.0e-12 {
            return Err("1d beam element length must be positive".to_string());
        }
    }

    Ok(())
}

fn validate_thermal_beam_1d_request(request: &SolveThermalBeam1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("thermal beam requires at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("thermal beam requires at least one element".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() {
            return Err(format!("thermal beam node {index} has invalid x"));
        }
        if !node.load_y.is_finite() || !node.moment_z.is_finite() {
            return Err(format!("thermal beam node {index} has invalid load"));
        }
    }

    for (index, element) in request.elements.iter().enumerate() {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err(format!(
                "thermal beam element {index} references an unknown node"
            ));
        }

        if element.node_i == element.node_j {
            return Err(format!(
                "thermal beam element {index} must connect two distinct nodes"
            ));
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        if length <= f64::EPSILON {
            return Err(format!("thermal beam element {index} has zero length"));
        }

        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0)
            || !(element.moment_of_inertia.is_finite() && element.moment_of_inertia > 0.0)
            || !(element.section_modulus.is_finite() && element.section_modulus > 0.0)
            || !(element.section_depth.is_finite() && element.section_depth > 0.0)
        {
            return Err(format!(
                "thermal beam element {index} must have positive stiffness and section properties"
            ));
        }

        if !element.thermal_expansion.is_finite()
            || !element.distributed_load_y.is_finite()
            || !element.temperature_gradient_y.is_finite()
        {
            return Err(format!(
                "thermal beam element {index} has invalid thermal load data"
            ));
        }
    }

    Ok(())
}

fn beam_local_stiffness(youngs_modulus: f64, moment_of_inertia: f64, length: f64) -> [[f64; 4]; 4] {
    let flexural = youngs_modulus * moment_of_inertia;
    let l2 = length * length;
    let l3 = l2 * length;

    [
        [
            12.0 * flexural / l3,
            6.0 * flexural / l2,
            -12.0 * flexural / l3,
            6.0 * flexural / l2,
        ],
        [
            6.0 * flexural / l2,
            4.0 * flexural / length,
            -6.0 * flexural / l2,
            2.0 * flexural / length,
        ],
        [
            -12.0 * flexural / l3,
            -6.0 * flexural / l2,
            12.0 * flexural / l3,
            -6.0 * flexural / l2,
        ],
        [
            6.0 * flexural / l2,
            2.0 * flexural / length,
            -6.0 * flexural / l2,
            4.0 * flexural / length,
        ],
    ]
}

fn beam_uniform_load_vector(length: f64, distributed_load_y: f64) -> [f64; 4] {
    let l2 = length * length;

    [
        distributed_load_y * length / 2.0,
        distributed_load_y * l2 / 12.0,
        distributed_load_y * length / 2.0,
        -distributed_load_y * l2 / 12.0,
    ]
}

fn beam_thermal_gradient_vector(
    youngs_modulus: f64,
    moment_of_inertia: f64,
    thermal_expansion: f64,
    section_depth: f64,
    temperature_gradient_y: f64,
) -> [f64; 4] {
    let thermal_curvature = thermal_expansion * temperature_gradient_y / section_depth;
    let thermal_moment = youngs_modulus * moment_of_inertia * thermal_curvature;

    [0.0, -thermal_moment, 0.0, thermal_moment]
}

fn multiply_matrix_vector_4x4(matrix: &[[f64; 4]; 4], vector: &[f64; 4]) -> [f64; 4] {
    let mut output = [0.0; 4];
    for row in 0..4 {
        output[row] = (0..4).map(|index| matrix[row][index] * vector[index]).sum();
    }
    output
}

fn subtract_vector_4(lhs: &[f64; 4], rhs: &[f64; 4]) -> [f64; 4] {
    let mut output = [0.0; 4];
    for index in 0..4 {
        output[index] = lhs[index] - rhs[index];
    }
    output
}

fn beam_strain_energy(local_forces: &[f64; 4], local_displacements: &[f64; 4]) -> f64 {
    0.5 * (0..4)
        .map(|index| local_forces[index] * local_displacements[index])
        .sum::<f64>()
}

fn add_vector_4(lhs: &[f64; 4], rhs: &[f64; 4]) -> [f64; 4] {
    let mut output = [0.0; 4];
    for index in 0..4 {
        output[index] = lhs[index] + rhs[index];
    }
    output
}
