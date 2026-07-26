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
use crate::cohesive_interface_mesh_2d_connector::ConnectorSpring;
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
    let mut steps = Vec::with_capacity(model.controls.len());
    let mut completed_load_factor = 0.0;
    let mut residual_norm = 0.0;
    let mut failure_reason = None;

    for (step_index, control) in model.controls.iter().enumerate() {
        let outcome = solve_load_step(&model, step_index, control, &displacements, &states);
        residual_norm = outcome.residual_norm;
        let summary_assembly =
            assemble(&model, step_index, &outcome.displacements, &outcome.states);
        let summary = step_summary(&model, control, &outcome.displacements, &summary_assembly);
        steps.push(CohesiveInterfaceMesh2dLoadStepResult {
            step: step_index,
            load_factor: control.load_factor,
            iterations: outcome.iterations,
            residual_norm,
            converged: outcome.converged,
            max_displacement: summary.max_displacement,
            prescribed_displacement_norm: load_norm(&control.prescribed_displacements),
            reaction_norm: summary.reaction_norm,
            max_resultant_traction: summary.max_resultant_traction,
            max_shear_damage: summary.max_shear_damage,
            max_normal_damage: summary.max_normal_damage,
            max_connector_force: summary.max_connector_force,
        });
        if !outcome.converged {
            failure_reason = outcome.failure_reason;
            break;
        }
        displacements = outcome.displacements;
        states = outcome.states;
        completed_load_factor = control.load_factor;
    }

    let final_assembly = assemble(&model, steps.len(), &displacements, &states);
    let reactions = reactions(
        &model,
        completed_load_factor,
        &final_assembly.internal_forces,
    );
    let nodes = node_results(request, &displacements, &reactions);
    let elements = element_results(&model, &final_assembly.evaluations);
    let connector_springs = model
        .connector_springs
        .iter()
        .map(|spring| spring.result(&displacements))
        .collect::<Vec<_>>();
    let final_max_displacement = nodes
        .iter()
        .map(|node| node.displacement[0].hypot(node.displacement[1]))
        .fold(0.0_f64, f64::max);
    let final_max_shear_damage = elements
        .iter()
        .map(|element| element.max_shear_damage)
        .fold(0.0_f64, f64::max);
    let final_max_normal_damage = elements
        .iter()
        .map(|element| element.max_normal_damage)
        .fold(0.0_f64, f64::max);
    let final_max_connector_force = connector_springs
        .iter()
        .map(|spring| spring.force[0].hypot(spring.force[1]))
        .fold(0.0_f64, f64::max);
    let converged = failure_reason.is_none() && steps.len() == model.controls.len();
    let max_displacement = steps
        .iter()
        .map(|step| step.max_displacement)
        .fold(final_max_displacement, f64::max);
    let max_shear_damage = steps
        .iter()
        .map(|step| step.max_shear_damage)
        .fold(final_max_shear_damage, f64::max);
    let max_normal_damage = steps
        .iter()
        .map(|step| step.max_normal_damage)
        .fold(final_max_normal_damage, f64::max);
    let max_connector_force = steps
        .iter()
        .map(|step| step.max_connector_force)
        .fold(final_max_connector_force, f64::max);

    Ok(SolveCohesiveInterfaceMesh2dResult {
        input: request.clone(),
        nodes,
        elements,
        connector_springs,
        steps,
        converged,
        completed_load_factor,
        residual_norm,
        max_displacement,
        max_shear_damage,
        max_normal_damage,
        max_connector_force,
        failure_reason,
    })
}

struct ValidatedModel<'a> {
    elements: Vec<ModelElement<'a>>,
    connector_springs: Vec<ConnectorSpring<'a>>,
    free_dofs: Vec<usize>,
    fixed_dofs: Vec<usize>,
    external_loads: Vec<f64>,
    controls: Vec<ControlStep>,
    dof_count: usize,
    max_iterations: usize,
    tolerance: f64,
}

struct ControlStep {
    load_factor: f64,
    prescribed_displacements: Vec<f64>,
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
        let connector_springs =
            ConnectorSpring::build_all(&request.connector_springs, request.nodes.len())?;

        let dof_count = 2 * request.nodes.len();
        let mut free_dofs = Vec::new();
        let mut fixed_dofs = Vec::new();
        let mut external_loads = vec![0.0; dof_count];
        let mut prescribed_displacements = vec![0.0; dof_count];
        for (node_index, node) in request.nodes.iter().enumerate() {
            validate_id(&node.id)?;
            let prescribed = node.prescribed_displacement.unwrap_or([0.0; 2]);
            if !node.x.is_finite()
                || !node.y.is_finite()
                || node.load.iter().any(|value| !value.is_finite())
                || prescribed.iter().any(|value| !value.is_finite())
            {
                return Err("cohesive interface mesh 2d node data must be finite".to_string());
            }
            for axis in 0..2 {
                let dof = 2 * node_index + axis;
                external_loads[dof] = node.load[axis];
                if node.fixed[axis] {
                    fixed_dofs.push(dof);
                    prescribed_displacements[dof] = prescribed[axis];
                } else {
                    if prescribed[axis] != 0.0 {
                        return Err(format!(
                            "node '{}' has a prescribed displacement on free axis {axis}",
                            node.id
                        ));
                    }
                    free_dofs.push(dof);
                }
            }
        }
        if fixed_dofs.is_empty() {
            return Err("cohesive interface mesh 2d requires constrained dofs".to_string());
        }
        let controls = build_controls(request, dof_count, &free_dofs, &prescribed_displacements)?;
        let free_load_norm = free_residual_norm(&external_loads, &free_dofs);
        let has_driving_step = controls.iter().any(|control| {
            control.load_factor.abs() * free_load_norm > 0.0
                || load_norm(&control.prescribed_displacements) > 0.0
        });
        if !has_driving_step {
            return Err(
                "cohesive interface mesh 2d requires a free-dof load or prescribed displacement"
                    .to_string(),
            );
        }

        let max_iterations = request.max_iterations.unwrap_or(DEFAULT_MAX_ITERATIONS);
        let tolerance = request.tolerance.unwrap_or(DEFAULT_TOLERANCE);
        if max_iterations == 0 || max_iterations > MAX_ITERATIONS {
            return Err(format!("max_iterations must be in 1..={MAX_ITERATIONS}"));
        }
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err("tolerance must be finite and positive".to_string());
        }

        Ok(Self {
            elements,
            connector_springs,
            free_dofs,
            fixed_dofs,
            external_loads,
            controls,
            dof_count,
            max_iterations,
            tolerance,
        })
    }
}

fn build_controls(
    request: &SolveCohesiveInterfaceMesh2dRequest,
    dof_count: usize,
    free_dofs: &[usize],
    target_displacements: &[f64],
) -> Result<Vec<ControlStep>, String> {
    if let Some(history) = &request.control_history {
        if request.load_steps.is_some() {
            return Err("control_history and load_steps are mutually exclusive".to_string());
        }
        if target_displacements.iter().any(|value| *value != 0.0) {
            return Err(
                "control_history and node prescribed_displacement targets are mutually exclusive"
                    .to_string(),
            );
        }
        if history.is_empty() || history.len() > MAX_LOAD_STEPS {
            return Err(format!(
                "control_history must contain 1..={MAX_LOAD_STEPS} steps"
            ));
        }
        let free = free_dofs.iter().copied().collect::<HashSet<_>>();
        return history
            .iter()
            .enumerate()
            .map(|(step, input)| {
                if !input.load_factor.is_finite() {
                    return Err(format!("control_history step {step} load_factor is not finite"));
                }
                if input.prescribed_displacements.len() * 2 != dof_count
                    || input
                        .prescribed_displacements
                        .iter()
                        .flatten()
                        .any(|value| !value.is_finite())
                {
                    return Err(format!(
                        "control_history step {step} displacement vector must match finite node data"
                    ));
                }
                let values = input
                    .prescribed_displacements
                    .iter()
                    .flatten()
                    .copied()
                    .collect::<Vec<_>>();
                if free.iter().any(|&dof| values[dof] != 0.0) {
                    return Err(format!(
                        "control_history step {step} prescribes a free dof"
                    ));
                }
                Ok(ControlStep {
                    load_factor: input.load_factor,
                    prescribed_displacements: values,
                })
            })
            .collect();
    }

    let load_steps = request.load_steps.unwrap_or(DEFAULT_LOAD_STEPS);
    if load_steps == 0 || load_steps > MAX_LOAD_STEPS {
        return Err(format!("load_steps must be in 1..={MAX_LOAD_STEPS}"));
    }
    Ok((0..load_steps)
        .map(|step| {
            let factor = (step + 1) as f64 / load_steps as f64;
            ControlStep {
                load_factor: factor,
                prescribed_displacements: target_displacements
                    .iter()
                    .map(|value| factor * value)
                    .collect(),
            }
        })
        .collect())
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
    control: &ControlStep,
    committed_displacements: &[f64],
    committed_states: &[CohesiveInterface2dState],
) -> LoadStepOutcome {
    let mut trial_displacements = committed_displacements.to_vec();
    for &dof in &model.fixed_dofs {
        trial_displacements[dof] = control.prescribed_displacements[dof];
    }
    let load_scale = load_norm(&model.external_loads).max(1.0);
    let mut last_norm = f64::INFINITY;

    for iteration in 1..=model.max_iterations {
        let assembly = assemble(model, step, &trial_displacements, committed_states);
        let residual = model
            .external_loads
            .iter()
            .zip(&assembly.internal_forces)
            .map(|(external, internal)| control.load_factor * external - internal)
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

struct StepSummary {
    max_displacement: f64,
    reaction_norm: f64,
    max_resultant_traction: f64,
    max_shear_damage: f64,
    max_normal_damage: f64,
    max_connector_force: f64,
}

fn step_summary(
    model: &ValidatedModel<'_>,
    control: &ControlStep,
    displacements: &[f64],
    assembly: &Assembly,
) -> StepSummary {
    let reactions = reactions(model, control.load_factor, &assembly.internal_forces);
    let max_resultant_traction = assembly
        .evaluations
        .iter()
        .flat_map(|evaluation| &evaluation.step.integration_points)
        .map(|point| point.local_traction[0].hypot(point.local_traction[1]))
        .fold(0.0_f64, f64::max);
    let max_shear_damage = assembly
        .evaluations
        .iter()
        .map(|evaluation| evaluation.step.shear_damage)
        .fold(0.0_f64, f64::max);
    let max_normal_damage = assembly
        .evaluations
        .iter()
        .map(|evaluation| evaluation.step.normal_damage)
        .fold(0.0_f64, f64::max);
    let max_connector_force = model
        .connector_springs
        .iter()
        .map(|spring| {
            let result = spring.result(displacements);
            result.force[0].hypot(result.force[1])
        })
        .fold(0.0_f64, f64::max);
    StepSummary {
        max_displacement: displacements
            .chunks_exact(2)
            .map(|value| value[0].hypot(value[1]))
            .fold(0.0_f64, f64::max),
        reaction_norm: load_norm(&reactions),
        max_resultant_traction,
        max_shear_damage,
        max_normal_damage,
        max_connector_force,
    }
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
    for spring in &model.connector_springs {
        spring.assemble(displacements, &mut internal_forces, &mut tangent);
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
