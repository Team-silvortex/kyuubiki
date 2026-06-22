use kyuubiki_protocol::{
    Beam1dElementInput, Beam1dNodeInput, SolveBeam1dRequest, SolveSpring1dRequest,
    SolveSpring2dRequest, SolveSpring3dRequest, SolveThermalBeam1dRequest, SolveTorsion1dRequest,
    Spring1dElementInput, Spring1dNodeInput, Spring2dElementInput, Spring2dNodeInput,
    Spring3dElementInput, Spring3dNodeInput, ThermalBeam1dElementInput, ThermalBeam1dNodeInput,
    Torsion1dElementInput, Torsion1dNodeInput,
};
use kyuubiki_solver::{
    solve_beam_1d, solve_spring_1d, solve_spring_2d, solve_spring_3d, solve_thermal_beam_1d,
    solve_torsion_1d,
};

#[test]
fn valid_spring_operator_results_stay_finite() {
    let spring_1d = solve_spring_1d(&spring_1d_request()).expect("valid spring 1d should solve");
    assert_finite("spring 1d max_displacement", spring_1d.max_displacement);
    assert_finite("spring 1d max_force", spring_1d.max_force);
    assert!(spring_1d.max_displacement > 0.0);
    assert!(spring_1d.max_force > 0.0);
    for element in &spring_1d.elements {
        assert_finite("spring 1d element extension", element.extension);
        assert_finite("spring 1d element force", element.force);
    }

    let spring_2d = solve_spring_2d(&spring_2d_request()).expect("valid spring 2d should solve");
    assert_finite("spring 2d max_displacement", spring_2d.max_displacement);
    assert_finite("spring 2d max_force", spring_2d.max_force);
    assert!(spring_2d.max_displacement > 0.0);
    assert!(spring_2d.max_force > 0.0);
    for element in &spring_2d.elements {
        assert_finite("spring 2d element extension", element.extension);
        assert_finite("spring 2d element force", element.force);
    }

    let spring_3d = solve_spring_3d(&spring_3d_request()).expect("valid spring 3d should solve");
    assert_finite("spring 3d max_displacement", spring_3d.max_displacement);
    assert_finite("spring 3d max_force", spring_3d.max_force);
    assert!(spring_3d.max_displacement > 0.0);
    assert!(spring_3d.max_force > 0.0);
    for element in &spring_3d.elements {
        assert_finite("spring 3d element extension", element.extension);
        assert_finite("spring 3d element force", element.force);
    }
}

#[test]
fn valid_beam_and_torsion_operator_results_stay_finite() {
    let beam = solve_beam_1d(&beam_1d_request()).expect("valid beam should solve");
    assert_finite("beam max_displacement", beam.max_displacement);
    assert_finite("beam max_rotation", beam.max_rotation);
    assert_finite("beam max_moment", beam.max_moment);
    assert_finite("beam max_stress", beam.max_stress);
    assert!(beam.max_displacement > 0.0);
    assert!(beam.max_stress > 0.0);
    for element in &beam.elements {
        assert_finite("beam element moment_i", element.moment_i);
        assert_finite("beam element stress", element.max_bending_stress);
    }

    let thermal_beam =
        solve_thermal_beam_1d(&thermal_beam_1d_request()).expect("valid thermal beam should solve");
    assert_finite(
        "thermal beam max_displacement",
        thermal_beam.max_displacement,
    );
    assert_finite("thermal beam max_rotation", thermal_beam.max_rotation);
    assert_finite("thermal beam max_moment", thermal_beam.max_moment);
    assert_finite("thermal beam max_stress", thermal_beam.max_stress);
    assert_finite(
        "thermal beam max_temperature_gradient",
        thermal_beam.max_temperature_gradient,
    );
    assert!(thermal_beam.max_displacement > 0.0);
    for element in &thermal_beam.elements {
        assert_finite("thermal beam curvature", element.thermal_curvature);
        assert_finite("thermal beam stress", element.max_bending_stress);
    }

    let torsion = solve_torsion_1d(&torsion_1d_request()).expect("valid torsion should solve");
    assert_finite("torsion max_rotation", torsion.max_rotation);
    assert_finite("torsion max_torque", torsion.max_torque);
    assert_finite("torsion max_stress", torsion.max_stress);
    assert!(torsion.max_rotation > 0.0);
    assert!(torsion.max_stress > 0.0);
    for element in &torsion.elements {
        assert_finite("torsion element twist", element.twist);
        assert_finite("torsion element torque", element.torque);
        assert_finite("torsion element shear_stress", element.shear_stress);
    }
}

fn spring_1d_request() -> SolveSpring1dRequest {
    SolveSpring1dRequest {
        nodes: vec![
            Spring1dNodeInput {
                id: "s0".to_string(),
                x: 0.0,
                fix_x: true,
                load_x: 0.0,
            },
            Spring1dNodeInput {
                id: "s1".to_string(),
                x: 1.2,
                fix_x: false,
                load_x: 0.0,
            },
            Spring1dNodeInput {
                id: "s2".to_string(),
                x: 2.4,
                fix_x: false,
                load_x: 1200.0,
            },
        ],
        elements: vec![
            spring_1d_element("k0", 0, 1, 35000.0),
            spring_1d_element("k1", 1, 2, 20000.0),
        ],
    }
}

fn spring_2d_request() -> SolveSpring2dRequest {
    SolveSpring2dRequest {
        nodes: vec![
            spring_2d_node("s0", 0.0, 0.0, true, true, 0.0, 0.0),
            spring_2d_node("s1", 1.0, 0.0, false, true, 0.0, 0.0),
            spring_2d_node("s2", 1.0, 1.0, false, false, 1200.0, -600.0),
            spring_2d_node("s3", 0.0, 1.0, true, false, 0.0, 0.0),
        ],
        elements: vec![
            spring_2d_element("sp0", 0, 1, 25000.0),
            spring_2d_element("sp1", 1, 2, 18000.0),
            spring_2d_element("sp2", 2, 3, 22000.0),
            spring_2d_element("sp3", 3, 0, 18000.0),
            spring_2d_element("sp4", 0, 2, 12000.0),
        ],
    }
}

fn spring_3d_request() -> SolveSpring3dRequest {
    SolveSpring3dRequest {
        nodes: vec![
            spring_3d_node("s0", 0.0, 0.0, 0.0, true, true, true, 0.0, 0.0, 0.0),
            spring_3d_node("s1", 1.2, 0.0, 0.0, true, true, true, 0.0, 0.0, 0.0),
            spring_3d_node("s2", 0.0, 1.0, 0.0, true, true, true, 0.0, 0.0, 0.0),
            spring_3d_node(
                "top", 0.45, 0.35, 1.1, false, false, false, 250.0, 0.0, -1100.0,
            ),
        ],
        elements: vec![
            spring_3d_element("k0", 0, 3, 18000.0),
            spring_3d_element("k1", 1, 3, 22000.0),
            spring_3d_element("k2", 2, 3, 16000.0),
            spring_3d_element("k3", 0, 1, 9000.0),
            spring_3d_element("k4", 1, 2, 9000.0),
            spring_3d_element("k5", 2, 0, 9000.0),
        ],
    }
}

fn beam_1d_request() -> SolveBeam1dRequest {
    SolveBeam1dRequest {
        nodes: vec![
            beam_node("n0", 0.0, true, true, 0.0),
            beam_node("n1", 2.0, false, false, -1000.0),
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
    }
}

fn thermal_beam_1d_request() -> SolveThermalBeam1dRequest {
    SolveThermalBeam1dRequest {
        nodes: vec![
            thermal_beam_node("tb0", 0.0, true, true),
            thermal_beam_node("tb1", 2.4, false, false),
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
    }
}

fn torsion_1d_request() -> SolveTorsion1dRequest {
    SolveTorsion1dRequest {
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
    }
}

fn spring_1d_element(
    id: &str,
    node_i: usize,
    node_j: usize,
    stiffness: f64,
) -> Spring1dElementInput {
    Spring1dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        stiffness,
    }
}

fn spring_2d_node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    load_x: f64,
    load_y: f64,
) -> Spring2dNodeInput {
    Spring2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x,
        load_y,
    }
}

fn spring_2d_element(
    id: &str,
    node_i: usize,
    node_j: usize,
    stiffness: f64,
) -> Spring2dElementInput {
    Spring2dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        stiffness,
    }
}

#[allow(clippy::too_many_arguments)]
fn spring_3d_node(
    id: &str,
    x: f64,
    y: f64,
    z: f64,
    fix_x: bool,
    fix_y: bool,
    fix_z: bool,
    load_x: f64,
    load_y: f64,
    load_z: f64,
) -> Spring3dNodeInput {
    Spring3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x,
        fix_y,
        fix_z,
        load_x,
        load_y,
        load_z,
    }
}

fn spring_3d_element(
    id: &str,
    node_i: usize,
    node_j: usize,
    stiffness: f64,
) -> Spring3dElementInput {
    Spring3dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        stiffness,
    }
}

fn beam_node(id: &str, x: f64, fix_y: bool, fix_rz: bool, load_y: f64) -> Beam1dNodeInput {
    Beam1dNodeInput {
        id: id.to_string(),
        x,
        fix_y,
        fix_rz,
        load_y,
        moment_z: 0.0,
    }
}

fn thermal_beam_node(id: &str, x: f64, fix_y: bool, fix_rz: bool) -> ThermalBeam1dNodeInput {
    ThermalBeam1dNodeInput {
        id: id.to_string(),
        x,
        fix_y,
        fix_rz,
        load_y: 0.0,
        moment_z: 0.0,
    }
}

fn assert_finite(label: &str, value: f64) {
    assert!(value.is_finite(), "{label} must be finite, got {value}");
}
