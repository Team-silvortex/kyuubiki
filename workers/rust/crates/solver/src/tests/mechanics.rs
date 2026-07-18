use super::*;

#[test]
fn solves_a_small_beam_1d_cantilever_with_uniform_load() {
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
                load_y: 0.0,
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
            distributed_load_y: -1000.0,
        }],
    })
    .expect("1d beam with uniform load should solve");

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!((result.max_displacement - 0.0011904761904761906).abs() < 1.0e-12);
    assert!((result.max_rotation - 0.0007936507936507938).abs() < 1.0e-12);
    assert!((result.max_moment - 2000.0).abs() < 1.0e-6);
    assert!((result.max_stress - 1.25e7).abs() < 1.0e-2);
    assert!((result.elements[0].shear_force_i - 2000.0).abs() < 1.0e-6);
    assert!((result.elements[0].moment_i - 2000.0).abs() < 1.0e-6);
    assert!(result.elements[0].shear_force_j.abs() < 1.0e-6);
    assert!(result.elements[0].moment_j.abs() < 1.0e-6);
}

#[test]
fn solves_a_small_torsion_1d_shaft() {
    let result = solve_torsion_1d(&SolveTorsion1dRequest {
        nodes: vec![
            Torsion1dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                fix_rz: true,
                torque_z: 0.0,
            },
            Torsion1dNodeInput {
                id: "n1".to_string(),
                x: 2.0,
                fix_rz: false,
                torque_z: 1200.0,
            },
        ],
        elements: vec![Torsion1dElementInput {
            id: "t0".to_string(),
            node_i: 0,
            node_j: 1,
            shear_modulus: 80.0e9,
            polar_moment: 3.0e-6,
            section_modulus: 2.0e-4,
        }],
    })
    .expect("1d torsion shaft should solve");

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!(result.nodes[1].rz > 0.0);
    assert!((result.max_torque - 1200.0).abs() < 1.0e-6);
    assert!((result.elements[0].torque - 1200.0).abs() < 1.0e-6);
    assert!((result.max_stress - 6.0e6).abs() < 1.0e-3);
}

#[test]
fn solves_a_small_spring_1d_chain() {
    let result = solve_spring_1d(&SolveSpring1dRequest {
        nodes: vec![
            Spring1dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                fix_x: true,
                load_x: 0.0,
            },
            Spring1dNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                fix_x: false,
                load_x: 1000.0,
            },
        ],
        elements: vec![Spring1dElementInput {
            id: "s0".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: 25_000.0,
        }],
    })
    .expect("1d spring should solve");

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!((result.max_displacement - 0.04).abs() < 1.0e-12);
    assert!((result.max_force - 1000.0).abs() < 1.0e-9);
    assert!((result.elements[0].extension - 0.04).abs() < 1.0e-12);
    assert!((result.elements[0].force - 1000.0).abs() < 1.0e-9);
}

#[test]
fn solves_a_multi_element_spring_1d_chain() {
    let result = solve_spring_1d(&SolveSpring1dRequest {
        nodes: vec![
            Spring1dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                fix_x: true,
                load_x: 0.0,
            },
            Spring1dNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                fix_x: false,
                load_x: 0.0,
            },
            Spring1dNodeInput {
                id: "n2".to_string(),
                x: 2.0,
                fix_x: false,
                load_x: 0.0,
            },
            Spring1dNodeInput {
                id: "n3".to_string(),
                x: 3.0,
                fix_x: false,
                load_x: 120.0,
            },
        ],
        elements: (0..3)
            .map(|index| Spring1dElementInput {
                id: format!("s{index}"),
                node_i: index,
                node_j: index + 1,
                stiffness: 120.0,
            })
            .collect(),
    })
    .expect("multi-element spring chain should solve");

    assert!((result.nodes[3].ux - 3.0).abs() < 1.0e-12);
    assert!((result.max_force - 120.0).abs() < 1.0e-9);
}

#[test]
fn solves_a_small_spring_2d_chain() {
    let result = solve_spring_2d(&SolveSpring2dRequest {
        nodes: vec![
            Spring2dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_x: true,
                fix_y: true,
                load_x: 0.0,
                load_y: 0.0,
            },
            Spring2dNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_x: false,
                fix_y: true,
                load_x: 1000.0,
                load_y: 0.0,
            },
        ],
        elements: vec![Spring2dElementInput {
            id: "s0".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: 25_000.0,
        }],
    })
    .expect("2d spring should solve");

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!((result.max_displacement - 0.04).abs() < 1.0e-12);
    assert!((result.max_force - 1000.0).abs() < 1.0e-9);
    assert!((result.nodes[1].ux - 0.04).abs() < 1.0e-12);
    assert!(result.nodes[1].uy.abs() < 1.0e-12);
    assert!((result.elements[0].extension - 0.04).abs() < 1.0e-12);
    assert!((result.elements[0].force - 1000.0).abs() < 1.0e-9);
}

#[test]
fn solves_a_small_spring_3d_chain() {
    let result = solve_spring_3d(&SolveSpring3dRequest {
        nodes: vec![
            Spring3dNodeInput {
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
            Spring3dNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                z: 0.0,
                fix_x: false,
                fix_y: true,
                fix_z: true,
                load_x: 1000.0,
                load_y: 0.0,
                load_z: 0.0,
            },
        ],
        elements: vec![Spring3dElementInput {
            id: "s0".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: 25_000.0,
        }],
    })
    .expect("3d spring should solve");

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert!((result.max_displacement - 0.04).abs() < 1.0e-12);
    assert!((result.max_force - 1000.0).abs() < 1.0e-9);
    assert!((result.nodes[1].ux - 0.04).abs() < 1.0e-12);
    assert!(result.nodes[1].uy.abs() < 1.0e-12);
    assert!(result.nodes[1].uz.abs() < 1.0e-12);
    assert!((result.elements[0].extension - 0.04).abs() < 1.0e-12);
    assert!((result.elements[0].force - 1000.0).abs() < 1.0e-9);
}

#[test]
fn solves_a_small_two_dimensional_truss() {
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
    .expect("2d truss should solve");

    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 3);
    assert!(result.max_displacement > 0.0);
    assert!(result.max_stress > 0.0);
}

#[test]
fn truss_profile_exposes_iterative_solver_hotspots() {
    let node_count = 1027;
    let nodes = (0..node_count)
        .map(|index| TrussNodeInput {
            id: format!("n{index}"),
            x: index as f64,
            y: 0.0,
            fix_x: index == 0,
            fix_y: true,
            load_x: if index == node_count - 1 { 1000.0 } else { 0.0 },
            load_y: 0.0,
        })
        .collect::<Vec<_>>();
    let elements = (0..node_count - 1)
        .map(|index| TrussElementInput {
            id: format!("e{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.01,
            youngs_modulus: 70.0e9,
        })
        .collect::<Vec<_>>();

    let profile =
        profile_truss_2d(&SolveTruss2dRequest { nodes, elements }).expect("truss should solve");
    let labels = profile
        .stages
        .iter()
        .map(|stage| stage.label)
        .collect::<Vec<_>>();

    assert!(labels.contains(&"solve_spd_matvec"));
    assert!(labels.contains(&"solve_spd_preconditioner"));
    assert!(labels.contains(&"solve_spd_vector_update"));
    assert!(labels.contains(&"solve_spd_direction_update"));
    assert!(labels.contains(&"solve_spd_dot"));
}

#[test]
fn rejects_truss_responses_that_blow_past_small_displacement_limits() {
    let error = solve_truss_2d(&SolveTruss2dRequest {
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
                area: 1.0e-12,
                youngs_modulus: 70.0e9,
            },
            TrussElementInput {
                id: "e1".to_string(),
                node_i: 1,
                node_j: 2,
                area: 1.0e-12,
                youngs_modulus: 70.0e9,
            },
            TrussElementInput {
                id: "e2".to_string(),
                node_i: 0,
                node_j: 1,
                area: 1.0e-12,
                youngs_modulus: 70.0e9,
            },
        ],
    })
    .expect_err("overly soft truss should be rejected");

    assert!(error.contains("small-deformation"));
}
