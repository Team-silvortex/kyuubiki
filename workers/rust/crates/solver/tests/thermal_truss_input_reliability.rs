use kyuubiki_protocol::{
    SolveThermalTruss2dRequest, SolveThermalTruss3dRequest, ThermalTruss2dElementInput,
    ThermalTruss2dNodeInput, ThermalTruss3dElementInput, ThermalTruss3dNodeInput,
};
use kyuubiki_solver::{solve_thermal_truss_2d, solve_thermal_truss_3d};

#[test]
fn thermal_truss_2d_rejects_non_finite_node_load() {
    let mut request = valid_thermal_truss_2d_request();
    request.nodes[2].load_y = f64::NAN;

    let error = solve_thermal_truss_2d(&request)
        .expect_err("non-finite 2d thermal truss node load should be rejected");
    assert!(
        error.contains("load must be finite"),
        "unexpected non-finite load error: {error}"
    );
}

#[test]
fn thermal_truss_2d_rejects_duplicate_element_nodes() {
    let mut request = valid_thermal_truss_2d_request();
    request.elements[0].node_j = request.elements[0].node_i;

    let error = solve_thermal_truss_2d(&request)
        .expect_err("duplicate 2d thermal truss element nodes should be rejected");
    assert!(
        error.contains("two distinct nodes"),
        "unexpected duplicate-node error: {error}"
    );
}

#[test]
fn thermal_truss_3d_rejects_non_finite_node_coordinate() {
    let mut request = valid_thermal_truss_3d_request();
    request.nodes[1].z = f64::INFINITY;

    let error = solve_thermal_truss_3d(&request)
        .expect_err("non-finite 3d thermal truss coordinate should be rejected");
    assert!(
        error.contains("coordinates must be finite"),
        "unexpected non-finite coordinate error: {error}"
    );
}

#[test]
fn thermal_truss_3d_rejects_duplicate_element_nodes() {
    let mut request = valid_thermal_truss_3d_request();
    request.elements[1].node_j = request.elements[1].node_i;

    let error = solve_thermal_truss_3d(&request)
        .expect_err("duplicate 3d thermal truss element nodes should be rejected");
    assert!(
        error.contains("two distinct nodes"),
        "unexpected duplicate-node error: {error}"
    );
}

fn valid_thermal_truss_2d_request() -> SolveThermalTruss2dRequest {
    SolveThermalTruss2dRequest {
        nodes: vec![
            thermal_truss_2d_node("n0", 0.0, 0.0, true, true),
            thermal_truss_2d_node("n1", 1.0, 0.0, false, true),
            thermal_truss_2d_node("n2", 0.5, 0.75, false, false),
        ],
        elements: vec![
            thermal_truss_2d_element("left", 0, 2),
            thermal_truss_2d_element("right", 1, 2),
            thermal_truss_2d_element("base", 0, 1),
        ],
    }
}

fn valid_thermal_truss_3d_request() -> SolveThermalTruss3dRequest {
    SolveThermalTruss3dRequest {
        nodes: vec![
            thermal_truss_3d_node("n0", 0.0, 0.0, 0.0),
            thermal_truss_3d_node("n1", 1.0, 0.0, 0.0),
            thermal_truss_3d_node("n2", 0.0, 1.0, 0.0),
        ],
        elements: vec![
            thermal_truss_3d_element("edge_01", 0, 1),
            thermal_truss_3d_element("edge_12", 1, 2),
            thermal_truss_3d_element("edge_20", 2, 0),
        ],
    }
}

fn thermal_truss_2d_node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
) -> ThermalTruss2dNodeInput {
    ThermalTruss2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x: 0.0,
        load_y: -10.0,
        temperature_delta: 20.0,
    }
}

fn thermal_truss_2d_element(id: &str, node_i: usize, node_j: usize) -> ThermalTruss2dElementInput {
    ThermalTruss2dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.01,
        youngs_modulus: 70.0e9,
        thermal_expansion: 12.0e-6,
    }
}

fn thermal_truss_3d_node(id: &str, x: f64, y: f64, z: f64) -> ThermalTruss3dNodeInput {
    ThermalTruss3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x: true,
        fix_y: true,
        fix_z: true,
        load_x: 0.0,
        load_y: 0.0,
        load_z: 0.0,
        temperature_delta: 20.0,
    }
}

fn thermal_truss_3d_element(id: &str, node_i: usize, node_j: usize) -> ThermalTruss3dElementInput {
    ThermalTruss3dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.01,
        youngs_modulus: 70.0e9,
        thermal_expansion: 12.0e-6,
    }
}
