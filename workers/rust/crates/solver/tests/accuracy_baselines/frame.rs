use super::common::*;

#[test]
fn accuracy_baseline_beam_1d_tip_loaded_cantilever() {
    let result = solve_beam_1d(&SolveBeam1dRequest {
        nodes: vec![
            Beam1dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                fix_y: true,
                fix_rz: true,
                load_y: 0.0,
                moment_z: 0.0,
            },
            Beam1dNodeInput {
                id: "n1".to_string(),
                x: 2.0,
                fix_y: false,
                fix_rz: false,
                load_y: -1000.0,
                moment_z: 0.0,
            },
        ],
        elements: vec![Beam1dElementInput {
            id: "b0".to_string(),
            node_i: 0,
            node_j: 1,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
            distributed_load_y: 0.0,
        }],
    })
    .expect("beam baseline should solve");

    assert_close_abs(
        result.max_displacement,
        0.0015873015873015873,
        1.0e-12,
        "beam_1d max displacement",
    );
    assert_close_abs(
        result.max_rotation,
        0.0011904761904761906,
        1.0e-12,
        "beam_1d max rotation",
    );
    assert_close_abs(result.max_moment, 2000.0, 1.0e-6, "beam_1d max moment");
    assert_close_abs(
        result.max_stress,
        1.25e7,
        1.0e-2,
        "beam_1d max bending stress",
    );
}

#[test]
fn accuracy_baseline_torsion_1d_sample_fixture() {
    let result = solve_torsion_1d(&SolveTorsion1dRequest {
        nodes: vec![
            Torsion1dNodeInput {
                id: "t0".to_string(),
                x: 0.0,
                fix_rz: true,
                torque_z: 0.0,
            },
            Torsion1dNodeInput {
                id: "t1".to_string(),
                x: 1.5,
                fix_rz: false,
                torque_z: 2500.0,
            },
        ],
        elements: vec![Torsion1dElementInput {
            id: "s0".to_string(),
            node_i: 0,
            node_j: 1,
            shear_modulus: 79.0e9,
            polar_moment: 1.8e-6,
            section_modulus: 1.2e-4,
        }],
    })
    .expect("torsion_1d baseline should solve");

    assert_close_abs(
        result.max_rotation,
        0.026371308016877638,
        1.0e-15,
        "torsion_1d max rotation",
    );
    assert_close_abs(result.max_torque, 2500.0, 1.0e-12, "torsion_1d max torque");
    assert_close_abs(
        result.max_stress,
        20833333.333333332,
        1.0e-9,
        "torsion_1d max stress",
    );
    assert_close_abs(
        result.nodes[1].rz,
        0.026371308016877638,
        1.0e-15,
        "torsion_1d tip rotation",
    );
    assert_close_abs(
        result.elements[0].torque,
        2500.0,
        1.0e-12,
        "torsion_1d element torque",
    );
    assert_close_abs(
        result.elements[0].twist,
        0.026371308016877638,
        1.0e-15,
        "torsion_1d element twist",
    );
    assert_close_abs(
        result.elements[0].shear_stress,
        20833333.333333332,
        1.0e-9,
        "torsion_1d element shear stress",
    );
}

#[test]
fn accuracy_baseline_frame_2d_tip_loaded_cantilever() {
    let result = solve_frame_2d(&SolveFrame2dRequest {
        nodes: vec![
            Frame2dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                fix_rz: true,
                load_x: 0.0,
                load_y: 0.0,
                moment_z: 0.0,
            },
            Frame2dNodeInput {
                id: "n1".to_string(),
                x: 2.0,
                y: 0.0,
                fix_x: false,
                fix_y: false,
                fix_rz: false,
                load_x: 0.0,
                load_y: -1000.0,
                moment_z: 0.0,
            },
        ],
        elements: vec![Frame2dElementInput {
            id: "f0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
        }],
    })
    .expect("frame baseline should solve");

    let expected_tip_uy = 0.0015873015873015873;
    let expected_tip_rz = 0.0011904761904761906;

    assert_close_abs(
        result.max_displacement,
        expected_tip_uy,
        1.0e-12,
        "frame_2d max displacement",
    );
    assert_close_abs(
        result.max_rotation,
        expected_tip_rz,
        1.0e-12,
        "frame_2d max rotation",
    );
    assert_close_abs(result.max_moment, 2000.0, 1.0e-6, "frame_2d max moment");
    assert_close_abs(
        result.max_stress,
        1.25e7,
        1.0e-2,
        "frame_2d max combined stress",
    );
    assert_close_abs(
        result.nodes[1].uy.abs(),
        expected_tip_uy,
        1.0e-12,
        "frame_2d tip uy magnitude",
    );
    assert_close_abs(
        result.nodes[1].rz.abs(),
        expected_tip_rz,
        1.0e-12,
        "frame_2d tip rotation magnitude",
    );
}

#[test]
fn accuracy_baseline_frame_3d_tip_loaded_cantilever() {
    let result = solve_frame_3d(&SolveFrame3dRequest {
        nodes: vec![
            Frame3dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                fix_x: true,
                fix_y: true,
                fix_z: true,
                fix_rx: true,
                fix_ry: true,
                fix_rz: true,
                load_x: 0.0,
                load_y: 0.0,
                load_z: 0.0,
                moment_x: 0.0,
                moment_y: 0.0,
                moment_z: 0.0,
            },
            Frame3dNodeInput {
                id: "n1".to_string(),
                x: 2.0,
                y: 0.0,
                z: 0.0,
                fix_x: false,
                fix_y: false,
                fix_z: false,
                fix_rx: false,
                fix_ry: false,
                fix_rz: false,
                load_x: 0.0,
                load_y: -1000.0,
                load_z: 0.0,
                moment_x: 0.0,
                moment_y: 0.0,
                moment_z: 0.0,
            },
        ],
        elements: vec![Frame3dElementInput {
            id: "f0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            shear_modulus: 80.0e9,
            torsion_constant: 5.0e-6,
            moment_of_inertia_y: 8.0e-6,
            moment_of_inertia_z: 8.0e-6,
            section_modulus_y: 1.6e-4,
            section_modulus_z: 1.6e-4,
        }],
    })
    .expect("frame_3d baseline should solve");

    assert_close_abs(
        result.max_displacement,
        0.0015873015873015873,
        1.0e-12,
        "frame_3d max displacement",
    );
    assert_close_abs(
        result.max_rotation,
        0.0011904761904761906,
        1.0e-12,
        "frame_3d max rotation",
    );
    assert_close_abs(result.max_moment, 2000.0, 1.0e-6, "frame_3d max moment");
    assert_close_abs(
        result.max_stress,
        1.25e7,
        1.0e-2,
        "frame_3d max combined stress",
    );
    assert_close_abs(
        result.nodes[1].uy,
        -0.0015873015873015873,
        1.0e-12,
        "frame_3d tip uy",
    );
    assert_close_abs(
        result.nodes[1].rz,
        -0.0011904761904761906,
        1.0e-12,
        "frame_3d tip rotation z",
    );
}

#[test]
fn accuracy_baseline_thermal_frame_3d_restrained_uniform_rise_and_gradients() {
    let result = solve_thermal_frame_3d(&SolveThermalFrame3dRequest {
        nodes: vec![
            ThermalFrame3dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                fix_x: true,
                fix_y: true,
                fix_z: true,
                fix_rx: true,
                fix_ry: true,
                fix_rz: true,
                load_x: 0.0,
                load_y: 0.0,
                load_z: 0.0,
                moment_x: 0.0,
                moment_y: 0.0,
                moment_z: 0.0,
                temperature_delta: 35.0,
            },
            ThermalFrame3dNodeInput {
                id: "n1".to_string(),
                x: 2.0,
                y: 0.0,
                z: 0.0,
                fix_x: true,
                fix_y: true,
                fix_z: true,
                fix_rx: true,
                fix_ry: true,
                fix_rz: true,
                load_x: 0.0,
                load_y: 0.0,
                load_z: 0.0,
                moment_x: 0.0,
                moment_y: 0.0,
                moment_z: 0.0,
                temperature_delta: 35.0,
            },
        ],
        elements: vec![ThermalFrame3dElementInput {
            id: "tf3-0".to_string(),
            node_i: 0,
            node_j: 1,
            local_y_axis: None,
            area: 0.02,
            youngs_modulus: 210.0e9,
            shear_modulus: 80.0e9,
            torsion_constant: 5.0e-6,
            moment_of_inertia_y: 8.0e-6,
            moment_of_inertia_z: 6.0e-6,
            section_modulus_y: 1.6e-4,
            section_modulus_z: 1.2e-4,
            thermal_expansion: 12.0e-6,
            section_depth_y: 0.2,
            section_depth_z: 0.15,
            temperature_gradient_y: 30.0,
            temperature_gradient_z: 20.0,
        }],
        directional_springs: Vec::new(),
        directional_rotational_springs: Vec::new(),
        directional_constraints: Vec::new(),
        directional_rotational_constraints: Vec::new(),
    })
    .expect("thermal_frame_3d baseline should solve");

    assert_close_abs(
        result.max_displacement,
        0.0,
        1.0e-12,
        "thermal_frame_3d max displacement",
    );
    assert_close_abs(
        result.max_rotation,
        0.0,
        1.0e-12,
        "thermal_frame_3d max rotation",
    );
    assert_close_rel(
        result.max_axial_force,
        1.764e6,
        1.0e-9,
        "thermal_frame_3d max axial force",
    );
    assert_close_rel(
        result.max_moment,
        2688.0,
        1.0e-9,
        "thermal_frame_3d max moment",
    );
    assert_close_rel(
        result.max_stress,
        1.239e8,
        1.0e-9,
        "thermal_frame_3d max combined stress",
    );
    assert_close_abs(
        result.max_temperature_delta,
        35.0,
        1.0e-12,
        "thermal_frame_3d max temperature delta",
    );
    assert_close_abs(
        result.max_temperature_gradient,
        30.0,
        1.0e-12,
        "thermal_frame_3d max temperature gradient",
    );
}

#[test]
fn accuracy_baseline_thermal_frame_2d_sample_fixture() {
    let result = solve_thermal_frame_2d(&SolveThermalFrame2dRequest {
        nodes: vec![
            kyuubiki_protocol::ThermalFrame2dNodeInput {
                id: "tf0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                fix_rz: true,
                load_x: 0.0,
                load_y: 0.0,
                moment_z: 0.0,
                temperature_delta: 0.0,
            },
            kyuubiki_protocol::ThermalFrame2dNodeInput {
                id: "tf1".to_string(),
                x: 0.0,
                y: 3.0,
                fix_x: false,
                fix_y: false,
                fix_rz: false,
                load_x: 0.0,
                load_y: 0.0,
                moment_z: 0.0,
                temperature_delta: 35.0,
            },
            kyuubiki_protocol::ThermalFrame2dNodeInput {
                id: "tf2".to_string(),
                x: 4.0,
                y: 3.0,
                fix_x: false,
                fix_y: false,
                fix_rz: false,
                load_x: 0.0,
                load_y: 0.0,
                moment_z: 0.0,
                temperature_delta: 35.0,
            },
            kyuubiki_protocol::ThermalFrame2dNodeInput {
                id: "tf3".to_string(),
                x: 4.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                fix_rz: true,
                load_x: 0.0,
                load_y: 0.0,
                moment_z: 0.0,
                temperature_delta: 0.0,
            },
        ],
        elements: vec![
            kyuubiki_protocol::ThermalFrame2dElementInput {
                id: "te0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.02,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 0.00014,
                section_modulus: 0.0012,
                thermal_expansion: 12.0e-6,
                section_depth: 0.2,
                temperature_gradient_y: 0.0,
            },
            kyuubiki_protocol::ThermalFrame2dElementInput {
                id: "te1".to_string(),
                node_i: 1,
                node_j: 2,
                area: 0.02,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 0.00014,
                section_modulus: 0.0012,
                thermal_expansion: 12.0e-6,
                section_depth: 0.2,
                temperature_gradient_y: 30.0,
            },
            kyuubiki_protocol::ThermalFrame2dElementInput {
                id: "te2".to_string(),
                node_i: 2,
                node_j: 3,
                area: 0.02,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 0.00014,
                section_modulus: 0.0012,
                thermal_expansion: 12.0e-6,
                section_depth: 0.2,
                temperature_gradient_y: 0.0,
            },
        ],
    })
    .expect("thermal_frame_2d baseline should solve");

    assert_close_abs(
        result.max_displacement,
        0.0010408174194986581,
        1.0e-12,
        "thermal_frame_2d max displacement",
    );
    assert_close_abs(
        result.max_rotation,
        0.0006805479452054797,
        1.0e-12,
        "thermal_frame_2d max rotation",
    );
    assert_close_abs(
        result.max_axial_force,
        24164.383561644005,
        1.0e-9,
        "thermal_frame_2d max axial force",
    );
    assert_close_abs(
        result.max_moment,
        42915.94520547945,
        1.0e-9,
        "thermal_frame_2d max moment",
    );
    assert_close_abs(
        result.max_stress,
        36971506.84931508,
        1.0e-6,
        "thermal_frame_2d max stress",
    );
    assert_close_abs(
        result.max_temperature_delta,
        35.0,
        1.0e-12,
        "thermal_frame_2d max temperature delta",
    );
    assert_close_abs(
        result.max_temperature_gradient,
        30.0,
        1.0e-12,
        "thermal_frame_2d max temperature gradient",
    );
    assert_close_abs(
        result.nodes[1].ux,
        -0.0008284931506849309,
        1.0e-12,
        "thermal_frame_2d node 1 ux",
    );
    assert_close_abs(
        result.nodes[1].uy,
        0.00063,
        1.0e-12,
        "thermal_frame_2d node 1 uy",
    );
    assert_close_abs(
        result.elements[1].axial_stress,
        1208219.1780822002,
        1.0e-6,
        "thermal_frame_2d beam axial stress",
    );
    assert_close_abs(
        result.elements[1].max_combined_stress,
        36971506.84931508,
        1.0e-6,
        "thermal_frame_2d beam combined stress",
    );
}
