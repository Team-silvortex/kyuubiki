use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system_with_prescribed, solve_spd_system,
};
use kyuubiki_protocol::{
    HeatBar1dElementResult, HeatBar1dNodeResult, SolveTransientHeatBar1dRequest,
    SolveTransientHeatBar1dResult, TransientHeatBar1dElementInput, TransientHeatBar1dStepResult,
};

pub fn solve_transient_heat_bar_1d(
    request: &SolveTransientHeatBar1dRequest,
) -> Result<SolveTransientHeatBar1dResult, String> {
    validate_request(request)?;

    let node_count = request.nodes.len();
    let conductance = assemble_conductance(request);
    let capacity = lumped_capacity(request);
    let heat_load = request
        .nodes
        .iter()
        .map(|node| node.heat_load)
        .collect::<Vec<_>>();
    let prescribed = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.fix_temperature.then_some((index, node.temperature)))
        .collect::<Vec<_>>();
    let mut temperatures = request
        .nodes
        .iter()
        .map(|node| node.temperature)
        .collect::<Vec<_>>();
    let mut history = Vec::with_capacity(request.steps + 1);

    history.push(step_result(0, 0.0, &temperatures, &capacity));
    for step in 1..=request.steps {
        let mut system = SparseMatrix::new(node_count);
        let mut rhs = vec![0.0; node_count];

        for row in 0..node_count {
            add_at(&mut system, row, row, capacity[row] / request.time_step);
            rhs[row] = capacity[row] * temperatures[row] / request.time_step + heat_load[row];
        }
        for row in 0..node_count {
            for &(column, value) in &conductance[row] {
                add_at(&mut system, row, column, value);
            }
        }

        let (reduced_system, reduced_rhs, free) =
            reduce_sparse_system_with_prescribed(&system, &rhs, &prescribed);
        let solution = solve_spd_system(&reduced_system, &reduced_rhs)?;
        for (index, &dof) in free.iter().enumerate() {
            temperatures[dof] = solution[index];
        }
        for &(dof, value) in &prescribed {
            temperatures[dof] = value;
        }

        history.push(step_result(
            step,
            step as f64 * request.time_step,
            &temperatures,
            &capacity,
        ));
    }

    let nodes = final_nodes(request, &temperatures);
    let elements = final_elements(request, &temperatures);
    let max_heat_flux = elements
        .iter()
        .map(|element| element.heat_flux.abs())
        .fold(0.0_f64, f64::max);
    let total_thermal_energy = thermal_energy(&temperatures, &capacity);

    Ok(SolveTransientHeatBar1dResult {
        input: request.clone(),
        max_temperature: temperatures
            .iter()
            .map(|value| value.abs())
            .fold(0.0, f64::max),
        max_heat_flux,
        final_time: request.steps as f64 * request.time_step,
        total_thermal_energy,
        nodes,
        elements,
        history,
    })
}

fn assemble_conductance(request: &SolveTransientHeatBar1dRequest) -> Vec<Vec<(usize, f64)>> {
    let mut rows = vec![Vec::<(usize, f64)>::new(); request.nodes.len()];
    for element in &request.elements {
        let length = element_length(request, element);
        let value = element.conductivity * element.area / length;
        add_dense_entry(&mut rows, element.node_i, element.node_i, value);
        add_dense_entry(&mut rows, element.node_i, element.node_j, -value);
        add_dense_entry(&mut rows, element.node_j, element.node_i, -value);
        add_dense_entry(&mut rows, element.node_j, element.node_j, value);
    }
    rows
}

fn lumped_capacity(request: &SolveTransientHeatBar1dRequest) -> Vec<f64> {
    let mut capacity = vec![0.0; request.nodes.len()];
    for element in &request.elements {
        let length = element_length(request, element);
        let mass_heat_capacity = element.density * element.specific_heat * element.area * length;
        capacity[element.node_i] += 0.5 * mass_heat_capacity;
        capacity[element.node_j] += 0.5 * mass_heat_capacity;
    }
    capacity
}

fn final_nodes(
    request: &SolveTransientHeatBar1dRequest,
    temperatures: &[f64],
) -> Vec<HeatBar1dNodeResult> {
    request
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
        .collect()
}

fn final_elements(
    request: &SolveTransientHeatBar1dRequest,
    temperatures: &[f64],
) -> Vec<HeatBar1dElementResult> {
    request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let length = element_length(request, element);
            let ti = temperatures[element.node_i];
            let tj = temperatures[element.node_j];
            let gradient = (tj - ti) / length;
            HeatBar1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                average_temperature: 0.5 * (ti + tj),
                temperature_gradient: gradient,
                heat_flux: -element.conductivity * gradient,
            }
        })
        .collect()
}

fn step_result(
    step: usize,
    time: f64,
    temperatures: &[f64],
    capacity: &[f64],
) -> TransientHeatBar1dStepResult {
    TransientHeatBar1dStepResult {
        step,
        time,
        max_temperature: temperatures
            .iter()
            .map(|value| value.abs())
            .fold(0.0, f64::max),
        total_thermal_energy: thermal_energy(temperatures, capacity),
        nodal_temperatures: temperatures.to_vec(),
    }
}

fn thermal_energy(temperatures: &[f64], capacity: &[f64]) -> f64 {
    temperatures
        .iter()
        .zip(capacity.iter())
        .map(|(temperature, heat_capacity)| heat_capacity * temperature)
        .sum()
}

fn add_dense_entry(rows: &mut [Vec<(usize, f64)>], row: usize, column: usize, value: f64) {
    if let Some(entry) = rows[row]
        .iter_mut()
        .find(|(entry_column, _)| *entry_column == column)
    {
        entry.1 += value;
    } else {
        rows[row].push((column, value));
    }
}

fn element_length(
    request: &SolveTransientHeatBar1dRequest,
    element: &TransientHeatBar1dElementInput,
) -> f64 {
    (request.nodes[element.node_j].x - request.nodes[element.node_i].x).abs()
}

fn validate_request(request: &SolveTransientHeatBar1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("transient heat bar must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("transient heat bar must define at least one element".to_string());
    }
    if request.time_step <= 0.0 {
        return Err("transient heat bar time_step must be positive".to_string());
    }
    if request.steps == 0 {
        return Err("transient heat bar steps must be positive".to_string());
    }
    for element in &request.elements {
        validate_element(request, element)?;
    }
    if lumped_capacity(request).iter().any(|value| *value <= 0.0) {
        return Err(
            "transient heat bar every node must receive positive heat capacity".to_string(),
        );
    }
    Ok(())
}

fn validate_element(
    request: &SolveTransientHeatBar1dRequest,
    element: &TransientHeatBar1dElementInput,
) -> Result<(), String> {
    for index in [element.node_i, element.node_j] {
        if index >= request.nodes.len() {
            return Err(format!(
                "transient heat bar element {} references missing node {}",
                element.id, index
            ));
        }
    }
    if element.node_i == element.node_j || element_length(request, element) <= 0.0 {
        return Err(format!(
            "transient heat bar element {} must have non-zero length",
            element.id
        ));
    }
    if element.area <= 0.0 || element.conductivity <= 0.0 {
        return Err(format!(
            "transient heat bar element {} must have positive area and conductivity",
            element.id
        ));
    }
    if element.density <= 0.0 || element.specific_heat <= 0.0 {
        return Err(format!(
            "transient heat bar element {} must have positive density and specific_heat",
            element.id
        ));
    }
    Ok(())
}
