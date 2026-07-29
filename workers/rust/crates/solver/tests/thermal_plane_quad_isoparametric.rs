use kyuubiki_protocol::{
    SolveThermalPlaneQuad2dRequest, ThermalPlaneNodeInput, ThermalPlaneQuadElementInput,
};
use kyuubiki_solver::solve_thermal_plane_quad_2d;

const YOUNGS_MODULUS: f64 = 70.0e9;
const POISSON_RATIO: f64 = 0.33;
const THICKNESS: f64 = 0.02;
const THERMAL_EXPANSION: f64 = 11.0e-6;
const TEMPERATURE_DELTA: f64 = 30.0;
const LENGTH: f64 = 2.0;
const HEIGHT: f64 = 1.0;
const TOP_TAPER: f64 = 0.3;

#[test]
fn distorted_quad_mesh_reproduces_free_uniform_thermal_expansion() {
    let expected_strain = THERMAL_EXPANSION * TEMPERATURE_DELTA;
    for divisions in [1_usize, 2, 4] {
        let result = solve_thermal_plane_quad_2d(&thermal_mesh(divisions, true, false))
            .expect("distorted thermal Q4 patch should solve");

        assert_eq!(result.elements.len(), divisions * divisions);
        for node in &result.nodes {
            assert_close(node.ux, expected_strain * node.x, "free-expansion ux");
            assert_close(node.uy, expected_strain * node.y, "free-expansion uy");
        }
        for element in &result.elements {
            assert_close(element.total_strain_x, expected_strain, "total strain x");
            assert_close(element.total_strain_y, expected_strain, "total strain y");
            assert_near_zero(element.mechanical_strain_x, 1.0e-11, "mechanical strain x");
            assert_near_zero(element.mechanical_strain_y, 1.0e-11, "mechanical strain y");
            assert_near_zero(element.gamma_xy, 1.0e-11, "shear strain");
            assert_near_zero(element.von_mises, 1.0, "free-expansion stress");
            assert_near_zero(
                element.strain_energy_density,
                1.0e-6,
                "free-expansion energy",
            );
        }
        assert_close(
            result.elements.iter().map(|element| element.area).sum(),
            2.15,
            "distorted mesh area",
        );
    }
}

#[test]
fn distorted_restrained_quad_integrates_linear_temperature_at_gauss_points() {
    let gradient = 8.0;
    let base_temperature = 20.0;
    let mut request = thermal_mesh(1, false, true);
    for node in &mut request.nodes {
        node.temperature_delta = base_temperature + gradient * node.x;
    }

    let result = solve_thermal_plane_quad_2d(&request)
        .expect("restrained nonuniform-temperature Q4 patch should solve");
    let element = &result.elements[0];
    let centroid_x = polygon_centroid_x(&request);
    let average_temperature = base_temperature + gradient * centroid_x;
    let expected_thermal_strain = THERMAL_EXPANSION * average_temperature;
    let expected_stress = -YOUNGS_MODULUS * expected_thermal_strain / (1.0 - POISSON_RATIO);

    assert_close(element.area, 2.15, "restrained quad area");
    assert_close(
        element.average_temperature_delta,
        average_temperature,
        "integrated temperature",
    );
    assert_close(
        element.thermal_strain,
        expected_thermal_strain,
        "integrated thermal strain",
    );
    assert_close(
        element.mechanical_strain_x,
        -expected_thermal_strain,
        "mechanical strain x",
    );
    assert_close(
        element.mechanical_strain_y,
        -expected_thermal_strain,
        "mechanical strain y",
    );
    assert_close(element.stress_x, expected_stress, "stress x");
    assert_close(element.stress_y, expected_stress, "stress y");
    assert_near_zero(element.tau_xy, 1.0e-6, "shear stress");
}

#[test]
fn thermal_quad_rejects_inverted_node_ordering() {
    let mut request = thermal_mesh(1, false, false);
    let element = &mut request.elements[0];
    std::mem::swap(&mut element.node_j, &mut element.node_l);

    let error =
        solve_thermal_plane_quad_2d(&request).expect_err("inverted thermal Q4 must be rejected");
    assert!(
        error.contains("positive Jacobian"),
        "unexpected inverted thermal Q4 error: {error}",
    );
}

fn thermal_mesh(
    divisions: usize,
    distort_interior: bool,
    fully_restrained: bool,
) -> SolveThermalPlaneQuad2dRequest {
    let mut nodes = Vec::with_capacity((divisions + 1).pow(2));
    for row in 0..=divisions {
        let eta = row as f64 / divisions as f64;
        for column in 0..=divisions {
            let xi = column as f64 / divisions as f64;
            let mut x = xi * (LENGTH + TOP_TAPER * eta);
            let mut y = HEIGHT * eta;
            if distort_interior && row > 0 && row < divisions && column > 0 && column < divisions {
                let bubble = (std::f64::consts::PI * xi).sin() * (std::f64::consts::PI * eta).sin();
                x += 0.08 * bubble;
                y += 0.05 * bubble;
            }
            nodes.push(ThermalPlaneNodeInput {
                id: format!("n{column}_{row}"),
                x,
                y,
                fix_x: fully_restrained || (row == 0 && column == 0),
                fix_y: fully_restrained || (row == 0 && (column == 0 || column == divisions)),
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: TEMPERATURE_DELTA,
            });
        }
    }

    let mut elements = Vec::with_capacity(divisions * divisions);
    for row in 0..divisions {
        for column in 0..divisions {
            elements.push(ThermalPlaneQuadElementInput {
                id: format!("q{column}_{row}"),
                node_i: node_index(column, row, divisions),
                node_j: node_index(column + 1, row, divisions),
                node_k: node_index(column + 1, row + 1, divisions),
                node_l: node_index(column, row + 1, divisions),
                thickness: THICKNESS,
                youngs_modulus: YOUNGS_MODULUS,
                poisson_ratio: POISSON_RATIO,
                thermal_expansion: THERMAL_EXPANSION,
            });
        }
    }
    SolveThermalPlaneQuad2dRequest { nodes, elements }
}

fn node_index(column: usize, row: usize, divisions: usize) -> usize {
    row * (divisions + 1) + column
}

fn polygon_centroid_x(request: &SolveThermalPlaneQuad2dRequest) -> f64 {
    let element = &request.elements[0];
    let indices = [
        element.node_i,
        element.node_j,
        element.node_k,
        element.node_l,
    ];
    let mut signed_area_twice = 0.0;
    let mut centroid_numerator = 0.0;
    for edge in 0..4 {
        let current = &request.nodes[indices[edge]];
        let next = &request.nodes[indices[(edge + 1) % 4]];
        let cross = current.x * next.y - next.x * current.y;
        signed_area_twice += cross;
        centroid_numerator += (current.x + next.x) * cross;
    }
    centroid_numerator / (3.0 * signed_area_twice)
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= 1.0e-8 * scale,
        "{label}: expected {actual} to be close to {expected}",
    );
}

fn assert_near_zero(actual: f64, tolerance: f64, label: &str) {
    assert!(
        actual.abs() <= tolerance,
        "{label}: expected {actual} to be within {tolerance} of zero",
    );
}
