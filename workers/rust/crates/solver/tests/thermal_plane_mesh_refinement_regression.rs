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
