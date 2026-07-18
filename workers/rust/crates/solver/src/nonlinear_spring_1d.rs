use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system, solve_spd_system, solve_tridiagonal_system,
};
use crate::nonlinear_spring_1d_validation::{validate_contact_request, validate_request};
use kyuubiki_protocol::{
    ContactGap1dContactResult, NonlinearSpring1dElementResult, NonlinearSpring1dNodeResult,
    NonlinearSpring1dStepResult, SolveContactGap1dRequest, SolveContactGap1dResult,
    SolveNonlinearSpring1dRequest, SolveNonlinearSpring1dResult,
};

pub fn solve_nonlinear_spring_1d(
    request: &SolveNonlinearSpring1dRequest,
) -> Result<SolveNonlinearSpring1dResult, String> {
    validate_request(request)?;

    let dof_count = request.nodes.len();
    let constrained = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| if node.fix_x { Some(index) } else { None })
        .collect::<Vec<_>>();
    let external_force = request
        .nodes
        .iter()
        .map(|node| node.load_x)
        .collect::<Vec<_>>();
    let load_steps = request.load_steps.unwrap_or(8).clamp(1, 256);
    let max_iterations = request.max_iterations.unwrap_or(32).clamp(1, 256);
    let tolerance = request.tolerance.unwrap_or(1.0e-8).max(1.0e-14);

    let mut displacement = vec![0.0; dof_count];
    let mut steps = Vec::with_capacity(load_steps);

    for step in 1..=load_steps {
        let load_factor = step as f64 / load_steps as f64;
        let mut converged = false;
        let mut residual_norm = f64::INFINITY;
        let mut iterations = 0;

        for iteration in 1..=max_iterations {
            iterations = iteration;
            let (tangent, internal_force) = assemble_tangent_and_internal(request, &displacement);
            let residual = external_force
                .iter()
                .zip(internal_force.iter())
                .map(|(external, internal)| load_factor * external - internal)
                .collect::<Vec<_>>();

            let (reduced_tangent, reduced_residual, free) =
                reduce_sparse_system(&tangent, &residual, &constrained);
            residual_norm = reduced_residual
                .iter()
                .map(|entry| entry.abs())
                .fold(0.0_f64, f64::max);

            if residual_norm <= tolerance {
                converged = true;
                break;
            }

            let delta = solve_chain_tangent(&reduced_tangent, &reduced_residual)?;
            for (index, &dof) in free.iter().enumerate() {
                displacement[dof] += delta[index];
            }
        }

        steps.push(NonlinearSpring1dStepResult {
            step,
            load_factor,
            iterations,
            residual_norm,
            converged,
        });

        if !converged {
            break;
        }
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| NonlinearSpring1dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            ux: displacement[index],
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let extension = displacement[element.node_j] - displacement[element.node_i];
            let force = element.stiffness * extension + element.cubic_stiffness * extension.powi(3);
            let tangent_stiffness =
                element.stiffness + 3.0 * element.cubic_stiffness * extension.powi(2);

            NonlinearSpring1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length: (request.nodes[element.node_j].x - request.nodes[element.node_i].x).abs(),
                extension,
                force,
                tangent_stiffness,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| node.ux.abs())
        .fold(0.0_f64, f64::max);
    let max_force = elements
        .iter()
        .map(|element| element.force.abs())
        .fold(0.0_f64, f64::max);
    let residual_norm = steps.last().map(|step| step.residual_norm).unwrap_or(0.0);
    let converged = steps.last().is_some_and(|step| step.converged) && steps.len() == load_steps;

    Ok(SolveNonlinearSpring1dResult {
        input: request.clone(),
        nodes,
        elements,
        steps,
        converged,
        residual_norm,
        max_displacement,
        max_force,
    })
}

pub fn solve_contact_gap_1d(
    request: &SolveContactGap1dRequest,
) -> Result<SolveContactGap1dResult, String> {
    validate_contact_request(request)?;

    let dof_count = request.nodes.len();
    let constrained = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| if node.fix_x { Some(index) } else { None })
        .collect::<Vec<_>>();
    let external_force = request
        .nodes
        .iter()
        .map(|node| node.load_x)
        .collect::<Vec<_>>();
    let load_steps = request.load_steps.unwrap_or(8).clamp(1, 256);
    let max_iterations = request.max_iterations.unwrap_or(32).clamp(1, 256);
    let tolerance = request.tolerance.unwrap_or(1.0e-8).max(1.0e-14);

    let mut displacement = vec![0.0; dof_count];
    let mut steps = Vec::with_capacity(load_steps);

    for step in 1..=load_steps {
        let load_factor = step as f64 / load_steps as f64;
        let mut converged = false;
        let mut residual_norm = f64::INFINITY;
        let mut iterations = 0;

        for iteration in 1..=max_iterations {
            iterations = iteration;
            let (tangent, internal_force) =
                assemble_contact_tangent_and_internal(request, &displacement);
            let residual = external_force
                .iter()
                .zip(internal_force.iter())
                .map(|(external, internal)| load_factor * external - internal)
                .collect::<Vec<_>>();

            let (reduced_tangent, reduced_residual, free) =
                reduce_sparse_system(&tangent, &residual, &constrained);
            residual_norm = reduced_residual
                .iter()
                .map(|entry| entry.abs())
                .fold(0.0_f64, f64::max);

            if residual_norm <= tolerance {
                converged = true;
                break;
            }

            let delta = solve_chain_tangent(&reduced_tangent, &reduced_residual)?;
            for (index, &dof) in free.iter().enumerate() {
                displacement[dof] += delta[index];
            }
        }

        steps.push(NonlinearSpring1dStepResult {
            step,
            load_factor,
            iterations,
            residual_norm,
            converged,
        });

        if !converged {
            break;
        }
    }

    let nodes = nonlinear_nodes(&request.nodes, &displacement);
    let elements = nonlinear_elements(&request.nodes, &request.elements, &displacement);
    let contacts = contact_results(request, &displacement);
    let max_displacement = nodes
        .iter()
        .map(|node| node.ux.abs())
        .fold(0.0_f64, f64::max);
    let max_force = elements
        .iter()
        .map(|element| element.force.abs())
        .fold(0.0_f64, f64::max);
    let max_contact_force = contacts
        .iter()
        .map(|contact| contact.force.abs())
        .fold(0.0_f64, f64::max);
    let active_contact_count = contacts.iter().filter(|contact| contact.active).count();
    let residual_norm = steps.last().map(|step| step.residual_norm).unwrap_or(0.0);
    let converged = steps.last().is_some_and(|step| step.converged) && steps.len() == load_steps;

    Ok(SolveContactGap1dResult {
        input: request.clone(),
        nodes,
        elements,
        contacts,
        steps,
        converged,
        residual_norm,
        max_displacement,
        max_force,
        max_contact_force,
        active_contact_count,
    })
}

// Chain benchmarks retain a tridiagonal tangent after constraints and contact penalties.
// Preserve the general sparse fallback for caller-provided non-chain topologies.
fn solve_chain_tangent(tangent: &SparseMatrix, residual: &[f64]) -> Result<Vec<f64>, String> {
    match solve_tridiagonal_system(tangent, residual) {
        Some(result) => result,
        None => solve_spd_system(tangent, residual),
    }
}

fn assemble_tangent_and_internal(
    request: &SolveNonlinearSpring1dRequest,
    displacement: &[f64],
) -> (SparseMatrix, Vec<f64>) {
    let mut tangent = SparseMatrix::new(request.nodes.len());
    let mut internal_force = vec![0.0; request.nodes.len()];

    for element in &request.elements {
        let extension = displacement[element.node_j] - displacement[element.node_i];
        let force = element.stiffness * extension + element.cubic_stiffness * extension.powi(3);
        let tangent_stiffness =
            element.stiffness + 3.0 * element.cubic_stiffness * extension.powi(2);
        let map = [element.node_i, element.node_j];
        let local = [
            [tangent_stiffness, -tangent_stiffness],
            [-tangent_stiffness, tangent_stiffness],
        ];

        internal_force[element.node_i] -= force;
        internal_force[element.node_j] += force;

        for row in 0..2 {
            for column in 0..2 {
                add_at(&mut tangent, map[row], map[column], local[row][column]);
            }
        }
    }

    (tangent, internal_force)
}

fn assemble_contact_tangent_and_internal(
    request: &SolveContactGap1dRequest,
    displacement: &[f64],
) -> (SparseMatrix, Vec<f64>) {
    let mut tangent = SparseMatrix::new(request.nodes.len());
    let mut internal_force = vec![0.0; request.nodes.len()];

    assemble_element_terms(
        &mut tangent,
        &mut internal_force,
        &request.elements,
        displacement,
    );

    for contact in &request.contacts {
        let penetration = (displacement[contact.node] - contact.gap).max(0.0);
        if penetration <= 0.0 {
            continue;
        }
        internal_force[contact.node] += contact.normal_stiffness * penetration;
        add_at(
            &mut tangent,
            contact.node,
            contact.node,
            contact.normal_stiffness,
        );
    }

    (tangent, internal_force)
}

fn assemble_element_terms(
    tangent: &mut SparseMatrix,
    internal_force: &mut [f64],
    elements: &[kyuubiki_protocol::NonlinearSpring1dElementInput],
    displacement: &[f64],
) {
    for element in elements {
        let extension = displacement[element.node_j] - displacement[element.node_i];
        let force = element.stiffness * extension + element.cubic_stiffness * extension.powi(3);
        let tangent_stiffness =
            element.stiffness + 3.0 * element.cubic_stiffness * extension.powi(2);
        let map = [element.node_i, element.node_j];
        let local = [
            [tangent_stiffness, -tangent_stiffness],
            [-tangent_stiffness, tangent_stiffness],
        ];

        internal_force[element.node_i] -= force;
        internal_force[element.node_j] += force;

        for row in 0..2 {
            for column in 0..2 {
                add_at(tangent, map[row], map[column], local[row][column]);
            }
        }
    }
}

fn nonlinear_nodes(
    nodes: &[kyuubiki_protocol::NonlinearSpring1dNodeInput],
    displacement: &[f64],
) -> Vec<NonlinearSpring1dNodeResult> {
    nodes
        .iter()
        .enumerate()
        .map(|(index, node)| NonlinearSpring1dNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            ux: displacement[index],
        })
        .collect()
}

fn nonlinear_elements(
    nodes: &[kyuubiki_protocol::NonlinearSpring1dNodeInput],
    elements: &[kyuubiki_protocol::NonlinearSpring1dElementInput],
    displacement: &[f64],
) -> Vec<NonlinearSpring1dElementResult> {
    elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let extension = displacement[element.node_j] - displacement[element.node_i];
            let force = element.stiffness * extension + element.cubic_stiffness * extension.powi(3);
            let tangent_stiffness =
                element.stiffness + 3.0 * element.cubic_stiffness * extension.powi(2);

            NonlinearSpring1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length: (nodes[element.node_j].x - nodes[element.node_i].x).abs(),
                extension,
                force,
                tangent_stiffness,
            }
        })
        .collect()
}

fn contact_results(
    request: &SolveContactGap1dRequest,
    displacement: &[f64],
) -> Vec<ContactGap1dContactResult> {
    request
        .contacts
        .iter()
        .enumerate()
        .map(|(index, contact)| {
            let penetration = (displacement[contact.node] - contact.gap).max(0.0);
            ContactGap1dContactResult {
                index,
                id: contact.id.clone(),
                node: contact.node,
                gap: contact.gap,
                penetration,
                force: contact.normal_stiffness * penetration,
                active: penetration > 0.0,
            }
        })
        .collect()
}
