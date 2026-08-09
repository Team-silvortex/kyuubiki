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
use crate::cohesive_interface_mesh_2d_control::{
    ControlStep, build_controls, restricted_norm, vector_norm,
};
use crate::cohesive_interface_mesh_2d_newton::solve_load_step;
use crate::cohesive_interface_mesh_2d_plane::{HostPlaneQuad, HostPlaneTriangle};
use crate::cohesive_interface_mesh_2d_truss::HostTruss;
use crate::linear_dense::zero_matrix;

const DEFAULT_MAX_ITERATIONS: usize = 30;
const DEFAULT_TOLERANCE: f64 = 1.0e-9;
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
            prescribed_displacement_norm: vector_norm(&control.prescribed_displacements),
            reaction_norm: summary.reaction_norm,
            max_resultant_traction: summary.max_resultant_traction,
            max_shear_damage: summary.max_shear_damage,
            max_normal_damage: summary.max_normal_damage,
            max_connector_force: summary.max_connector_force,
            max_host_truss_axial_force: summary.max_host_truss_axial_force,
            max_host_truss_stress: summary.max_host_truss_stress,
            max_host_plane_stress: summary.max_host_plane_stress,
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
    let host_trusses = model
        .host_trusses
        .iter()
        .map(|truss| truss.result(&displacements))
        .collect::<Vec<_>>();
    let host_plane_triangles = model
        .host_plane_triangles
        .iter()
        .map(|element| element.result(&displacements))
        .collect::<Vec<_>>();
    let host_plane_quads = model
        .host_plane_quads
        .iter()
        .map(|element| element.result(&displacements))
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
    let final_max_host_truss_axial_force = host_trusses
        .iter()
        .map(|truss| truss.axial_force.abs())
        .fold(0.0_f64, f64::max);
    let final_max_host_truss_stress = host_trusses
        .iter()
        .map(|truss| truss.stress.abs())
        .fold(0.0_f64, f64::max);
    let final_max_host_plane_stress = host_plane_triangles
        .iter()
        .map(|element| element.von_mises.abs())
        .chain(
            host_plane_quads
                .iter()
                .map(|element| element.von_mises.abs()),
        )
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
    let max_host_truss_axial_force = steps
        .iter()
        .map(|step| step.max_host_truss_axial_force)
        .fold(final_max_host_truss_axial_force, f64::max);
    let max_host_truss_stress = steps
        .iter()
        .map(|step| step.max_host_truss_stress)
        .fold(final_max_host_truss_stress, f64::max);
    let max_host_plane_stress = steps
        .iter()
        .map(|step| step.max_host_plane_stress)
        .fold(final_max_host_plane_stress, f64::max);

    Ok(SolveCohesiveInterfaceMesh2dResult {
        input: request.clone(),
        nodes,
        elements,
        connector_springs,
        host_trusses,
        host_plane_triangles,
        host_plane_quads,
        steps,
        converged,
        completed_load_factor,
        residual_norm,
        max_displacement,
        max_shear_damage,
        max_normal_damage,
        max_connector_force,
        max_host_truss_axial_force,
        max_host_truss_stress,
        max_host_plane_stress,
        failure_reason,
    })
}

pub(crate) struct ValidatedModel<'a> {
    elements: Vec<ModelElement<'a>>,
    connector_springs: Vec<ConnectorSpring<'a>>,
    host_trusses: Vec<HostTruss<'a>>,
    host_plane_triangles: Vec<HostPlaneTriangle<'a>>,
    host_plane_quads: Vec<HostPlaneQuad<'a>>,
    pub(crate) free_dofs: Vec<usize>,
    pub(crate) fixed_dofs: Vec<usize>,
    pub(crate) external_loads: Vec<f64>,
    controls: Vec<ControlStep>,
    dof_count: usize,
    pub(crate) max_iterations: usize,
    pub(crate) tolerance: f64,
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
        let host_trusses = HostTruss::build_all(&request.host_trusses, &request.nodes)?;
        let host_plane_triangles =
            HostPlaneTriangle::build_all(&request.host_plane_triangles, &request.nodes)?;
        let host_plane_quads = HostPlaneQuad::build_all(&request.host_plane_quads, &request.nodes)?;

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
        let free_load_norm = restricted_norm(&external_loads, &free_dofs);
        let has_driving_step = controls.iter().any(|control| {
            control.load_factor.abs() * free_load_norm > 0.0
                || vector_norm(&control.prescribed_displacements) > 0.0
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
            host_trusses,
            host_plane_triangles,
            host_plane_quads,
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

pub(crate) struct Assembly {
    pub(crate) internal_forces: Vec<f64>,
    pub(crate) tangent: Vec<Vec<f64>>,
    pub(crate) evaluations: Vec<CohesiveInterface2dEvaluation>,
}

struct StepSummary {
    max_displacement: f64,
    reaction_norm: f64,
    max_resultant_traction: f64,
    max_shear_damage: f64,
    max_normal_damage: f64,
    max_connector_force: f64,
    max_host_truss_axial_force: f64,
    max_host_truss_stress: f64,
    max_host_plane_stress: f64,
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
    let host_truss_results = model
        .host_trusses
        .iter()
        .map(|truss| truss.result(displacements))
        .collect::<Vec<_>>();
    StepSummary {
        max_displacement: displacements
            .chunks_exact(2)
            .map(|value| value[0].hypot(value[1]))
            .fold(0.0_f64, f64::max),
        reaction_norm: vector_norm(&reactions),
        max_resultant_traction,
        max_shear_damage,
        max_normal_damage,
        max_connector_force,
        max_host_truss_axial_force: host_truss_results
            .iter()
            .map(|truss| truss.axial_force.abs())
            .fold(0.0_f64, f64::max),
        max_host_truss_stress: host_truss_results
            .iter()
            .map(|truss| truss.stress.abs())
            .fold(0.0_f64, f64::max),
        max_host_plane_stress: model
            .host_plane_triangles
            .iter()
            .map(|element| element.result(displacements).von_mises.abs())
            .chain(
                model
                    .host_plane_quads
                    .iter()
                    .map(|element| element.result(displacements).von_mises.abs()),
            )
            .fold(0.0_f64, f64::max),
    }
}

pub(crate) fn assemble(
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
    for truss in &model.host_trusses {
        truss.assemble(displacements, &mut internal_forces, &mut tangent);
    }
    for element in &model.host_plane_triangles {
        element.assemble(displacements, &mut internal_forces, &mut tangent);
    }
    for element in &model.host_plane_quads {
        element.assemble(displacements, &mut internal_forces, &mut tangent);
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
