use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system, reduce_sparse_system_with_prescribed,
    solve_spd_system,
};
use crate::thermal_bar_1d_fast::{build_thermal_bar_1d_result, solve_thermal_bar_1d_chain};
use kyuubiki_protocol::{
    ElectrostaticBar1dElementResult, ElectrostaticBar1dNodeResult, ElementResult,
    HeatBar1dElementResult, HeatBar1dNodeResult, NodeResult, SolveBarRequest, SolveBarResult,
    SolveElectrostaticBar1dRequest, SolveElectrostaticBar1dResult, SolveHeatBar1dRequest,
    SolveHeatBar1dResult, SolveThermalBar1dRequest, SolveThermalBar1dResult,
};

pub fn solve_bar_1d(request: &SolveBarRequest) -> Result<SolveBarResult, String> {
    validate_request(request)?;

    let node_count = request.elements + 1;
    let element_length = request.length / request.elements as f64;
    let stiffness = request.youngs_modulus * request.area / element_length;
    let displacement_step = request.tip_force / stiffness;

    let displacements = (0..node_count)
        .map(|index| displacement_step * index as f64)
        .collect::<Vec<_>>();

    let nodes = displacements
        .iter()
        .enumerate()
        .map(|(index, displacement)| NodeResult {
            index,
            x: request.length * index as f64 / request.elements as f64,
            displacement: *displacement,
        })
        .collect::<Vec<_>>();

    let elements = (0..request.elements)
        .map(|index| {
            let left = &nodes[index];
            let right = &nodes[index + 1];
            let strain = (right.displacement - left.displacement) / element_length;
            let stress = request.youngs_modulus * strain;

            ElementResult {
                index,
                x1: left.x,
                x2: right.x,
                strain,
                stress,
                axial_force: stress * request.area,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| node.displacement.abs())
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.stress.abs())
        .fold(0.0_f64, f64::max);

    Ok(SolveBarResult {
        input: request.clone(),
        nodes,
        elements,
        tip_displacement: *displacements.last().unwrap_or(&0.0),
        reaction_force: -request.tip_force,
        max_displacement,
        max_stress,
    })
}

pub fn solve_thermal_bar_1d(
    request: &SolveThermalBar1dRequest,
) -> Result<SolveThermalBar1dResult, String> {
    validate_thermal_bar_1d_request(request)?;
    if let Some(result) = solve_thermal_bar_1d_chain(request) {
        return result;
    }

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::with_uniform_row_capacity(dof_count, 12);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index] = node.load_x;
    }

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
        let map = [element.node_i, element.node_j];
        let local_stiffness = [[stiffness, -stiffness], [-stiffness, stiffness]];
        let equivalent_load = [-thermal_force, thermal_force];

        for row in 0..2 {
            force_vector[map[row]] += equivalent_load[row];

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
        .filter_map(|(index, node)| node.fix_x.then_some(index))
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(&global_stiffness, &force_vector, &constrained);
    let reduced_displacements = solve_spd_system(&reduced_stiffness, &reduced_force)?;

    let mut displacements = vec![0.0; dof_count];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }

    Ok(build_thermal_bar_1d_result(request, displacements))
}

pub fn solve_heat_bar_1d(request: &SolveHeatBar1dRequest) -> Result<SolveHeatBar1dResult, String> {
    validate_heat_bar_1d_request(request)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut heat_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        heat_vector[index] = node.heat_load;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        let conductance = element.conductivity * element.area / length;
        let local = [[conductance, -conductance], [-conductance, conductance]];
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
        .filter_map(|(index, node)| node.fix_temperature.then_some((index, node.temperature)))
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_heat, free) =
        reduce_sparse_system_with_prescribed(&global_stiffness, &heat_vector, &prescribed);
    let reduced_temperatures = solve_spd_system(&reduced_stiffness, &reduced_heat)?;

    let mut temperatures = vec![0.0; dof_count];
    for &(index, value) in &prescribed {
        temperatures[index] = value;
    }
    for (index, &dof) in free.iter().enumerate() {
        temperatures[dof] = reduced_temperatures[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| HeatBar1dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            temperature: temperatures[index],
            heat_load: node.heat_load,
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
            let average_temperature =
                0.5 * (temperatures[element.node_i] + temperatures[element.node_j]);
            let temperature_gradient =
                (temperatures[element.node_j] - temperatures[element.node_i]) / length;
            let heat_flux = -element.conductivity * temperature_gradient;

            HeatBar1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                average_temperature,
                temperature_gradient,
                heat_flux,
            }
        })
        .collect::<Vec<_>>();

    let max_temperature = nodes
        .iter()
        .map(|node| node.temperature.abs())
        .fold(0.0_f64, f64::max);
    let max_heat_flux = elements
        .iter()
        .map(|element| element.heat_flux.abs())
        .fold(0.0_f64, f64::max);

    Ok(SolveHeatBar1dResult {
        input: request.clone(),
        nodes,
        elements,
        max_temperature,
        max_heat_flux,
    })
}

pub fn solve_electrostatic_bar_1d(
    request: &SolveElectrostaticBar1dRequest,
) -> Result<SolveElectrostaticBar1dResult, String> {
    validate_electrostatic_bar_1d_request(request)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut source_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        source_vector[index] = node.charge_density;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        let conductance = element.permittivity * element.area / length;
        let local = [[conductance, -conductance], [-conductance, conductance]];
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
        .filter_map(|(index, node)| node.fix_potential.then_some((index, node.potential)))
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_source, free) =
        reduce_sparse_system_with_prescribed(&global_stiffness, &source_vector, &prescribed);
    let reduced_potentials = solve_spd_system(&reduced_stiffness, &reduced_source)?;

    let mut potentials = vec![0.0; dof_count];
    for &(index, value) in &prescribed {
        potentials[index] = value;
    }
    for (index, &dof) in free.iter().enumerate() {
        potentials[dof] = reduced_potentials[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| ElectrostaticBar1dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            potential: potentials[index],
            charge_density: node.charge_density,
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
            let average_potential = 0.5 * (potentials[element.node_i] + potentials[element.node_j]);
            let potential_gradient =
                (potentials[element.node_j] - potentials[element.node_i]) / length;
            let electric_field = -potential_gradient;
            let electric_flux_density = element.permittivity * electric_field;
            let stored_energy = 0.5
                * element.permittivity
                * electric_field
                * electric_field
                * element.area
                * length;

            ElectrostaticBar1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                average_potential,
                potential_gradient,
                electric_field,
                electric_flux_density,
                stored_energy,
            }
        })
        .collect::<Vec<_>>();

    let max_potential = nodes
        .iter()
        .map(|node| node.potential.abs())
        .fold(0.0_f64, f64::max);
    let max_electric_field = elements
        .iter()
        .map(|element| element.electric_field.abs())
        .fold(0.0_f64, f64::max);
    let max_flux_density = elements
        .iter()
        .map(|element| element.electric_flux_density.abs())
        .fold(0.0_f64, f64::max);
    let total_stored_energy = elements.iter().map(|element| element.stored_energy).sum();

    Ok(SolveElectrostaticBar1dResult {
        input: request.clone(),
        nodes,
        elements,
        max_potential,
        max_electric_field,
        max_flux_density,
        total_stored_energy,
    })
}

fn validate_request(request: &SolveBarRequest) -> Result<(), String> {
    if !(request.length.is_finite() && request.length > 0.0) {
        return Err("length must be a positive finite number".to_string());
    }
    if !(request.area.is_finite() && request.area > 0.0) {
        return Err("area must be a positive finite number".to_string());
    }
    if !(request.youngs_modulus.is_finite() && request.youngs_modulus > 0.0) {
        return Err("youngs_modulus must be a positive finite number".to_string());
    }
    if request.elements == 0 {
        return Err("elements must be a positive integer".to_string());
    }
    if !request.tip_force.is_finite() {
        return Err("tip_force must be a finite number".to_string());
    }
    Ok(())
}

fn validate_thermal_bar_1d_request(request: &SolveThermalBar1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d thermal bar model must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("1d thermal bar model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_x) {
        return Err("1d thermal bar model must include at least one axial support".to_string());
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("1d thermal bar element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("1d thermal bar element must connect two distinct nodes".to_string());
        }
        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("1d thermal bar element area must be positive".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("1d thermal bar element youngs_modulus must be positive".to_string());
        }
        if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
            return Err(
                "1d thermal bar element thermal_expansion must be non-negative".to_string(),
            );
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        if length <= 1.0e-12 {
            return Err("1d thermal bar element length must be positive".to_string());
        }
    }

    Ok(())
}

fn validate_heat_bar_1d_request(request: &SolveHeatBar1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d heat bar model must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("1d heat bar model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_temperature) {
        return Err("1d heat bar model must include at least one temperature support".to_string());
    }

    for node in &request.nodes {
        if !node.x.is_finite() {
            return Err("1d heat bar node x must be finite".to_string());
        }
        if !node.temperature.is_finite() {
            return Err("1d heat bar node temperature must be finite".to_string());
        }
        if !node.heat_load.is_finite() {
            return Err("1d heat bar node heat_load must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("1d heat bar element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("1d heat bar element must connect two distinct nodes".to_string());
        }
        if !element.area.is_finite() || element.area <= 0.0 {
            return Err("1d heat bar element area must be positive".to_string());
        }
        if !element.conductivity.is_finite() || element.conductivity <= 0.0 {
            return Err("1d heat bar element conductivity must be positive".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        if !length.is_finite() || length <= 0.0 {
            return Err("1d heat bar element length must be positive".to_string());
        }
    }

    Ok(())
}

fn validate_electrostatic_bar_1d_request(
    request: &SolveElectrostaticBar1dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d electrostatic bar model must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("1d electrostatic bar model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_potential) {
        return Err(
            "1d electrostatic bar model must include at least one potential support".to_string(),
        );
    }

    for node in &request.nodes {
        if !node.x.is_finite() {
            return Err("1d electrostatic bar node x must be finite".to_string());
        }
        if !node.potential.is_finite() {
            return Err("1d electrostatic bar node potential must be finite".to_string());
        }
        if !node.charge_density.is_finite() {
            return Err("1d electrostatic bar node charge_density must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("1d electrostatic bar element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("1d electrostatic bar element must connect two distinct nodes".to_string());
        }
        if !element.area.is_finite() || element.area <= 0.0 {
            return Err("1d electrostatic bar element area must be positive".to_string());
        }
        if !element.permittivity.is_finite() || element.permittivity <= 0.0 {
            return Err("1d electrostatic bar element permittivity must be positive".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        if !length.is_finite() || length <= 0.0 {
            return Err("1d electrostatic bar element length must be positive".to_string());
        }
    }

    Ok(())
}
