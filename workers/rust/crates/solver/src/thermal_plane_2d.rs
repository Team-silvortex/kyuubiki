use std::time::Instant;

use crate::linear_algebra::{SparseMatrix, add_at};
use crate::linear_solver_profile::SpdSolveOptions;
use crate::plane_2d_math::{
    PlaneTriangleComputed, precompute_plane_triangle_element_from_nodes,
    thermal_plane_triangle_equivalent_load,
};
use crate::thermal_plane_2d_profile::{
    ThermalPlaneQuadProfile, ThermalPlaneTriangleProfile, push_thermal_plane_stage,
};
use crate::thermal_plane_2d_results::{
    build_thermal_plane_nodes, max_temperature_delta, max_thermal_plane_displacement,
    max_thermal_quad_strain_energy_density, max_thermal_quad_stress,
    max_thermal_triangle_strain_energy_density, max_thermal_triangle_stress,
    thermal_plane_triangle_state, thermal_quad_total_strain_energy,
    thermal_triangle_total_strain_energy,
};
use crate::thermal_plane_2d_solve::solve_thermal_plane_displacements;
use crate::thermal_plane_2d_util::{
    build_force_vector, build_quad_force_vector, to_plane_nodes, to_triangle_request,
    triangle_displacements, triangle_dof_map,
};
use crate::thermal_plane_2d_validation::{
    validate_thermal_plane_quad_request, validate_thermal_plane_triangle_request,
};
use kyuubiki_protocol::{
    PlaneTriangleElementInput, SolveThermalPlaneQuad2dRequest, SolveThermalPlaneQuad2dResult,
    SolveThermalPlaneTriangle2dRequest, SolveThermalPlaneTriangle2dResult,
    ThermalPlaneQuadElementInput, ThermalPlaneQuadElementResult, ThermalPlaneTriangleElementInput,
    ThermalPlaneTriangleElementResult,
};

#[derive(Debug, Clone)]
pub(crate) struct ThermalPlaneTriangleComputed {
    pub(crate) stiffness: [[f64; 6]; 6],
    pub(crate) area: f64,
    pub(crate) b_matrix: [[f64; 6]; 3],
    pub(crate) d_matrix: [[f64; 3]; 3],
    pub(crate) average_temperature_delta: f64,
}

#[derive(Debug, Clone)]
struct ThermalPlaneQuadComputed {
    first: ThermalPlaneTriangleComputed,
    second: ThermalPlaneTriangleComputed,
}

pub fn solve_thermal_plane_triangle_2d(
    request: &SolveThermalPlaneTriangle2dRequest,
) -> Result<SolveThermalPlaneTriangle2dResult, String> {
    solve_thermal_plane_triangle_2d_internal(request, false, SpdSolveOptions::default())
        .map(|profile| profile.result)
}

pub fn profile_thermal_plane_triangle_2d_with_options(
    request: &SolveThermalPlaneTriangle2dRequest,
    options: SpdSolveOptions,
) -> Result<ThermalPlaneTriangleProfile, String> {
    solve_thermal_plane_triangle_2d_internal(request, true, options)
}

fn solve_thermal_plane_triangle_2d_internal(
    request: &SolveThermalPlaneTriangle2dRequest,
    collect_stages: bool,
    solve_options: SpdSolveOptions,
) -> Result<ThermalPlaneTriangleProfile, String> {
    validate_thermal_plane_triangle_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::with_uniform_row_capacity(dof_count, 18);
    let mut force_vector = build_force_vector(request);
    let mut stages = Vec::new();
    let mut stage_started = Instant::now();
    let plane_nodes = to_plane_nodes(request);
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_thermal_plane_triangle_element(&plane_nodes, request, element))
        .collect::<Result<Vec<_>, String>>()?;
    push_thermal_plane_stage(
        &mut stages,
        collect_stages,
        "precompute",
        stage_started.elapsed(),
    );

    stage_started = Instant::now();
    for (element, computed) in request.elements.iter().zip(computed_elements.iter()) {
        assemble_thermal_triangle(element, computed, &mut global_stiffness, &mut force_vector);
    }
    push_thermal_plane_stage(
        &mut stages,
        collect_stages,
        "assemble_global",
        stage_started.elapsed(),
    );

    stage_started = Instant::now();
    let solved = solve_thermal_plane_displacements(
        request,
        &global_stiffness,
        &force_vector,
        solve_options,
        &mut stages,
        collect_stages,
    )?;
    push_thermal_plane_stage(
        &mut stages,
        collect_stages,
        "solve_total",
        stage_started.elapsed(),
    );

    stage_started = Instant::now();
    let displacements = solved.displacements;
    let nodes = build_thermal_plane_nodes(request, &displacements);
    let elements = build_thermal_triangle_elements(request, &computed_elements, &displacements);
    let total_strain_energy = thermal_triangle_total_strain_energy(request, &elements);
    let max_strain_energy_density = max_thermal_triangle_strain_energy_density(&elements);
    push_thermal_plane_stage(
        &mut stages,
        collect_stages,
        "build_result",
        stage_started.elapsed(),
    );

    Ok(ThermalPlaneTriangleProfile {
        result: SolveThermalPlaneTriangle2dResult {
            input: request.clone(),
            max_displacement: max_thermal_plane_displacement(&nodes),
            max_stress: max_thermal_triangle_stress(&elements),
            max_temperature_delta: max_temperature_delta(&nodes),
            total_strain_energy,
            max_strain_energy_density,
            nodes,
            elements,
        },
        stages,
        solver_iterations: solved.solver_iterations,
        solver_matrix_non_zero_count: solved.solver_matrix_non_zero_count,
        solver_residual_norm: solved.solver_residual_norm,
    })
}

pub fn solve_thermal_plane_quad_2d(
    request: &SolveThermalPlaneQuad2dRequest,
) -> Result<SolveThermalPlaneQuad2dResult, String> {
    solve_thermal_plane_quad_2d_internal(request, false, SpdSolveOptions::default())
        .map(|profile| profile.result)
}

pub fn profile_thermal_plane_quad_2d_with_options(
    request: &SolveThermalPlaneQuad2dRequest,
    options: SpdSolveOptions,
) -> Result<ThermalPlaneQuadProfile, String> {
    solve_thermal_plane_quad_2d_internal(request, true, options)
}

fn solve_thermal_plane_quad_2d_internal(
    request: &SolveThermalPlaneQuad2dRequest,
    collect_stages: bool,
    solve_options: SpdSolveOptions,
) -> Result<ThermalPlaneQuadProfile, String> {
    validate_thermal_plane_quad_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::with_uniform_row_capacity(dof_count, 24);
    let mut force_vector = build_quad_force_vector(request);
    let mut stages = Vec::new();
    let mut stage_started = Instant::now();
    let triangle_request = to_triangle_request(request);
    let plane_nodes = to_plane_nodes(&triangle_request);
    let computed_elements = request
        .elements
        .iter()
        .map(|element| {
            precompute_thermal_plane_quad_element(&triangle_request, &plane_nodes, element)
        })
        .collect::<Result<Vec<_>, String>>()?;
    push_thermal_plane_stage(
        &mut stages,
        collect_stages,
        "precompute",
        stage_started.elapsed(),
    );

    stage_started = Instant::now();
    for (element, computed) in request.elements.iter().zip(computed_elements.iter()) {
        for (nodes, triangle) in [
            (
                [element.node_i, element.node_j, element.node_k],
                &computed.first,
            ),
            (
                [element.node_i, element.node_k, element.node_l],
                &computed.second,
            ),
        ] {
            assemble_thermal_triangle_nodes(
                nodes,
                triangle,
                element.thickness,
                element.thermal_expansion,
                &mut global_stiffness,
                &mut force_vector,
            );
        }
    }
    push_thermal_plane_stage(
        &mut stages,
        collect_stages,
        "assemble_global",
        stage_started.elapsed(),
    );

    stage_started = Instant::now();
    let solved = solve_thermal_plane_displacements(
        &triangle_request,
        &global_stiffness,
        &force_vector,
        solve_options,
        &mut stages,
        collect_stages,
    )?;
    push_thermal_plane_stage(
        &mut stages,
        collect_stages,
        "solve_total",
        stage_started.elapsed(),
    );

    stage_started = Instant::now();
    let displacements = solved.displacements;
    let nodes = build_thermal_plane_nodes(&triangle_request, &displacements);
    let elements = build_thermal_quad_elements(request, &computed_elements, &displacements);
    let total_strain_energy = thermal_quad_total_strain_energy(request, &elements);
    let max_strain_energy_density = max_thermal_quad_strain_energy_density(&elements);
    push_thermal_plane_stage(
        &mut stages,
        collect_stages,
        "build_result",
        stage_started.elapsed(),
    );

    Ok(ThermalPlaneQuadProfile {
        result: SolveThermalPlaneQuad2dResult {
            input: request.clone(),
            max_displacement: max_thermal_plane_displacement(&nodes),
            max_stress: max_thermal_quad_stress(&elements),
            max_temperature_delta: max_temperature_delta(&nodes),
            total_strain_energy,
            max_strain_energy_density,
            nodes,
            elements,
        },
        stages,
        solver_iterations: solved.solver_iterations,
        solver_matrix_non_zero_count: solved.solver_matrix_non_zero_count,
        solver_residual_norm: solved.solver_residual_norm,
    })
}

fn assemble_thermal_triangle(
    element: &ThermalPlaneTriangleElementInput,
    computed: &ThermalPlaneTriangleComputed,
    global_stiffness: &mut SparseMatrix,
    force_vector: &mut [f64],
) {
    assemble_thermal_triangle_nodes(
        [element.node_i, element.node_j, element.node_k],
        computed,
        element.thickness,
        element.thermal_expansion,
        global_stiffness,
        force_vector,
    );
}

fn assemble_thermal_triangle_nodes(
    nodes: [usize; 3],
    computed: &ThermalPlaneTriangleComputed,
    thickness: f64,
    thermal_expansion: f64,
    global_stiffness: &mut SparseMatrix,
    force_vector: &mut [f64],
) {
    let map = triangle_dof_map(nodes[0], nodes[1], nodes[2]);
    let equivalent_load = thermal_plane_triangle_equivalent_load(
        &computed.b_matrix,
        &computed.d_matrix,
        computed.area,
        thickness,
        thermal_expansion,
        computed.average_temperature_delta,
    );

    for row in 0..6 {
        force_vector[map[row]] += equivalent_load[row];
        for column in 0..6 {
            add_at(
                global_stiffness,
                map[row],
                map[column],
                computed.stiffness[row][column],
            );
        }
    }
}

fn build_thermal_triangle_elements(
    request: &SolveThermalPlaneTriangle2dRequest,
    computed_elements: &[ThermalPlaneTriangleComputed],
    displacements: &[f64],
) -> Vec<ThermalPlaneTriangleElementResult> {
    request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let element_displacements = triangle_displacements(
                displacements,
                element.node_i,
                element.node_j,
                element.node_k,
            );
            let state = thermal_plane_triangle_state(
                computed,
                &element_displacements,
                element.thermal_expansion,
            );

            ThermalPlaneTriangleElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                area: computed.area,
                average_temperature_delta: computed.average_temperature_delta,
                thermal_strain: state.thermal_strain,
                mechanical_strain_x: state.mechanical_strain[0],
                mechanical_strain_y: state.mechanical_strain[1],
                total_strain_x: state.total_strain[0],
                total_strain_y: state.total_strain[1],
                gamma_xy: state.total_strain[2],
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
        .collect()
}

fn build_thermal_quad_elements(
    request: &SolveThermalPlaneQuad2dRequest,
    computed_elements: &[ThermalPlaneQuadComputed],
    displacements: &[f64],
) -> Vec<ThermalPlaneQuadElementResult> {
    request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let first_state = thermal_plane_triangle_state(
                &computed.first,
                &triangle_displacements(
                    displacements,
                    element.node_i,
                    element.node_j,
                    element.node_k,
                ),
                element.thermal_expansion,
            );
            let second_state = thermal_plane_triangle_state(
                &computed.second,
                &triangle_displacements(
                    displacements,
                    element.node_i,
                    element.node_k,
                    element.node_l,
                ),
                element.thermal_expansion,
            );
            let total_area = computed.first.area + computed.second.area;
            let weighted = |left: f64, right: f64| -> f64 {
                ((left * computed.first.area) + (right * computed.second.area)) / total_area
            };

            ThermalPlaneQuadElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                node_l: element.node_l,
                area: total_area,
                average_temperature_delta: weighted(
                    computed.first.average_temperature_delta,
                    computed.second.average_temperature_delta,
                ),
                thermal_strain: weighted(first_state.thermal_strain, second_state.thermal_strain),
                mechanical_strain_x: weighted(
                    first_state.mechanical_strain[0],
                    second_state.mechanical_strain[0],
                ),
                mechanical_strain_y: weighted(
                    first_state.mechanical_strain[1],
                    second_state.mechanical_strain[1],
                ),
                total_strain_x: weighted(first_state.total_strain[0], second_state.total_strain[0]),
                total_strain_y: weighted(first_state.total_strain[1], second_state.total_strain[1]),
                gamma_xy: weighted(first_state.total_strain[2], second_state.total_strain[2]),
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

fn precompute_thermal_plane_triangle_element(
    plane_nodes: &[kyuubiki_protocol::PlaneNodeInput],
    request: &SolveThermalPlaneTriangle2dRequest,
    element: &ThermalPlaneTriangleElementInput,
) -> Result<ThermalPlaneTriangleComputed, String> {
    let plane_element = PlaneTriangleElementInput {
        id: element.id.clone(),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        thickness: element.thickness,
        youngs_modulus: element.youngs_modulus,
        poisson_ratio: element.poisson_ratio,
    };
    let PlaneTriangleComputed {
        stiffness,
        area,
        b_matrix,
        d_matrix,
    } = precompute_plane_triangle_element_from_nodes(plane_nodes, &plane_element)?;
    let average_temperature_delta = (request.nodes[element.node_i].temperature_delta
        + request.nodes[element.node_j].temperature_delta
        + request.nodes[element.node_k].temperature_delta)
        / 3.0;

    Ok(ThermalPlaneTriangleComputed {
        stiffness,
        area,
        b_matrix,
        d_matrix,
        average_temperature_delta,
    })
}

fn precompute_thermal_plane_quad_element(
    triangle_request: &SolveThermalPlaneTriangle2dRequest,
    plane_nodes: &[kyuubiki_protocol::PlaneNodeInput],
    element: &ThermalPlaneQuadElementInput,
) -> Result<ThermalPlaneQuadComputed, String> {
    let first = ThermalPlaneTriangleElementInput {
        id: format!("{}#0", element.id),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        thickness: element.thickness,
        youngs_modulus: element.youngs_modulus,
        poisson_ratio: element.poisson_ratio,
        thermal_expansion: element.thermal_expansion,
    };
    let second = ThermalPlaneTriangleElementInput {
        id: format!("{}#1", element.id),
        node_i: element.node_i,
        node_j: element.node_k,
        node_k: element.node_l,
        thickness: element.thickness,
        youngs_modulus: element.youngs_modulus,
        poisson_ratio: element.poisson_ratio,
        thermal_expansion: element.thermal_expansion,
    };
    Ok(ThermalPlaneQuadComputed {
        first: precompute_thermal_plane_triangle_element(plane_nodes, triangle_request, &first)?,
        second: precompute_thermal_plane_triangle_element(plane_nodes, triangle_request, &second)?,
    })
}
