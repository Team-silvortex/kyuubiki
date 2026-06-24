use super::common::*;

#[test]
fn accuracy_baseline_electrostatic_plane_triangle_2d_patch() {
    let result = solve_electrostatic_plane_triangle_2d(&SolveElectrostaticPlaneTriangle2dRequest {
        nodes: vec![
            ElectrostaticPlaneNodeInput {
                id: "e0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_potential: true,
                potential: 12.0,
                charge_density: 0.0,
            },
            ElectrostaticPlaneNodeInput {
                id: "e1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_potential: true,
                potential: 4.0,
                charge_density: 0.0,
            },
            ElectrostaticPlaneNodeInput {
                id: "e2".to_string(),
                x: 0.0,
                y: 1.0,
                fix_potential: true,
                potential: 12.0,
                charge_density: 0.0,
            },
        ],
        elements: vec![ElectrostaticPlaneTriangleElementInput {
            id: "ep0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.1,
            permittivity: 3.0,
        }],
    })
    .expect("electrostatic_plane_triangle_2d baseline should solve");

    assert_close_abs(
        result.max_potential,
        12.0,
        1.0e-12,
        "electrostatic_plane_triangle_2d max potential",
    );
    assert_close_abs(
        result.max_electric_field,
        8.0,
        1.0e-12,
        "electrostatic_plane_triangle_2d max electric field",
    );
    assert_close_abs(
        result.max_flux_density,
        24.0,
        1.0e-12,
        "electrostatic_plane_triangle_2d max flux density",
    );
    assert_close_abs(
        result.elements[0].potential_gradient_x,
        -8.0,
        1.0e-12,
        "electrostatic_plane_triangle_2d gradient x",
    );
    assert_close_abs(
        result.elements[0].potential_gradient_y,
        0.0,
        1.0e-12,
        "electrostatic_plane_triangle_2d gradient y",
    );
}

#[test]
fn accuracy_baseline_electrostatic_plane_quad_2d_patch() {
    let result = solve_electrostatic_plane_quad_2d(&SolveElectrostaticPlaneQuad2dRequest {
        nodes: vec![
            ElectrostaticPlaneNodeInput {
                id: "e0".to_string(),
                x: 0.0,
                y: 0.0,
                fix_potential: true,
                potential: 12.0,
                charge_density: 0.0,
            },
            ElectrostaticPlaneNodeInput {
                id: "e1".to_string(),
                x: 1.0,
                y: 0.0,
                fix_potential: true,
                potential: 4.0,
                charge_density: 0.0,
            },
            ElectrostaticPlaneNodeInput {
                id: "e2".to_string(),
                x: 1.0,
                y: 1.0,
                fix_potential: true,
                potential: 4.0,
                charge_density: 0.0,
            },
            ElectrostaticPlaneNodeInput {
                id: "e3".to_string(),
                x: 0.0,
                y: 1.0,
                fix_potential: true,
                potential: 12.0,
                charge_density: 0.0,
            },
        ],
        elements: vec![ElectrostaticPlaneQuadElementInput {
            id: "epq0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.1,
            permittivity: 3.0,
        }],
    })
    .expect("electrostatic_plane_quad_2d baseline should solve");

    assert_close_abs(
        result.max_potential,
        12.0,
        1.0e-12,
        "electrostatic_plane_quad_2d max potential",
    );
    assert_close_abs(
        result.max_electric_field,
        8.0,
        1.0e-12,
        "electrostatic_plane_quad_2d max electric field",
    );
    assert_close_abs(
        result.max_flux_density,
        24.0,
        1.0e-12,
        "electrostatic_plane_quad_2d max flux density",
    );
    assert_close_abs(
        result.elements[0].potential_gradient_x,
        -8.0,
        1.0e-12,
        "electrostatic_plane_quad_2d gradient x",
    );
    assert_close_abs(
        result.elements[0].potential_gradient_y,
        0.0,
        1.0e-12,
        "electrostatic_plane_quad_2d gradient y",
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
