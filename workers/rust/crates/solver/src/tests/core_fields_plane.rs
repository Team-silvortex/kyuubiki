use super::*;

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
    assert!((result.elements[0].stored_energy - 2.5).abs() < 1.0e-9);
    assert!((result.total_stored_energy - 2.5).abs() < 1.0e-9);
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
    assert!((result.elements[0].stored_energy - 5.0).abs() < 1.0e-9);
    assert!((result.total_stored_energy - 5.0).abs() < 1.0e-9);
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

#[test]
fn solves_a_small_stokes_flow_quad_2d_patch() {
    let result = solve_stokes_flow_plane_quad_2d(&SolveStokesFlowPlaneQuad2dRequest {
        nodes: vec![
            StokesFlowPlaneNodeInput {
                id: "n0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_velocity_x: true,
                velocity_x: 0.0,
                fix_velocity_y: true,
                velocity_y: 0.0,
                fix_pressure: true,
                pressure: 1.0,
                body_force_x: 0.0,
                body_force_y: 0.0,
            },
            StokesFlowPlaneNodeInput {
                id: "n1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_velocity_x: false,
                velocity_x: 0.0,
                fix_velocity_y: true,
                velocity_y: 0.0,
                fix_pressure: false,
                pressure: 0.0,
                body_force_x: 2.0,
                body_force_y: 0.0,
            },
            StokesFlowPlaneNodeInput {
                id: "n2".to_string(),
                x: 1.0,
                y: 1.0,
                fix_velocity_x: false,
                velocity_x: 0.0,
                fix_velocity_y: false,
                velocity_y: 0.0,
                fix_pressure: false,
                pressure: 0.0,
                body_force_x: 2.0,
                body_force_y: 0.5,
            },
            StokesFlowPlaneNodeInput {
                id: "n3".to_string(),
                x: 0.0,
                y: 1.0,
                fix_velocity_x: true,
                velocity_x: 0.0,
                fix_velocity_y: true,
                velocity_y: 0.0,
                fix_pressure: false,
                pressure: 0.0,
                body_force_x: 0.0,
                body_force_y: 0.0,
            },
        ],
        elements: vec![StokesFlowPlaneQuadElementInput {
            id: "sf0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.1,
            viscosity: 2.0,
            density: 1.0,
        }],
    })
    .expect("stokes flow quad should solve");

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_velocity > 0.0);
    assert!(result.max_reynolds_number > 0.0);
    assert!(result.max_divergence_error >= 0.0);
}
