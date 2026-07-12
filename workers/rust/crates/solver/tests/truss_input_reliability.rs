use kyuubiki_protocol::{
    SolveTruss2dRequest, SolveTruss3dRequest, Truss3dElementInput, Truss3dNodeInput,
    TrussElementInput, TrussNodeInput,
};
use kyuubiki_solver::{solve_truss_2d, solve_truss_3d};

#[test]
fn truss_2d_rejects_non_finite_node_inputs_and_invalid_lengths() {
    let mut request = truss_2d_request();
    request.nodes[2].x = f64::NAN;
    assert!(solve_truss_2d(&request).is_err());

    let mut request = truss_2d_request();
    request.nodes[2].load_y = f64::INFINITY;
    assert!(solve_truss_2d(&request).is_err());

    let mut request = truss_2d_request();
    request.elements[0].node_j = 0;
    assert!(solve_truss_2d(&request).is_err());

    let mut request = truss_2d_request();
    request.nodes[2].x = request.nodes[0].x;
    request.nodes[2].y = request.nodes[0].y;
    assert!(solve_truss_2d(&request).is_err());
}

#[test]
fn truss_3d_rejects_non_finite_node_inputs_and_invalid_lengths() {
    let mut request = truss_3d_request();
    request.nodes[3].z = f64::NAN;
    assert!(solve_truss_3d(&request).is_err());

    let mut request = truss_3d_request();
    request.nodes[3].load_z = f64::NEG_INFINITY;
    assert!(solve_truss_3d(&request).is_err());

    let mut request = truss_3d_request();
    request.elements[3].node_j = 0;
    assert!(solve_truss_3d(&request).is_err());

    let mut request = truss_3d_request();
    request.nodes[3].x = request.nodes[0].x;
    request.nodes[3].y = request.nodes[0].y;
    request.nodes[3].z = request.nodes[0].z;
    assert!(solve_truss_3d(&request).is_err());
}

fn truss_2d_request() -> SolveTruss2dRequest {
    SolveTruss2dRequest {
        nodes: vec![
            truss_2d_node("left_support", 0.0, 0.0, true, true, 0.0),
            truss_2d_node("right_roller", 1.0, 0.0, false, true, 0.0),
            truss_2d_node("loaded_apex", 0.5, 0.75, false, false, -1000.0),
        ],
        elements: vec![
            TrussElementInput {
                id: "left_web".to_string(),
                node_i: 0,
                node_j: 2,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
            TrussElementInput {
                id: "right_web".to_string(),
                node_i: 1,
                node_j: 2,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
        ],
    }
}

fn truss_2d_node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    load_y: f64,
) -> TrussNodeInput {
    TrussNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x: 0.0,
        load_y,
    }
}

fn truss_3d_request() -> SolveTruss3dRequest {
    SolveTruss3dRequest {
        nodes: vec![
            truss_3d_node("base_0", 0.0, 0.0, 0.0, true, true, true, 0.0),
            truss_3d_node("base_1", 1.2, 0.0, 0.0, true, true, true, 0.0),
            truss_3d_node("base_2", 0.0, 1.2, 0.0, true, true, true, 0.0),
            truss_3d_node("loaded_top", 0.35, 0.35, 1.0, false, false, false, -1600.0),
        ],
        elements: vec![
            truss_3d_element("base_01", 0, 1),
            truss_3d_element("base_12", 1, 2),
            truss_3d_element("base_20", 2, 0),
            truss_3d_element("leg_0", 0, 3),
        ],
    }
}

fn truss_3d_node(
    id: &str,
    x: f64,
    y: f64,
    z: f64,
    fix_x: bool,
    fix_y: bool,
    fix_z: bool,
    load_z: f64,
) -> Truss3dNodeInput {
    Truss3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x,
        fix_y,
        fix_z,
        load_x: 0.0,
        load_y: 0.0,
        load_z,
    }
}

fn truss_3d_element(id: &str, node_i: usize, node_j: usize) -> Truss3dElementInput {
    Truss3dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.01,
        youngs_modulus: 70.0e9,
    }
}
