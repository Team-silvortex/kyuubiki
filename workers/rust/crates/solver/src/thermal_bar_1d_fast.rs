use kyuubiki_protocol::{
    SolveThermalBar1dRequest, SolveThermalBar1dResult, ThermalBar1dElementResult,
    ThermalBar1dNodeResult,
};

use crate::chain_tridiagonal::{is_indexed_chain, solve_with_prescribed};

pub(crate) fn solve_thermal_bar_1d_chain(
    request: &SolveThermalBar1dRequest,
) -> Option<Result<SolveThermalBar1dResult, String>> {
    if !is_single_span_chain(request) {
        return None;
    }

    Some(
        solve_chain_displacements(request)
            .map(|displacements| build_thermal_bar_1d_result(request, displacements)),
    )
}

pub(crate) fn build_thermal_bar_1d_result(
    request: &SolveThermalBar1dRequest,
    displacements: Vec<f64>,
) -> SolveThermalBar1dResult {
    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| ThermalBar1dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            ux: displacements[index],
            temperature_delta: node.temperature_delta,
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| build_element_result(index, element, request, &displacements))
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| node.ux.abs())
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.stress.abs())
        .fold(0.0_f64, f64::max);
    let max_axial_force = elements
        .iter()
        .map(|element| element.axial_force.abs())
        .fold(0.0_f64, f64::max);
    let max_temperature_delta = nodes
        .iter()
        .map(|node| node.temperature_delta.abs())
        .fold(0.0_f64, f64::max);
    let total_strain_energy = elements
        .iter()
        .zip(request.elements.iter())
        .map(|(element, input)| element.strain_energy_density * input.area * element.length)
        .sum();
    let max_strain_energy_density = elements
        .iter()
        .map(|element| element.strain_energy_density.abs())
        .fold(0.0_f64, f64::max);

    SolveThermalBar1dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_stress,
        max_axial_force,
        max_temperature_delta,
        total_strain_energy,
        max_strain_energy_density,
    }
}

fn is_single_span_chain(request: &SolveThermalBar1dRequest) -> bool {
    is_indexed_chain(
        request.nodes.len(),
        request
            .elements
            .iter()
            .map(|element| (element.node_i, element.node_j)),
    )
}

fn solve_chain_displacements(request: &SolveThermalBar1dRequest) -> Result<Vec<f64>, String> {
    let node_count = request.nodes.len();
    let mut diagonal = vec![0.0; node_count];
    let mut lower = vec![0.0; node_count.saturating_sub(1)];
    let mut upper = vec![0.0; node_count.saturating_sub(1)];
    let mut force = request
        .nodes
        .iter()
        .map(|node| node.load_x)
        .collect::<Vec<_>>();

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        let stiffness = element.youngs_modulus * element.area / length;
        let average_temperature_delta = 0.5 * (node_i.temperature_delta + node_j.temperature_delta);
        let thermal_force = element.youngs_modulus
            * element.area
            * element.thermal_expansion
            * average_temperature_delta;

        diagonal[element.node_i] += stiffness;
        diagonal[element.node_j] += stiffness;
        let left = element.node_i.min(element.node_j);
        lower[left] -= stiffness;
        upper[left] -= stiffness;
        force[element.node_i] -= thermal_force;
        force[element.node_j] += thermal_force;
    }

    let prescribed = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.fix_x.then_some((index, 0.0)))
        .collect::<Vec<_>>();
    solve_with_prescribed(&diagonal, &lower, &upper, &force, &prescribed)
}

fn build_element_result(
    index: usize,
    element: &kyuubiki_protocol::ThermalBar1dElementInput,
    request: &SolveThermalBar1dRequest,
    displacements: &[f64],
) -> ThermalBar1dElementResult {
    let node_i = &request.nodes[element.node_i];
    let node_j = &request.nodes[element.node_j];
    let length = (node_j.x - node_i.x).abs();
    let average_temperature_delta = 0.5 * (node_i.temperature_delta + node_j.temperature_delta);
    let total_strain = (displacements[element.node_j] - displacements[element.node_i]) / length;
    let thermal_strain = element.thermal_expansion * average_temperature_delta;
    let mechanical_strain = total_strain - thermal_strain;
    let stress = element.youngs_modulus * mechanical_strain;
    let axial_force = stress * element.area;
    let strain_energy_density = 0.5 * stress * mechanical_strain;

    ThermalBar1dElementResult {
        index,
        id: element.id.clone(),
        node_i: element.node_i,
        node_j: element.node_j,
        length,
        average_temperature_delta,
        thermal_strain,
        mechanical_strain,
        total_strain,
        stress,
        axial_force,
        strain_energy_density,
    }
}
