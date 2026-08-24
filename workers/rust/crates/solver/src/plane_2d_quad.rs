use crate::plane_2d_math::{
    derive_planar_stress_metrics, multiply_matrix_vector_3x3, plane_stress_d_matrix,
    strain_energy_density,
};
use kyuubiki_protocol::{PlaneQuadElementInput, SolvePlaneQuad2dRequest};

const GAUSS_COORDINATE: f64 = 0.577_350_269_189_625_8;

#[derive(Debug, Clone)]
pub(crate) struct PlaneQuadComputed {
    pub(crate) stiffness: [[f64; 8]; 8],
    pub(crate) area: f64,
    pub(crate) gauss_points: [PlaneQuadGaussPoint; 4],
    pub(crate) d_matrix: [[f64; 3]; 3],
}

#[derive(Debug, Clone)]
pub(crate) struct PlaneQuadGaussPoint {
    pub(crate) shape_functions: [f64; 4],
    pub(crate) b_matrix: [[f64; 8]; 3],
    pub(crate) det_jacobian: f64,
}

#[derive(Debug, Clone)]
pub(super) struct PlaneQuadState {
    pub(super) strain: [f64; 3],
    pub(super) stress: [f64; 3],
    pub(super) principal_stress_1: f64,
    pub(super) principal_stress_2: f64,
    pub(super) max_in_plane_shear: f64,
    pub(super) von_mises: f64,
    pub(super) strain_energy_density: f64,
}

pub(super) fn precompute_plane_quad_element(
    request: &SolvePlaneQuad2dRequest,
    element: &PlaneQuadElementInput,
) -> Result<PlaneQuadComputed, String> {
    let coordinates = [
        [
            request.nodes[element.node_i].x,
            request.nodes[element.node_i].y,
        ],
        [
            request.nodes[element.node_j].x,
            request.nodes[element.node_j].y,
        ],
        [
            request.nodes[element.node_k].x,
            request.nodes[element.node_k].y,
        ],
        [
            request.nodes[element.node_l].x,
            request.nodes[element.node_l].y,
        ],
    ];
    precompute_plane_quad_element_from_coordinates(
        coordinates,
        element.thickness,
        element.youngs_modulus,
        element.poisson_ratio,
    )
}

pub(crate) fn precompute_plane_quad_element_from_coordinates(
    coordinates: [[f64; 2]; 4],
    thickness: f64,
    youngs_modulus: f64,
    poisson_ratio: f64,
) -> Result<PlaneQuadComputed, String> {
    let d_matrix = plane_stress_d_matrix(youngs_modulus, poisson_ratio);
    let gauss_points = [
        quad_gauss_point(coordinates, -GAUSS_COORDINATE, -GAUSS_COORDINATE)?,
        quad_gauss_point(coordinates, GAUSS_COORDINATE, -GAUSS_COORDINATE)?,
        quad_gauss_point(coordinates, GAUSS_COORDINATE, GAUSS_COORDINATE)?,
        quad_gauss_point(coordinates, -GAUSS_COORDINATE, GAUSS_COORDINATE)?,
    ];

    let mut stiffness = [[0.0; 8]; 8];
    let mut area = 0.0;
    for point in &gauss_points {
        area += point.det_jacobian;
        accumulate_stiffness(
            &mut stiffness,
            &point.b_matrix,
            &d_matrix,
            thickness * point.det_jacobian,
        );
    }

    Ok(PlaneQuadComputed {
        stiffness,
        area,
        gauss_points,
        d_matrix,
    })
}

pub(super) fn plane_quad_state(
    computed: &PlaneQuadComputed,
    element_displacements: &[f64; 8],
) -> PlaneQuadState {
    let mut strain = [0.0; 3];
    let mut stress = [0.0; 3];
    let mut energy = 0.0;

    for point in &computed.gauss_points {
        let point_strain = multiply_matrix_vector_3x8(&point.b_matrix, element_displacements);
        let point_stress = multiply_matrix_vector_3x3(&computed.d_matrix, &point_strain);
        for component in 0..3 {
            strain[component] += point_strain[component] * point.det_jacobian;
            stress[component] += point_stress[component] * point.det_jacobian;
        }
        energy += strain_energy_density(&point_stress, &point_strain) * point.det_jacobian;
    }

    for component in 0..3 {
        strain[component] /= computed.area;
        stress[component] /= computed.area;
    }
    let derived = derive_planar_stress_metrics(stress[0], stress[1], stress[2]);

    PlaneQuadState {
        strain,
        stress,
        principal_stress_1: derived.principal_stress_1,
        principal_stress_2: derived.principal_stress_2,
        max_in_plane_shear: derived.max_in_plane_shear,
        von_mises: derived.von_mises,
        strain_energy_density: energy / computed.area,
    }
}

fn quad_gauss_point(
    coordinates: [[f64; 2]; 4],
    xi: f64,
    eta: f64,
) -> Result<PlaneQuadGaussPoint, String> {
    let shape_functions = [
        0.25 * (1.0 - xi) * (1.0 - eta),
        0.25 * (1.0 + xi) * (1.0 - eta),
        0.25 * (1.0 + xi) * (1.0 + eta),
        0.25 * (1.0 - xi) * (1.0 + eta),
    ];
    let dxi = [
        -0.25 * (1.0 - eta),
        0.25 * (1.0 - eta),
        0.25 * (1.0 + eta),
        -0.25 * (1.0 + eta),
    ];
    let deta = [
        -0.25 * (1.0 - xi),
        -0.25 * (1.0 + xi),
        0.25 * (1.0 + xi),
        0.25 * (1.0 - xi),
    ];

    let dx_dxi = dot_coordinates(&coordinates, &dxi, 0);
    let dy_dxi = dot_coordinates(&coordinates, &dxi, 1);
    let dx_deta = dot_coordinates(&coordinates, &deta, 0);
    let dy_deta = dot_coordinates(&coordinates, &deta, 1);
    let det_jacobian = dx_dxi * dy_deta - dy_dxi * dx_deta;
    if !(det_jacobian.is_finite() && det_jacobian > 1.0e-12) {
        return Err(
            "plane quad element must preserve a positive Jacobian at every Gauss point".to_string(),
        );
    }

    let mut b_matrix = [[0.0; 8]; 3];
    for node in 0..4 {
        let dx = (dy_deta * dxi[node] - dy_dxi * deta[node]) / det_jacobian;
        let dy = (-dx_deta * dxi[node] + dx_dxi * deta[node]) / det_jacobian;
        b_matrix[0][node * 2] = dx;
        b_matrix[1][node * 2 + 1] = dy;
        b_matrix[2][node * 2] = dy;
        b_matrix[2][node * 2 + 1] = dx;
    }

    Ok(PlaneQuadGaussPoint {
        shape_functions,
        b_matrix,
        det_jacobian,
    })
}

fn dot_coordinates(coordinates: &[[f64; 2]; 4], derivative: &[f64; 4], component: usize) -> f64 {
    (0..4)
        .map(|index| derivative[index] * coordinates[index][component])
        .sum()
}

fn accumulate_stiffness(
    stiffness: &mut [[f64; 8]; 8],
    b_matrix: &[[f64; 8]; 3],
    d_matrix: &[[f64; 3]; 3],
    scale: f64,
) {
    for (row, stiffness_row) in stiffness.iter_mut().enumerate() {
        for (column, stiffness_value) in stiffness_row.iter_mut().enumerate() {
            let value = (0..3)
                .flat_map(|left| {
                    (0..3).map(move |right| {
                        b_matrix[left][row] * d_matrix[left][right] * b_matrix[right][column]
                    })
                })
                .sum::<f64>();
            *stiffness_value += value * scale;
        }
    }
}

pub(crate) fn multiply_matrix_vector_3x8(matrix: &[[f64; 8]; 3], vector: &[f64; 8]) -> [f64; 3] {
    std::array::from_fn(|row| {
        (0..8)
            .map(|column| matrix[row][column] * vector[column])
            .sum()
    })
}
