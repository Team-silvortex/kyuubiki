use kyuubiki_protocol::{PlaneNodeInput, PlaneTriangleElementInput, SolvePlaneTriangle2dRequest};

#[cfg(test)]
#[path = "plane_triangle_kernel_tests.rs"]
mod kernel_tests;

#[cfg(test)]
#[path = "plane_triangle_kernel_reference.rs"]
mod reference;

#[cfg(test)]
#[path = "plane_triangle_kernel_benchmark.rs"]
mod benchmark;

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
    let coordinates = [
        [nodes[element.node_i].x, nodes[element.node_i].y],
        [nodes[element.node_j].x, nodes[element.node_j].y],
        [nodes[element.node_k].x, nodes[element.node_k].y],
    ];
    precompute_plane_triangle_element_from_coordinates(
        coordinates,
        element.thickness,
        element.youngs_modulus,
        element.poisson_ratio,
    )
    .map_err(|error| format!("plane triangle element {}: {error}", element.id))
}

pub(super) fn precompute_plane_triangle_element_from_coordinates(
    coordinates: [[f64; 2]; 3],
    thickness: f64,
    youngs_modulus: f64,
    poisson_ratio: f64,
) -> Result<PlaneTriangleComputed, String> {
    let (stiffness, area, b_matrix, d_matrix) =
        triangle_element_data(coordinates, thickness, youngs_modulus, poisson_ratio)?;
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
    signed_triangle_area_from_coordinates([
        [node_i.x, node_i.y],
        [node_j.x, node_j.y],
        [node_k.x, node_k.y],
    ])
}

pub(super) fn signed_triangle_area_from_coordinates(coordinates: [[f64; 2]; 3]) -> f64 {
    let [node_i, node_j, node_k] = coordinates;
    0.5 * ((node_j[0] - node_i[0]) * (node_k[1] - node_i[1])
        - (node_k[0] - node_i[0]) * (node_j[1] - node_i[1]))
}

type PlaneTriangleElementData = ([[f64; 6]; 6], f64, [[f64; 6]; 3], [[f64; 3]; 3]);

fn triangle_element_data(
    coordinates: [[f64; 2]; 3],
    thickness: f64,
    youngs_modulus: f64,
    poisson_ratio: f64,
) -> Result<PlaneTriangleElementData, String> {
    let [node_i, node_j, node_k] = coordinates;
    let signed_area = signed_triangle_area_from_coordinates(coordinates);
    let area = signed_area.abs();
    if !area.is_finite() {
        return Err("plane triangle area is not representable".to_string());
    }
    if area <= 1.0e-12 {
        return Err("plane element area must be positive".to_string());
    }

    let b1 = node_j[1] - node_k[1];
    let b2 = node_k[1] - node_i[1];
    let b3 = node_i[1] - node_j[1];
    let c1 = node_k[0] - node_j[0];
    let c2 = node_i[0] - node_k[0];
    let c3 = node_j[0] - node_i[0];
    // Shape gradients retain orientation; only the integration measure is unsigned.
    let factor = 0.5 / signed_area;
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
    if b_matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err("plane triangle shape gradients are not representable".to_string());
    }

    let d_matrix = plane_stress_d_matrix(youngs_modulus, poisson_ratio);
    if d_matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err("plane triangle constitutive coefficients are not representable".to_string());
    }
    let scale = thickness * area;
    let mut stiffness = [[0.0; 6]; 6];
    // Isotropic D and CST B have known zero entries. Form symmetric 2x2
    // nodal blocks without dense products through those zeros.
    for i in 0..3 {
        let x = 2 * i;
        let y = x + 1;
        let dx = b_matrix[0][x];
        let dy = b_matrix[1][y];
        let nx = d_matrix[0][0] * dx;
        let ny = d_matrix[1][1] * dy;
        let nux = d_matrix[0][1] * dx;
        let nuy = d_matrix[1][0] * dy;
        let sx = d_matrix[2][2] * dx;
        let sy = d_matrix[2][2] * dy;
        for j in i..3 {
            let u = 2 * j;
            let v = u + 1;
            let dxj = b_matrix[0][u];
            let dyj = b_matrix[1][v];
            let xx = (nx * dxj + sy * dyj) * scale;
            let yy = (sx * dxj + ny * dyj) * scale;
            let xy = (nux * dyj + sy * dxj) * scale;
            stiffness[x][u] = xx;
            stiffness[u][x] = xx;
            stiffness[y][v] = yy;
            stiffness[v][y] = yy;
            stiffness[x][v] = xy;
            stiffness[v][x] = xy;
            if i != j {
                let yx = (sx * dyj + nuy * dxj) * scale;
                stiffness[y][u] = yx;
                stiffness[u][y] = yx;
            }
        }
    }
    if stiffness.iter().flatten().any(|value| !value.is_finite()) {
        return Err("plane triangle stiffness coefficients are not representable".to_string());
    }

    Ok((stiffness, area, b_matrix, d_matrix))
}

pub(super) fn plane_stress_d_matrix(e: f64, nu: f64) -> [[f64; 3]; 3] {
    let coeff = e / (1.0 - nu * nu);
    [
        [coeff, coeff * nu, 0.0],
        [coeff * nu, coeff, 0.0],
        [0.0, 0.0, coeff * (1.0 - nu) * 0.5],
    ]
}

pub(super) fn derive_planar_stress_metrics(
    sigma_x: f64,
    sigma_y: f64,
    tau_xy: f64,
) -> PlanarStressMetrics {
    let half_difference = sigma_x.midpoint(-sigma_y).abs();
    let radius = half_difference.hypot(tau_xy);
    // Rationalize radius - half_difference to retain a small second eigenvalue.
    let shift = if radius == 0.0 {
        0.0
    } else {
        tau_xy.abs() * ((tau_xy.abs() / radius) / (1.0 + half_difference / radius))
    };
    let principal_stress_1 = sigma_x.max(sigma_y) + shift;
    let principal_stress_2 = sigma_x.min(sigma_y) - shift;
    let max_in_plane_shear = radius;
    let scale = sigma_x.abs().max(sigma_y.abs()).max(tau_xy.abs());
    let von_mises = if scale == 0.0 {
        0.0
    } else {
        let x = sigma_x / scale;
        let y = sigma_y / scale;
        let shear = tau_xy / scale;
        (x * x - x * y + y * y + 3.0 * shear * shear).sqrt() * scale
    };

    PlanarStressMetrics {
        principal_stress_1,
        principal_stress_2,
        max_in_plane_shear,
        von_mises,
    }
}

fn transpose_3x6(input: &[[f64; 6]; 3]) -> [[f64; 3]; 6] {
    let mut output = [[0.0; 3]; 6];
    for (row, input_row) in input.iter().enumerate() {
        for (column, value) in input_row.iter().enumerate() {
            output[column][row] = *value;
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

#[cfg(test)]
mod output_range_tests {
    use super::derive_planar_stress_metrics;

    #[test]
    fn planar_metrics_preserve_pure_shear_and_biaxial_scales() {
        for scale in [1e-300, 1e-200, 1.0, 1e200, 1e300] {
            let shear = derive_planar_stress_metrics(0.0, 0.0, scale);
            assert_eq!(shear.principal_stress_1, scale);
            assert_eq!(shear.principal_stress_2, -scale);
            assert!((shear.von_mises / scale - 3.0_f64.sqrt()).abs() < 1e-14);
            let biaxial = derive_planar_stress_metrics(scale, scale, 0.0);
            assert_eq!(biaxial.principal_stress_1, scale);
            assert_eq!(biaxial.principal_stress_2, scale);
            assert_eq!(biaxial.von_mises, scale);
        }
    }

    #[test]
    fn principal_stress_preserves_small_eigenvalue_without_center_cancellation() {
        let diagonal = derive_planar_stress_metrics(1e200, 1.0, 0.0);
        assert_eq!(diagonal.principal_stress_2, 1.0);
        let coupled = derive_planar_stress_metrics(1e200, 1.0, 5e99);
        assert!((coupled.principal_stress_2 - 0.75).abs() < 1e-14);
        let near_limit = derive_planar_stress_metrics(1e308, 1e308, 0.0);
        assert_eq!(near_limit.principal_stress_1, 1e308);
        assert_eq!(near_limit.principal_stress_2, 1e308);
    }
}
