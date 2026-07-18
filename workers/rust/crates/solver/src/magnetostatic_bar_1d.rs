use crate::chain_tridiagonal::{is_indexed_chain, solve_with_prescribed};
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

    let magnetic_potentials = solve_magnetic_potentials(request)?;

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

fn solve_magnetic_potentials(request: &SolveMagnetostaticBar1dRequest) -> Result<Vec<f64>, String> {
    let node_count = request.nodes.len();
    let prescribed = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            node.fix_magnetic_potential
                .then_some((index, node.magnetic_potential))
        })
        .collect::<Vec<_>>();
    let source_vector = request
        .nodes
        .iter()
        .map(|node| node.magnetomotive_source)
        .collect::<Vec<_>>();

    if is_indexed_chain(
        node_count,
        request
            .elements
            .iter()
            .map(|element| (element.node_i, element.node_j)),
    ) {
        let mut diagonal = vec![0.0; node_count];
        let mut lower = vec![0.0; node_count - 1];
        let mut upper = vec![0.0; node_count - 1];
        for element in &request.elements {
            let length = (request.nodes[element.node_j].x - request.nodes[element.node_i].x).abs();
            let permeance = element.permeability * element.area / length;
            let left = element.node_i.min(element.node_j);
            diagonal[element.node_i] += permeance;
            diagonal[element.node_j] += permeance;
            lower[left] -= permeance;
            upper[left] -= permeance;
        }
        return solve_with_prescribed(&diagonal, &lower, &upper, &source_vector, &prescribed);
    }

    let mut global_stiffness = SparseMatrix::new(node_count);
    for element in &request.elements {
        let length = (request.nodes[element.node_j].x - request.nodes[element.node_i].x).abs();
        let permeance = element.permeability * element.area / length;
        for (row, column, value) in [
            (element.node_i, element.node_i, permeance),
            (element.node_i, element.node_j, -permeance),
            (element.node_j, element.node_i, -permeance),
            (element.node_j, element.node_j, permeance),
        ] {
            add_at(&mut global_stiffness, row, column, value);
        }
    }
    let (reduced_stiffness, reduced_rhs, free) =
        reduce_sparse_system_with_prescribed(&global_stiffness, &source_vector, &prescribed);
    let reduced_values = solve_spd_system(&reduced_stiffness, &reduced_rhs)?;
    let mut values = vec![0.0; node_count];
    for &(index, value) in &prescribed {
        values[index] = value;
    }
    for (index, &dof) in free.iter().enumerate() {
        values[dof] = reduced_values[index];
    }
    Ok(values)
}
