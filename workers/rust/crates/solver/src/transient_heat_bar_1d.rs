use crate::linear_algebra::{
    PreparedSpdSolver, SparseMatrix, add_at, reduce_sparse_system_with_prescribed,
};
use crate::transient_heat_bar_1d_validation::validate_request;
use crate::transient_history::TransientHistoryPlan;
use kyuubiki_protocol::{
    HeatBar1dElementResult, HeatBar1dNodeResult, SolveTransientHeatBar1dRequest,
    SolveTransientHeatBar1dResult, TransientHeatBar1dElementInput, TransientHeatBar1dStepResult,
};

pub fn solve_transient_heat_bar_1d(
    request: &SolveTransientHeatBar1dRequest,
) -> Result<SolveTransientHeatBar1dResult, String> {
    validate_request(request)?;
    let history_plan = TransientHistoryPlan::new(
        "transient heat bar",
        request.nodes.len(),
        request.steps,
        request.history_stride,
        1,
    )?;

    let capacity = lumped_capacity(request)?;
    let capacity_rate = capacity
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let rate = value / request.time_step;
            if rate.is_finite() {
                Ok(rate)
            } else {
                Err(format!(
                    "transient heat bar node {index} produces non-finite capacity rate"
                ))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut system = assemble_conductance(request)?;
    for (index, value) in capacity_rate.iter().enumerate() {
        add_at(&mut system, index, index, *value);
    }

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
    let (reduced_system, reduced_base_rhs, free) =
        reduce_sparse_system_with_prescribed(&system, &heat_load, &prescribed);
    let solver = PreparedSpdSolver::factor(reduced_system)
        .map_err(|error| format!("transient heat bar effective system failed: {error}"))?;

    let mut temperatures = request
        .nodes
        .iter()
        .map(|node| node.temperature)
        .collect::<Vec<_>>();
    let mut history = Vec::new();
    history
        .try_reserve_exact(history_plan.frame_count())
        .map_err(|_| "transient heat bar history allocation is too large".to_string())?;
    history.push(step_result(0, 0.0, &temperatures, &capacity)?);
    for step in 1..=request.steps {
        let mut rhs = reduced_base_rhs.clone();
        for (reduced_index, &dof) in free.iter().enumerate() {
            rhs[reduced_index] += capacity_rate[dof] * temperatures[dof];
        }
        if rhs.iter().any(|value| !value.is_finite()) {
            return Err("transient heat bar right-hand side became non-finite".to_string());
        }
        let solution = solver
            .solve(&rhs)
            .map_err(|error| format!("transient heat bar implicit solve failed: {error}"))?;
        for (index, &dof) in free.iter().enumerate() {
            temperatures[dof] = solution[index];
        }
        for &(dof, value) in &prescribed {
            temperatures[dof] = value;
        }

        if history_plan.captures(step, request.steps) {
            history.push(step_result(
                step,
                checked_time(step, request.time_step)?,
                &temperatures,
                &capacity,
            )?);
        }
    }

    let nodes = final_nodes(request, &temperatures);
    let elements = final_elements(request, &temperatures)?;
    let max_heat_flux = elements
        .iter()
        .map(|element| element.heat_flux.abs())
        .fold(0.0_f64, f64::max);
    let total_thermal_energy = thermal_energy(&temperatures, &capacity)?;

    Ok(SolveTransientHeatBar1dResult {
        input: request.clone(),
        max_temperature: temperatures
            .iter()
            .map(|value| value.abs())
            .fold(0.0, f64::max),
        max_heat_flux,
        final_time: checked_time(request.steps, request.time_step)?,
        total_thermal_energy,
        nodes,
        elements,
        history,
    })
}

fn assemble_conductance(request: &SolveTransientHeatBar1dRequest) -> Result<SparseMatrix, String> {
    let mut matrix = SparseMatrix::with_uniform_row_capacity(request.nodes.len(), 3);
    for element in &request.elements {
        let length = element_length(request, element);
        let value = element.conductivity * element.area / length;
        if !value.is_finite() {
            return Err(format!(
                "transient heat bar element {} produces non-finite conductance",
                element.id
            ));
        }
        add_two_node_matrix(&mut matrix, element.node_i, element.node_j, value);
    }
    Ok(matrix)
}

fn add_two_node_matrix(matrix: &mut SparseMatrix, node_i: usize, node_j: usize, value: f64) {
    add_at(matrix, node_i, node_i, value);
    add_at(matrix, node_i, node_j, -value);
    add_at(matrix, node_j, node_i, -value);
    add_at(matrix, node_j, node_j, value);
}

fn lumped_capacity(request: &SolveTransientHeatBar1dRequest) -> Result<Vec<f64>, String> {
    let mut capacity = vec![0.0; request.nodes.len()];
    for element in &request.elements {
        let length = element_length(request, element);
        let value = 0.5 * element.density * element.specific_heat * element.area * length;
        if !(value.is_finite() && value > 0.0) {
            return Err(format!(
                "transient heat bar element {} produces invalid lumped capacity",
                element.id
            ));
        }
        capacity[element.node_i] += value;
        capacity[element.node_j] += value;
    }
    if capacity.iter().any(|value| !value.is_finite()) {
        return Err("transient heat bar assembled capacity became non-finite".to_string());
    }
    Ok(capacity)
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
) -> Result<Vec<HeatBar1dElementResult>, String> {
    request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let length = element_length(request, element);
            let ti = temperatures[element.node_i];
            let tj = temperatures[element.node_j];
            let average_temperature = 0.5 * (ti + tj);
            let temperature_gradient = (tj - ti) / length;
            let heat_flux = -element.conductivity * temperature_gradient;
            if [average_temperature, temperature_gradient, heat_flux]
                .iter()
                .any(|value| !value.is_finite())
            {
                return Err(format!(
                    "transient heat bar element {} produced a non-finite result",
                    element.id
                ));
            }
            Ok(HeatBar1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                average_temperature,
                temperature_gradient,
                heat_flux,
            })
        })
        .collect()
}

fn step_result(
    step: usize,
    time: f64,
    temperatures: &[f64],
    capacity: &[f64],
) -> Result<TransientHeatBar1dStepResult, String> {
    Ok(TransientHeatBar1dStepResult {
        step,
        time,
        max_temperature: temperatures
            .iter()
            .map(|value| value.abs())
            .fold(0.0, f64::max),
        total_thermal_energy: thermal_energy(temperatures, capacity)?,
        nodal_temperatures: temperatures.to_vec(),
    })
}

fn thermal_energy(temperatures: &[f64], capacity: &[f64]) -> Result<f64, String> {
    let mut energy = 0.0;
    for (temperature, heat_capacity) in temperatures.iter().zip(capacity) {
        energy += heat_capacity * temperature;
        if !energy.is_finite() {
            return Err("transient heat bar thermal energy became non-finite".to_string());
        }
    }
    Ok(energy)
}

fn element_length(
    request: &SolveTransientHeatBar1dRequest,
    element: &TransientHeatBar1dElementInput,
) -> f64 {
    (request.nodes[element.node_j].x - request.nodes[element.node_i].x).abs()
}

fn checked_time(step: usize, time_step: f64) -> Result<f64, String> {
    let time = step as f64 * time_step;
    if time.is_finite() {
        Ok(time)
    } else {
        Err("transient heat bar simulation time became non-finite".to_string())
    }
}
