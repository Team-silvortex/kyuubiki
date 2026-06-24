use super::common::*;

#[test]
fn accuracy_baseline_truss_2d_small_triangular_patch() {
    let result = solve_truss_2d(&SolveTruss2dRequest {
        nodes: vec![
            TrussNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
            },
            TrussNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_x: false,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
            },
            TrussNodeInput {
                id: "n2".to_string(),
                x: 0.5,
                y: 0.75,
                fix_x: false,
                fix_y: false,
                load_x: 0.0,
                load_y: -1000.0,
            },
        ],
        elements: vec![
            TrussElementInput {
                id: "e0".to_string(),
                node_i: 0,
                node_j: 2,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
            TrussElementInput {
                id: "e1".to_string(),
                node_i: 1,
                node_j: 2,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
            TrussElementInput {
                id: "e2".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
        ],
    })
    .expect("truss baseline should solve");

    assert_close_abs(
        result.max_displacement,
        1.114463950892853e-6,
        1.0e-15,
        "truss_2d max displacement",
    );
    assert_close_abs(
        result.max_stress,
        6.009252125773316e4,
        1.0e-6,
        "truss_2d max stress",
    );
    assert_close_abs(
        result.nodes[2].ux,
        2.380952380952381e-7,
        1.0e-15,
        "truss_2d tip ux",
    );
    assert_close_abs(
        result.nodes[2].uy,
        -1.088733463909362e-6,
        1.0e-15,
        "truss_2d tip uy",
    );
    assert_close_abs(
        result.elements[0].axial_force,
        -6.009252125773316e2,
        1.0e-9,
        "truss_2d leading element axial force",
    );
}

#[test]
fn accuracy_baseline_truss_3d_space_frame_pyramid_fixture() {
    let result = solve_truss_3d(&SolveTruss3dRequest {
        nodes: vec![
            Truss3dNodeInput {
                id: "b0".to_string(),
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
            Truss3dNodeInput {
                id: "b1".to_string(),
                x: 1.2,
                y: 0.0,
                z: 0.0,
                fix_x: true,
                fix_y: true,
                fix_z: true,
                load_x: 0.0,
                load_y: 0.0,
                load_z: 0.0,
            },
            Truss3dNodeInput {
                id: "b2".to_string(),
                x: 0.0,
                y: 1.2,
                z: 0.0,
                fix_x: true,
                fix_y: true,
                fix_z: true,
                load_x: 0.0,
                load_y: 0.0,
                load_z: 0.0,
            },
            Truss3dNodeInput {
                id: "top".to_string(),
                x: 0.35,
                y: 0.35,
                z: 1.0,
                fix_x: false,
                fix_y: false,
                fix_z: false,
                load_x: 0.0,
                load_y: 0.0,
                load_z: -1600.0,
            },
        ],
        elements: vec![
            Truss3dElementInput {
                id: "e0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
            Truss3dElementInput {
                id: "e1".to_string(),
                node_i: 1,
                node_j: 2,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
            Truss3dElementInput {
                id: "e2".to_string(),
                node_i: 2,
                node_j: 0,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
            Truss3dElementInput {
                id: "e3".to_string(),
                node_i: 0,
                node_j: 3,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
            Truss3dElementInput {
                id: "e4".to_string(),
                node_i: 1,
                node_j: 3,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
            Truss3dElementInput {
                id: "e5".to_string(),
                node_i: 2,
                node_j: 3,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
        ],
    })
    .expect("truss_3d baseline should solve");

    assert_close_abs(
        result.max_displacement,
        0.0000015799074540869988,
        1.0e-18,
        "truss_3d max displacement",
    );
    assert_close_abs(
        result.max_stress,
        74386.37868140468,
        1.0e-9,
        "truss_3d max stress",
    );
    assert_close_abs(
        result.nodes[3].ux,
        2.897530666749509e-7,
        1.0e-18,
        "truss_3d top ux",
    );
    assert_close_abs(
        result.nodes[3].uy,
        2.897530666749509e-7,
        1.0e-18,
        "truss_3d top uy",
    );
    assert_close_abs(
        result.nodes[3].uz,
        -0.0000015258420246488773,
        1.0e-18,
        "truss_3d top uz",
    );
    assert_close_abs(
        result.elements[3].stress,
        -74386.37868140468,
        1.0e-9,
        "truss_3d element-3 stress",
    );
    assert_close_abs(
        result.elements[4].stress,
        -63387.6959669619,
        1.0e-9,
        "truss_3d element-4 stress",
    );
    assert_close_abs(
        result.elements[5].stress,
        -63387.6959669619,
        1.0e-9,
        "truss_3d element-5 stress",
    );
}
