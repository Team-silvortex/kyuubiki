use std::collections::{HashMap, HashSet};

use kyuubiki_protocol::{
    CohesiveInterfaceMesh2dElementInput, CohesiveInterfaceMesh2dElementResult,
    CohesiveInterfaceMesh2dLoadStepResult, CohesiveInterfaceMesh2dNodeResult,
    SolveCohesiveInterfaceMesh2dRequest, SolveCohesiveInterfaceMesh2dResult,
};

use crate::cohesive_interface_1d::validate_id;
use crate::cohesive_interface_2d::{
    CohesiveInterface2dEvaluation, CohesiveInterface2dKernel, CohesiveInterface2dState,
};
use crate::linear_dense::{solve_linear_system, zero_matrix};

const DEFAULT_LOAD_STEPS: usize = 10;
const DEFAULT_MAX_ITERATIONS: usize = 30;
const DEFAULT_TOLERANCE: f64 = 1.0e-9;
const MAX_LOAD_STEPS: usize = 4096;
const MAX_ITERATIONS: usize = 200;
const MAX_NODES: usize = 512;
const MAX_MATERIALS: usize = 256;
const MAX_ELEMENTS: usize = 4096;

pub fn solve_cohesive_interface_mesh_2d(
    request: &SolveCohesiveInterfaceMesh2dRequest,
) -> Result<SolveCohesiveInterfaceMesh2dResult, String> {
    let model = ValidatedModel::new(request)?;
    let mut displacements = vec![0.0; model.dof_count];
    let mut states = vec![CohesiveInterface2dState::default(); model.elements.len()];
    let mut steps = Vec::with_capacity(model.load_steps);
    let mut completed_load_factor = 0.0;
    let mut residual_norm = 0.0;
    let mut failure_reason = None;

    for step_index in 0..model.load_steps {
        let load_factor = (step_index + 1) as f64 / model.load_steps as f64;
        let outcome = solve_load_step(&model, step_index, load_factor, &displacements, &states);
        residual_norm = outcome.residual_norm;
        steps.push(CohesiveInterfaceMesh2dLoadStepResult {
            step: step_index,
            load_factor,
            iterations: outcome.iterations,
            residual_norm,
            converged: outcome.converged,
        });
        if !outcome.converged {
            failure_reason = outcome.failure_reason;
            break;
        }
        displacements = outcome.displacements;
        states = outcome.states;
        completed_load_factor = load_factor;
    }

    let final_assembly = assemble(&model, steps.len(), &displacements, &states);
    let reactions = reactions(
        &model,
        completed_load_factor,
        &final_assembly.internal_forces,
    );
    let nodes = node_results(request, &displacements, &reactions);
    let elements = element_results(&model, &final_assembly.evaluations);
    let max_displacement = nodes
        .iter()
        .map(|node| node.displacement[0].hypot(node.displacement[1]))
        .fold(0.0_f64, f64::max);
    let max_shear_damage = elements
        .iter()
        .map(|element| element.max_shear_damage)
        .fold(0.0_f64, f64::max);
    let max_normal_damage = elements
        .iter()
        .map(|element| element.max_normal_damage)
        .fold(0.0_f64, f64::max);

    Ok(SolveCohesiveInterfaceMesh2dResult {
        input: request.clone(),
        nodes,
        elements,
        steps,
        converged: failure_reason.is_none() && completed_load_factor >= 1.0,
        completed_load_factor,
        residual_norm,
        max_displacement,
        max_shear_damage,
        max_normal_damage,
        failure_reason,
    })
}

struct ValidatedModel<'a> {
    elements: Vec<ModelElement<'a>>,
    free_dofs: Vec<usize>,
    fixed_dofs: Vec<usize>,
    external_loads: Vec<f64>,
    dof_count: usize,
    load_steps: usize,
    max_iterations: usize,
    tolerance: f64,
}

struct ModelElement<'a> {
    input: &'a CohesiveInterfaceMesh2dElementInput,
    nodes: [usize; 4],
    dofs: [usize; 8],
    kernel: CohesiveInterface2dKernel,
}

impl<'a> ValidatedModel<'a> {
    fn new(request: &'a SolveCohesiveInterfaceMesh2dRequest) -> Result<Self, String> {
        validate_id(&request.id)?;
        if request.nodes.is_empty() || request.nodes.len() > MAX_NODES {
            return Err(format!(
                "cohesive interface mesh 2d requires 1..={MAX_NODES} nodes"
            ));
        }
        if request.materials.is_empty() || request.materials.len() > MAX_MATERIALS {
            return Err(format!(
                "cohesive interface mesh 2d requires 1..={MAX_MATERIALS} materials"
            ));
        }
        if request.elements.is_empty() || request.elements.len() > MAX_ELEMENTS {
            return Err(format!(
                "cohesive interface mesh 2d requires 1..={MAX_ELEMENTS} elements"
            ));
        }

        validate_unique_ids(request)?;
        let material_indices = request
            .materials
            .iter()
            .enumerate()
            .map(|(index, material)| (material.id.as_str(), index))
            .collect::<HashMap<_, _>>();
        let elements = request
            .elements
            .iter()
            .map(|element| build_element(request, element, &material_indices))
            .collect::<Result<Vec<_>, _>>()?;

        let dof_count = 2 * request.nodes.len();
        let mut free_dofs = Vec::new();
        let mut fixed_dofs = Vec::new();
        let mut external_loads = vec![0.0; dof_count];
        for (node_index, node) in request.nodes.iter().enumerate() {
            validate_id(&node.id)?;
            if !node.x.is_finite()
                || !node.y.is_finite()
                || node.load.iter().any(|value| !value.is_finite())
            {
                return Err("cohesive interface mesh 2d node data must be finite".to_string());
            }
            for axis in 0..2 {
                let dof = 2 * node_index + axis;
                external_loads[dof] = node.load[axis];
                if node.fixed[axis] {
                    fixed_dofs.push(dof);
                } else {
                    free_dofs.push(dof);
                }
            }
        }
        if free_dofs.is_empty() {
            return Err("cohesive interface mesh 2d requires at least one free dof".to_string());
        }
        if fixed_dofs.is_empty() {
            return Err("cohesive interface mesh 2d requires constrained dofs".to_string());
        }
        if free_residual_norm(&external_loads, &free_dofs) <= 0.0 {
            return Err(
                "cohesive interface mesh 2d requires a non-zero load on a free dof".to_string(),
            );
        }

        let load_steps = request.load_steps.unwrap_or(DEFAULT_LOAD_STEPS);
        let max_iterations = request.max_iterations.unwrap_or(DEFAULT_MAX_ITERATIONS);
        let tolerance = request.tolerance.unwrap_or(DEFAULT_TOLERANCE);
        if load_steps == 0 || load_steps > MAX_LOAD_STEPS {
            return Err(format!("load_steps must be in 1..={MAX_LOAD_STEPS}"));
        }
        if max_iterations == 0 || max_iterations > MAX_ITERATIONS {
            return Err(format!("max_iterations must be in 1..={MAX_ITERATIONS}"));
        }
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err("tolerance must be finite and positive".to_string());
        }

        Ok(Self {
            elements,
            free_dofs,
            fixed_dofs,
            external_loads,
            dof_count,
            load_steps,
            max_iterations,
            tolerance,
        })
    }
}

fn validate_unique_ids(request: &SolveCohesiveInterfaceMesh2dRequest) -> Result<(), String> {
    let mut node_ids = HashSet::new();
    for node in &request.nodes {
        if !node_ids.insert(node.id.as_str()) {
            return Err(format!("duplicate cohesive mesh node id '{}'", node.id));
        }
    }
    let mut material_ids = HashSet::new();
    for material in &request.materials {
        validate_id(&material.id)?;
        if !material_ids.insert(material.id.as_str()) {
            return Err(format!(
                "duplicate cohesive mesh material id '{}'",
                material.id
            ));
        }
    }
    let mut element_ids = HashSet::new();
    for element in &request.elements {
        if !element_ids.insert(element.id.as_str()) {
            return Err(format!(
                "duplicate cohesive mesh element id '{}'",
                element.id
            ));
        }
    }
    Ok(())
}

fn build_element<'a>(
    request: &'a SolveCohesiveInterfaceMesh2dRequest,
    element: &'a CohesiveInterfaceMesh2dElementInput,
    material_indices: &HashMap<&str, usize>,
) -> Result<ModelElement<'a>, String> {
    validate_id(&element.id)?;
    let nodes = [
        element.lower_i,
        element.lower_j,
        element.upper_i,
        element.upper_j,
    ];
    if nodes.iter().any(|&index| index >= request.nodes.len()) {
        return Err(format!(
            "element '{}' node index is out of bounds",
            element.id
        ));
    }
    let unique = nodes.into_iter().collect::<HashSet<_>>();
    if unique.len() != 4 {
        return Err(format!(
            "element '{}' requires four distinct nodes",
            element.id
        ));
    }
    let material_index = material_indices
        .get(element.material_id.as_str())
        .copied()
        .ok_or_else(|| {
            format!(
                "element '{}' references unknown material '{}'",
                element.id, element.material_id
            )
        })?;
    let points = nodes.map(|index| {
        let node = &request.nodes[index];
        [node.x, node.y]
    });
    let kernel = CohesiveInterface2dKernel::new(
        &element.id,
        points,
        element.thickness,
        &request.materials[material_index].properties,
    )?;
    let mut dofs = [0; 8];
    for (local_node, node) in nodes.into_iter().enumerate() {
        dofs[2 * local_node] = 2 * node;
        dofs[2 * local_node + 1] = 2 * node + 1;
    }
    Ok(ModelElement {
        input: element,
        nodes,
        dofs,
        kernel,
    })
}

struct LoadStepOutcome {
    displacements: Vec<f64>,
    states: Vec<CohesiveInterface2dState>,
    iterations: usize,
    residual_norm: f64,
    converged: bool,
    failure_reason: Option<String>,
}

fn solve_load_step(
    model: &ValidatedModel<'_>,
    step: usize,
    load_factor: f64,
    committed_displacements: &[f64],
    committed_states: &[CohesiveInterface2dState],
) -> LoadStepOutcome {
    let mut trial_displacements = committed_displacements.to_vec();
    let load_scale = load_norm(&model.external_loads).max(1.0);
    let mut last_norm = f64::INFINITY;

    for iteration in 1..=model.max_iterations {
        let assembly = assemble(model, step, &trial_displacements, committed_states);
        let residual = model
            .external_loads
            .iter()
            .zip(&assembly.internal_forces)
            .map(|(external, internal)| load_factor * external - internal)
            .collect::<Vec<_>>();
        last_norm = free_residual_norm(&residual, &model.free_dofs);
        if last_norm <= model.tolerance * load_scale {
            let states = assembly
                .evaluations
                .into_iter()
                .map(|evaluation| evaluation.state)
                .collect();
            return LoadStepOutcome {
                displacements: trial_displacements,
                states,
                iterations: iteration,
                residual_norm: last_norm,
                converged: true,
                failure_reason: None,
            };
        }

        let reduced_matrix = model
            .free_dofs
            .iter()
            .map(|&row| {
                model
                    .free_dofs
                    .iter()
                    .map(|&column| assembly.tangent[row][column])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let reduced_residual = model
            .free_dofs
            .iter()
            .map(|&dof| residual[dof])
            .collect::<Vec<_>>();
        let increment = match solve_linear_system(reduced_matrix, reduced_residual) {
            Ok(increment) => increment,
            Err(error) => {
                return failed_step(
                    committed_displacements,
                    committed_states,
                    iteration,
                    last_norm,
                    format!(
                        "load step {} tangent solve failed: {error}; check constraints and connectivity",
                        step + 1
                    ),
                );
            }
        };
        for (&dof, delta) in model.free_dofs.iter().zip(increment) {
            trial_displacements[dof] += delta;
        }
        if trial_displacements.iter().any(|value| !value.is_finite()) {
            return failed_step(
                committed_displacements,
                committed_states,
                iteration,
                last_norm,
                format!("load step {} produced non-finite displacement", step + 1),
            );
        }
    }

    failed_step(
        committed_displacements,
        committed_states,
        model.max_iterations,
        last_norm,
        format!(
            "load step {} did not converge within {} iterations",
            step + 1,
            model.max_iterations
        ),
    )
}

fn failed_step(
    displacements: &[f64],
    states: &[CohesiveInterface2dState],
    iterations: usize,
    residual_norm: f64,
    reason: String,
) -> LoadStepOutcome {
    LoadStepOutcome {
        displacements: displacements.to_vec(),
        states: states.to_vec(),
        iterations,
        residual_norm,
        converged: false,
        failure_reason: Some(reason),
    }
}

struct Assembly {
    internal_forces: Vec<f64>,
    tangent: Vec<Vec<f64>>,
    evaluations: Vec<CohesiveInterface2dEvaluation>,
}

fn assemble(
    model: &ValidatedModel<'_>,
    step: usize,
    displacements: &[f64],
    committed_states: &[CohesiveInterface2dState],
) -> Assembly {
    let mut internal_forces = vec![0.0; model.dof_count];
    let mut tangent = zero_matrix(model.dof_count);
    let mut evaluations = Vec::with_capacity(model.elements.len());
    for (element_index, element) in model.elements.iter().enumerate() {
        let local_displacements = element
            .nodes
            .map(|node| [displacements[2 * node], displacements[2 * node + 1]]);
        let evaluation =
            element
                .kernel
                .trial(step, local_displacements, &committed_states[element_index]);
        for local_dof in 0..8 {
            let global_dof = element.dofs[local_dof];
            let local_node = local_dof / 2;
            let axis = local_dof % 2;
            internal_forces[global_dof] +=
                evaluation.step.element_nodal_internal_forces[local_node][axis];
            for local_column in 0..8 {
                tangent[global_dof][element.dofs[local_column]] +=
                    evaluation.step.element_tangent[local_dof][local_column];
            }
        }
        evaluations.push(evaluation);
    }
    Assembly {
        internal_forces,
        tangent,
        evaluations,
    }
}

fn reactions(model: &ValidatedModel<'_>, load_factor: f64, internal: &[f64]) -> Vec<f64> {
    let mut reactions = vec![0.0; model.dof_count];
    for &dof in &model.fixed_dofs {
        reactions[dof] = internal[dof] - load_factor * model.external_loads[dof];
    }
    reactions
}

fn node_results(
    request: &SolveCohesiveInterfaceMesh2dRequest,
    displacements: &[f64],
    reactions: &[f64],
) -> Vec<CohesiveInterfaceMesh2dNodeResult> {
    request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| CohesiveInterfaceMesh2dNodeResult {
            id: node.id.clone(),
            displacement: [displacements[2 * index], displacements[2 * index + 1]],
            reaction: [reactions[2 * index], reactions[2 * index + 1]],
        })
        .collect()
}

fn element_results(
    model: &ValidatedModel<'_>,
    evaluations: &[CohesiveInterface2dEvaluation],
) -> Vec<CohesiveInterfaceMesh2dElementResult> {
    model
        .elements
        .iter()
        .zip(evaluations)
        .map(
            |(element, evaluation)| CohesiveInterfaceMesh2dElementResult {
                id: element.input.id.clone(),
                material_id: element.input.material_id.clone(),
                local_separation: evaluation.step.local_separation,
                local_traction: evaluation.step.local_traction,
                max_shear_damage: evaluation.step.shear_damage,
                max_normal_damage: evaluation.step.normal_damage,
            },
        )
        .collect()
}

fn free_residual_norm(residual: &[f64], free_dofs: &[usize]) -> f64 {
    free_dofs
        .iter()
        .map(|&dof| residual[dof] * residual[dof])
        .sum::<f64>()
        .sqrt()
}

fn load_norm(loads: &[f64]) -> f64 {
    loads.iter().map(|value| value * value).sum::<f64>().sqrt()
}
