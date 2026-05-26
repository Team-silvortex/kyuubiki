use kyuubiki_protocol::{
    Beam1dElementInput, Beam1dNodeInput, Frame2dElementInput, Frame2dNodeInput,
    Frame3dElementInput, Frame3dNodeInput, PlaneNodeInput, PlaneQuadElementInput,
    PlaneTriangleElementInput,
    HeatBar1dElementInput, HeatBar1dNodeInput, SolveBarRequest, SolveBeam1dRequest,
    HeatPlaneNodeInput, HeatPlaneQuadElementInput, HeatPlaneTriangleElementInput,
    SolveFrame2dRequest, SolveFrame3dRequest,
    SolveHeatBar1dRequest, SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest, SolvePlaneQuad2dRequest,
    SolvePlaneTriangle2dRequest, SolveThermalBar1dRequest, SolveThermalBeam1dRequest,
    SolveThermalFrame2dRequest,
    SolveThermalFrame3dRequest,
    SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest,
    SolveThermalTruss2dRequest, SolveThermalTruss3dRequest, SolveTruss2dRequest, SolveTruss3dRequest,
    ThermalBar1dElementInput, ThermalBar1dNodeInput, ThermalBeam1dElementInput,
    ThermalBeam1dNodeInput, ThermalFrame3dElementInput,
    ThermalFrame3dNodeInput, ThermalPlaneNodeInput, ThermalPlaneQuadElementInput,
    ThermalPlaneTriangleElementInput,
    ThermalTruss2dElementInput, ThermalTruss2dNodeInput,
    ThermalTruss3dElementInput, ThermalTruss3dNodeInput, Truss3dElementInput, Truss3dNodeInput, TrussElementInput, TrussNodeInput,
};
use kyuubiki_solver::{
    solve_bar_1d, solve_beam_1d, solve_frame_2d, solve_frame_3d, solve_heat_bar_1d,
    solve_heat_plane_quad_2d, solve_heat_plane_triangle_2d, solve_plane_quad_2d, solve_plane_triangle_2d, solve_thermal_bar_1d,
    solve_thermal_beam_1d, solve_thermal_frame_2d, solve_thermal_frame_3d,
    solve_thermal_plane_quad_2d,
    solve_thermal_plane_triangle_2d, solve_thermal_truss_2d, solve_thermal_truss_3d,
    solve_truss_2d, solve_truss_3d,
};

fn assert_close_abs(actual: f64, expected: f64, tolerance: f64, label: &str) {
    let delta = (actual - expected).abs();
    assert!(
        delta <= tolerance,
        "{label} mismatch: actual={actual:.12e} expected={expected:.12e} tolerance={tolerance:.12e} delta={delta:.12e}"
    );
}

fn assert_close_rel(actual: f64, expected: f64, tolerance: f64, label: &str) {
    let scale = expected.abs().max(1.0);
    let delta = (actual - expected).abs();
    let relative = delta / scale;
    assert!(
        relative <= tolerance,
        "{label} mismatch: actual={actual:.12e} expected={expected:.12e} rel_tol={tolerance:.12e} rel_err={relative:.12e}"
    );
}

#[test]
fn accuracy_baseline_axial_bar_1d_closed_form() {
    let result = solve_bar_1d(&SolveBarRequest {
        length: 1.0,
        area: 0.01,
        youngs_modulus: 210.0e9,
        elements: 1,
        tip_force: 1000.0,
    })
    .expect("axial bar baseline should solve");

    assert_close_abs(
        result.tip_displacement,
        4.761904761904762e-7,
        1.0e-12,
        "axial_bar_1d tip displacement",
    );
    assert_close_abs(
        result.max_stress,
        100_000.0,
        1.0e-6,
        "axial_bar_1d max stress",
    );
    assert_close_abs(
        result.reaction_force,
        -1000.0,
        1.0e-6,
        "axial_bar_1d reaction force",
    );
}

#[test]
fn accuracy_baseline_thermal_bar_1d_restrained_uniform_rise() {
    let result = solve_thermal_bar_1d(&SolveThermalBar1dRequest {
        nodes: vec![
            ThermalBar1dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                fix_x: true,
                load_x: 0.0,
                temperature_delta: 40.0,
            },
            ThermalBar1dNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                fix_x: true,
                load_x: 0.0,
                temperature_delta: 40.0,
            },
        ],
        elements: vec![ThermalBar1dElementInput {
            id: "tb0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            youngs_modulus: 210.0e9,
            thermal_expansion: 12.0e-6,
        }],
    })
    .expect("thermal bar baseline should solve");

    assert_close_abs(
        result.max_displacement,
        0.0,
        1.0e-12,
        "thermal_bar_1d max displacement",
    );
    assert_close_rel(
        result.max_stress,
        100_800_000.0,
        1.0e-9,
        "thermal_bar_1d max stress magnitude",
    );
    assert_close_rel(
        result.max_axial_force,
        1_008_000.0,
        1.0e-9,
        "thermal_bar_1d max axial force magnitude",
    );
    assert_close_abs(
        result.max_temperature_delta,
        40.0,
        1.0e-12,
        "thermal_bar_1d max temperature delta",
    );
    assert!(
        result.elements[0].stress < 0.0,
        "thermal_bar_1d stress sign should indicate compression"
    );
}

#[test]
fn accuracy_baseline_thermal_beam_1d_free_gradient_response() {
    let result = solve_thermal_beam_1d(&SolveThermalBeam1dRequest {
        nodes: vec![
            ThermalBeam1dNodeInput {
                id: "tb0".to_string(),
                x: 0.0,
                fix_y: true,
                fix_rz: true,
                load_y: 0.0,
                moment_z: 0.0,
            },
            ThermalBeam1dNodeInput {
                id: "tb1".to_string(),
                x: 2.4,
                fix_y: false,
                fix_rz: false,
                load_y: 0.0,
                moment_z: 0.0,
            },
        ],
        elements: vec![ThermalBeam1dElementInput {
            id: "tm0".to_string(),
            node_i: 0,
            node_j: 1,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 0.00012,
            section_modulus: 0.0011,
            thermal_expansion: 12.0e-6,
            section_depth: 0.3,
            distributed_load_y: 0.0,
            temperature_gradient_y: 45.0,
        }],
    })
    .expect("thermal_beam_1d baseline should solve");

    assert_close_abs(
        result.max_displacement,
        0.005184000000000001,
        1.0e-15,
        "thermal_beam_1d max displacement",
    );
    assert_close_abs(
        result.max_rotation,
        0.004320000000000001,
        1.0e-15,
        "thermal_beam_1d max rotation",
    );
    assert_close_abs(
        result.max_temperature_gradient,
        45.0,
        1.0e-12,
        "thermal_beam_1d max temperature gradient",
    );
    assert_close_abs(
        result.nodes[1].uy,
        0.005184000000000001,
        1.0e-15,
        "thermal_beam_1d tip uy",
    );
    assert_close_abs(
        result.nodes[1].rz,
        0.004320000000000001,
        1.0e-15,
        "thermal_beam_1d tip rotation",
    );
    assert_close_abs(
        result.max_moment,
        7.275957614183426e-12,
        1.0e-18,
        "thermal_beam_1d max moment",
    );
    assert_close_abs(
        result.max_stress,
        6.614506921984932e-9,
        1.0e-15,
        "thermal_beam_1d max stress",
    );
}

#[test]
fn accuracy_baseline_heat_bar_1d_two_element_gradient() {
    let result = solve_heat_bar_1d(&SolveHeatBar1dRequest {
        nodes: vec![
            HeatBar1dNodeInput {
                id: "h0".to_string(),
                x: 0.0,
                fix_temperature: true,
                temperature: 100.0,
                heat_load: 0.0,
            },
            HeatBar1dNodeInput {
                id: "h1".to_string(),
                x: 1.0,
                fix_temperature: false,
                temperature: 0.0,
                heat_load: 0.0,
            },
            HeatBar1dNodeInput {
                id: "h2".to_string(),
                x: 2.0,
                fix_temperature: true,
                temperature: 20.0,
                heat_load: 0.0,
            },
        ],
        elements: vec![
            HeatBar1dElementInput {
                id: "he0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                conductivity: 45.0,
            },
            HeatBar1dElementInput {
                id: "he1".to_string(),
                node_i: 1,
                node_j: 2,
                area: 0.01,
                conductivity: 45.0,
            },
        ],
    })
    .expect("heat_bar_1d baseline should solve");

    assert_close_abs(
        result.max_temperature,
        100.0,
        1.0e-12,
        "heat_bar_1d max temperature",
    );
    assert_close_abs(
        result.max_heat_flux,
        1800.0,
        1.0e-9,
        "heat_bar_1d max heat flux",
    );
    assert_close_abs(
        result.nodes[1].temperature,
        60.0,
        1.0e-12,
        "heat_bar_1d middle node temperature",
    );
    assert_close_abs(
        result.elements[0].temperature_gradient,
        -40.0,
        1.0e-12,
        "heat_bar_1d first element temperature gradient",
    );
    assert_close_abs(
        result.elements[1].temperature_gradient,
        -40.0,
        1.0e-12,
        "heat_bar_1d second element temperature gradient",
    );
}

#[test]
fn accuracy_baseline_heat_plane_quad_2d_single_patch() {
    let result = solve_heat_plane_quad_2d(&SolveHeatPlaneQuad2dRequest {
        nodes: vec![
            HeatPlaneNodeInput {
                id: "h0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_temperature: true,
                temperature: 100.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "h1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_temperature: false,
                temperature: 0.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "h2".to_string(),
                x: 1.0,
                y: 1.0,
                fix_temperature: true,
                temperature: 20.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "h3".to_string(),
                x: 0.0,
                y: 1.0,
                fix_temperature: true,
                temperature: 20.0,
                heat_load: 0.0,
            },
        ],
        elements: vec![HeatPlaneQuadElementInput {
            id: "hq0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02,
            conductivity: 45.0,
        }],
    })
    .expect("heat_plane_quad_2d baseline should solve");

    assert_close_abs(
        result.max_temperature,
        100.0,
        1.0e-12,
        "heat_plane_quad_2d max temperature",
    );
    assert_close_abs(
        result.max_heat_flux,
        2846.0498941515416,
        1.0e-9,
        "heat_plane_quad_2d max heat flux",
    );
    assert_close_abs(
        result.nodes[1].temperature,
        60.0,
        1.0e-12,
        "heat_plane_quad_2d node-1 temperature",
    );
    assert_close_abs(
        result.elements[0].temperature_gradient_x,
        -20.0,
        1.0e-12,
        "heat_plane_quad_2d gradient x",
    );
    assert_close_abs(
        result.elements[0].temperature_gradient_y,
        -60.0,
        1.0e-12,
        "heat_plane_quad_2d gradient y",
    );
}

#[test]
fn accuracy_baseline_heat_plane_triangle_2d_sample_fixture() {
    let result = solve_heat_plane_triangle_2d(&SolveHeatPlaneTriangle2dRequest {
        nodes: vec![
            HeatPlaneNodeInput {
                id: "h0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_temperature: true,
                temperature: 100.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "h1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_temperature: false,
                temperature: 0.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "h2".to_string(),
                x: 1.0,
                y: 1.0,
                fix_temperature: true,
                temperature: 20.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "h3".to_string(),
                x: 0.0,
                y: 1.0,
                fix_temperature: true,
                temperature: 20.0,
                heat_load: 0.0,
            },
        ],
        elements: vec![
            HeatPlaneTriangleElementInput {
                id: "hp0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.02,
                conductivity: 45.0,
            },
            HeatPlaneTriangleElementInput {
                id: "hp1".to_string(),
                node_i: 0,
                node_j: 2,
                node_k: 3,
                thickness: 0.02,
                conductivity: 45.0,
            },
        ],
    })
    .expect("heat_plane_triangle_2d baseline should solve");

    assert_close_abs(
        result.max_temperature,
        100.0,
        1.0e-12,
        "heat_plane_triangle_2d max temperature",
    );
    assert_close_abs(
        result.max_heat_flux,
        3600.0,
        1.0e-9,
        "heat_plane_triangle_2d max heat flux",
    );
    assert_close_abs(
        result.nodes[1].temperature,
        60.0,
        1.0e-12,
        "heat_plane_triangle_2d node-1 temperature",
    );
    assert_close_abs(
        result.elements[0].temperature_gradient_x,
        -40.0,
        1.0e-12,
        "heat_plane_triangle_2d element-0 gradient x",
    );
    assert_close_abs(
        result.elements[0].temperature_gradient_y,
        -40.0,
        1.0e-12,
        "heat_plane_triangle_2d element-0 gradient y",
    );
    assert_close_abs(
        result.elements[1].temperature_gradient_x,
        0.0,
        1.0e-12,
        "heat_plane_triangle_2d element-1 gradient x",
    );
    assert_close_abs(
        result.elements[1].temperature_gradient_y,
        -80.0,
        1.0e-12,
        "heat_plane_triangle_2d element-1 gradient y",
    );
}

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
    assert_close_abs(
        result.max_moment,
        2000.0,
        1.0e-6,
        "beam_1d max moment",
    );
    assert_close_abs(
        result.max_stress,
        1.25e7,
        1.0e-2,
        "beam_1d max bending stress",
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
    assert_close_abs(
        result.max_moment,
        2000.0,
        1.0e-6,
        "frame_2d max moment",
    );
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
    assert_close_abs(
        result.max_moment,
        2000.0,
        1.0e-6,
        "frame_3d max moment",
    );
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
