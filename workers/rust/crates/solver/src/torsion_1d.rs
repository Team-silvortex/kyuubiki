use crate::linear_algebra::{SparseMatrix, add_at, reduce_sparse_system, solve_spd_system};
use kyuubiki_protocol::{
    SolveTorsion1dRequest, SolveTorsion1dResult, Torsion1dElementResult, Torsion1dNodeResult,
};

pub fn solve_torsion_1d(request: &SolveTorsion1dRequest) -> Result<SolveTorsion1dResult, String> {
    validate_torsion_1d_request(request)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut torque_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        torque_vector[index] = node.torque_z;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        let stiffness = element.shear_modulus * element.polar_moment / length;
        let map = [element.node_i, element.node_j];
        let local_stiffness = [[stiffness, -stiffness], [-stiffness, stiffness]];

        for row in 0..2 {
            for column in 0..2 {
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
        .filter_map(|(index, node)| node.fix_rz.then_some(index))
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_torque, free) =
        reduce_sparse_system(&global_stiffness, &torque_vector, &constrained);
    let reduced_rotations = solve_spd_system(&reduced_stiffness, &reduced_torque)?;

    let mut rotations = vec![0.0; dof_count];
    for (index, &dof) in free.iter().enumerate() {
        rotations[dof] = reduced_rotations[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| Torsion1dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            rz: rotations[index],
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
            let twist = rotations[element.node_j] - rotations[element.node_i];
            let torque = element.shear_modulus * element.polar_moment * twist / length;
            let shear_stress = torque.abs() / element.section_modulus;
            let strain_energy = 0.5 * torque * twist;

            Torsion1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                twist,
                torque,
                shear_stress,
                strain_energy,
            }
        })
        .collect::<Vec<_>>();

    let max_rotation = nodes
        .iter()
        .map(|node| node.rz.abs())
        .fold(0.0_f64, f64::max);
    let max_torque = elements
        .iter()
        .map(|element| element.torque.abs())
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.shear_stress)
        .fold(0.0_f64, f64::max);
    let total_strain_energy = elements.iter().map(|element| element.strain_energy).sum();

    Ok(SolveTorsion1dResult {
        input: request.clone(),
        nodes,
        elements,
        max_rotation,
        max_torque,
        max_stress,
        total_strain_energy,
    })
}

fn validate_torsion_1d_request(request: &SolveTorsion1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d torsion model must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("1d torsion model must define at least one element".to_string());
    }

    if !request.nodes.iter().any(|node| node.fix_rz) {
        return Err("1d torsion model must include at least one rotational support".to_string());
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("1d torsion element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("1d torsion element must connect two distinct nodes".to_string());
        }
        if !(element.shear_modulus.is_finite() && element.shear_modulus > 0.0) {
            return Err("1d torsion element shear_modulus must be positive".to_string());
        }
        if !(element.polar_moment.is_finite() && element.polar_moment > 0.0) {
            return Err("1d torsion element polar_moment must be positive".to_string());
        }
        if !(element.section_modulus.is_finite() && element.section_modulus > 0.0) {
            return Err("1d torsion element section_modulus must be positive".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        if length <= 1.0e-12 {
            return Err("1d torsion element length must be positive".to_string());
        }
    }

    Ok(())
}
