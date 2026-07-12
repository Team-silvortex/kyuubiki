use kyuubiki_protocol::{
    SolveSpring1dRequest, SolveSpring2dRequest, SolveSpring3dRequest, Spring1dElementInput,
    Spring1dNodeInput, Spring2dElementInput, Spring2dNodeInput, Spring3dElementInput,
    Spring3dNodeInput,
};
use kyuubiki_solver::{solve_spring_1d, solve_spring_2d, solve_spring_3d};

#[test]
fn spring_1d_rejects_non_finite_node_coordinates_and_loads() {
    let mut request = spring_1d_request();
    request.nodes[1].x = f64::NAN;
    let error = solve_spring_1d(&request).expect_err("NaN 1D spring coordinate should fail");
    assert!(
        error.contains("coordinates and loads must be finite"),
        "unexpected 1D spring node error: {error}"
    );

    let mut request = spring_1d_request();
    request.nodes[1].load_x = f64::INFINITY;
    let error = solve_spring_1d(&request).expect_err("infinite 1D spring load should fail");
    assert!(
        error.contains("coordinates and loads must be finite"),
        "unexpected 1D spring load error: {error}"
    );
}

#[test]
fn spring_1d_rejects_duplicate_element_nodes() {
    let mut request = spring_1d_request();
    request.elements[0].node_j = request.elements[0].node_i;

    let error = solve_spring_1d(&request).expect_err("duplicate 1D spring nodes should fail");
    assert!(
        error.contains("two distinct nodes"),
        "unexpected 1D spring duplicate-node error: {error}"
    );
}

#[test]
fn spring_2d_rejects_non_finite_node_coordinates_and_loads() {
    let mut request = spring_2d_request();
    request.nodes[1].y = f64::NAN;
    let error = solve_spring_2d(&request).expect_err("NaN 2D spring coordinate should fail");
    assert!(
        error.contains("coordinates and loads must be finite"),
        "unexpected 2D spring node error: {error}"
    );

    let mut request = spring_2d_request();
    request.nodes[1].load_y = f64::INFINITY;
    let error = solve_spring_2d(&request).expect_err("infinite 2D spring load should fail");
    assert!(
        error.contains("coordinates and loads must be finite"),
        "unexpected 2D spring load error: {error}"
    );
}

#[test]
fn spring_2d_rejects_duplicate_element_nodes() {
    let mut request = spring_2d_request();
    request.elements[0].node_j = request.elements[0].node_i;

    let error = solve_spring_2d(&request).expect_err("duplicate 2D spring nodes should fail");
    assert!(
        error.contains("two distinct nodes"),
        "unexpected 2D spring duplicate-node error: {error}"
    );
}

#[test]
fn spring_3d_rejects_non_finite_node_coordinates_and_loads() {
    let mut request = spring_3d_request();
    request.nodes[1].z = f64::NAN;
    let error = solve_spring_3d(&request).expect_err("NaN 3D spring coordinate should fail");
    assert!(
        error.contains("coordinates and loads must be finite"),
        "unexpected 3D spring node error: {error}"
    );

    let mut request = spring_3d_request();
    request.nodes[1].load_z = f64::INFINITY;
    let error = solve_spring_3d(&request).expect_err("infinite 3D spring load should fail");
    assert!(
        error.contains("coordinates and loads must be finite"),
        "unexpected 3D spring load error: {error}"
    );
}

#[test]
fn spring_3d_rejects_duplicate_element_nodes() {
    let mut request = spring_3d_request();
    request.elements[0].node_j = request.elements[0].node_i;

    let error = solve_spring_3d(&request).expect_err("duplicate 3D spring nodes should fail");
    assert!(
        error.contains("two distinct nodes"),
        "unexpected 3D spring duplicate-node error: {error}"
    );
}

fn spring_1d_request() -> SolveSpring1dRequest {
    SolveSpring1dRequest {
        nodes: vec![
            Spring1dNodeInput {
                id: "fixed".to_string(),
                x: 0.0,
                fix_x: true,
                load_x: 0.0,
            },
            Spring1dNodeInput {
                id: "tip".to_string(),
                x: 1.0,
                fix_x: false,
                load_x: 10.0,
            },
        ],
        elements: vec![Spring1dElementInput {
            id: "k0".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: 100.0,
        }],
    }
}

fn spring_2d_request() -> SolveSpring2dRequest {
    SolveSpring2dRequest {
        nodes: vec![
            Spring2dNodeInput {
                id: "fixed".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
            },
            Spring2dNodeInput {
                id: "tip".to_string(),
                x: 1.0,
                y: 0.0,
                fix_x: false,
                fix_y: false,
                load_x: 10.0,
                load_y: -5.0,
            },
        ],
        elements: vec![Spring2dElementInput {
            id: "k0".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: 100.0,
        }],
    }
}

fn spring_3d_request() -> SolveSpring3dRequest {
    SolveSpring3dRequest {
        nodes: vec![
            Spring3dNodeInput {
                id: "fixed".to_string(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                fix_x: true,
                fix_y: true,
                fix_z: true,
                load_x: 0.0,
                load_y: 0.0,
                load_z: 0.0,
            },
            Spring3dNodeInput {
                id: "tip".to_string(),
                x: 1.0,
                y: 0.0,
                z: 0.0,
                fix_x: false,
                fix_y: false,
                fix_z: false,
                load_x: 10.0,
                load_y: -5.0,
                load_z: 3.0,
            },
        ],
        elements: vec![Spring3dElementInput {
            id: "k0".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: 100.0,
        }],
    }
}
