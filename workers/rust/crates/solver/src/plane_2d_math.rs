use kyuubiki_protocol::{PlaneNodeInput, PlaneTriangleElementInput, SolvePlaneTriangle2dRequest};

#[derive(Debug, Clone)]
pub(super) struct PlaneTriangleComputed {
    pub(super) stiffness: [[f64; 6]; 6],
    pub(super) area: f64,
    pub(super) b_matrix: [[f64; 6]; 3],
    pub(super) d_matrix: [[f64; 3]; 3],
}

#[derive(Debug, Clone)]
pub(super) struct PlaneTriangleState {
    pub(super) strain: [f64; 3],
    pub(super) stress: [f64; 3],
    pub(super) principal_stress_1: f64,
    pub(super) principal_stress_2: f64,
    pub(super) max_in_plane_shear: f64,
    pub(super) von_mises: f64,
    pub(super) strain_energy_density: f64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PlanarStressMetrics {
    pub(super) principal_stress_1: f64,
    pub(super) principal_stress_2: f64,
    pub(super) max_in_plane_shear: f64,
    pub(super) von_mises: f64,
}

pub(super) fn precompute_plane_triangle_element(
    request: &SolvePlaneTriangle2dRequest,
    element: &PlaneTriangleElementInput,
) -> Result<PlaneTriangleComputed, String> {
    precompute_plane_triangle_element_from_nodes(&request.nodes, element)
}

pub(super) fn precompute_plane_triangle_element_from_nodes(
    nodes: &[PlaneNodeInput],
    element: &PlaneTriangleElementInput,
) -> Result<PlaneTriangleComputed, String> {
    let (stiffness, area, b_matrix, d_matrix) = triangle_element_data(nodes, element)?;
    Ok(PlaneTriangleComputed {
        stiffness,
        area,
        b_matrix,
        d_matrix,
    })
}

pub(super) fn plane_triangle_state(
    computed: &PlaneTriangleComputed,
    element_displacements: &[f64; 6],
) -> PlaneTriangleState {
    let strain = multiply_matrix_vector_3x6(&computed.b_matrix, element_displacements);
    let stress = multiply_matrix_vector_3x3(&computed.d_matrix, &strain);
    let derived = derive_planar_stress_metrics(stress[0], stress[1], stress[2]);

    PlaneTriangleState {
        strain,
        stress,
        principal_stress_1: derived.principal_stress_1,
        principal_stress_2: derived.principal_stress_2,
        max_in_plane_shear: derived.max_in_plane_shear,
        von_mises: derived.von_mises,
        strain_energy_density: strain_energy_density(&stress, &strain),
    }
}

pub(crate) fn strain_energy_density(stress: &[f64; 3], strain: &[f64; 3]) -> f64 {
    0.5 * ((stress[0] * strain[0]) + (stress[1] * strain[1]) + (stress[2] * strain[2]))
}

pub(super) fn signed_triangle_area(
    node_i: &PlaneNodeInput,
    node_j: &PlaneNodeInput,
    node_k: &PlaneNodeInput,
) -> f64 {
    0.5 * ((node_j.x - node_i.x) * (node_k.y - node_i.y)
        - (node_k.x - node_i.x) * (node_j.y - node_i.y))
}

fn triangle_element_data(
    nodes: &[PlaneNodeInput],
    element: &PlaneTriangleElementInput,
) -> Result<([[f64; 6]; 6], f64, [[f64; 6]; 3], [[f64; 3]; 3]), String> {
    let node_i = &nodes[element.node_i];
    let node_j = &nodes[element.node_j];
    let node_k = &nodes[element.node_k];
    let area = signed_triangle_area(node_i, node_j, node_k).abs();
    if area <= 1.0e-12 {
        return Err("plane element area must be positive".to_string());
    }

    let b1 = node_j.y - node_k.y;
    let b2 = node_k.y - node_i.y;
    let b3 = node_i.y - node_j.y;
    let c1 = node_k.x - node_j.x;
    let c2 = node_i.x - node_k.x;
    let c3 = node_j.x - node_i.x;
    let factor = 1.0 / (2.0 * area);
    let b_matrix = [
        [b1 * factor, 0.0, b2 * factor, 0.0, b3 * factor, 0.0],
        [0.0, c1 * factor, 0.0, c2 * factor, 0.0, c3 * factor],
        [
            c1 * factor,
            b1 * factor,
            c2 * factor,
            b2 * factor,
            c3 * factor,
            b3 * factor,
        ],
    ];

    let e = element.youngs_modulus;
    let nu = element.poisson_ratio;
    let coeff = e / (1.0 - nu * nu);
    let d_matrix = [
        [coeff, coeff * nu, 0.0],
        [coeff * nu, coeff, 0.0],
        [0.0, 0.0, coeff * (1.0 - nu) * 0.5],
    ];

    let bt = transpose_3x6(&b_matrix);
    let bt_d = multiply_matrix_6x3_3x3(&bt, &d_matrix);
    let mut stiffness = multiply_matrix_6x3_3x6(&bt_d, &b_matrix);
    let scale = element.thickness * area;
    for row in 0..6 {
        for column in 0..6 {
            stiffness[row][column] *= scale;
        }
    }

    Ok((stiffness, area, b_matrix, d_matrix))
}

pub(super) fn derive_planar_stress_metrics(
    sigma_x: f64,
    sigma_y: f64,
    tau_xy: f64,
) -> PlanarStressMetrics {
    let center = 0.5 * (sigma_x + sigma_y);
    let radius = (((0.5 * (sigma_x - sigma_y)).powi(2)) + tau_xy.powi(2)).sqrt();
    let principal_stress_1 = center + radius;
    let principal_stress_2 = center - radius;
    let max_in_plane_shear = radius;
    let von_mises =
        ((sigma_x * sigma_x) - (sigma_x * sigma_y) + (sigma_y * sigma_y) + 3.0 * tau_xy * tau_xy)
            .sqrt();

    PlanarStressMetrics {
        principal_stress_1,
        principal_stress_2,
        max_in_plane_shear,
        von_mises,
    }
}

fn transpose_3x6(input: &[[f64; 6]; 3]) -> [[f64; 3]; 6] {
    let mut output = [[0.0; 3]; 6];
    for row in 0..3 {
        for column in 0..6 {
            output[column][row] = input[row][column];
        }
    }
    output
}

fn multiply_matrix_6x3_3x3(lhs: &[[f64; 3]; 6], rhs: &[[f64; 3]; 3]) -> [[f64; 3]; 6] {
    let mut output = [[0.0; 3]; 6];
    for row in 0..6 {
        for column in 0..3 {
            output[row][column] = (0..3)
                .map(|index| lhs[row][index] * rhs[index][column])
                .sum();
        }
    }
    output
}

fn multiply_matrix_6x3_3x6(lhs: &[[f64; 3]; 6], rhs: &[[f64; 6]; 3]) -> [[f64; 6]; 6] {
    let mut output = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            output[row][column] = (0..3)
                .map(|index| lhs[row][index] * rhs[index][column])
                .sum();
        }
    }
    output
}

pub(super) fn multiply_matrix_vector_3x6(matrix: &[[f64; 6]; 3], vector: &[f64; 6]) -> [f64; 3] {
    let mut output = [0.0; 3];
    for row in 0..3 {
        output[row] = (0..6).map(|index| matrix[row][index] * vector[index]).sum();
    }
    output
}

pub(super) fn multiply_matrix_vector_3x3(matrix: &[[f64; 3]; 3], vector: &[f64; 3]) -> [f64; 3] {
    let mut output = [0.0; 3];
    for row in 0..3 {
        output[row] = (0..3).map(|index| matrix[row][index] * vector[index]).sum();
    }
    output
}

pub(super) fn subtract_vector_3(left: &[f64; 3], right: &[f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

pub(super) fn thermal_plane_triangle_equivalent_load(
    b_matrix: &[[f64; 6]; 3],
    d_matrix: &[[f64; 3]; 3],
    area: f64,
    thickness: f64,
    thermal_expansion: f64,
    average_temperature_delta: f64,
) -> [f64; 6] {
    let thermal_strain = [
        thermal_expansion * average_temperature_delta,
        thermal_expansion * average_temperature_delta,
        0.0,
    ];
    let thermal_stress = multiply_matrix_vector_3x3(d_matrix, &thermal_strain);
    let bt = transpose_3x6(b_matrix);
    let mut equivalent_load = [0.0; 6];

    for row in 0..6 {
        equivalent_load[row] = (0..3)
            .map(|index| bt[row][index] * thermal_stress[index])
            .sum::<f64>()
            * thickness
            * area;
    }

    equivalent_load
}
