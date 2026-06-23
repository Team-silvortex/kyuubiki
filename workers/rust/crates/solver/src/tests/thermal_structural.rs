use super::*;

#[test]
fn solves_a_small_thermal_truss_2d_with_restrained_expansion() {
    let result = solve_thermal_truss_2d(&SolveThermalTruss2dRequest {
        nodes: vec![
            ThermalTruss2dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 40.0,
            },
            ThermalTruss2dNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 40.0,
            },
        ],
        elements: vec![ThermalTruss2dElementInput {
            id: "tt0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            youngs_modulus: 210.0e9,
            thermal_expansion: 12.0e-6,
        }],
    })
    .expect("thermal truss 2d should solve");

    assert!(result.max_displacement.abs() < 1.0e-12);
    assert!(result.max_stress > 1.0e8);
    assert!(result.max_axial_force > 1.0e6);
    assert_eq!(result.max_temperature_delta, 40.0);
    assert!(result.elements[0].stress < 0.0);
}

#[test]
fn solves_a_small_thermal_truss_3d_with_restrained_expansion() {
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
                temperature_delta: 0.0,
            },
        ],
        elements: vec![ThermalTruss3dElementInput {
            id: "tt0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.01,
            youngs_modulus: 210.0e9,
            thermal_expansion: 12.0e-6,
        }],
    })
    .expect("thermal truss 3d should solve");

    assert!(result.max_displacement.abs() < 1.0e-12);
    assert!(result.max_stress > 1.0e8);
    assert!(result.max_axial_force > 1.0e6);
    assert_eq!(result.max_temperature_delta, 40.0);
    assert!(result.elements[0].stress < 0.0);
}

#[test]
fn solves_a_small_beam_1d_cantilever() {
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
    .expect("1d beam should solve");

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!((result.max_displacement - 0.0015873015873015873).abs() < 1.0e-12);
    assert!((result.max_rotation - 0.0011904761904761906).abs() < 1.0e-12);
    assert!((result.max_moment - 2000.0).abs() < 1.0e-6);
    assert!((result.max_stress - 1.25e7).abs() < 1.0e-2);
}

#[test]
fn solves_a_small_thermal_beam_1d_with_restrained_gradient() {
    let result = solve_thermal_beam_1d(&SolveThermalBeam1dRequest {
        nodes: vec![
            ThermalBeam1dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                fix_y: true,
                fix_rz: true,
                load_y: 0.0,
                moment_z: 0.0,
            },
            ThermalBeam1dNodeInput {
                id: "n1".to_string(),
                x: 2.0,
                fix_y: true,
                fix_rz: true,
                load_y: 0.0,
                moment_z: 0.0,
            },
        ],
        elements: vec![ThermalBeam1dElementInput {
            id: "tb0".to_string(),
            node_i: 0,
            node_j: 1,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
            thermal_expansion: 12.0e-6,
            section_depth: 0.2,
            distributed_load_y: 0.0,
            temperature_gradient_y: 40.0,
        }],
    })
    .expect("thermal beam should solve");

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_moment > 0.0);
    assert!(result.max_stress > 0.0);
    assert_eq!(result.max_temperature_gradient, 40.0);
}

#[test]
fn solves_a_small_thermal_frame_2d_with_restrained_expansion() {
    let result = solve_thermal_frame_2d(&SolveThermalFrame2dRequest {
        nodes: vec![
            kyuubiki_protocol::ThermalFrame2dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                fix_rz: true,
                load_x: 0.0,
                load_y: 0.0,
                moment_z: 0.0,
                temperature_delta: 35.0,
            },
            kyuubiki_protocol::ThermalFrame2dNodeInput {
                id: "n1".to_string(),
                x: 2.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                fix_rz: true,
                load_x: 0.0,
                load_y: 0.0,
                moment_z: 0.0,
                temperature_delta: 35.0,
            },
        ],
        elements: vec![kyuubiki_protocol::ThermalFrame2dElementInput {
            id: "tf0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
            thermal_expansion: 12.0e-6,
            section_depth: 0.2,
            temperature_gradient_y: 30.0,
        }],
    })
    .expect("thermal frame should solve");

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_axial_force > 0.0);
    assert!(result.max_moment > 0.0);
    assert!(result.max_stress > 0.0);
    assert_eq!(result.max_temperature_delta, 35.0);
    assert_eq!(result.max_temperature_gradient, 30.0);
}

#[test]
fn solves_a_small_thermal_frame_3d_with_restrained_expansion() {
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
    .expect("thermal frame 3d should solve");

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_displacement.abs() < 1.0e-12);
    assert!(result.max_axial_force > 0.0);
    assert!(result.max_moment > 0.0);
    assert!(result.max_stress > 0.0);
    assert_eq!(result.max_temperature_delta, 35.0);
    assert_eq!(result.max_temperature_gradient, 30.0);
}
