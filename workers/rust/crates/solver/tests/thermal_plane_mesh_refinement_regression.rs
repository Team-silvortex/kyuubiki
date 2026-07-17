use kyuubiki_protocol::{
    HeatPlaneNodeInput, HeatPlaneQuadElementInput, HeatPlaneTriangleElementInput,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest, SolveThermalPlaneQuad2dRequest,
    SolveThermalPlaneTriangle2dRequest, ThermalPlaneNodeInput, ThermalPlaneQuadElementInput,
    ThermalPlaneTriangleElementInput,
};
use kyuubiki_solver::{
    solve_heat_plane_quad_2d, solve_heat_plane_triangle_2d, solve_thermal_plane_quad_2d,
    solve_thermal_plane_triangle_2d,
};

const TOL: f64 = 1.0e-10;
const THICKNESS: f64 = 0.02;
const CONDUCTIVITY: f64 = 45.0;
const YOUNGS_MODULUS: f64 = 70.0e9;
const POISSON_RATIO: f64 = 0.33;
const THERMAL_EXPANSION: f64 = 11.0e-6;
const TEMPERATURE_DELTA: f64 = 30.0;

#[test]
fn heat_plane_triangle_refinement_matches_quad_patch_temperatures_and_heat_flow() {
    let triangle = solve_heat_plane_triangle_2d(&heat_triangle_patch())
        .expect("two-triangle heat patch should solve");
    let quad = solve_heat_plane_quad_2d(&heat_quad_patch()).expect("quad heat patch should solve");

    assert_eq!(triangle.nodes.len(), quad.nodes.len());
    for (triangle_node, quad_node) in triangle.nodes.iter().zip(quad.nodes.iter()) {
        assert_close(triangle_node.temperature, quad_node.temperature);
    }

    assert_close(triangle.max_temperature, quad.max_temperature);
    assert_close(triangle.max_heat_flux, quad.max_heat_flux);
    assert_close(
        triangle.total_abs_heat_flow_rate,
        quad.total_abs_heat_flow_rate,
    );
}

#[test]
fn heat_plane_triangle_linear_field_is_diagonal_invariant_and_conductivity_scaled() {
    let diagonal_a = solve_heat_plane_triangle_2d(&heat_triangle_patch())
        .expect("first diagonal heat patch should solve");
    let diagonal_b = solve_heat_plane_triangle_2d(&heat_triangle_cross_diagonal_patch(CONDUCTIVITY))
        .expect("second diagonal heat patch should solve");
    let perturbed =
        solve_heat_plane_triangle_2d(&heat_triangle_cross_diagonal_patch(CONDUCTIVITY * 1.07))
            .expect("conductivity-perturbed heat patch should solve");

    assert_eq!(diagonal_a.nodes.len(), diagonal_b.nodes.len());
    for (left, right) in diagonal_a.nodes.iter().zip(diagonal_b.nodes.iter()) {
        assert_close(left.temperature, right.temperature);
    }

    for element in diagonal_b.elements.iter() {
        assert_close(element.temperature_gradient_x, 0.0);
        assert_close(element.temperature_gradient_y, -80.0);
        assert_close(element.heat_flux_x, 0.0);
        assert_close(element.heat_flux_y, 3600.0);
    }

    assert_close(diagonal_a.max_heat_flux, diagonal_b.max_heat_flux);
    assert_close(
        diagonal_a.total_abs_heat_flow_rate,
        diagonal_b.total_abs_heat_flow_rate,
    );
    assert_close(perturbed.max_heat_flux / diagonal_b.max_heat_flux, 1.07);
    assert_close(
        perturbed.total_abs_heat_flow_rate / diagonal_b.total_abs_heat_flow_rate,
        1.07,
    );
    assert_close(
        perturbed.elements[0].temperature_gradient_y,
        diagonal_b.elements[0].temperature_gradient_y,
    );
}

#[test]
fn thermal_plane_triangle_refinement_matches_quad_patch_stress_and_energy() {
    let triangle = solve_thermal_plane_triangle_2d(&thermal_triangle_patch())
        .expect("two-triangle thermal patch should solve");
    let quad = solve_thermal_plane_quad_2d(&thermal_quad_patch())
        .expect("quad thermal patch should solve");

    assert_eq!(triangle.nodes.len(), quad.nodes.len());
    for (triangle_node, quad_node) in triangle.nodes.iter().zip(quad.nodes.iter()) {
        assert_close(triangle_node.ux, quad_node.ux);
        assert_close(triangle_node.uy, quad_node.uy);
        assert_close(triangle_node.temperature_delta, quad_node.temperature_delta);
    }

    assert_close(triangle.max_displacement, quad.max_displacement);
    assert_close(triangle.max_temperature_delta, quad.max_temperature_delta);
    assert_close(triangle.max_stress, quad.max_stress);
    assert_close(triangle.total_strain_energy, quad.total_strain_energy);
    assert_close(
        triangle.max_strain_energy_density,
        quad.max_strain_energy_density,
    );
}

fn heat_triangle_patch() -> SolveHeatPlaneTriangle2dRequest {
    SolveHeatPlaneTriangle2dRequest {
        nodes: heat_nodes(),
        elements: vec![heat_tri("lower", 0, 1, 2), heat_tri("upper", 0, 2, 3)],
    }
}

fn heat_quad_patch() -> SolveHeatPlaneQuad2dRequest {
    SolveHeatPlaneQuad2dRequest {
        nodes: heat_nodes(),
        elements: vec![HeatPlaneQuadElementInput {
            id: "quad".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: THICKNESS,
            conductivity: CONDUCTIVITY,
        }],
    }
}

fn heat_triangle_cross_diagonal_patch(conductivity: f64) -> SolveHeatPlaneTriangle2dRequest {
    SolveHeatPlaneTriangle2dRequest {
        nodes: heat_nodes(),
        elements: vec![
            heat_tri_with_conductivity("left", 0, 1, 3, conductivity),
            heat_tri_with_conductivity("right", 1, 2, 3, conductivity),
        ],
    }
}

fn thermal_triangle_patch() -> SolveThermalPlaneTriangle2dRequest {
    SolveThermalPlaneTriangle2dRequest {
        nodes: thermal_nodes(),
        elements: vec![thermal_tri("lower", 0, 1, 2), thermal_tri("upper", 0, 2, 3)],
    }
}

fn thermal_quad_patch() -> SolveThermalPlaneQuad2dRequest {
    SolveThermalPlaneQuad2dRequest {
        nodes: thermal_nodes(),
        elements: vec![ThermalPlaneQuadElementInput {
            id: "quad".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: THICKNESS,
            youngs_modulus: YOUNGS_MODULUS,
            poisson_ratio: POISSON_RATIO,
            thermal_expansion: THERMAL_EXPANSION,
        }],
    }
}

fn heat_nodes() -> Vec<HeatPlaneNodeInput> {
    vec![
        heat_node("hot-left", 0.0, 0.0, true, 100.0),
        heat_node("hot-right", 1.0, 0.0, true, 100.0),
        heat_node("cold-right", 1.0, 1.0, true, 20.0),
        heat_node("cold-left", 0.0, 1.0, true, 20.0),
    ]
}

fn thermal_nodes() -> Vec<ThermalPlaneNodeInput> {
    vec![
        thermal_node("n0", 0.0, 0.0),
        thermal_node("n1", 1.0, 0.0),
        thermal_node("n2", 1.0, 1.0),
        thermal_node("n3", 0.0, 1.0),
    ]
}

fn heat_node(id: &str, x: f64, y: f64, fixed: bool, temperature: f64) -> HeatPlaneNodeInput {
    HeatPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_temperature: fixed,
        temperature,
        heat_load: 0.0,
    }
}

fn thermal_node(id: &str, x: f64, y: f64) -> ThermalPlaneNodeInput {
    ThermalPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x: true,
        fix_y: true,
        load_x: 0.0,
        load_y: 0.0,
        temperature_delta: TEMPERATURE_DELTA,
    }
}

fn heat_tri(
    id: &str,
    node_i: usize,
    node_j: usize,
    node_k: usize,
) -> HeatPlaneTriangleElementInput {
    HeatPlaneTriangleElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        thickness: THICKNESS,
        conductivity: CONDUCTIVITY,
    }
}

fn heat_tri_with_conductivity(
    id: &str,
    node_i: usize,
    node_j: usize,
    node_k: usize,
    conductivity: f64,
) -> HeatPlaneTriangleElementInput {
    HeatPlaneTriangleElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        thickness: THICKNESS,
        conductivity,
    }
}

fn thermal_tri(
    id: &str,
    node_i: usize,
    node_j: usize,
    node_k: usize,
) -> ThermalPlaneTriangleElementInput {
    ThermalPlaneTriangleElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        thickness: THICKNESS,
        youngs_modulus: YOUNGS_MODULUS,
        poisson_ratio: POISSON_RATIO,
        thermal_expansion: THERMAL_EXPANSION,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
