use kyuubiki_protocol::{
    SolveThermalBar1dRequest, SolveThermalBar1dResult, ThermalBar1dElementResult,
    ThermalBar1dNodeResult,
};

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
    if request.elements.len() + 1 != request.nodes.len() {
        return false;
    }

    let mut seen_spans = vec![false; request.elements.len()];
    for element in &request.elements {
        let (left, right) = if element.node_i < element.node_j {
            (element.node_i, element.node_j)
        } else {
            (element.node_j, element.node_i)
        };
        if right != left + 1 || left >= seen_spans.len() || seen_spans[left] {
            return false;
        }
        seen_spans[left] = true;
    }

    seen_spans.into_iter().all(|seen| seen)
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

    solve_fixed_zero_tridiagonal(&diagonal, &lower, &upper, &force, request)
}

fn solve_fixed_zero_tridiagonal(
    diagonal: &[f64],
    lower: &[f64],
    upper: &[f64],
    force: &[f64],
    request: &SolveThermalBar1dRequest,
) -> Result<Vec<f64>, String> {
    let mut displacements = vec![0.0; request.nodes.len()];
    let mut cursor = 0;
    while cursor < request.nodes.len() {
        if request.nodes[cursor].fix_x {
            cursor += 1;
            continue;
        }
        let start = cursor;
        while cursor < request.nodes.len() && !request.nodes[cursor].fix_x {
            cursor += 1;
        }
        solve_free_segment(
            start,
            cursor,
            diagonal,
            lower,
            upper,
            force,
            &mut displacements,
        )?;
    }
    Ok(displacements)
}

fn solve_free_segment(
    start: usize,
    end: usize,
    diagonal: &[f64],
    lower: &[f64],
    upper: &[f64],
    force: &[f64],
    displacements: &mut [f64],
) -> Result<(), String> {
    let count = end - start;
    if count == 0 {
        return Ok(());
    }

    let mut a = vec![0.0; count];
    let mut b = vec![0.0; count];
    let mut c = vec![0.0; count];
    let mut d = vec![0.0; count];
    for local in 0..count {
        let global = start + local;
        b[local] = diagonal[global];
        d[local] = force[global];
        if local > 0 {
            a[local] = lower[global - 1];
        }
        if local + 1 < count {
            c[local] = upper[global];
        }
    }

    for index in 1..count {
        let pivot = b[index - 1];
        if pivot.abs() <= 1.0e-18 {
            return Err("1d thermal bar chain solver encountered a zero pivot".to_string());
        }
        let factor = a[index] / pivot;
        b[index] -= factor * c[index - 1];
        d[index] -= factor * d[index - 1];
    }

    if b[count - 1].abs() <= 1.0e-18 {
        return Err("1d thermal bar chain solver encountered a zero pivot".to_string());
    }
    displacements[end - 1] = d[count - 1] / b[count - 1];
    for local in (0..count - 1).rev() {
        let global = start + local;
        displacements[global] = (d[local] - c[local] * displacements[global + 1]) / b[local];
    }
    Ok(())
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
