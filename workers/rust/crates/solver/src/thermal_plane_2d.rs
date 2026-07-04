use crate::linear_algebra::{SparseMatrix, add_at, reduce_sparse_system, solve_spd_system};
use crate::plane_2d_math::{
    PlaneTriangleComputed, derive_planar_stress_metrics, multiply_matrix_vector_3x3,
    multiply_matrix_vector_3x6, precompute_plane_triangle_element, subtract_vector_3,
    thermal_plane_triangle_equivalent_load,
};
use crate::thermal_plane_2d_validation::{
    validate_thermal_plane_quad_request, validate_thermal_plane_triangle_request,
};
use kyuubiki_protocol::{
    PlaneTriangleElementInput, SolvePlaneTriangle2dRequest, SolveThermalPlaneQuad2dRequest,
    SolveThermalPlaneQuad2dResult, SolveThermalPlaneTriangle2dRequest,
    SolveThermalPlaneTriangle2dResult, ThermalPlaneNodeResult, ThermalPlaneQuadElementInput,
    ThermalPlaneQuadElementResult, ThermalPlaneTriangleElementInput,
    ThermalPlaneTriangleElementResult,
};

#[derive(Debug, Clone)]
struct ThermalPlaneTriangleComputed {
    stiffness: [[f64; 6]; 6],
    area: f64,
    b_matrix: [[f64; 6]; 3],
    d_matrix: [[f64; 3]; 3],
    average_temperature_delta: f64,
}

#[derive(Debug, Clone)]
struct ThermalPlaneQuadComputed {
    first: ThermalPlaneTriangleComputed,
    second: ThermalPlaneTriangleComputed,
}

#[derive(Debug, Clone)]
struct ThermalPlaneTriangleState {
    total_strain: [f64; 3],
    mechanical_strain: [f64; 3],
    thermal_strain: f64,
    stress: [f64; 3],
    principal_stress_1: f64,
    principal_stress_2: f64,
    max_in_plane_shear: f64,
    von_mises: f64,
}

pub fn solve_thermal_plane_triangle_2d(
    request: &SolveThermalPlaneTriangle2dRequest,
) -> Result<SolveThermalPlaneTriangle2dResult, String> {
    validate_thermal_plane_triangle_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::with_uniform_row_capacity(dof_count, 18);
    let mut force_vector = build_force_vector(request);
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_thermal_plane_triangle_element(request, element))
        .collect::<Result<Vec<_>, String>>()?;

    for (element, computed) in request.elements.iter().zip(computed_elements.iter()) {
        assemble_thermal_triangle(element, computed, &mut global_stiffness, &mut force_vector);
    }

    let displacements =
        solve_thermal_plane_displacements(request, &global_stiffness, &force_vector)?;
    let nodes = build_thermal_plane_nodes(request, &displacements);
    let elements = build_thermal_triangle_elements(request, &computed_elements, &displacements);

    Ok(SolveThermalPlaneTriangle2dResult {
        input: request.clone(),
        max_displacement: max_thermal_plane_displacement(&nodes),
        max_stress: elements
            .iter()
            .map(|element| element.von_mises.abs())
            .fold(0.0_f64, f64::max),
        max_temperature_delta: max_temperature_delta(&nodes),
        nodes,
        elements,
    })
}

pub fn solve_thermal_plane_quad_2d(
    request: &SolveThermalPlaneQuad2dRequest,
) -> Result<SolveThermalPlaneQuad2dResult, String> {
    validate_thermal_plane_quad_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::with_uniform_row_capacity(dof_count, 24);
    let mut force_vector = build_quad_force_vector(request);
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_thermal_plane_quad_element(request, element))
        .collect::<Result<Vec<_>, String>>()?;

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

    let triangle_request = to_triangle_request(request);
    let displacements =
        solve_thermal_plane_displacements(&triangle_request, &global_stiffness, &force_vector)?;
    let nodes = build_thermal_plane_nodes(&triangle_request, &displacements);
    let elements = build_thermal_quad_elements(request, &computed_elements, &displacements);

    Ok(SolveThermalPlaneQuad2dResult {
        input: request.clone(),
        max_displacement: max_thermal_plane_displacement(&nodes),
        max_stress: elements
            .iter()
            .map(|element| element.von_mises.abs())
            .fold(0.0_f64, f64::max),
        max_temperature_delta: max_temperature_delta(&nodes),
        nodes,
        elements,
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
            }
        })
        .collect()
}

fn precompute_thermal_plane_triangle_element(
    request: &SolveThermalPlaneTriangle2dRequest,
    element: &ThermalPlaneTriangleElementInput,
) -> Result<ThermalPlaneTriangleComputed, String> {
    let plane_request = to_plane_triangle_request(request);
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
    } = precompute_plane_triangle_element(&plane_request, &plane_element)?;
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
    request: &SolveThermalPlaneQuad2dRequest,
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
    let triangle_request = to_triangle_request(request);

    Ok(ThermalPlaneQuadComputed {
        first: precompute_thermal_plane_triangle_element(&triangle_request, &first)?,
        second: precompute_thermal_plane_triangle_element(&triangle_request, &second)?,
    })
}

fn thermal_plane_triangle_state(
    computed: &ThermalPlaneTriangleComputed,
    element_displacements: &[f64; 6],
    thermal_expansion: f64,
) -> ThermalPlaneTriangleState {
    let total_strain = multiply_matrix_vector_3x6(&computed.b_matrix, element_displacements);
    let thermal_strain = thermal_expansion * computed.average_temperature_delta;
    let thermal_vector = [thermal_strain, thermal_strain, 0.0];
    let mechanical_strain = subtract_vector_3(&total_strain, &thermal_vector);
    let stress = multiply_matrix_vector_3x3(&computed.d_matrix, &mechanical_strain);
    let derived = derive_planar_stress_metrics(stress[0], stress[1], stress[2]);

    ThermalPlaneTriangleState {
        total_strain,
        mechanical_strain,
        thermal_strain,
        stress,
        principal_stress_1: derived.principal_stress_1,
        principal_stress_2: derived.principal_stress_2,
        max_in_plane_shear: derived.max_in_plane_shear,
        von_mises: derived.von_mises,
    }
}

fn solve_thermal_plane_displacements(
    request: &SolveThermalPlaneTriangle2dRequest,
    global_stiffness: &SparseMatrix,
    force_vector: &[f64],
) -> Result<Vec<f64>, String> {
    let constrained = request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            [
                node.fix_x.then_some(index * 2),
                node.fix_y.then_some(index * 2 + 1),
            ]
        })
        .flatten()
        .collect::<Vec<_>>();
    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(global_stiffness, force_vector, &constrained);
    let reduced_displacements = solve_spd_system(&reduced_stiffness, &reduced_force)?;
    let mut displacements = vec![0.0; request.nodes.len() * 2];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }
    Ok(displacements)
}

fn build_thermal_plane_nodes(
    request: &SolveThermalPlaneTriangle2dRequest,
    displacements: &[f64],
) -> Vec<ThermalPlaneNodeResult> {
    request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let ux = displacements[index * 2];
            let uy = displacements[index * 2 + 1];
            ThermalPlaneNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                ux,
                uy,
                displacement_magnitude: (ux * ux + uy * uy).sqrt(),
                temperature_delta: node.temperature_delta,
            }
        })
        .collect()
}

fn build_force_vector(request: &SolveThermalPlaneTriangle2dRequest) -> Vec<f64> {
    let mut force_vector = vec![0.0; request.nodes.len() * 2];
    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_x;
        force_vector[index * 2 + 1] = node.load_y;
    }
    force_vector
}

fn build_quad_force_vector(request: &SolveThermalPlaneQuad2dRequest) -> Vec<f64> {
    let mut force_vector = vec![0.0; request.nodes.len() * 2];
    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_x;
        force_vector[index * 2 + 1] = node.load_y;
    }
    force_vector
}

fn to_plane_triangle_request(
    request: &SolveThermalPlaneTriangle2dRequest,
) -> SolvePlaneTriangle2dRequest {
    SolvePlaneTriangle2dRequest {
        nodes: request
            .nodes
            .iter()
            .map(|node| kyuubiki_protocol::PlaneNodeInput {
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                fix_x: node.fix_x,
                fix_y: node.fix_y,
                load_x: node.load_x,
                load_y: node.load_y,
            })
            .collect(),
        elements: vec![],
    }
}

fn to_triangle_request(
    request: &SolveThermalPlaneQuad2dRequest,
) -> SolveThermalPlaneTriangle2dRequest {
    SolveThermalPlaneTriangle2dRequest {
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

fn max_thermal_plane_displacement(nodes: &[ThermalPlaneNodeResult]) -> f64 {
    nodes
        .iter()
        .map(|node| node.displacement_magnitude)
        .fold(0.0_f64, f64::max)
}

fn max_temperature_delta(nodes: &[ThermalPlaneNodeResult]) -> f64 {
    nodes
        .iter()
        .map(|node| node.temperature_delta.abs())
        .fold(0.0_f64, f64::max)
}
