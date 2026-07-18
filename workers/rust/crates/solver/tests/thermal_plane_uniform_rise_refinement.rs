use kyuubiki_protocol::{
    SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest, ThermalPlaneNodeInput,
    ThermalPlaneQuadElementInput, ThermalPlaneTriangleElementInput,
};
use kyuubiki_solver::{solve_thermal_plane_quad_2d, solve_thermal_plane_triangle_2d};

const TOL: f64 = 1.0e-9;
const THICKNESS: f64 = 0.02;
const YOUNGS_MODULUS: f64 = 70.0e9;
const POISSON_RATIO: f64 = 0.33;
const THERMAL_EXPANSION: f64 = 11.0e-6;
const TEMPERATURE_DELTA: f64 = 30.0;

#[test]
fn fully_restrained_uniform_rise_is_refinement_invariant_for_triangles_and_quads() {
    let triangle_baseline = solve_thermal_plane_triangle_2d(&triangle_mesh(1)).unwrap();
    let quad_baseline = solve_thermal_plane_quad_2d(&quad_mesh(1)).unwrap();
    for subdivisions in [1_usize, 2, 4, 8] {
        let triangle = solve_thermal_plane_triangle_2d(&triangle_mesh(subdivisions)).unwrap();
        let quad = solve_thermal_plane_quad_2d(&quad_mesh(subdivisions)).unwrap();
        assert_eq!(triangle.elements.len(), subdivisions * subdivisions * 2);
        assert_eq!(quad.elements.len(), subdivisions * subdivisions);
        assert_thermal_triangle(&triangle, &triangle_baseline);
        assert_thermal_quad(&quad, &quad_baseline);
    }
}

fn assert_thermal_triangle(
    result: &kyuubiki_protocol::SolveThermalPlaneTriangle2dResult,
    baseline: &kyuubiki_protocol::SolveThermalPlaneTriangle2dResult,
) {
    assert_close(result.max_displacement, 0.0);
    assert_close(result.max_temperature_delta, TEMPERATURE_DELTA);
    assert_close(result.max_stress, baseline.max_stress);
    assert_close(
        result.max_strain_energy_density,
        baseline.max_strain_energy_density,
    );
    assert_close(result.total_strain_energy, baseline.total_strain_energy);
    for element in &result.elements {
        assert_close(
            element.thermal_strain,
            THERMAL_EXPANSION * TEMPERATURE_DELTA,
        );
    }
}

fn assert_thermal_quad(
    result: &kyuubiki_protocol::SolveThermalPlaneQuad2dResult,
    baseline: &kyuubiki_protocol::SolveThermalPlaneQuad2dResult,
) {
    assert_close(result.max_displacement, 0.0);
    assert_close(result.max_temperature_delta, TEMPERATURE_DELTA);
    assert_close(result.max_stress, baseline.max_stress);
    assert_close(
        result.max_strain_energy_density,
        baseline.max_strain_energy_density,
    );
    assert_close(result.total_strain_energy, baseline.total_strain_energy);
    for element in &result.elements {
        assert_close(
            element.thermal_strain,
            THERMAL_EXPANSION * TEMPERATURE_DELTA,
        );
    }
}

fn quad_mesh(n: usize) -> SolveThermalPlaneQuad2dRequest {
    let mut elements = Vec::new();
    for y in 0..n {
        for x in 0..n {
            elements.push(ThermalPlaneQuadElementInput {
                id: format!("q-{x}-{y}"),
                node_i: index(x, y, n),
                node_j: index(x + 1, y, n),
                node_k: index(x + 1, y + 1, n),
                node_l: index(x, y + 1, n),
                thickness: THICKNESS,
                youngs_modulus: YOUNGS_MODULUS,
                poisson_ratio: POISSON_RATIO,
                thermal_expansion: THERMAL_EXPANSION,
            });
        }
    }
    SolveThermalPlaneQuad2dRequest {
        nodes: nodes(n),
        elements,
    }
}

fn triangle_mesh(n: usize) -> SolveThermalPlaneTriangle2dRequest {
    let mut elements = Vec::new();
    for y in 0..n {
        for x in 0..n {
            let a = index(x, y, n);
            let b = index(x + 1, y, n);
            let c = index(x + 1, y + 1, n);
            let d = index(x, y + 1, n);
            for (id, i, j, k) in [
                (format!("a-{x}-{y}"), a, b, c),
                (format!("b-{x}-{y}"), a, c, d),
            ] {
                elements.push(ThermalPlaneTriangleElementInput {
                    id,
                    node_i: i,
                    node_j: j,
                    node_k: k,
                    thickness: THICKNESS,
                    youngs_modulus: YOUNGS_MODULUS,
                    poisson_ratio: POISSON_RATIO,
                    thermal_expansion: THERMAL_EXPANSION,
                });
            }
        }
    }
    SolveThermalPlaneTriangle2dRequest {
        nodes: nodes(n),
        elements,
    }
}

fn nodes(n: usize) -> Vec<ThermalPlaneNodeInput> {
    (0..=n)
        .flat_map(|y| {
            (0..=n).map(move |x| ThermalPlaneNodeInput {
                id: format!("n-{x}-{y}"),
                x: x as f64 / n as f64,
                y: y as f64 / n as f64,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: TEMPERATURE_DELTA,
            })
        })
        .collect()
}
fn index(x: usize, y: usize, n: usize) -> usize {
    y * (n + 1) + x
}
fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOL * expected.abs().max(1.0),
        "expected {actual} to be close to {expected}"
    );
}
