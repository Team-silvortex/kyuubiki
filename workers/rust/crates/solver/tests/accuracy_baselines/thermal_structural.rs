use super::common::*;

#[test]
fn accuracy_baseline_thermal_truss_3d_restrained_uniform_rise() {
    let result = solve_thermal_truss_3d(&SolveThermalTruss3dRequest {
        nodes: vec![
            ThermalTruss3dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                fix_x: true,
                fix_y: true,
                fix_z: true,
                load_x: 0.0,
                load_y: 0.0,
                load_z: 0.0,
                temperature_delta: 40.0,
            },
            ThermalTruss3dNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                z: 0.0,
                fix_x: true,
                fix_y: true,
                fix_z: true,
                load_x: 0.0,
                load_y: 0.0,
                load_z: 0.0,
                temperature_delta: 40.0,
            },
            ThermalTruss3dNodeInput {
                id: "n2".to_string(),
                x: 0.0,
                y: 1.0,
                z: 0.0,
                fix_x: true,
                fix_y: true,
                fix_z: true,
                load_x: 0.0,
                load_y: 0.0,
                load_z: 0.0,
                temperature_delta: 40.0,
            },
        ],
        elements: vec![
            ThermalTruss3dElementInput {
                id: "tt3-0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                youngs_modulus: 210.0e9,
                thermal_expansion: 12.0e-6,
            },
            ThermalTruss3dElementInput {
                id: "tt3-1".to_string(),
                node_i: 1,
                node_j: 2,
                area: 0.01,
                youngs_modulus: 210.0e9,
                thermal_expansion: 12.0e-6,
            },
            ThermalTruss3dElementInput {
                id: "tt3-2".to_string(),
                node_i: 2,
                node_j: 0,
                area: 0.01,
                youngs_modulus: 210.0e9,
                thermal_expansion: 12.0e-6,
            },
        ],
    })
    .expect("thermal_truss_3d baseline should solve");

    assert_close_abs(
        result.max_displacement,
        0.0,
        1.0e-12,
        "thermal_truss_3d max displacement",
    );
    assert_close_rel(
        result.max_stress,
        100_800_000.0,
        1.0e-9,
        "thermal_truss_3d max stress magnitude",
    );
    assert_close_rel(
        result.max_axial_force,
        1_008_000.0,
        1.0e-9,
        "thermal_truss_3d max axial force magnitude",
    );
    assert_close_abs(
        result.max_temperature_delta,
        40.0,
        1.0e-12,
        "thermal_truss_3d max temperature delta",
    );
    assert!(
        result.elements[0].stress < 0.0,
        "thermal_truss_3d stress sign should indicate compression"
    );
}

#[test]
fn accuracy_baseline_thermal_truss_2d_sample_fixture() {
    let result = solve_thermal_truss_2d(&SolveThermalTruss2dRequest {
        nodes: vec![
            ThermalTruss2dNodeInput {
                id: "tt0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 40.0,
            },
            ThermalTruss2dNodeInput {
                id: "tt1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_x: false,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 40.0,
            },
            ThermalTruss2dNodeInput {
                id: "tt2".to_string(),
                x: 0.5,
                y: 0.8,
                fix_x: false,
                fix_y: false,
                load_x: 0.0,
                load_y: -400.0,
                temperature_delta: 25.0,
            },
        ],
        elements: vec![
            ThermalTruss2dElementInput {
                id: "tte0".to_string(),
                node_i: 0,
                node_j: 2,
                area: 0.01,
                youngs_modulus: 70.0e9,
                thermal_expansion: 12.0e-6,
            },
            ThermalTruss2dElementInput {
                id: "tte1".to_string(),
                node_i: 1,
                node_j: 2,
                area: 0.01,
                youngs_modulus: 70.0e9,
                thermal_expansion: 12.0e-6,
            },
            ThermalTruss2dElementInput {
                id: "tte2".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                youngs_modulus: 70.0e9,
                thermal_expansion: 12.0e-6,
            },
        ],
    })
    .expect("thermal_truss_2d baseline should solve");

    assert_close_abs(
        result.max_displacement,
        4.801785714285713e-4,
        1.0e-12,
        "thermal_truss_2d max displacement",
    );
    assert_close_abs(
        result.max_axial_force,
        235.84952830143558,
        1.0e-9,
        "thermal_truss_2d max axial force",
    );
    assert_close_abs(
        result.max_stress,
        23584.952830143557,
        1.0e-6,
        "thermal_truss_2d max stress",
    );
    assert_close_abs(
        result.max_temperature_delta,
        40.0,
        1.0e-12,
        "thermal_truss_2d max temperature delta",
    );
    assert_close_abs(
        result.nodes[1].ux,
        4.801785714285713e-4,
        1.0e-12,
        "thermal_truss_2d node-1 ux",
    );
    assert_close_abs(
        result.nodes[2].uy,
        2.834443641425211e-4,
        1.0e-12,
        "thermal_truss_2d node-2 uy",
    );
}

#[test]
fn accuracy_baseline_thermal_plane_triangle_2d_restrained_patch() {
    let result = solve_thermal_plane_triangle_2d(&SolveThermalPlaneTriangle2dRequest {
        nodes: vec![
            ThermalPlaneNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 40.0,
            },
            ThermalPlaneNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 40.0,
            },
            ThermalPlaneNodeInput {
                id: "n2".to_string(),
                x: 1.0,
                y: 1.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 40.0,
            },
            ThermalPlaneNodeInput {
                id: "n3".to_string(),
                x: 0.0,
                y: 1.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 40.0,
            },
        ],
        elements: vec![
            ThermalPlaneTriangleElementInput {
                id: "tp0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.02,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
                thermal_expansion: 12.0e-6,
            },
            ThermalPlaneTriangleElementInput {
                id: "tp1".to_string(),
                node_i: 0,
                node_j: 2,
                node_k: 3,
                thickness: 0.02,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
                thermal_expansion: 12.0e-6,
            },
        ],
    })
    .expect("thermal_plane_triangle_2d baseline should solve");

    assert_close_abs(
        result.max_displacement,
        0.0,
        1.0e-12,
        "thermal_plane_triangle_2d max displacement",
    );
    assert_close_abs(
        result.max_stress,
        50149253.731343284,
        1.0e-6,
        "thermal_plane_triangle_2d max stress",
    );
    assert_close_abs(
        result.max_temperature_delta,
        40.0,
        1.0e-12,
        "thermal_plane_triangle_2d max temperature delta",
    );
    assert_close_abs(
        result.elements[0].stress_x,
        -50149253.731343284,
        1.0e-6,
        "thermal_plane_triangle_2d first element stress x",
    );
    assert_close_abs(
        result.elements[1].stress_y,
        -50149253.731343284,
        1.0e-6,
        "thermal_plane_triangle_2d second element stress y",
    );
}

#[test]
fn accuracy_baseline_thermal_plane_quad_2d_restrained_patch() {
    let result = solve_thermal_plane_quad_2d(&SolveThermalPlaneQuad2dRequest {
        nodes: vec![
            ThermalPlaneNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 30.0,
            },
            ThermalPlaneNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 30.0,
            },
            ThermalPlaneNodeInput {
                id: "n2".to_string(),
                x: 1.0,
                y: 1.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 30.0,
            },
            ThermalPlaneNodeInput {
                id: "n3".to_string(),
                x: 0.0,
                y: 1.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 30.0,
            },
        ],
        elements: vec![ThermalPlaneQuadElementInput {
            id: "tq0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
            thermal_expansion: 11.0e-6,
        }],
    })
    .expect("thermal_plane_quad_2d baseline should solve");

    assert_close_abs(
        result.max_displacement,
        0.0,
        1.0e-12,
        "thermal_plane_quad_2d max displacement",
    );
    assert_close_abs(
        result.max_stress,
        34477611.940298505,
        1.0e-6,
        "thermal_plane_quad_2d max stress",
    );
    assert_close_abs(
        result.max_temperature_delta,
        30.0,
        1.0e-12,
        "thermal_plane_quad_2d max temperature delta",
    );
    assert_close_abs(
        result.elements[0].stress_x,
        -34477611.940298505,
        1.0e-6,
        "thermal_plane_quad_2d stress_x",
    );
    assert_close_abs(
        result.elements[0].stress_y,
        -34477611.940298505,
        1.0e-6,
        "thermal_plane_quad_2d stress_y",
    );
    assert_close_abs(
        result.elements[0].average_temperature_delta,
        30.0,
        1.0e-12,
        "thermal_plane_quad_2d average temperature delta",
    );
    assert_close_abs(
        result.elements[0].mechanical_strain_x,
        -3.3e-4,
        1.0e-12,
        "thermal_plane_quad_2d mechanical_strain_x",
    );
    assert_close_abs(
        result.elements[0].mechanical_strain_y,
        -3.3e-4,
        1.0e-12,
        "thermal_plane_quad_2d mechanical_strain_y",
    );
}
