use crate::linear_algebra::{SparseMatrix, add_at, reduce_sparse_system, solve_spd_system};
use kyuubiki_protocol::{
    SolveTransientSpring1dRequest, SolveTransientSpring1dResult, TransientSpring1dElementInput,
    TransientSpring1dElementResult, TransientSpring1dNodeResult, TransientSpring1dStepResult,
};

const BETA: f64 = 0.25;
const GAMMA: f64 = 0.5;

pub fn solve_transient_spring_1d(
    request: &SolveTransientSpring1dRequest,
) -> Result<SolveTransientSpring1dResult, String> {
    validate_request(request)?;

    let count = request.nodes.len();
    let stiffness = assemble_matrix(count, request, |element| element.stiffness);
    let damping = assemble_matrix(count, request, |element| element.damping);
    let mass = request
        .nodes
        .iter()
        .map(|node| node.mass)
        .collect::<Vec<_>>();
    let force = request
        .nodes
        .iter()
        .map(|node| node.load_x)
        .collect::<Vec<_>>();
    let constrained = constrained_dofs(request);
    let mut u = request
        .nodes
        .iter()
        .map(|node| {
            if node.fix_x {
                0.0
            } else {
                node.initial_displacement
            }
        })
        .collect::<Vec<_>>();
    let mut v = request
        .nodes
        .iter()
        .map(|node| {
            if node.fix_x {
                0.0
            } else {
                node.initial_velocity
            }
        })
        .collect::<Vec<_>>();
    let mut a = initial_acceleration(&mass, &damping, &stiffness, &force, &u, &v, &constrained);
    let mut history = Vec::with_capacity(request.steps + 1);

    history.push(step_result(0, 0.0, &u, &v, &mass, &stiffness));
    for step in 1..=request.steps {
        let (next_u, next_v, next_a) = newmark_step(
            request.time_step,
            &mass,
            &damping,
            &stiffness,
            &force,
            &u,
            &v,
            &a,
            &constrained,
        )?;
        u = next_u;
        v = next_v;
        a = next_a;
        history.push(step_result(
            step,
            step as f64 * request.time_step,
            &u,
            &v,
            &mass,
            &stiffness,
        ));
    }

    let nodes = final_nodes(request, &u, &v, &a);
    let elements = final_elements(request, &u, &v);

    Ok(SolveTransientSpring1dResult {
        input: request.clone(),
        final_time: request.steps as f64 * request.time_step,
        max_displacement: history
            .iter()
            .map(|step| step.max_displacement)
            .fold(0.0_f64, f64::max),
        max_velocity: history
            .iter()
            .map(|step| step.max_velocity)
            .fold(0.0_f64, f64::max),
        max_force: elements
            .iter()
            .map(|element| (element.spring_force + element.damping_force).abs())
            .fold(0.0_f64, f64::max),
        nodes,
        elements,
        history,
    })
}

fn newmark_step(
    dt: f64,
    mass: &[f64],
    damping: &[Vec<f64>],
    stiffness: &[Vec<f64>],
    force: &[f64],
    u: &[f64],
    v: &[f64],
    a: &[f64],
    constrained: &[usize],
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>), String> {
    let count = mass.len();
    let a0 = 1.0 / (BETA * dt * dt);
    let a1 = GAMMA / (BETA * dt);
    let a2 = 1.0 / (BETA * dt);
    let a3 = 1.0 / (2.0 * BETA) - 1.0;
    let a4 = GAMMA / BETA - 1.0;
    let a5 = dt * (GAMMA / (2.0 * BETA) - 1.0);
    let mut effective = SparseMatrix::new(count);
    let mut rhs = vec![0.0; count];

    for row in 0..count {
        rhs[row] = force[row] + mass[row] * (a0 * u[row] + a2 * v[row] + a3 * a[row]);
        for column in 0..count {
            rhs[row] += damping[row][column] * (a1 * u[column] + a4 * v[column] + a5 * a[column]);
            add_at(
                &mut effective,
                row,
                column,
                stiffness[row][column] + a1 * damping[row][column],
            );
        }
        add_at(&mut effective, row, row, a0 * mass[row]);
    }

    let (reduced_system, reduced_rhs, free) = reduce_sparse_system(&effective, &rhs, constrained);
    let reduced_u = solve_spd_system(&reduced_system, &reduced_rhs)?;
    let mut next_u = vec![0.0; count];
    for (index, &dof) in free.iter().enumerate() {
        next_u[dof] = reduced_u[index];
    }
    let next_a = (0..count)
        .map(|index| a0 * (next_u[index] - u[index]) - a2 * v[index] - a3 * a[index])
        .collect::<Vec<_>>();
    let next_v = (0..count)
        .map(|index| v[index] + dt * ((1.0 - GAMMA) * a[index] + GAMMA * next_a[index]))
        .collect::<Vec<_>>();
    Ok((next_u, next_v, next_a))
}

fn initial_acceleration(
    mass: &[f64],
    damping: &[Vec<f64>],
    stiffness: &[Vec<f64>],
    force: &[f64],
    u: &[f64],
    v: &[f64],
    constrained: &[usize],
) -> Vec<f64> {
    let mut is_constrained = vec![false; mass.len()];
    for &dof in constrained {
        is_constrained[dof] = true;
    }
    (0..mass.len())
        .map(|row| {
            if is_constrained[row] {
                0.0
            } else {
                let ku = dot(&stiffness[row], u);
                let cv = dot(&damping[row], v);
                (force[row] - ku - cv) / mass[row]
            }
        })
        .collect()
}

fn assemble_matrix(
    count: usize,
    request: &SolveTransientSpring1dRequest,
    value: impl Fn(&TransientSpring1dElementInput) -> f64,
) -> Vec<Vec<f64>> {
    let mut matrix = vec![vec![0.0; count]; count];
    for element in &request.elements {
        let k = value(element);
        matrix[element.node_i][element.node_i] += k;
        matrix[element.node_i][element.node_j] -= k;
        matrix[element.node_j][element.node_i] -= k;
        matrix[element.node_j][element.node_j] += k;
    }
    matrix
}

fn final_nodes(
    request: &SolveTransientSpring1dRequest,
    u: &[f64],
    v: &[f64],
    a: &[f64],
) -> Vec<TransientSpring1dNodeResult> {
    request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| TransientSpring1dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            ux: u[index],
            vx: v[index],
            ax: a[index],
        })
        .collect()
}

fn final_elements(
    request: &SolveTransientSpring1dRequest,
    u: &[f64],
    v: &[f64],
) -> Vec<TransientSpring1dElementResult> {
    request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let extension = u[element.node_j] - u[element.node_i];
            let relative_velocity = v[element.node_j] - v[element.node_i];
            TransientSpring1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                extension,
                relative_velocity,
                spring_force: element.stiffness * extension,
                damping_force: element.damping * relative_velocity,
            }
        })
        .collect()
}

fn step_result(
    step: usize,
    time: f64,
    u: &[f64],
    v: &[f64],
    mass: &[f64],
    stiffness: &[Vec<f64>],
) -> TransientSpring1dStepResult {
    TransientSpring1dStepResult {
        step,
        time,
        max_displacement: u.iter().map(|value| value.abs()).fold(0.0, f64::max),
        max_velocity: v.iter().map(|value| value.abs()).fold(0.0, f64::max),
        kinetic_energy: 0.5
            * v.iter()
                .zip(mass.iter())
                .map(|(velocity, mass)| mass * velocity * velocity)
                .sum::<f64>(),
        strain_energy: 0.5 * dot(u, &multiply_matrix_vector(stiffness, u)),
        displacements: u.to_vec(),
        velocities: v.to_vec(),
    }
}

fn constrained_dofs(request: &SolveTransientSpring1dRequest) -> Vec<usize> {
    request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.fix_x.then_some(index))
        .collect()
}

fn validate_request(request: &SolveTransientSpring1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("transient spring 1d must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("transient spring 1d must define at least one element".to_string());
    }
    if request.time_step <= 0.0 || !request.time_step.is_finite() || request.steps == 0 {
        return Err("transient spring 1d requires positive time_step and steps".to_string());
    }
    for node in &request.nodes {
        if !node.x.is_finite()
            || !node.load_x.is_finite()
            || !node.mass.is_finite()
            || !node.initial_displacement.is_finite()
            || !node.initial_velocity.is_finite()
            || node.mass <= 0.0
        {
            return Err(format!(
                "transient spring node {} must have finite coordinates, load, initial state, and positive mass",
                node.id
            ));
        }
    }
    for element in &request.elements {
        validate_element(request, element)?;
    }
    Ok(())
}

fn validate_element(
    request: &SolveTransientSpring1dRequest,
    element: &TransientSpring1dElementInput,
) -> Result<(), String> {
    for index in [element.node_i, element.node_j] {
        if index >= request.nodes.len() {
            return Err(format!(
                "transient spring element {} references missing node {}",
                element.id, index
            ));
        }
    }
    if element.node_i == element.node_j
        || !element.stiffness.is_finite()
        || element.stiffness <= 0.0
        || !element.damping.is_finite()
        || element.damping < 0.0
    {
        return Err(format!(
            "transient spring element {} must have valid connectivity, stiffness, and damping",
            element.id
        ));
    }
    Ok(())
}

fn multiply_matrix_vector(matrix: &[Vec<f64>], vector: &[f64]) -> Vec<f64> {
    matrix.iter().map(|row| dot(row, vector)).collect()
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}
