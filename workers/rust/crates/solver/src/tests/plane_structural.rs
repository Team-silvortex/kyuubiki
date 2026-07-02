use super::*;

#[test]
fn solves_a_small_plane_triangle_patch() {
    let request = SolvePlaneTriangle2dRequest {
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
    };

    let result = solve_plane_triangle_2d(&request).expect("plane solve should succeed");

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 2);
    assert!(result.max_displacement > 0.0);
    assert!(result.max_stress > 0.0);
}

#[test]
fn solves_a_small_plane_quad_patch() {
    let request = SolvePlaneQuad2dRequest {
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
        elements: vec![PlaneQuadElementInput {
            id: "q0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
        }],
    };

    let result = solve_plane_quad_2d(&request).expect("plane quad solve should succeed");

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_displacement > 0.0);
    assert!(result.max_stress > 0.0);
    assert!(result.elements[0].area > 0.0);
}

#[test]
fn solves_a_small_thermal_plane_triangle_patch_with_restrained_expansion() {
    let request = SolveThermalPlaneTriangle2dRequest {
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
                x: 0.0,
                y: 1.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
                temperature_delta: 40.0,
            },
        ],
        elements: vec![ThermalPlaneTriangleElementInput {
            id: "tp0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.02,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
            thermal_expansion: 12.0e-6,
        }],
    };

    let result =
        solve_thermal_plane_triangle_2d(&request).expect("thermal plane triangle should solve");

    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_displacement.abs() < 1.0e-12);
    assert!(result.max_stress > 1.0e7);
    assert_eq!(result.max_temperature_delta, 40.0);
    assert!(result.elements[0].stress_x < 0.0);
    assert!(result.elements[0].mechanical_strain_x < 0.0);
    assert_eq!(result.elements[0].average_temperature_delta, 40.0);
}

#[test]
fn solves_a_small_thermal_plane_quad_patch_with_restrained_expansion() {
    let request = SolveThermalPlaneQuad2dRequest {
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
    };

    let result = solve_thermal_plane_quad_2d(&request).expect("thermal plane quad should solve");

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_displacement.abs() < 1.0e-12);
    assert!(result.max_stress > 1.0e7);
    assert_eq!(result.max_temperature_delta, 30.0);
    assert!(result.elements[0].stress_x < 0.0);
    assert!(result.elements[0].mechanical_strain_x < 0.0);
    assert_eq!(result.elements[0].average_temperature_delta, 30.0);
}

#[test]
fn solves_a_small_frame_2d_cantilever() {
    let request = SolveFrame2dRequest {
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
    };

    let result = solve_frame_2d(&request).expect("frame solve should succeed");

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_displacement > 0.0);
    assert!(result.max_rotation > 0.0);
    assert!(result.max_moment > 0.0);
    assert!(result.max_stress > 0.0);

    let tip = &result.nodes[1];
    let expected_tip_uy = (1000.0 * 2.0_f64.powi(3)) / (3.0 * 210.0e9 * 8.0e-6);
    let expected_tip_rz = (1000.0 * 2.0_f64.powi(2)) / (2.0 * 210.0e9 * 8.0e-6);

    assert!((tip.uy.abs() - expected_tip_uy).abs() / expected_tip_uy < 1.0e-6);
    assert!((tip.rz.abs() - expected_tip_rz).abs() / expected_tip_rz < 1.0e-6);
}

#[test]
fn solves_a_small_frame_3d_cantilever() {
    let request = SolveFrame3dRequest {
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
    };

    let result = solve_frame_3d(&request).expect("3d frame solve should succeed");

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_displacement > 0.0);
    assert!(result.max_rotation > 0.0);
    assert!(result.max_moment > 0.0);
    assert!(result.max_stress > 0.0);

    let tip = &result.nodes[1];
    let expected_tip_uy = (1000.0 * 2.0_f64.powi(3)) / (3.0 * 210.0e9 * 8.0e-6);
    let expected_tip_rz = (1000.0 * 2.0_f64.powi(2)) / (2.0 * 210.0e9 * 8.0e-6);

    assert!((tip.uy.abs() - expected_tip_uy).abs() / expected_tip_uy < 1.0e-6);
    assert!((tip.rz.abs() - expected_tip_rz).abs() / expected_tip_rz < 1.0e-6);
}

#[test]
fn solves_a_small_three_dimensional_truss() {
    let request = SolveTruss3dRequest {
        nodes: vec![
            Truss3dNodeInput {
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
            },
            Truss3dNodeInput {
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
            },
            Truss3dNodeInput {
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
            },
            Truss3dNodeInput {
                id: "n3".to_string(),
                x: 0.2,
                y: 0.2,
                z: 1.0,
                fix_x: false,
                fix_y: false,
                fix_z: false,
                load_x: 0.0,
                load_y: 0.0,
                load_z: -1000.0,
            },
        ],
        elements: vec![
            Truss3dElementInput {
                id: "e0".to_string(),
                node_i: 0,
                node_j: 3,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
            Truss3dElementInput {
                id: "e1".to_string(),
                node_i: 1,
                node_j: 3,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
            Truss3dElementInput {
                id: "e2".to_string(),
                node_i: 2,
                node_j: 3,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
            Truss3dElementInput {
                id: "e3".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
            Truss3dElementInput {
                id: "e4".to_string(),
                node_i: 1,
                node_j: 2,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
            Truss3dElementInput {
                id: "e5".to_string(),
                node_i: 2,
                node_j: 0,
                area: 0.01,
                youngs_modulus: 70.0e9,
            },
        ],
    };

    let result = solve_truss_3d(&request).expect("3d truss should solve");

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 6);
    assert!(result.max_displacement > 0.0);
    assert!(result.max_stress > 0.0);
}

#[test]
fn solves_a_small_three_dimensional_solid_tetra() {
    let request = SolveSolidTetra3dRequest {
        nodes: vec![
            solid_node("n0", 0.0, 0.0, 0.0, true, [0.0, 0.0, 0.0]),
            solid_node("n1", 1.0, 0.0, 0.0, true, [0.0, 0.0, 0.0]),
            solid_node("n2", 0.0, 1.0, 0.0, true, [0.0, 0.0, 0.0]),
            solid_node("n3", 0.0, 0.0, 1.0, false, [0.0, 0.0, -1000.0]),
        ],
        elements: vec![SolidTetra3dElementInput {
            id: "t0".to_string(),
            node_a: 0,
            node_b: 1,
            node_c: 2,
            node_d: 3,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
        }],
    };

    let result = solve_solid_tetra_3d(&request).expect("3d solid tetra should solve");

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 1);
    assert!((result.total_volume - (1.0 / 6.0)).abs() < 1.0e-12);
    assert!(result.max_displacement > 0.0);
    assert!(result.max_von_mises_stress > 0.0);
    assert!(result.nodes[3].uz < 0.0);
}

fn solid_node(
    id: &str,
    x: f64,
    y: f64,
    z: f64,
    fixed: bool,
    load: [f64; 3],
) -> SolidTetra3dNodeInput {
    SolidTetra3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        load_x: load[0],
        load_y: load[1],
        load_z: load[2],
    }
}
