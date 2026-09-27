use kyuubiki_protocol::{
    PlaneNodeInput, PlaneTriangleElementInput, SolvePlaneTriangle2dRequest,
    SolveThermalPlaneTriangle2dRequest, ThermalPlaneNodeInput, ThermalPlaneTriangleElementInput,
};
use kyuubiki_solver::{solve_plane_triangle_2d, solve_thermal_plane_triangle_2d};

const POINTS: [[f64; 2]; 4] = [[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]];

fn connectivity(mask: usize) -> [[usize; 3]; 2] {
    let mut elements = [[0, 1, 2], [0, 2, 3]];
    for (i, nodes) in elements.iter_mut().enumerate() {
        if mask & (1 << i) != 0 {
            nodes.swap(1, 2);
        }
    }
    elements
}

fn mechanical(mask: usize) -> SolvePlaneTriangle2dRequest {
    SolvePlaneTriangle2dRequest {
        nodes: POINTS
            .into_iter()
            .enumerate()
            .map(|(i, [x, y])| PlaneNodeInput {
                id: format!("n{i}"),
                x,
                y,
                fix_x: x == 0.0,
                fix_y: i == 0,
                load_x: if x == 2.0 { 1.0 } else { 0.0 },
                load_y: 0.0,
            })
            .collect(),
        elements: connectivity(mask)
            .into_iter()
            .enumerate()
            .map(|(i, [node_i, node_j, node_k])| PlaneTriangleElementInput {
                id: format!("triangle-{i}"),
                node_i,
                node_j,
                node_k,
                thickness: 0.5,
                youngs_modulus: 1200.0,
                poisson_ratio: 0.25,
            })
            .collect(),
    }
}

fn thermal(mask: usize, restrained: bool, variable: bool) -> SolveThermalPlaneTriangle2dRequest {
    SolveThermalPlaneTriangle2dRequest {
        nodes: POINTS
            .into_iter()
            .enumerate()
            .map(|(i, [x, y])| ThermalPlaneNodeInput {
                id: format!("n{i}"),
                x,
                y,
                fix_x: restrained || i == 0,
                fix_y: restrained || i == 0 || i == 1,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: if variable { 30.0 + 6.0 * x } else { 50.0 },
            })
            .collect(),
        elements: connectivity(mask)
            .into_iter()
            .enumerate()
            .map(
                |(i, [node_i, node_j, node_k])| ThermalPlaneTriangleElementInput {
                    id: format!("thermal-triangle-{i}"),
                    node_i,
                    node_j,
                    node_k,
                    thickness: 0.5,
                    youngs_modulus: 1200.0,
                    poisson_ratio: 0.25,
                    thermal_expansion: 1e-5,
                },
            )
            .collect(),
    }
}

fn close(actual: f64, expected: f64, scale: f64) {
    assert!(
        actual.is_finite() && (actual - expected).abs() <= 1e-11 * scale,
        "actual={actual:e} expected={expected:e} scale={scale:e}"
    );
}

#[test]
fn mixed_orientation_triangles_keep_analytic_signed_stress_and_displacement() {
    for mask in 0..4 {
        let result = solve_plane_triangle_2d(&mechanical(mask)).unwrap();
        for (node, [x, y]) in result.nodes.iter().zip(POINTS) {
            close(node.ux, 4.0 / 1200.0 * x, 0.01);
            close(node.uy, -0.25 * 4.0 / 1200.0 * y, 0.01);
        }
        for element in &result.elements {
            close(element.area, 1.0, 1.0);
            close(element.stress_x, 4.0, 4.0);
            close(element.stress_y, 0.0, 4.0);
            close(element.tau_xy, 0.0, 4.0);
        }
        close(result.total_strain_energy, 0.5 * 4.0 * (4.0 / 1200.0), 0.01);
    }
}

#[test]
fn mixed_orientation_thermal_triangles_expand_instead_of_contracting() {
    for mask in 0..4 {
        let result = solve_thermal_plane_triangle_2d(&thermal(mask, false, false)).unwrap();
        for (node, [x, y]) in result.nodes.iter().zip(POINTS) {
            close(node.ux, 0.0005 * x, 0.01);
            close(node.uy, 0.0005 * y, 0.01);
        }
        for element in result.elements {
            close(element.total_strain_x, 0.0005, 0.01);
            close(element.total_strain_y, 0.0005, 0.01);
            close(element.stress_x, 0.0, 1.0);
            close(element.stress_y, 0.0, 1.0);
            close(element.tau_xy, 0.0, 1.0);
        }
    }
}

#[test]
fn thermal_triangle_variable_temperature_is_invariant_and_restraint_is_compressive() {
    for restrained in [false, true] {
        let reference = solve_thermal_plane_triangle_2d(&thermal(0, restrained, true)).unwrap();
        for mask in 1..4 {
            let result = solve_thermal_plane_triangle_2d(&thermal(mask, restrained, true)).unwrap();
            for (node, original) in result.nodes.iter().zip(&reference.nodes) {
                close(node.ux, original.ux, 0.01);
                close(node.uy, original.uy, 0.01);
            }
            for (element, original) in result.elements.iter().zip(&reference.elements) {
                close(element.stress_x, original.stress_x, 1.0);
                close(element.stress_y, original.stress_y, 1.0);
                close(element.tau_xy, original.tau_xy, 1.0);
                close(
                    element.average_temperature_delta,
                    original.average_temperature_delta,
                    50.0,
                );
                if restrained {
                    let stress = -1200.0 * 1e-5 * element.average_temperature_delta / 0.75;
                    close(element.stress_x, stress, 1.0);
                    close(element.stress_y, stress, 1.0);
                }
            }
        }
    }
}

#[test]
fn triangle_coefficient_overflow_identifies_element_and_does_not_poison_replay() {
    let mut input = mechanical(0);
    input.elements[0].youngs_modulus = f64::MAX;
    input.elements[0].poisson_ratio = 0.49;
    let error = solve_plane_triangle_2d(&input).unwrap_err();
    assert!(
        error.contains("triangle-0") && error.contains("coefficients are not representable"),
        "{error}"
    );
    assert!(solve_plane_triangle_2d(&mechanical(3)).is_ok());
    let mut input = thermal(0, false, false);
    input.elements[0].youngs_modulus = f64::MAX;
    input.elements[0].poisson_ratio = 0.49;
    let error = solve_thermal_plane_triangle_2d(&input).unwrap_err();
    assert!(
        error.contains("thermal-triangle-0")
            && error.contains("coefficients are not representable"),
        "{error}"
    );
    assert!(solve_thermal_plane_triangle_2d(&thermal(3, false, false)).is_ok());
}
