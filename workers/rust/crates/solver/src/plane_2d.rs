use std::time::Instant;

use crate::linear_algebra::{SparseMatrix, add_at};
use crate::linear_solver_profile::SpdSolveOptions;
use crate::plane_2d_math::{
    PlaneTriangleComputed, plane_triangle_state, precompute_plane_triangle_element,
    precompute_plane_triangle_element_from_nodes, signed_triangle_area,
};
use crate::plane_2d_profile::{
    PlaneProfileStage, PlaneQuadProfile, PlaneTriangleProfile,
    profile_plane_displacements_with_options, push_plane_profile_stage,
};
use crate::plane_2d_summary::{
    max_plane_displacement, max_quad_strain_energy_density, max_quad_stress,
    max_triangle_strain_energy_density, max_triangle_stress, quad_total_strain_energy,
    triangle_total_strain_energy,
};
use kyuubiki_protocol::{
    PlaneNodeResult, PlaneQuadElementInput, PlaneQuadElementResult, PlaneTriangleElementInput,
    PlaneTriangleElementResult, SolvePlaneQuad2dRequest, SolvePlaneQuad2dResult,
    SolvePlaneTriangle2dRequest, SolvePlaneTriangle2dResult,
};

#[derive(Debug, Clone)]
struct PlaneQuadComputed {
    first: PlaneTriangleComputed,
    second: PlaneTriangleComputed,
}

pub fn solve_plane_triangle_2d(
    request: &SolvePlaneTriangle2dRequest,
) -> Result<SolvePlaneTriangle2dResult, String> {
    solve_plane_triangle_2d_internal(request, false, SpdSolveOptions::default())
        .map(|profile| profile.result)
}

pub fn profile_plane_triangle_2d_with_options(
    request: &SolvePlaneTriangle2dRequest,
    solve_options: SpdSolveOptions,
) -> Result<PlaneTriangleProfile, String> {
    solve_plane_triangle_2d_internal(request, true, solve_options)
}

fn solve_plane_triangle_2d_internal(
    request: &SolvePlaneTriangle2dRequest,
    collect_stages: bool,
    solve_options: SpdSolveOptions,
) -> Result<PlaneTriangleProfile, String> {
    validate_plane_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];
    let mut stages = Vec::new();
    let mut stage_started = Instant::now();
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_plane_triangle_element(request, element))
        .collect::<Result<Vec<_>, String>>()?;
    push_plane_profile_stage(&mut stages, collect_stages, "precompute", stage_started);

    stage_started = Instant::now();
    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_x;
        force_vector[index * 2 + 1] = node.load_y;
    }

    for (element, computed) in request.elements.iter().zip(computed_elements.iter()) {
        let map = triangle_dof_map(element.node_i, element.node_j, element.node_k);
        for row in 0..6 {
            for column in 0..6 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    computed.stiffness[row][column],
                );
            }
        }
    }
    push_plane_profile_stage(
        &mut stages,
        collect_stages,
        "assemble_global",
        stage_started,
    );

    stage_started = Instant::now();
    let displacement_profile = profile_plane_displacements_with_options(
        request,
        &global_stiffness,
        &force_vector,
        solve_options,
    )?;
    let displacements = displacement_profile.displacements;
    push_plane_profile_stage(&mut stages, collect_stages, "solve_system", stage_started);
    if collect_stages {
        stages.extend(
            displacement_profile
                .stages
                .into_iter()
                .map(|stage| PlaneProfileStage {
                    label: stage.label,
                    rss_kib: stage.rss_kib,
                    elapsed_ms: stage.elapsed_ms,
                }),
        );
    }

    stage_started = Instant::now();
    let nodes = build_plane_nodes(request, &displacements);
    let elements = request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let element_displacements = triangle_displacements(
                &displacements,
                element.node_i,
                element.node_j,
                element.node_k,
            );
            let state = plane_triangle_state(computed, &element_displacements);

            PlaneTriangleElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                area: computed.area,
                strain_x: state.strain[0],
                strain_y: state.strain[1],
                gamma_xy: state.strain[2],
                stress_x: state.stress[0],
                stress_y: state.stress[1],
                tau_xy: state.stress[2],
                principal_stress_1: state.principal_stress_1,
                principal_stress_2: state.principal_stress_2,
                max_in_plane_shear: state.max_in_plane_shear,
                von_mises: state.von_mises,
                strain_energy_density: state.strain_energy_density,
            }
        })
        .collect::<Vec<_>>();
    let total_strain_energy = triangle_total_strain_energy(&elements, &request.elements);
    let max_strain_energy_density = max_triangle_strain_energy_density(&elements);
    push_plane_profile_stage(&mut stages, collect_stages, "assemble", stage_started);

    Ok(PlaneTriangleProfile {
        result: SolvePlaneTriangle2dResult {
            input: request.clone(),
            max_displacement: max_plane_displacement(&nodes),
            max_stress: max_triangle_stress(&elements),
            total_strain_energy,
            max_strain_energy_density,
            nodes,
            elements,
        },
        solver_iterations: displacement_profile.solver_iterations,
        solver_matrix_non_zero_count: displacement_profile.solver_matrix_non_zero_count,
        solver_residual_norm: displacement_profile.solver_residual_norm,
        stages,
    })
}

pub fn solve_plane_quad_2d(
    request: &SolvePlaneQuad2dRequest,
) -> Result<SolvePlaneQuad2dResult, String> {
    solve_plane_quad_2d_internal(request, false, SpdSolveOptions::default())
        .map(|profile| profile.result)
}

pub fn profile_plane_quad_2d(
    request: &SolvePlaneQuad2dRequest,
) -> Result<PlaneQuadProfile, String> {
    profile_plane_quad_2d_with_options(request, SpdSolveOptions::default())
}

pub fn profile_plane_quad_2d_with_options(
    request: &SolvePlaneQuad2dRequest,
    solve_options: SpdSolveOptions,
) -> Result<PlaneQuadProfile, String> {
    solve_plane_quad_2d_internal(request, true, solve_options)
}

fn solve_plane_quad_2d_internal(
    request: &SolvePlaneQuad2dRequest,
    collect_stages: bool,
    solve_options: SpdSolveOptions,
) -> Result<PlaneQuadProfile, String> {
    validate_plane_quad_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];
    let mut stages = Vec::new();
    let mut stage_started = Instant::now();
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_plane_quad_element(request, element))
        .collect::<Result<Vec<_>, String>>()?;
    push_plane_profile_stage(&mut stages, collect_stages, "precompute", stage_started);

    stage_started = Instant::now();
    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_x;
        force_vector[index * 2 + 1] = node.load_y;
    }

    for (element, computed) in request.elements.iter().zip(computed_elements.iter()) {
        let triangles = [
            (
                [element.node_i, element.node_j, element.node_k],
                &computed.first,
            ),
            (
                [element.node_i, element.node_k, element.node_l],
                &computed.second,
            ),
        ];
        for (nodes, triangle) in triangles {
            let map = triangle_dof_map(nodes[0], nodes[1], nodes[2]);
            for row in 0..6 {
                for column in 0..6 {
                    add_at(
                        &mut global_stiffness,
                        map[row],
                        map[column],
                        triangle.stiffness[row][column],
                    );
                }
            }
        }
    }
    push_plane_profile_stage(
        &mut stages,
        collect_stages,
        "assemble_global",
        stage_started,
    );

    let triangle_request = to_triangle_request(request);
    stage_started = Instant::now();
    let displacement_profile = profile_plane_displacements_with_options(
        &triangle_request,
        &global_stiffness,
        &force_vector,
        solve_options,
    )?;
    let displacements = displacement_profile.displacements;
    push_plane_profile_stage(&mut stages, collect_stages, "solve_system", stage_started);
    if collect_stages {
        stages.extend(
            displacement_profile
                .stages
                .into_iter()
                .map(|stage| PlaneProfileStage {
                    label: stage.label,
                    rss_kib: stage.rss_kib,
                    elapsed_ms: stage.elapsed_ms,
                }),
        );
    }

    stage_started = Instant::now();
    let nodes = build_plane_nodes(&triangle_request, &displacements);
    let elements = build_plane_quad_elements(request, &computed_elements, &displacements);
    let total_strain_energy = quad_total_strain_energy(&elements, &request.elements);
    let max_strain_energy_density = max_quad_strain_energy_density(&elements);
    push_plane_profile_stage(&mut stages, collect_stages, "assemble", stage_started);

    Ok(PlaneQuadProfile {
        result: SolvePlaneQuad2dResult {
            input: request.clone(),
            max_displacement: max_plane_displacement(&nodes),
            max_stress: max_quad_stress(&elements),
            total_strain_energy,
            max_strain_energy_density,
            nodes,
            elements,
        },
        solver_iterations: displacement_profile.solver_iterations,
        solver_matrix_non_zero_count: displacement_profile.solver_matrix_non_zero_count,
        solver_residual_norm: displacement_profile.solver_residual_norm,
        stages,
    })
}

fn build_plane_quad_elements(
    request: &SolvePlaneQuad2dRequest,
    computed_elements: &[PlaneQuadComputed],
    displacements: &[f64],
) -> Vec<PlaneQuadElementResult> {
    request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let first_state = plane_triangle_state(
                &computed.first,
                &triangle_displacements(
                    displacements,
                    element.node_i,
                    element.node_j,
                    element.node_k,
                ),
            );
            let second_state = plane_triangle_state(
                &computed.second,
                &triangle_displacements(
                    displacements,
                    element.node_i,
                    element.node_k,
                    element.node_l,
                ),
            );
            let total_area = computed.first.area + computed.second.area;
            let weighted = |left: f64, right: f64| -> f64 {
                ((left * computed.first.area) + (right * computed.second.area)) / total_area
            };

            PlaneQuadElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                node_l: element.node_l,
                area: total_area,
                strain_x: weighted(first_state.strain[0], second_state.strain[0]),
                strain_y: weighted(first_state.strain[1], second_state.strain[1]),
                gamma_xy: weighted(first_state.strain[2], second_state.strain[2]),
                stress_x: weighted(first_state.stress[0], second_state.stress[0]),
                stress_y: weighted(first_state.stress[1], second_state.stress[1]),
                tau_xy: weighted(first_state.stress[2], second_state.stress[2]),
                principal_stress_1: weighted(
                    first_state.principal_stress_1,
                    second_state.principal_stress_1,
                ),
                principal_stress_2: weighted(
                    first_state.principal_stress_2,
                    second_state.principal_stress_2,
                ),
                max_in_plane_shear: weighted(
                    first_state.max_in_plane_shear,
                    second_state.max_in_plane_shear,
                ),
                von_mises: weighted(first_state.von_mises, second_state.von_mises),
                strain_energy_density: weighted(
                    first_state.strain_energy_density,
                    second_state.strain_energy_density,
                ),
            }
        })
        .collect()
}

fn validate_plane_request(request: &SolvePlaneTriangle2dRequest) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err("plane model must define at least three nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("plane model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("plane model must include at least one support".to_string());
    }

    for element in &request.elements {
        validate_plane_triangle_element(request, element)?;
    }
    Ok(())
}

fn validate_plane_quad_request(request: &SolvePlaneQuad2dRequest) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("plane quad model must define at least four nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("plane quad model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("plane quad model must include at least one support".to_string());
    }

    let triangle_request = to_triangle_request(request);
    for element in &request.elements {
        let indices = [
            element.node_i,
            element.node_j,
            element.node_k,
            element.node_l,
        ];
        if indices.iter().any(|&index| index >= request.nodes.len()) {
            return Err("plane quad element references an out-of-range node".to_string());
        }
        let unique_count = indices
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        if unique_count < 4 {
            return Err("plane quad element must reference four distinct nodes".to_string());
        }
        validate_plane_material(
            element.thickness,
            element.youngs_modulus,
            element.poisson_ratio,
        )?;
        validate_positive_triangle_area(
            &triangle_request,
            element.node_i,
            element.node_j,
            element.node_k,
            "plane quad element must decompose into positive-area triangles",
        )?;
        validate_positive_triangle_area(
            &triangle_request,
            element.node_i,
            element.node_k,
            element.node_l,
            "plane quad element must decompose into positive-area triangles",
        )?;
    }
    Ok(())
}

fn validate_plane_triangle_element(
    request: &SolvePlaneTriangle2dRequest,
    element: &PlaneTriangleElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len()
        || element.node_j >= request.nodes.len()
        || element.node_k >= request.nodes.len()
    {
        return Err("plane element references an out-of-range node".to_string());
    }
    validate_plane_material(
        element.thickness,
        element.youngs_modulus,
        element.poisson_ratio,
    )?;
    validate_positive_triangle_area(
        request,
        element.node_i,
        element.node_j,
        element.node_k,
        "plane element area must be positive",
    )
}

fn validate_plane_material(
    thickness: f64,
    youngs_modulus: f64,
    poisson_ratio: f64,
) -> Result<(), String> {
    if !(thickness.is_finite() && thickness > 0.0) {
        return Err("plane element thickness must be positive".to_string());
    }
    if !(youngs_modulus.is_finite() && youngs_modulus > 0.0) {
        return Err("plane element youngs_modulus must be positive".to_string());
    }
    if !(poisson_ratio.is_finite() && poisson_ratio > -1.0 && poisson_ratio < 0.5) {
        return Err("plane element poisson_ratio must be between -1.0 and 0.5".to_string());
    }
    Ok(())
}

fn precompute_plane_quad_element(
    request: &SolvePlaneQuad2dRequest,
    element: &PlaneQuadElementInput,
) -> Result<PlaneQuadComputed, String> {
    let first = PlaneTriangleElementInput {
        id: format!("{}#0", element.id),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        thickness: element.thickness,
        youngs_modulus: element.youngs_modulus,
        poisson_ratio: element.poisson_ratio,
    };
    let second = PlaneTriangleElementInput {
        id: format!("{}#1", element.id),
        node_i: element.node_i,
        node_j: element.node_k,
        node_k: element.node_l,
        thickness: element.thickness,
        youngs_modulus: element.youngs_modulus,
        poisson_ratio: element.poisson_ratio,
    };
    Ok(PlaneQuadComputed {
        first: precompute_plane_triangle_element_from_nodes(&request.nodes, &first)?,
        second: precompute_plane_triangle_element_from_nodes(&request.nodes, &second)?,
    })
}

fn build_plane_nodes(
    request: &SolvePlaneTriangle2dRequest,
    displacements: &[f64],
) -> Vec<PlaneNodeResult> {
    request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let ux = displacements[index * 2];
            let uy = displacements[index * 2 + 1];
            PlaneNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                ux,
                uy,
                displacement_magnitude: (ux * ux + uy * uy).sqrt(),
            }
        })
        .collect()
}

fn to_triangle_request(request: &SolvePlaneQuad2dRequest) -> SolvePlaneTriangle2dRequest {
    SolvePlaneTriangle2dRequest {
        nodes: request.nodes.clone(),
        elements: vec![],
    }
}

fn triangle_dof_map(node_i: usize, node_j: usize, node_k: usize) -> [usize; 6] {
    [
        node_i * 2,
        node_i * 2 + 1,
        node_j * 2,
        node_j * 2 + 1,
        node_k * 2,
        node_k * 2 + 1,
    ]
}

fn triangle_displacements(
    displacements: &[f64],
    node_i: usize,
    node_j: usize,
    node_k: usize,
) -> [f64; 6] {
    let map = triangle_dof_map(node_i, node_j, node_k);
    std::array::from_fn(|index| displacements[map[index]])
}

fn validate_positive_triangle_area(
    request: &SolvePlaneTriangle2dRequest,
    node_i: usize,
    node_j: usize,
    node_k: usize,
    message: &str,
) -> Result<(), String> {
    let area = signed_triangle_area(
        &request.nodes[node_i],
        &request.nodes[node_j],
        &request.nodes[node_k],
    )
    .abs();
    if area <= 1.0e-12 {
        return Err(message.to_string());
    }
    Ok(())
}
