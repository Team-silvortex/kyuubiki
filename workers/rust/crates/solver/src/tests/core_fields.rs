use super::*;

#[test]
fn emits_solving_events_and_completion() {
    let solver = MockSolver::new(3);
    let job = Job::new("job-1", "project-1", "case-1");

    let events = solver.solve(&job);

    assert_eq!(events.len(), 4);
    assert_eq!(events[0].stage, JobStatus::Solving);
    assert_eq!(events[2].progress, 1.0);
    assert_eq!(events[3].stage, JobStatus::Completed);
}

#[test]
fn solves_a_one_element_tensile_bar() {
    let result = solve_bar_1d(&SolveBarRequest {
        length: 1.0,
        area: 0.01,
        youngs_modulus: 210.0e9,
        elements: 1,
        tip_force: 1000.0,
    })
    .expect("solver should succeed");

    assert!((result.tip_displacement - 4.761904761904762e-7).abs() < 1.0e-12);
    assert!((result.max_stress - 100_000.0).abs() < 1.0e-6);
    assert!((result.reaction_force + 1000.0).abs() < 1.0e-6);
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
}

#[test]
fn rejects_invalid_requests() {
    let error = solve_bar_1d(&SolveBarRequest {
        length: 0.0,
        area: 0.01,
        youngs_modulus: 210.0e9,
        elements: 1,
        tip_force: 1000.0,
    })
    .expect_err("invalid request should fail");

    assert!(error.contains("length"));
}

#[test]
fn solves_a_small_thermal_bar_1d_with_restrained_expansion() {
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
    .expect("thermal bar should solve");

    assert!(result.max_displacement.abs() < 1.0e-12);
    assert!(result.max_stress > 1.0e8);
    assert!(result.max_axial_force > 1.0e6);
    assert_eq!(result.max_temperature_delta, 40.0);
    assert!(result.elements[0].stress < 0.0);
}

#[test]
fn solves_a_small_heat_bar_1d_gradient() {
    let result = solve_heat_bar_1d(&SolveHeatBar1dRequest {
        nodes: vec![
            HeatBar1dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                fix_temperature: true,
                temperature: 100.0,
                heat_load: 0.0,
            },
            HeatBar1dNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                fix_temperature: true,
                temperature: 0.0,
                heat_load: 0.0,
            },
        ],
        elements: vec![HeatBar1dElementInput {
            id: "hb0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            conductivity: 50.0,
        }],
    })
    .expect("heat bar should solve");

    assert_eq!(result.nodes[0].temperature, 100.0);
    assert_eq!(result.nodes[1].temperature, 0.0);
    assert!((result.elements[0].temperature_gradient + 100.0).abs() < 1.0e-9);
    assert!((result.elements[0].heat_flux - 5_000.0).abs() < 1.0e-6);
    assert_eq!(result.max_temperature, 100.0);
    assert!((result.max_heat_flux - 5_000.0).abs() < 1.0e-6);
}

#[test]
fn solves_a_small_electrostatic_bar_1d_gradient() {
    let result = solve_electrostatic_bar_1d(&SolveElectrostaticBar1dRequest {
        nodes: vec![
            ElectrostaticBar1dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                fix_potential: true,
                potential: 10.0,
                charge_density: 0.0,
            },
            ElectrostaticBar1dNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                fix_potential: true,
                potential: 0.0,
                charge_density: 0.0,
            },
        ],
        elements: vec![ElectrostaticBar1dElementInput {
            id: "eb0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            permittivity: 2.0,
        }],
    })
    .expect("electrostatic bar should solve");

    assert_eq!(result.nodes[0].potential, 10.0);
    assert_eq!(result.nodes[1].potential, 0.0);
    assert!((result.elements[0].potential_gradient + 10.0).abs() < 1.0e-9);
    assert!((result.elements[0].electric_field - 10.0).abs() < 1.0e-9);
    assert!((result.elements[0].electric_flux_density - 20.0).abs() < 1.0e-9);
    assert_eq!(result.max_potential, 10.0);
    assert!((result.max_electric_field - 10.0).abs() < 1.0e-9);
    assert!((result.max_flux_density - 20.0).abs() < 1.0e-9);
}

#[test]
fn solves_a_small_magnetostatic_bar_1d_gradient() {
    let result = solve_magnetostatic_bar_1d(&SolveMagnetostaticBar1dRequest {
        nodes: vec![
            MagnetostaticBar1dNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                fix_magnetic_potential: true,
                magnetic_potential: 10.0,
                magnetomotive_source: 0.0,
            },
            MagnetostaticBar1dNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                fix_magnetic_potential: true,
                magnetic_potential: 0.0,
                magnetomotive_source: 0.0,
            },
        ],
        elements: vec![MagnetostaticBar1dElementInput {
            id: "mb0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            permeability: 2.0,
        }],
    })
    .expect("magnetostatic bar should solve");

    assert_eq!(result.nodes[0].magnetic_potential, 10.0);
    assert_eq!(result.nodes[1].magnetic_potential, 0.0);
    assert!((result.elements[0].magnetic_potential_gradient + 10.0).abs() < 1.0e-9);
    assert!((result.elements[0].magnetic_field_strength - 10.0).abs() < 1.0e-9);
    assert!((result.elements[0].magnetic_flux_density - 20.0).abs() < 1.0e-9);
    assert_eq!(result.max_magnetic_potential, 10.0);
    assert!((result.max_magnetic_field_strength - 10.0).abs() < 1.0e-9);
    assert!((result.max_flux_density - 20.0).abs() < 1.0e-9);
}

#[test]
fn solves_a_small_electrostatic_plane_triangle_2d_patch() {
    let result = solve_electrostatic_plane_triangle_2d(&SolveElectrostaticPlaneTriangle2dRequest {
        nodes: vec![
            ElectrostaticPlaneNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_potential: true,
                potential: 10.0,
                charge_density: 0.0,
            },
            ElectrostaticPlaneNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_potential: true,
                potential: 0.0,
                charge_density: 0.0,
            },
            ElectrostaticPlaneNodeInput {
                id: "n2".to_string(),
                x: 0.0,
                y: 1.0,
                fix_potential: true,
                potential: 10.0,
                charge_density: 0.0,
            },
        ],
        elements: vec![ElectrostaticPlaneTriangleElementInput {
            id: "ep0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.05,
            permittivity: 2.0,
        }],
    })
    .expect("electrostatic plane triangle should solve");

    assert_eq!(result.nodes[0].potential, 10.0);
    assert_eq!(result.nodes[1].potential, 0.0);
    assert_eq!(result.nodes[2].potential, 10.0);
    assert!((result.elements[0].potential_gradient_x + 10.0).abs() < 1.0e-9);
    assert!(result.elements[0].potential_gradient_y.abs() < 1.0e-9);
    assert!((result.elements[0].electric_field_x - 10.0).abs() < 1.0e-9);
    assert!(result.elements[0].electric_field_y.abs() < 1.0e-9);
    assert!((result.elements[0].electric_flux_density_x - 20.0).abs() < 1.0e-9);
    assert_eq!(result.max_potential, 10.0);
    assert!((result.max_electric_field - 10.0).abs() < 1.0e-9);
    assert!((result.max_flux_density - 20.0).abs() < 1.0e-9);
}

#[test]
fn solves_a_small_electrostatic_plane_quad_2d_patch() {
    let result = solve_electrostatic_plane_quad_2d(&SolveElectrostaticPlaneQuad2dRequest {
        nodes: vec![
            ElectrostaticPlaneNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_potential: true,
                potential: 10.0,
                charge_density: 0.0,
            },
            ElectrostaticPlaneNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_potential: true,
                potential: 0.0,
                charge_density: 0.0,
            },
            ElectrostaticPlaneNodeInput {
                id: "n2".to_string(),
                x: 1.0,
                y: 1.0,
                fix_potential: true,
                potential: 0.0,
                charge_density: 0.0,
            },
            ElectrostaticPlaneNodeInput {
                id: "n3".to_string(),
                x: 0.0,
                y: 1.0,
                fix_potential: true,
                potential: 10.0,
                charge_density: 0.0,
            },
        ],
        elements: vec![ElectrostaticPlaneQuadElementInput {
            id: "epq0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.05,
            permittivity: 2.0,
        }],
    })
    .expect("electrostatic plane quad should solve");

    assert_eq!(result.nodes[0].potential, 10.0);
    assert_eq!(result.nodes[1].potential, 0.0);
    assert_eq!(result.nodes[2].potential, 0.0);
    assert_eq!(result.nodes[3].potential, 10.0);
    assert!((result.elements[0].potential_gradient_x + 10.0).abs() < 1.0e-9);
    assert!(result.elements[0].potential_gradient_y.abs() < 1.0e-9);
    assert!((result.elements[0].electric_field_x - 10.0).abs() < 1.0e-9);
    assert!(result.elements[0].electric_field_y.abs() < 1.0e-9);
    assert!((result.elements[0].electric_flux_density_x - 20.0).abs() < 1.0e-9);
    assert_eq!(result.max_potential, 10.0);
    assert!((result.max_electric_field - 10.0).abs() < 1.0e-9);
    assert!((result.max_flux_density - 20.0).abs() < 1.0e-9);
}

#[test]
fn solves_a_small_heat_plane_triangle_2d_patch() {
    let result = solve_heat_plane_triangle_2d(&SolveHeatPlaneTriangle2dRequest {
        nodes: vec![
            HeatPlaneNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_temperature: true,
                temperature: 100.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_temperature: true,
                temperature: 0.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "n2".to_string(),
                x: 0.0,
                y: 1.0,
                fix_temperature: false,
                temperature: 0.0,
                heat_load: 0.0,
            },
        ],
        elements: vec![HeatPlaneTriangleElementInput {
            id: "hp0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.02,
            conductivity: 10.0,
        }],
    })
    .expect("heat plane triangle should solve");

    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 1);
    assert_eq!(result.max_temperature, 100.0);
    assert!(result.max_heat_flux > 0.0);
}

#[test]
fn solves_a_small_heat_plane_quad_2d_patch() {
    let result = solve_heat_plane_quad_2d(&SolveHeatPlaneQuad2dRequest {
        nodes: vec![
            HeatPlaneNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_temperature: true,
                temperature: 100.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_temperature: true,
                temperature: 0.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "n2".to_string(),
                x: 1.0,
                y: 1.0,
                fix_temperature: false,
                temperature: 0.0,
                heat_load: 0.0,
            },
            HeatPlaneNodeInput {
                id: "n3".to_string(),
                x: 0.0,
                y: 1.0,
                fix_temperature: false,
                temperature: 0.0,
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
            conductivity: 10.0,
        }],
    })
    .expect("heat plane quad should solve");

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 1);
    assert_eq!(result.max_temperature, 100.0);
    assert!(result.max_heat_flux > 0.0);
}
