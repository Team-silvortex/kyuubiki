use crate::dynamic_spring_1d_validation::validate_transient_request;
use crate::linear_algebra::{PreparedSpdSolver, SparseMatrix, add_at, reduce_sparse_system};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};
use crate::solver_postprocess::{collect_results, max_results, try_collect_results};
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
}

struct NewmarkSystem<'a> {
    time_step: f64,
    coefficients: NewmarkCoefficients,
    mass: &'a [f64],
    elements: &'a [TransientSpring1dElementInput],
    free: &'a [usize],
    solver: &'a PreparedSpdSolver,
    request: &'a SolveTransientSpring1dRequest,
}

struct NewmarkState<'a> {
    displacement: &'a [f64],
    velocity: &'a [f64],
    acceleration: &'a [f64],
}

struct StepMetrics {
    max_displacement: f64,
    max_velocity: f64,
    kinetic_energy: f64,
    strain_energy: f64,
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
    let final_time = checked_time(request.steps, request.time_step)?;
    checkpoint(SolverStage::TransientStep, 0)?;
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
    let mut a = state_acceleration(request.as_ref(), &u, &v, "initial")?;

    let effective = assemble_effective_system(request.as_ref(), coefficients)?;
    let (reduced_effective, _, free) =
        reduce_sparse_system(&effective, &vec![0.0; count], &constrained)?;
    let solver = PreparedSpdSolver::factor(reduced_effective)
        .map_err(|error| format!("transient spring effective system failed: {error}"))?;
    let system = NewmarkSystem {
        time_step: request.time_step,
        coefficients,
        mass: &mass,
        elements: &request.elements,
        free: &free,
        solver: &solver,
        request: request.as_ref(),
    };

    let mut history = Vec::new();
    history
        .try_reserve_exact(history_plan.frame_count())
        .map_err(|_| "transient spring history allocation is too large".to_string())?;
    let metrics = step_metrics(request.as_ref(), &u, &v)?;
    let mut max_displacement = metrics.max_displacement;
    let mut max_velocity = metrics.max_velocity;
    history.push(step_result(0, 0.0, metrics, &u, &v));
    for step in 1..=request.steps {
        checkpoint(SolverStage::TransientStep, step)?;
        (u, v, a) = newmark_step(
            &system,
            NewmarkState {
                displacement: &u,
                velocity: &v,
                acceleration: &a,
            },
        )
        .map_err(|error| format!("transient spring step {step}: {error}"))?;
        // History decimation changes storage, never the physical validation policy.
        let metrics = step_metrics(request.as_ref(), &u, &v)
            .map_err(|error| format!("transient spring step {step}: {error}"))?;
        max_displacement = max_displacement.max(metrics.max_displacement);
        max_velocity = max_velocity.max(metrics.max_velocity);
        if history_plan.captures(step, request.steps) {
            history.push(step_result(
                step,
                checked_time(step, request.time_step)?,
                metrics,
                &u,
                &v,
            ));
        }
    }

    let nodes = final_nodes(request.as_ref(), &u, &v, &a)?;
    let elements = final_elements(request.as_ref(), &u, &v)?;
    let max_force = max_results(SolverStage::ResultElementSummary, &elements, |element| {
        (element.spring_force + element.damping_force).abs()
    })?;

    Ok(SolveTransientSpring1dResult {
        input: request.into_owned(),
        final_time,
        max_displacement,
        max_velocity,
        max_force,
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
    };
    if [coefficients.a0, coefficients.a1, coefficients.a2]
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
    for (index, element) in request.elements.iter().enumerate() {
        checkpoint_chunk(SolverStage::ElementAssembly, index, request.elements.len())?;
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
        checkpoint_chunk(SolverStage::ElementAssembly, index, request.nodes.len())?;
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
    // Average-acceleration Newmark in velocity-increment form:
    // (K + 2C/h + 4M/h^2) dv = (4/h) M a_n - 2 K v_n.
    // Avoid recovering acceleration from (u_next - u_n), which loses small increments.
    for (index, rhs) in rhs.iter_mut().enumerate() {
        checkpoint_chunk(SolverStage::TransientState, index, count)?;
        *rhs = c.a2 * (system.mass[index] * state.acceleration[index]);
    }
    for (index, element) in system.elements.iter().enumerate() {
        checkpoint_chunk(SolverStage::TransientState, index, system.elements.len())?;
        let transmitted = 2.0
            * (element.stiffness
                * (state.velocity[element.node_j] - state.velocity[element.node_i]));
        rhs[element.node_i] += transmitted;
        rhs[element.node_j] -= transmitted;
    }
    if rhs.iter().any(|value| !value.is_finite()) {
        return Err("transient spring Newmark right-hand side became non-finite".to_string());
    }

    let reduced_rhs = system.free.iter().map(|&dof| rhs[dof]).collect::<Vec<_>>();
    let reduced_dv = system
        .solver
        .solve(&reduced_rhs)
        .map_err(|error| format!("transient spring Newmark solve failed: {error}"))?;
    let mut next_u = vec![0.0; count];
    let mut next_v = vec![0.0; count];
    for (index, &dof) in system.free.iter().enumerate() {
        checkpoint_chunk(SolverStage::TransientState, index, system.free.len())?;
        let dv = reduced_dv[index];
        next_v[dof] = state.velocity[dof] + dv;
        next_u[dof] = system
            .time_step
            .mul_add(state.velocity[dof] + 0.5 * dv, state.displacement[dof]);
    }
    let next_a = state_acceleration(system.request, &next_u, &next_v, "Newmark")?;
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

fn state_acceleration(
    request: &SolveTransientSpring1dRequest,
    displacement: &[f64],
    velocity: &[f64],
    phase: &str,
) -> Result<Vec<f64>, String> {
    let mut residual = request
        .nodes
        .iter()
        .map(|node| node.load_x)
        .collect::<Vec<_>>();
    for (index, element) in request.elements.iter().enumerate() {
        checkpoint_chunk(SolverStage::TransientState, index, request.elements.len())?;
        let transmitted = element.stiffness
            * (displacement[element.node_j] - displacement[element.node_i])
            + element.damping * (velocity[element.node_j] - velocity[element.node_i]);
        if !transmitted.is_finite() {
            return Err(format!(
                "transient spring {phase} element {} internal force became non-finite",
                element.id
            ));
        }
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
        return Err(format!(
            "transient spring {phase} acceleration became non-finite"
        ));
    }
    Ok(acceleration)
}

fn final_nodes(
    request: &SolveTransientSpring1dRequest,
    u: &[f64],
    v: &[f64],
    a: &[f64],
) -> Result<Vec<TransientSpring1dNodeResult>, String> {
    collect_results(
        SolverStage::ResultNodes,
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
            }),
    )
}

fn final_elements(
    request: &SolveTransientSpring1dRequest,
    u: &[f64],
    v: &[f64],
) -> Result<Vec<TransientSpring1dElementResult>, String> {
    try_collect_results(
        SolverStage::ResultElements,
        request.elements.iter().enumerate().map(|(index, element)| {
            let extension = u[element.node_j] - u[element.node_i];
            let relative_velocity = v[element.node_j] - v[element.node_i];
            let spring_force = element.stiffness * extension;
            let damping_force = element.damping * relative_velocity;
            if [
                extension,
                relative_velocity,
                spring_force,
                damping_force,
                spring_force + damping_force,
            ]
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
        }),
    )
}

fn step_result(
    step: usize,
    time: f64,
    metrics: StepMetrics,
    u: &[f64],
    v: &[f64],
) -> TransientSpring1dStepResult {
    TransientSpring1dStepResult {
        step,
        time,
        max_displacement: metrics.max_displacement,
        max_velocity: metrics.max_velocity,
        kinetic_energy: metrics.kinetic_energy,
        strain_energy: metrics.strain_energy,
        displacements: u.to_vec(),
        velocities: v.to_vec(),
    }
}

fn step_metrics(
    request: &SolveTransientSpring1dRequest,
    u: &[f64],
    v: &[f64],
) -> Result<StepMetrics, String> {
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
    Ok(StepMetrics {
        max_displacement: max_results(SolverStage::ResultNodeSummary, u, |v| v.abs())?,
        max_velocity: max_results(SolverStage::ResultNodeSummary, v, |v| v.abs())?,
        kinetic_energy,
        strain_energy,
    })
}

fn finite_sum(values: impl ExactSizeIterator<Item = f64>, label: &str) -> Result<f64, String> {
    let mut sum = 0.0;
    let count = values.len();
    checkpoint(SolverStage::TransientEnergy, 0)?;
    for (index, value) in values.enumerate() {
        sum += value;
        if !sum.is_finite() {
            return Err(format!("transient spring {label} became non-finite"));
        }
        checkpoint_chunk(SolverStage::TransientEnergy, index + 1, count)?;
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
