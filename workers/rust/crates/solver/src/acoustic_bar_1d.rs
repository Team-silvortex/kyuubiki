use crate::acoustic_bar_1d_validation::validate_request;
use crate::chain_tridiagonal::{is_indexed_chain, solve_with_prescribed};
use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system_with_prescribed, solve_spd_system,
};
use kyuubiki_protocol::{
    AcousticBar1dElementInput, AcousticBar1dElementResult, AcousticBar1dNodeResult,
    SolveAcousticBar1dRequest, SolveAcousticBar1dResult,
};

const REFERENCE_PRESSURE_PA: f64 = 20.0e-6;

pub fn solve_acoustic_bar_1d(
    request: &SolveAcousticBar1dRequest,
) -> Result<SolveAcousticBar1dResult, String> {
    validate_request(request)?;
    let omega = 2.0 * std::f64::consts::PI * request.frequency_hz;
    let pressures = solve_pressures(request, omega)?;

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| AcousticBar1dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            pressure: pressures[index],
            sound_pressure_level_db: sound_pressure_level_db(pressures[index]),
            volume_velocity_source: node.volume_velocity_source,
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| element_result(request, element, index, &pressures, omega))
        .collect::<Result<Vec<_>, String>>()?;

    let max_pressure = nodes
        .iter()
        .map(|node| node.pressure.abs())
        .fold(0.0_f64, f64::max);
    let max_sound_pressure_level_db = nodes
        .iter()
        .map(|node| node.sound_pressure_level_db)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_particle_velocity = elements
        .iter()
        .map(|element| element.particle_velocity.abs())
        .fold(0.0_f64, f64::max);
    let max_acoustic_intensity = elements
        .iter()
        .map(|element| element.acoustic_intensity.abs())
        .fold(0.0_f64, f64::max);
    let total_damping_loss = elements.iter().map(|element| element.damping_loss).sum();

    Ok(SolveAcousticBar1dResult {
        input: request.clone(),
        nodes,
        elements,
        frequency_hz: request.frequency_hz,
        angular_frequency: omega,
        max_pressure,
        max_sound_pressure_level_db,
        max_particle_velocity,
        max_acoustic_intensity,
        total_damping_loss,
    })
}

fn solve_pressures(request: &SolveAcousticBar1dRequest, omega: f64) -> Result<Vec<f64>, String> {
    let dof_count = request.nodes.len();
    let rhs = request
        .nodes
        .iter()
        .map(|node| node.volume_velocity_source * omega)
        .collect::<Vec<_>>();
    let prescribed = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.fix_pressure.then_some((index, node.pressure)))
        .collect::<Vec<_>>();

    if is_indexed_chain(
        dof_count,
        request
            .elements
            .iter()
            .map(|element| (element.node_i, element.node_j)),
    ) {
        let mut diagonal = vec![0.0; dof_count];
        let mut lower = vec![0.0; dof_count - 1];
        let mut upper = vec![0.0; dof_count - 1];
        for element in &request.elements {
            let local = acoustic_local_matrix(request, element, omega)?;
            let map = [element.node_i, element.node_j];
            for row in 0..2 {
                for column in 0..2 {
                    let row_index = map[row];
                    let column_index = map[column];
                    if row_index == column_index {
                        diagonal[row_index] += local[row][column];
                    } else {
                        let left = row_index.min(column_index);
                        if row_index == left {
                            upper[left] += local[row][column];
                        } else {
                            lower[left] += local[row][column];
                        }
                    }
                }
            }
        }
        return solve_with_prescribed(&diagonal, &lower, &upper, &rhs, &prescribed);
    }

    let mut system = SparseMatrix::new(dof_count);
    for element in &request.elements {
        let local = acoustic_local_matrix(request, element, omega)?;
        let map = [element.node_i, element.node_j];
        for row in 0..2 {
            for column in 0..2 {
                add_at(&mut system, map[row], map[column], local[row][column]);
            }
        }
    }
    let (reduced, reduced_rhs, free) =
        reduce_sparse_system_with_prescribed(&system, &rhs, &prescribed);
    let solved = solve_spd_system(&reduced, &reduced_rhs)?;
    let mut pressures = vec![0.0; dof_count];
    for &(index, value) in &prescribed {
        pressures[index] = value;
    }
    for (index, &dof) in free.iter().enumerate() {
        pressures[dof] = solved[index];
    }
    Ok(pressures)
}

fn acoustic_local_matrix(
    request: &SolveAcousticBar1dRequest,
    element: &AcousticBar1dElementInput,
    omega: f64,
) -> Result<[[f64; 2]; 2], String> {
    let length = element_length(request, element)?;
    let damping = 1.0 + element.damping_ratio.max(0.0);
    let stiffness = element.area / (element.density * length);
    let mass = element.area * length / (6.0 * element.bulk_modulus);
    let dynamic = omega * omega * mass * damping;
    Ok([
        [stiffness + 2.0 * dynamic, -stiffness + dynamic],
        [-stiffness + dynamic, stiffness + 2.0 * dynamic],
    ])
}

fn element_result(
    request: &SolveAcousticBar1dRequest,
    element: &AcousticBar1dElementInput,
    index: usize,
    pressures: &[f64],
    omega: f64,
) -> Result<AcousticBar1dElementResult, String> {
    let length = element_length(request, element)?;
    let c = speed_of_sound(element)?;
    let pressure_gradient = (pressures[element.node_j] - pressures[element.node_i]) / length;
    let particle_velocity = pressure_gradient.abs() / (element.density * omega);
    let average_pressure =
        (pressures[element.node_i].abs() + pressures[element.node_j].abs()) / 2.0;
    let acoustic_intensity = average_pressure * particle_velocity / 2.0;
    let damping_loss = acoustic_intensity * element.damping_ratio * element.area * length;
    Ok(AcousticBar1dElementResult {
        index,
        id: element.id.clone(),
        node_i: element.node_i,
        node_j: element.node_j,
        length,
        area: element.area,
        density: element.density,
        bulk_modulus: element.bulk_modulus,
        speed_of_sound: c,
        wave_number: omega / c,
        pressure_gradient,
        particle_velocity,
        acoustic_intensity,
        damping_loss,
    })
}

fn element_length(
    request: &SolveAcousticBar1dRequest,
    element: &AcousticBar1dElementInput,
) -> Result<f64, String> {
    let xi = request.nodes[element.node_i].x;
    let xj = request.nodes[element.node_j].x;
    Ok((xj - xi).abs())
}

fn speed_of_sound(element: &AcousticBar1dElementInput) -> Result<f64, String> {
    Ok((element.bulk_modulus / element.density).sqrt())
}

fn sound_pressure_level_db(pressure: f64) -> f64 {
    if pressure.abs() <= 0.0 {
        0.0
    } else {
        20.0 * (pressure.abs() / REFERENCE_PRESSURE_PA).log10()
    }
}
