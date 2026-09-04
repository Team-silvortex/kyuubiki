use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use kyuubiki_protocol::{
    CohesiveInterfaceMesh3dElementInput, CohesiveInterfaceMesh3dElementResult,
    CohesiveInterfaceMesh3dLoadStepResult, CohesiveInterfaceMesh3dNodeResult,
    SolveCohesiveInterfaceMesh3dRequest, SolveCohesiveInterfaceMesh3dResult,
};

use crate::cohesive_interface_1d::validate_id;
use crate::cohesive_interface_3d::{
    CohesiveInterface3dEvaluation, CohesiveInterface3dKernel, CohesiveInterface3dState,
};
use crate::cohesive_interface_mesh_3d_control::{
    ControlStep, build_controls, restricted_norm, vector_norm,
};
use crate::cohesive_interface_mesh_3d_newton::solve_load_step;
use crate::cohesive_interface_mesh_3d_solid::HostTetra;
use crate::linear_algebra::{SparseMatrix, add_at};

const DEFAULT_MAX_ITERATIONS: usize = 30;
const DEFAULT_TOLERANCE: f64 = 1.0e-9;
const MAX_ITERATIONS: usize = 200;
const MAX_NODES: usize = 512;
const MAX_MATERIALS: usize = 256;
const MAX_ELEMENTS: usize = 4096;
const MAX_HOST_TETRAHEDRA: usize = 4096;

pub fn solve_cohesive_interface_mesh_3d(
    request: &SolveCohesiveInterfaceMesh3dRequest,
) -> Result<SolveCohesiveInterfaceMesh3dResult, String> {
    solve_cohesive_interface_mesh_3d_internal(Cow::Borrowed(request))
}

pub fn solve_cohesive_interface_mesh_3d_owned(
    request: SolveCohesiveInterfaceMesh3dRequest,
) -> Result<SolveCohesiveInterfaceMesh3dResult, String> {
    solve_cohesive_interface_mesh_3d_internal(Cow::Owned(request))
}

fn solve_cohesive_interface_mesh_3d_internal(
    request: Cow<'_, SolveCohesiveInterfaceMesh3dRequest>,
) -> Result<SolveCohesiveInterfaceMesh3dResult, String> {
    let model = ValidatedModel::new(request.as_ref())?;
    let mut displacements = vec![0.0; model.dof_count];
    let mut states = vec![CohesiveInterface3dState::default(); model.elements.len()];
    let mut steps = Vec::with_capacity(model.controls.len());
    let mut completed_load_factor = 0.0;
    let mut residual_norm = 0.0;
    let mut failure_reason = None;

    for (step_index, control) in model.controls.iter().enumerate() {
        let outcome = solve_load_step(&model, control, &displacements, &states);
        residual_norm = outcome.residual_norm;
        let summary_assembly = assemble(&model, &outcome.displacements, &outcome.states);
        let summary = step_summary(&model, control, &outcome.displacements, &summary_assembly);
        steps.push(CohesiveInterfaceMesh3dLoadStepResult {
            step: step_index,
            load_factor: control.load_factor,
            iterations: outcome.iterations,
            residual_norm,
            converged: outcome.converged,
            max_displacement: summary.max_displacement,
            prescribed_displacement_norm: vector_norm(&control.prescribed_displacements),
            reaction_norm: summary.reaction_norm,
            max_resultant_traction: summary.max_resultant_traction,
            max_tangential_damage: summary.max_tangential_damage,
            max_normal_damage: summary.max_normal_damage,
            max_host_von_mises_stress: summary.max_host_von_mises_stress,
            tangent_non_zero_count: outcome.tangent_non_zero_count,
            tangent_fill_ratio: outcome.tangent_fill_ratio,
            linear_solver: outcome.linear_solver,
        });
        if !outcome.converged {
            failure_reason = outcome.failure_reason;
            break;
        }
        displacements = outcome.displacements;
        states = outcome.states;
        completed_load_factor = control.load_factor;
    }

    let final_assembly = assemble(&model, &displacements, &states);
    let reactions = reactions(
        &model,
        completed_load_factor,
        &final_assembly.internal_forces,
    );
    let nodes = node_results(request.as_ref(), &displacements, &reactions);
    let elements = element_results(&model, &final_assembly.evaluations);
    let host_tetrahedra = model
        .host_tetrahedra
        .iter()
        .map(|element| element.result(&displacements))
        .collect::<Vec<_>>();
    let converged = failure_reason.is_none() && steps.len() == model.controls.len();
    let max_displacement = max_step_or(
        &steps,
        |step| step.max_displacement,
        nodes
            .iter()
            .map(|node| norm(node.displacement))
            .fold(0.0_f64, f64::max),
    );
    let max_resultant_traction = max_step_or(
        &steps,
        |step| step.max_resultant_traction,
        max_element_traction(&elements),
    );
    let max_tangential_damage = max_step_or(
        &steps,
        |step| step.max_tangential_damage,
        elements
            .iter()
            .map(|element| element.max_tangential_damage)
            .fold(0.0_f64, f64::max),
    );
    let max_normal_damage = max_step_or(
        &steps,
        |step| step.max_normal_damage,
        elements
            .iter()
            .map(|element| element.max_normal_damage)
            .fold(0.0_f64, f64::max),
    );
    let max_host_von_mises_stress = max_step_or(
        &steps,
        |step| step.max_host_von_mises_stress,
        host_tetrahedra
            .iter()
            .map(|element| element.von_mises_stress)
            .fold(0.0_f64, f64::max),
    );
    let max_tangent_non_zero_count = steps
        .iter()
        .map(|step| step.tangent_non_zero_count)
        .max()
        .unwrap_or(0);
    let max_tangent_fill_ratio = steps
        .iter()
        .map(|step| step.tangent_fill_ratio)
        .fold(0.0_f64, f64::max);
    let mut linear_solver_methods = Vec::new();
    for method in steps
        .iter()
        .map(|step| step.linear_solver.as_str())
        .filter(|method| *method != "none")
    {
        if !linear_solver_methods.iter().any(|known| known == method) {
            linear_solver_methods.push(method.to_string());
        }
    }

    drop(model);

    Ok(SolveCohesiveInterfaceMesh3dResult {
        input: request.into_owned(),
        nodes,
        elements,
        host_tetrahedra,
        steps,
        converged,
        completed_load_factor,
        residual_norm,
        max_displacement,
        max_resultant_traction,
        max_tangential_damage,
        max_normal_damage,
        max_host_von_mises_stress,
        max_tangent_non_zero_count,
        max_tangent_fill_ratio,
        linear_solver_methods,
        failure_reason,
    })
}

pub(crate) struct ValidatedModel<'a> {
    elements: Vec<ModelElement<'a>>,
    host_tetrahedra: Vec<HostTetra<'a>>,
    pub(crate) free_dofs: Vec<usize>,
    pub(crate) fixed_dofs: Vec<usize>,
    pub(crate) external_loads: Vec<f64>,
    controls: Vec<ControlStep>,
    node_count: usize,
    dof_count: usize,
    pub(crate) max_iterations: usize,
    pub(crate) tolerance: f64,
}

struct ModelElement<'a> {
    input: &'a CohesiveInterfaceMesh3dElementInput,
    nodes: [usize; 6],
    dofs: [usize; 18],
    kernel: CohesiveInterface3dKernel,
}

impl<'a> ValidatedModel<'a> {
    fn new(request: &'a SolveCohesiveInterfaceMesh3dRequest) -> Result<Self, String> {
        validate_id(&request.id)?;
        if request.nodes.is_empty() || request.nodes.len() > MAX_NODES {
            return Err(format!(
                "cohesive interface mesh 3d requires 1..={MAX_NODES} nodes"
            ));
        }
        if request.materials.is_empty() || request.materials.len() > MAX_MATERIALS {
            return Err(format!(
                "cohesive interface mesh 3d requires 1..={MAX_MATERIALS} materials"
            ));
        }
        if request.elements.is_empty() || request.elements.len() > MAX_ELEMENTS {
            return Err(format!(
                "cohesive interface mesh 3d requires 1..={MAX_ELEMENTS} elements"
            ));
        }
        if request.host_tetrahedra.len() > MAX_HOST_TETRAHEDRA {
            return Err(format!(
                "cohesive interface mesh 3d permits at most {MAX_HOST_TETRAHEDRA} host tetrahedra"
            ));
        }
        validate_unique_ids(request)?;
        for material in &request.materials {
            CohesiveInterface3dKernel::validate_material(&material.properties)?;
        }

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
        let host_tetrahedra = HostTetra::build_all(&request.host_tetrahedra, &request.nodes)?;
        let dof_count = request.nodes.len() * 3;
        let mut free_dofs = Vec::new();
        let mut fixed_dofs = Vec::new();
        let mut external_loads = vec![0.0; dof_count];
        let mut prescribed_displacements = vec![0.0; dof_count];
        for (node_index, node) in request.nodes.iter().enumerate() {
            validate_id(&node.id)?;
            let prescribed = node.prescribed_displacement.unwrap_or([0.0; 3]);
            if !node.x.is_finite()
                || !node.y.is_finite()
                || !node.z.is_finite()
                || node.load.iter().any(|value| !value.is_finite())
                || prescribed.iter().any(|value| !value.is_finite())
            {
                return Err("cohesive interface mesh 3d node data must be finite".to_string());
            }
            for (axis, &prescribed_value) in prescribed.iter().enumerate() {
                let dof = node_index * 3 + axis;
                external_loads[dof] = node.load[axis];
                if node.fixed[axis] {
                    fixed_dofs.push(dof);
                    prescribed_displacements[dof] = prescribed_value;
                } else {
                    if prescribed_value != 0.0 {
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
            return Err("cohesive interface mesh 3d requires constrained dofs".to_string());
        }
        let controls = build_controls(request, &free_dofs, &prescribed_displacements)?;
        let free_load_norm = restricted_norm(&external_loads, &free_dofs);
        if !controls.iter().any(|control| {
            control.load_factor.abs() * free_load_norm > 0.0
                || vector_norm(&control.prescribed_displacements) > 0.0
        }) {
            return Err(
                "cohesive interface mesh 3d requires a free-dof load or prescribed displacement"
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
            host_tetrahedra,
            free_dofs,
            fixed_dofs,
            external_loads,
            controls,
            node_count: request.nodes.len(),
            dof_count,
            max_iterations,
            tolerance,
        })
    }
}

fn validate_unique_ids(request: &SolveCohesiveInterfaceMesh3dRequest) -> Result<(), String> {
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
    for id in request
        .elements
        .iter()
        .map(|element| element.id.as_str())
        .chain(
            request
                .host_tetrahedra
                .iter()
                .map(|element| element.id.as_str()),
        )
    {
        if !element_ids.insert(id) {
            return Err(format!("duplicate cohesive or host element id '{id}'"));
        }
    }
    Ok(())
}

fn build_element<'a>(
    request: &'a SolveCohesiveInterfaceMesh3dRequest,
    element: &'a CohesiveInterfaceMesh3dElementInput,
    material_indices: &HashMap<&str, usize>,
) -> Result<ModelElement<'a>, String> {
    validate_id(&element.id)?;
    let nodes = [
        element.lower_a,
        element.lower_b,
        element.lower_c,
        element.upper_a,
        element.upper_b,
        element.upper_c,
    ];
    if nodes.iter().any(|&index| index >= request.nodes.len()) {
        return Err(format!(
            "element '{}' node index is out of bounds",
            element.id
        ));
    }
    if nodes.into_iter().collect::<HashSet<_>>().len() != 6 {
        return Err(format!(
            "element '{}' requires six distinct nodes",
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
        [node.x, node.y, node.z]
    });
    let kernel = CohesiveInterface3dKernel::new(
        &element.id,
        points,
        &request.materials[material_index].properties,
    )?;
    let dofs = std::array::from_fn(|local_dof| nodes[local_dof / 3] * 3 + local_dof % 3);
    Ok(ModelElement {
        input: element,
        nodes,
        dofs,
        kernel,
    })
}

pub(crate) struct Assembly {
    pub(crate) internal_forces: Vec<f64>,
    pub(crate) tangent: SparseMatrix,
    pub(crate) evaluations: Vec<CohesiveInterface3dEvaluation>,
}

pub(crate) fn assemble(
    model: &ValidatedModel<'_>,
    displacements: &[f64],
    committed_states: &[CohesiveInterface3dState],
) -> Assembly {
    let mut internal_forces = vec![0.0; model.dof_count];
    let mut tangent = SparseMatrix::with_uniform_row_capacity(model.dof_count, 54);
    let mut evaluations = Vec::with_capacity(model.elements.len());
    for (element_index, element) in model.elements.iter().enumerate() {
        let local_displacements = element.nodes.map(|node| {
            [
                displacements[node * 3],
                displacements[node * 3 + 1],
                displacements[node * 3 + 2],
            ]
        });
        let evaluation = element
            .kernel
            .trial(local_displacements, &committed_states[element_index]);
        for local_row in 0..18 {
            let global_row = element.dofs[local_row];
            internal_forces[global_row] +=
                evaluation.step.nodal_internal_forces[local_row / 3][local_row % 3];
            for local_column in 0..18 {
                add_at(
                    &mut tangent,
                    global_row,
                    element.dofs[local_column],
                    evaluation.step.tangent[local_row][local_column],
                );
            }
        }
        evaluations.push(evaluation);
    }
    for element in &model.host_tetrahedra {
        element.assemble(displacements, &mut internal_forces, &mut tangent);
    }
    Assembly {
        internal_forces,
        tangent,
        evaluations,
    }
}

struct StepSummary {
    max_displacement: f64,
    reaction_norm: f64,
    max_resultant_traction: f64,
    max_tangential_damage: f64,
    max_normal_damage: f64,
    max_host_von_mises_stress: f64,
}

fn step_summary(
    model: &ValidatedModel<'_>,
    control: &ControlStep,
    displacements: &[f64],
    assembly: &Assembly,
) -> StepSummary {
    let reactions = reactions(model, control.load_factor, &assembly.internal_forces);
    StepSummary {
        max_displacement: (0..model.node_count)
            .map(|node| {
                norm([
                    displacements[node * 3],
                    displacements[node * 3 + 1],
                    displacements[node * 3 + 2],
                ])
            })
            .fold(0.0_f64, f64::max),
        reaction_norm: vector_norm(&reactions),
        max_resultant_traction: assembly
            .evaluations
            .iter()
            .flat_map(|evaluation| &evaluation.step.integration_points)
            .map(|point| norm(point.local_traction))
            .fold(0.0_f64, f64::max),
        max_tangential_damage: assembly
            .evaluations
            .iter()
            .map(|evaluation| evaluation.step.max_tangential_damage)
            .fold(0.0_f64, f64::max),
        max_normal_damage: assembly
            .evaluations
            .iter()
            .map(|evaluation| evaluation.step.max_normal_damage)
            .fold(0.0_f64, f64::max),
        max_host_von_mises_stress: model
            .host_tetrahedra
            .iter()
            .map(|element| element.result(displacements).von_mises_stress)
            .fold(0.0_f64, f64::max),
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
    request: &SolveCohesiveInterfaceMesh3dRequest,
    displacements: &[f64],
    reactions: &[f64],
) -> Vec<CohesiveInterfaceMesh3dNodeResult> {
    request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| CohesiveInterfaceMesh3dNodeResult {
            id: node.id.clone(),
            displacement: std::array::from_fn(|axis| displacements[index * 3 + axis]),
            reaction: std::array::from_fn(|axis| reactions[index * 3 + axis]),
        })
        .collect()
}

fn element_results(
    model: &ValidatedModel<'_>,
    evaluations: &[CohesiveInterface3dEvaluation],
) -> Vec<CohesiveInterfaceMesh3dElementResult> {
    model
        .elements
        .iter()
        .zip(evaluations)
        .map(|(element, evaluation)| {
            let basis = element.kernel.basis();
            CohesiveInterfaceMesh3dElementResult {
                id: element.input.id.clone(),
                material_id: element.input.material_id.clone(),
                area: element.kernel.area(),
                local_tangent_1_direction: basis[0],
                local_tangent_2_direction: basis[1],
                local_normal_direction: basis[2],
                local_separation: evaluation.step.local_separation,
                local_traction: evaluation.step.local_traction,
                global_traction: evaluation.step.global_traction,
                integration_points: evaluation.step.integration_points.clone(),
                max_tangential_damage: evaluation.step.max_tangential_damage,
                max_normal_damage: evaluation.step.max_normal_damage,
            }
        })
        .collect()
}

fn max_element_traction(elements: &[CohesiveInterfaceMesh3dElementResult]) -> f64 {
    elements
        .iter()
        .flat_map(|element| &element.integration_points)
        .map(|point| norm(point.local_traction))
        .fold(0.0_f64, f64::max)
}

fn max_step_or(
    steps: &[CohesiveInterfaceMesh3dLoadStepResult],
    value: impl Fn(&CohesiveInterfaceMesh3dLoadStepResult) -> f64,
    fallback: f64,
) -> f64 {
    steps.iter().map(value).fold(fallback, f64::max)
}

fn norm(vector: [f64; 3]) -> f64 {
    vector.iter().map(|value| value * value).sum::<f64>().sqrt()
}
