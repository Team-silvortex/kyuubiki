use crate::dynamic_spring_1d_validation::validate_transient_request;
use crate::linear_algebra::{PreparedSpdSolver, SparseMatrix, add_at, reduce_sparse_system};
use crate::transient_history::TransientHistoryPlan;
use kyuubiki_protocol::{
    SolveTransientSpring1dRequest, SolveTransientSpring1dResult, TransientSpring1dElementInput,
    TransientSpring1dElementResult, TransientSpring1dNodeResult, TransientSpring1dStepResult,
};
use std::borrow::Cow;

const BETA: f64 = 0.25;
const GAMMA: f64 = 0.5;
type NewmarkStateVectors = (Vec<f64>, Vec<f64>, Vec<f64>);

#[derive(Clone, Copy)]
struct NewmarkCoefficients {
    a0: f64,
    a1: f64,
    a2: f64,
    a3: f64,
    a4: f64,
    a5: f64,
}

struct NewmarkSystem<'a> {
    time_step: f64,
    coefficients: NewmarkCoefficients,
    mass: &'a [f64],
    force: &'a [f64],
    elements: &'a [TransientSpring1dElementInput],
    free: &'a [usize],
    solver: &'a PreparedSpdSolver,
}

struct NewmarkState<'a> {
    displacement: &'a [f64],
    velocity: &'a [f64],
    acceleration: &'a [f64],
}

pub fn solve_transient_spring_1d(
    request: &SolveTransientSpring1dRequest,
) -> Result<SolveTransientSpring1dResult, String> {
    solve_transient_spring_1d_internal(Cow::Borrowed(request))
}

pub fn solve_transient_spring_1d_owned(
    request: SolveTransientSpring1dRequest,
) -> Result<SolveTransientSpring1dResult, String> {
    solve_transient_spring_1d_internal(Cow::Owned(request))
}

fn solve_transient_spring_1d_internal(
    request: Cow<'_, SolveTransientSpring1dRequest>,
) -> Result<SolveTransientSpring1dResult, String> {
    validate_transient_request(request.as_ref())?;
    let history_plan = TransientHistoryPlan::new(
        "transient spring",
        request.nodes.len(),
        request.steps,
        request.history_stride,
        2,
    )?;
    let coefficients = newmark_coefficients(request.time_step)?;
    let count = request.nodes.len();
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
    let constrained = constrained_dofs(request.as_ref());
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
    let mut a = initial_acceleration(request.as_ref(), &u, &v)?;

    let effective = assemble_effective_system(request.as_ref(), coefficients)?;
    let (reduced_effective, _, free) =
        reduce_sparse_system(&effective, &vec![0.0; count], &constrained);
    let solver = PreparedSpdSolver::factor(reduced_effective)
        .map_err(|error| format!("transient spring effective system failed: {error}"))?;
    let system = NewmarkSystem {
        time_step: request.time_step,
        coefficients,
        mass: &mass,
        force: &force,
        elements: &request.elements,
        free: &free,
        solver: &solver,
    };

    let mut history = Vec::new();
    history
        .try_reserve_exact(history_plan.frame_count())
        .map_err(|_| "transient spring history allocation is too large".to_string())?;
    history.push(step_result(0, 0.0, request.as_ref(), &u, &v)?);
    let mut max_displacement = maximum_absolute(&u);
    let mut max_velocity = maximum_absolute(&v);
    for step in 1..=request.steps {
        (u, v, a) = newmark_step(
            &system,
            NewmarkState {
                displacement: &u,
                velocity: &v,
                acceleration: &a,
            },
        )?;
        max_displacement = max_displacement.max(maximum_absolute(&u));
        max_velocity = max_velocity.max(maximum_absolute(&v));
        if history_plan.captures(step, request.steps) {
            history.push(step_result(
                step,
                checked_time(step, request.time_step)?,
                request.as_ref(),
                &u,
                &v,
            )?);
        }
    }

    let nodes = final_nodes(request.as_ref(), &u, &v, &a);
    let elements = final_elements(request.as_ref(), &u, &v)?;
    let final_time = checked_time(request.steps, request.time_step)?;

    Ok(SolveTransientSpring1dResult {
        input: request.into_owned(),
        final_time,
        max_displacement,
        max_velocity,
        max_force: elements
            .iter()
            .map(|element| (element.spring_force + element.damping_force).abs())
            .fold(0.0_f64, f64::max),
        nodes,
        elements,
        history,
    })
}

fn newmark_coefficients(time_step: f64) -> Result<NewmarkCoefficients, String> {
    let coefficients = NewmarkCoefficients {
        a0: 1.0 / (BETA * time_step * time_step),
        a1: GAMMA / (BETA * time_step),
        a2: 1.0 / (BETA * time_step),
        a3: 1.0 / (2.0 * BETA) - 1.0,
        a4: GAMMA / BETA - 1.0,
        a5: time_step * (GAMMA / (2.0 * BETA) - 1.0),
    };
    if [
        coefficients.a0,
        coefficients.a1,
        coefficients.a2,
        coefficients.a3,
        coefficients.a4,
        coefficients.a5,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err("transient spring time_step produces non-finite Newmark coefficients".into());
    }
    Ok(coefficients)
}

fn assemble_effective_system(
    request: &SolveTransientSpring1dRequest,
    coefficients: NewmarkCoefficients,
) -> Result<SparseMatrix, String> {
    let mut effective = SparseMatrix::with_uniform_row_capacity(request.nodes.len(), 3);
    for element in &request.elements {
        let value = element.damping.mul_add(coefficients.a1, element.stiffness);
        if !value.is_finite() {
            return Err(format!(
                "transient spring element {} produces non-finite effective stiffness",
                element.id
            ));
        }
        add_two_node_matrix(&mut effective, element.node_i, element.node_j, value);
    }
    for (index, node) in request.nodes.iter().enumerate() {
        let inertia = coefficients.a0 * node.mass;
        if !inertia.is_finite() {
            return Err(format!(
                "transient spring node {} produces non-finite effective inertia",
                node.id
            ));
        }
        add_at(&mut effective, index, index, inertia);
    }
    Ok(effective)
}

fn add_two_node_matrix(matrix: &mut SparseMatrix, node_i: usize, node_j: usize, value: f64) {
    add_at(matrix, node_i, node_i, value);
    add_at(matrix, node_i, node_j, -value);
    add_at(matrix, node_j, node_i, -value);
    add_at(matrix, node_j, node_j, value);
}

fn newmark_step(
    system: &NewmarkSystem<'_>,
    state: NewmarkState<'_>,
) -> Result<NewmarkStateVectors, String> {
    let c = system.coefficients;
    let count = system.mass.len();
    let mut rhs = vec![0.0; count];
    let mut damping_state = vec![0.0; count];
    for index in 0..count {
        rhs[index] = system.force[index]
            + system.mass[index]
                * (c.a0 * state.displacement[index]
                    + c.a2 * state.velocity[index]
                    + c.a3 * state.acceleration[index]);
        damping_state[index] = c.a1 * state.displacement[index]
            + c.a4 * state.velocity[index]
            + c.a5 * state.acceleration[index];
    }
    for element in system.elements {
        let transmitted =
            element.damping * (damping_state[element.node_j] - damping_state[element.node_i]);
        rhs[element.node_i] -= transmitted;
        rhs[element.node_j] += transmitted;
    }
    if rhs.iter().any(|value| !value.is_finite()) {
        return Err("transient spring Newmark right-hand side became non-finite".to_string());
    }

    let reduced_rhs = system.free.iter().map(|&dof| rhs[dof]).collect::<Vec<_>>();
    let reduced_u = system
        .solver
        .solve(&reduced_rhs)
        .map_err(|error| format!("transient spring Newmark solve failed: {error}"))?;
    let mut next_u = vec![0.0; count];
    for (index, &dof) in system.free.iter().enumerate() {
        next_u[dof] = reduced_u[index];
    }
    let next_a = (0..count)
        .map(|index| {
            c.a0 * (next_u[index] - state.displacement[index])
                - c.a2 * state.velocity[index]
                - c.a3 * state.acceleration[index]
        })
        .collect::<Vec<_>>();
    let next_v = (0..count)
        .map(|index| {
            state.velocity[index]
                + system.time_step
                    * ((1.0 - GAMMA) * state.acceleration[index] + GAMMA * next_a[index])
        })
        .collect::<Vec<_>>();
    if next_u
        .iter()
        .chain(&next_v)
        .chain(&next_a)
        .any(|value| !value.is_finite())
    {
        return Err("transient spring Newmark state became non-finite".to_string());
    }
    Ok((next_u, next_v, next_a))
}

fn initial_acceleration(
    request: &SolveTransientSpring1dRequest,
    displacement: &[f64],
    velocity: &[f64],
) -> Result<Vec<f64>, String> {
    let mut residual = request
        .nodes
        .iter()
        .map(|node| node.load_x)
        .collect::<Vec<_>>();
    for element in &request.elements {
        let transmitted = element.stiffness
            * (displacement[element.node_j] - displacement[element.node_i])
            + element.damping * (velocity[element.node_j] - velocity[element.node_i]);
        residual[element.node_i] += transmitted;
        residual[element.node_j] -= transmitted;
    }
    let acceleration = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            if node.fix_x {
                0.0
            } else {
                residual[index] / node.mass
            }
        })
        .collect::<Vec<_>>();
    if acceleration.iter().any(|value| !value.is_finite()) {
        return Err("transient spring initial acceleration became non-finite".to_string());
    }
    Ok(acceleration)
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
) -> Result<Vec<TransientSpring1dElementResult>, String> {
    request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let extension = u[element.node_j] - u[element.node_i];
            let relative_velocity = v[element.node_j] - v[element.node_i];
            let spring_force = element.stiffness * extension;
            let damping_force = element.damping * relative_velocity;
            if [extension, relative_velocity, spring_force, damping_force]
                .iter()
                .any(|value| !value.is_finite())
            {
                return Err(format!(
                    "transient spring element {} produced a non-finite result",
                    element.id
                ));
            }
            Ok(TransientSpring1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                extension,
                relative_velocity,
                spring_force,
                damping_force,
            })
        })
        .collect()
}

fn step_result(
    step: usize,
    time: f64,
    request: &SolveTransientSpring1dRequest,
    u: &[f64],
    v: &[f64],
) -> Result<TransientSpring1dStepResult, String> {
    let kinetic_energy = finite_sum(
        request
            .nodes
            .iter()
            .zip(v)
            .map(|(node, velocity)| 0.5 * node.mass * velocity * velocity),
        "kinetic energy",
    )?;
    let strain_energy = finite_sum(
        request.elements.iter().map(|element| {
            let extension = u[element.node_j] - u[element.node_i];
            0.5 * element.stiffness * extension * extension
        }),
        "strain energy",
    )?;
    Ok(TransientSpring1dStepResult {
        step,
        time,
        max_displacement: maximum_absolute(u),
        max_velocity: maximum_absolute(v),
        kinetic_energy,
        strain_energy,
        displacements: u.to_vec(),
        velocities: v.to_vec(),
    })
}

fn maximum_absolute(values: &[f64]) -> f64 {
    values.iter().map(|value| value.abs()).fold(0.0, f64::max)
}

fn finite_sum(values: impl IntoIterator<Item = f64>, label: &str) -> Result<f64, String> {
    let mut sum = 0.0;
    for value in values {
        sum += value;
        if !sum.is_finite() {
            return Err(format!("transient spring {label} became non-finite"));
        }
    }
    Ok(sum)
}

fn constrained_dofs(request: &SolveTransientSpring1dRequest) -> Vec<usize> {
    request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.fix_x.then_some(index))
        .collect()
}

fn checked_time(step: usize, time_step: f64) -> Result<f64, String> {
    let time = step as f64 * time_step;
    if time.is_finite() {
        Ok(time)
    } else {
        Err("transient spring simulation time became non-finite".to_string())
    }
}
