use kyuubiki_protocol::{
    SolveThermalBar1dRequest, SolveThermalBar1dResult, ThermalBar1dElementResult,
    ThermalBar1dNodeResult,
};

use crate::chain_tridiagonal::solve_path_with_prescribed;

pub(crate) fn solve_thermal_bar_1d_chain(
    request: &SolveThermalBar1dRequest,
) -> Option<Result<SolveThermalBar1dResult, String>> {
    solve_path_displacements(request).map(|result| {
        result.map(|displacements| build_thermal_bar_1d_result(request, displacements))
    })
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

fn solve_path_displacements(
    request: &SolveThermalBar1dRequest,
) -> Option<Result<Vec<f64>, String>> {
    let node_count = request.nodes.len();
    let mut force = request
        .nodes
        .iter()
        .map(|node| node.load_x)
        .collect::<Vec<_>>();

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let average_temperature_delta = 0.5 * (node_i.temperature_delta + node_j.temperature_delta);
        let thermal_force = element.youngs_modulus
            * element.area
            * element.thermal_expansion
            * average_temperature_delta;

        force[element.node_i] -= thermal_force;
        force[element.node_j] += thermal_force;
    }

    let prescribed = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.fix_x.then_some((index, 0.0)))
        .collect::<Vec<_>>();
    solve_path_with_prescribed(
        node_count,
        &request.elements,
        |element| (element.node_i, element.node_j),
        |element| {
            let length = (request.nodes[element.node_j].x - request.nodes[element.node_i].x).abs();
            let stiffness = element.youngs_modulus * element.area / length;
            Ok([[stiffness, -stiffness], [-stiffness, stiffness]])
        },
        &force,
        &prescribed,
    )
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
