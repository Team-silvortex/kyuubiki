use super::common::*;

#[test]
fn accuracy_baseline_spring_2d_grid_fixture() {
    let result = solve_spring_2d(&SolveSpring2dRequest {
        nodes: vec![
            Spring2dNodeInput {
                id: "s0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
            },
            Spring2dNodeInput {
                id: "s1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_x: false,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
            },
            Spring2dNodeInput {
                id: "s2".to_string(),
                x: 1.0,
                y: 1.0,
                fix_x: false,
                fix_y: false,
                load_x: 1200.0,
                load_y: -600.0,
            },
            Spring2dNodeInput {
                id: "s3".to_string(),
                x: 0.0,
                y: 1.0,
                fix_x: true,
                fix_y: false,
                load_x: 0.0,
                load_y: 0.0,
            },
        ],
        elements: vec![
            Spring2dElementInput {
                id: "sp0".to_string(),
                node_i: 0,
                node_j: 1,
                stiffness: 25000.0,
            },
            Spring2dElementInput {
                id: "sp1".to_string(),
                node_i: 1,
                node_j: 2,
                stiffness: 18000.0,
            },
            Spring2dElementInput {
                id: "sp2".to_string(),
                node_i: 2,
                node_j: 3,
                stiffness: 22000.0,
            },
            Spring2dElementInput {
                id: "sp3".to_string(),
                node_i: 3,
                node_j: 0,
                stiffness: 18000.0,
            },
            Spring2dElementInput {
                id: "sp4".to_string(),
                node_i: 0,
                node_j: 2,
                stiffness: 12000.0,
            },
        ],
    })
    .expect("spring_2d baseline should solve");

    assert_close_abs(
        result.max_displacement,
        0.06339734949589224,
        1.0e-15,
        "spring_2d max displacement",
    );
    assert_close_abs(
        result.max_force,
        1120.754716981132,
        1.0e-12,
        "spring_2d max force",
    );
    assert_close_abs(
        result.nodes[2].ux,
        0.0509433962264151,
        1.0e-15,
        "spring_2d node-2 ux",
    );
    assert_close_abs(
        result.nodes[2].uy,
        -0.03773584905660377,
        1.0e-15,
        "spring_2d node-2 uy",
    );
    assert_close_abs(
        result.elements[2].force,
        1120.754716981132,
        1.0e-12,
        "spring_2d element-2 force",
    );
    assert_close_abs(
        result.elements[1].force,
        -679.2452830188679,
        1.0e-12,
        "spring_2d element-1 force",
    );
    assert_close_abs(
        result.elements[4].force,
        112.06975399937737,
        1.0e-12,
        "spring_2d diagonal force",
    );
}

#[test]
fn accuracy_baseline_spring_3d_cage_fixture() {
    let result = solve_spring_3d(&SolveSpring3dRequest {
        nodes: vec![
            Spring3dNodeInput {
                id: "s0".to_string(),
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
                id: "s1".to_string(),
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
            Spring3dNodeInput {
                id: "s2".to_string(),
                x: 0.0,
                y: 1.0,
                z: 0.0,
                fix_x: true,
                fix_y: true,
                fix_z: true,
                load_x: 0.0,
                load_y: 0.0,
                load_z: 0.0,
            },
            Spring3dNodeInput {
                id: "top".to_string(),
                x: 0.45,
                y: 0.35,
                z: 1.1,
                fix_x: false,
                fix_y: false,
                fix_z: false,
                load_x: 250.0,
                load_y: 0.0,
                load_z: -1100.0,
            },
        ],
        elements: vec![
            Spring3dElementInput {
                id: "k0".to_string(),
                node_i: 0,
                node_j: 3,
                stiffness: 18000.0,
            },
            Spring3dElementInput {
                id: "k1".to_string(),
                node_i: 1,
                node_j: 3,
                stiffness: 22000.0,
            },
            Spring3dElementInput {
                id: "k2".to_string(),
                node_i: 2,
                node_j: 3,
                stiffness: 16000.0,
            },
            Spring3dElementInput {
                id: "k3".to_string(),
                node_i: 0,
                node_j: 1,
                stiffness: 9000.0,
            },
            Spring3dElementInput {
                id: "k4".to_string(),
                node_i: 1,
                node_j: 2,
                stiffness: 9000.0,
            },
            Spring3dElementInput {
                id: "k5".to_string(),
                node_i: 2,
                node_j: 0,
                stiffness: 9000.0,
            },
        ],
    })
    .expect("spring_3d baseline should solve");

    assert_close_abs(
        result.max_displacement,
        0.05955868626521211,
        1.0e-15,
        "spring_3d max displacement",
    );
    assert_close_abs(
        result.max_force,
        803.0108273796119,
        1.0e-12,
        "spring_3d max force",
    );
    assert_close_abs(
        result.nodes[3].ux,
        0.037134189113355795,
        1.0e-15,
        "spring_3d top ux",
    );
    assert_close_abs(
        result.nodes[3].uy,
        0.03445543981481482,
        1.0e-15,
        "spring_3d top uy",
    );
    assert_close_abs(
        result.nodes[3].uz,
        -0.03132270383761861,
        1.0e-15,
        "spring_3d top uz",
    );
    assert_close_abs(
        result.elements[1].force,
        -803.0108273796119,
        1.0e-12,
        "spring_3d element-1 force",
    );
    assert_close_abs(
        result.elements[2].force,
        -474.11760144504234,
        1.0e-12,
        "spring_3d element-2 force",
    );
    assert_close_abs(
        result.elements[0].force,
        -82.59674462242567,
        1.0e-12,
        "spring_3d element-0 force",
    );
}

#[test]
fn accuracy_baseline_plane_triangle_2d_small_patch() {
    let result = solve_plane_triangle_2d(&SolvePlaneTriangle2dRequest {
        nodes: vec![
            PlaneNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
            },
            PlaneNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_x: false,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
            },
            PlaneNodeInput {
                id: "n2".to_string(),
                x: 1.0,
                y: 1.0,
                fix_x: false,
                fix_y: false,
                load_x: 0.0,
                load_y: -1000.0,
            },
            PlaneNodeInput {
                id: "n3".to_string(),
                x: 0.0,
                y: 1.0,
                fix_x: true,
                fix_y: false,
                load_x: 0.0,
                load_y: -1000.0,
            },
        ],
        elements: vec![
            PlaneTriangleElementInput {
                id: "p0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.02,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
            },
            PlaneTriangleElementInput {
                id: "p1".to_string(),
                node_i: 0,
                node_j: 2,
                node_k: 3,
                thickness: 0.02,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
            },
        ],
    })
    .expect("plane triangle baseline should solve");

    assert_close_abs(
        result.max_displacement,
        1.504347441414315e-6,
        1.0e-15,
        "plane_triangle_2d max displacement",
    );
    assert_close_abs(
        result.max_stress,
        1.0e5,
        1.0e-6,
        "plane_triangle_2d max stress",
    );
    assert_close_abs(
        result.nodes[2].ux,
        4.714285714285715e-7,
        1.0e-15,
        "plane_triangle_2d node 2 ux",
    );
    assert_close_abs(
        result.nodes[2].uy,
        -1.428571428571429e-6,
        1.0e-15,
        "plane_triangle_2d node 2 uy",
    );
    assert_close_abs(
        result.elements[0].von_mises,
        1.0e5,
        1.0e-6,
        "plane_triangle_2d element 0 von mises",
    );
}

#[test]
fn accuracy_baseline_plane_quad_2d_sample_fixture() {
    let result = solve_plane_quad_2d(&SolvePlaneQuad2dRequest {
        nodes: vec![
            PlaneNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
            },
            PlaneNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_x: false,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
            },
            PlaneNodeInput {
                id: "n2".to_string(),
                x: 1.0,
                y: 0.8,
                fix_x: false,
                fix_y: false,
                load_x: 200.0,
                load_y: -1200.0,
            },
            PlaneNodeInput {
                id: "n3".to_string(),
                x: 0.0,
                y: 0.8,
                fix_x: true,
                fix_y: false,
                load_x: 200.0,
                load_y: -1200.0,
            },
        ],
        elements: vec![PlaneQuadElementInput {
            id: "q0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02,
            youngs_modulus: 210.0e9,
            poisson_ratio: 0.3,
        }],
    })
    .expect("plane_quad_2d baseline should solve");

    assert_close_abs(
        result.max_displacement,
        5.333507749004975e-7,
        1.0e-12,
        "plane_quad_2d max displacement",
    );
    assert_close_abs(
        result.max_stress,
        126981.38527836032,
        1.0e-6,
        "plane_quad_2d max stress",
    );
    assert_close_abs(
        result.nodes[2].ux,
        2.576145151695419e-7,
        1.0e-12,
        "plane_quad_2d node 2 ux",
    );
    assert_close_abs(
        result.nodes[2].uy,
        -4.6700943316053366e-7,
        1.0e-12,
        "plane_quad_2d node 2 uy",
    );
    assert_close_abs(
        result.elements[0].stress_x,
        12500.0,
        1.0e-6,
        "plane_quad_2d stress_x",
    );
    assert_close_abs(
        result.elements[0].stress_y,
        -120000.0,
        1.0e-6,
        "plane_quad_2d stress_y",
    );
    assert_close_abs(
        result.elements[0].tau_xy,
        3048.7804878048746,
        1.0e-9,
        "plane_quad_2d tau_xy",
    );
}
