use super::asymmetric_trajectory::pinned_elastica_load;
use super::*;

const STAR_INERTIAS: [f64; 4] = [
    COLUMN_INERTIA,
    COLUMN_INERTIA * 1.005,
    COLUMN_INERTIA * 0.997,
    COLUMN_INERTIA * 1.008,
];
const STAR_EDGES: [(usize, usize, f64); 3] = [(1, 0, 20.0), (1, 2, 15.0), (1, 3, 25.0)];
const STAR_TRAJECTORY_STEPS: usize = 64;

#[test]
fn interacting_star_topology_tracks_the_external_elastica_graph_trajectory() {
    let (reference_load, reference_direction) = star_linear_reference();
    let mut request =
        coupled_column_network_request(&STAR_INERTIAS, &reference_direction, &STAR_EDGES);
    request.load_steps = Some(STAR_TRAJECTORY_STEPS);
    request.arc_length_radius = Some(2.0e-3);
    request.branch_switch = Frame2dBranchSwitchSelection::Disabled;
    request.branch_switch_amplitude = None;
    request.branch_switch_mode_count = None;
    request.branch_continuation_steps = None;
    request.continuation_state = Some(external_star_continuation_state(
        reference_direction,
        1.0e-3,
        1.0e-4,
    ));

    let buckling = solve_buckling_frame_2d(&request.buckling).expect("star buckling should solve");
    assert_relative(buckling.minimum_load_factor, reference_load, 5.0e-3);
    assert!(
        vector_alignment(
            &midpoint_vector(&buckling.modes[0].shape),
            &reference_direction
        ) > 0.98
    );

    let result = solve_frame_2d_p_delta(&request).expect("star postcritical path should solve");
    assert!(result.converged);
    assert_eq!(result.steps.len(), STAR_TRAJECTORY_STEPS);
    let trajectory = &result.steps;
    let seed_radius = vector_norm(&midpoint_vector(&trajectory[0].displacements));
    let mut previous_radius = seed_radius;
    let mut maximum_load_error = 0.0_f64;
    let mut maximum_external_residual = 0.0_f64;
    let mut minimum_neighbor_overlap = 1.0_f64;
    let mut minimum_radius_increment = f64::INFINITY;
    let mut radius_reversal_count = 0;
    let mut previous_direction = reference_direction;
    let mut terminal_direction = reference_direction;
    for (index, step) in trajectory.iter().enumerate() {
        assert!(step.converged);
        assert!(step.residual_norm < 1.0e-7);
        assert!(
            step.arc_length_constraint_error
                .is_some_and(|error| error < 1.0e-7)
        );
        let amplitudes = midpoint_vector(&step.displacements);
        let radius = vector_norm(&amplitudes);
        if index > 0 {
            let increment = radius - previous_radius;
            minimum_radius_increment = minimum_radius_increment.min(increment);
            radius_reversal_count += usize::from(increment < 0.0);
        }
        previous_radius = radius;

        let external = external_star_equilibrium(&amplitudes);
        let load_error = (step.load_factor - external.load).abs() / external.load.abs().max(1.0);
        let direction = normalize(amplitudes);
        maximum_load_error = maximum_load_error.max(load_error);
        maximum_external_residual = maximum_external_residual.max(external.normalized_residual);
        minimum_neighbor_overlap =
            minimum_neighbor_overlap.min(vector_alignment(&direction, &previous_direction));
        previous_direction = direction;
        terminal_direction = direction;
    }

    let minimum_terminal_participation = terminal_direction
        .iter()
        .map(|value| value.abs())
        .fold(f64::INFINITY, f64::min);
    println!(
        "interacting star trajectory: points={}, seed_radius={seed_radius:.8e}, terminal_radius={previous_radius:.8e}, max_load_error={maximum_load_error:.8e}, max_external_residual={maximum_external_residual:.8e}, min_neighbor_overlap={minimum_neighbor_overlap:.8e}, min_terminal_participation={minimum_terminal_participation:.8e}, radius_reversals={radius_reversal_count}, min_radius_increment={minimum_radius_increment:.8e}",
        trajectory.len()
    );
    assert!(previous_radius > seed_radius + 5.0e-3);
    assert!(maximum_load_error < 0.04);
    assert!(maximum_external_residual < 0.04);
    assert!(minimum_neighbor_overlap > 0.999);
    assert!(minimum_terminal_participation > 0.05);
}

fn external_star_continuation_state(
    direction: [f64; 4],
    radius: f64,
    radius_increment: f64,
) -> Frame2dPDeltaContinuationState {
    let mut displacements = vec![0.0; STAR_INERTIAS.len() * NODES_PER_COLUMN * 3];
    let mut displacement_increment = displacements.clone();
    for (column, weight) in direction.into_iter().enumerate() {
        for node in 0..NODES_PER_COLUMN {
            let ratio = node as f64 / ELEMENT_COUNT as f64;
            let phase = std::f64::consts::PI * ratio;
            let offset = (column * NODES_PER_COLUMN + node) * 3;
            displacements[offset] = radius * weight * phase.sin();
            displacements[offset + 2] =
                -radius * weight * std::f64::consts::PI * phase.cos() / LENGTH;
            displacement_increment[offset] = radius_increment * weight * phase.sin();
            displacement_increment[offset + 2] =
                -radius_increment * weight * std::f64::consts::PI * phase.cos() / LENGTH;
        }
    }
    let load = external_star_equilibrium(&direction.map(|value| radius * value)).load;
    let next_load =
        external_star_equilibrium(&direction.map(|value| (radius + radius_increment) * value)).load;
    Frame2dPDeltaContinuationState {
        displacements,
        load_factor: load,
        displacement_increment,
        load_factor_increment: next_load - load,
    }
}

#[derive(Clone, Copy)]
struct ExternalGraphEquilibrium {
    load: f64,
    normalized_residual: f64,
}

fn external_star_equilibrium(amplitudes: &[f64; 4]) -> ExternalGraphEquilibrium {
    let mut restoring = [0.0; 4];
    for (index, amplitude) in amplitudes.iter().enumerate() {
        restoring[index] = pinned_elastica_load(STAR_INERTIAS[index], amplitude.abs()) * amplitude;
    }
    for &(left, right, stiffness) in &STAR_EDGES {
        let coupling = reduced_coupling(stiffness);
        let force = coupling * (amplitudes[left] - amplitudes[right]);
        restoring[left] += force;
        restoring[right] -= force;
    }
    let squared_radius = amplitudes.iter().map(|value| value * value).sum::<f64>();
    let load = amplitudes
        .iter()
        .zip(restoring)
        .map(|(amplitude, force)| amplitude * force)
        .sum::<f64>()
        / squared_radius;
    let residual = amplitudes
        .iter()
        .zip(restoring)
        .map(|(amplitude, force)| (force - load * amplitude).powi(2))
        .sum::<f64>()
        .sqrt();
    ExternalGraphEquilibrium {
        load,
        normalized_residual: residual / (load.abs() * squared_radius.sqrt()).max(f64::EPSILON),
    }
}

fn star_linear_reference() -> (f64, [f64; 4]) {
    let mut matrix = [[0.0; 4]; 4];
    for index in 0..4 {
        matrix[index][index] =
            std::f64::consts::PI.powi(2) * YOUNGS_MODULUS * STAR_INERTIAS[index] / LENGTH.powi(2);
    }
    for &(left, right, stiffness) in &STAR_EDGES {
        let coupling = reduced_coupling(stiffness);
        matrix[left][left] += coupling;
        matrix[right][right] += coupling;
        matrix[left][right] -= coupling;
        matrix[right][left] -= coupling;
    }
    smallest_symmetric_eigenpair(matrix)
}

fn reduced_coupling(stiffness: f64) -> f64 {
    2.0 * stiffness * LENGTH / std::f64::consts::PI.powi(2)
}

fn smallest_symmetric_eigenpair(mut matrix: [[f64; 4]; 4]) -> (f64, [f64; 4]) {
    let mut vectors = [[0.0; 4]; 4];
    for (index, row) in vectors.iter_mut().enumerate() {
        row[index] = 1.0;
    }
    for _ in 0..64 {
        let mut pivot = (0, 1);
        let mut maximum = 0.0_f64;
        for row in 0..4 {
            for column in row + 1..4 {
                if matrix[row][column].abs() > maximum {
                    maximum = matrix[row][column].abs();
                    pivot = (row, column);
                }
            }
        }
        if maximum < 1.0e-12 {
            break;
        }
        jacobi_rotate(&mut matrix, &mut vectors, pivot.0, pivot.1);
    }
    let index = (0..4)
        .min_by(|left, right| matrix[*left][*left].total_cmp(&matrix[*right][*right]))
        .unwrap();
    let direction = normalize(vectors.map(|row| row[index]));
    (matrix[index][index], direction)
}

fn jacobi_rotate(
    matrix: &mut [[f64; 4]; 4],
    vectors: &mut [[f64; 4]; 4],
    row: usize,
    column: usize,
) {
    let angle = 0.5 * (2.0 * matrix[row][column]).atan2(matrix[column][column] - matrix[row][row]);
    let cosine = angle.cos();
    let sine = angle.sin();
    for index in 0..4 {
        let left = matrix[index][row];
        let right = matrix[index][column];
        matrix[index][row] = cosine * left - sine * right;
        matrix[index][column] = sine * left + cosine * right;
    }
    for index in 0..4 {
        let upper = matrix[row][index];
        let lower = matrix[column][index];
        matrix[row][index] = cosine * upper - sine * lower;
        matrix[column][index] = sine * upper + cosine * lower;
    }
    for vector in vectors {
        let left = vector[row];
        let right = vector[column];
        vector[row] = cosine * left - sine * right;
        vector[column] = sine * left + cosine * right;
    }
}

fn midpoint_vector(displacements: &[f64]) -> [f64; 4] {
    std::array::from_fn(|column| displacements[midpoint_node(column) * 3])
}

fn normalize(values: [f64; 4]) -> [f64; 4] {
    let norm = vector_norm(&values);
    values.map(|value| value / norm)
}

fn vector_norm(values: &[f64; 4]) -> f64 {
    values.iter().map(|value| value * value).sum::<f64>().sqrt()
}

fn vector_alignment(left: &[f64; 4], right: &[f64; 4]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum::<f64>()
        .abs()
        / (vector_norm(left) * vector_norm(right))
}
