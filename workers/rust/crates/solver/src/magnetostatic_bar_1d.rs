use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system_with_prescribed, solve_spd_system,
};
use crate::magnetostatic_bar_1d_validation::validate_request;
use kyuubiki_protocol::{
    MagnetostaticBar1dElementResult, MagnetostaticBar1dNodeResult, SolveMagnetostaticBar1dRequest,
    SolveMagnetostaticBar1dResult,
};

pub fn solve_magnetostatic_bar_1d(
    request: &SolveMagnetostaticBar1dRequest,
) -> Result<SolveMagnetostaticBar1dResult, String> {
    validate_request(request)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut source_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        source_vector[index] = node.magnetomotive_source;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        let permeance = element.permeability * element.area / length;
        let local = [[permeance, -permeance], [-permeance, permeance]];
        let map = [element.node_i, element.node_j];

        for row in 0..2 {
            for column in 0..2 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    local[row][column],
                );
            }
        }
    }

    let prescribed = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            node.fix_magnetic_potential
                .then_some((index, node.magnetic_potential))
        })
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_source, free) =
        reduce_sparse_system_with_prescribed(&global_stiffness, &source_vector, &prescribed);
    let reduced_potentials = solve_spd_system(&reduced_stiffness, &reduced_source)?;

    let mut magnetic_potentials = vec![0.0; dof_count];
    for &(index, value) in &prescribed {
        magnetic_potentials[index] = value;
    }
    for (index, &dof) in free.iter().enumerate() {
        magnetic_potentials[dof] = reduced_potentials[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| MagnetostaticBar1dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            magnetic_potential: magnetic_potentials[index],
            magnetomotive_source: node.magnetomotive_source,
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
            let average_magnetic_potential =
                0.5 * (magnetic_potentials[element.node_i] + magnetic_potentials[element.node_j]);
            let magnetic_potential_gradient = (magnetic_potentials[element.node_j]
                - magnetic_potentials[element.node_i])
                / length;
            let magnetic_field_strength = -magnetic_potential_gradient;
            let magnetic_flux_density = element.permeability * magnetic_field_strength;
            let stored_energy = 0.5
                * element.permeability
                * magnetic_field_strength
                * magnetic_field_strength
                * element.area
                * length;

            MagnetostaticBar1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                average_magnetic_potential,
                magnetic_potential_gradient,
                magnetic_field_strength,
                magnetic_flux_density,
                stored_energy,
            }
        })
        .collect::<Vec<_>>();

    let max_magnetic_potential = nodes
        .iter()
        .map(|node| node.magnetic_potential.abs())
        .fold(0.0_f64, f64::max);
    let max_magnetic_field_strength = elements
        .iter()
        .map(|element| element.magnetic_field_strength.abs())
        .fold(0.0_f64, f64::max);
    let max_flux_density = elements
        .iter()
        .map(|element| element.magnetic_flux_density.abs())
        .fold(0.0_f64, f64::max);
    let total_stored_energy = elements.iter().map(|element| element.stored_energy).sum();

    Ok(SolveMagnetostaticBar1dResult {
        input: request.clone(),
        nodes,
        elements,
        max_magnetic_potential,
        max_magnetic_field_strength,
        max_flux_density,
        total_stored_energy,
    })
}
