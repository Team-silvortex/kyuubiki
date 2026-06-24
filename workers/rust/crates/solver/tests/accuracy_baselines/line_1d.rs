use super::common::*;

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
fn accuracy_baseline_spring_1d_chain_fixture() {
    let result = solve_spring_1d(&SolveSpring1dRequest {
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
            Spring1dElementInput {
                id: "k0".to_string(),
                node_i: 0,
                node_j: 1,
                stiffness: 35000.0,
            },
            Spring1dElementInput {
                id: "k1".to_string(),
                node_i: 1,
                node_j: 2,
                stiffness: 20000.0,
            },
        ],
    })
    .expect("spring_1d baseline should solve");

    assert_close_abs(
        result.max_displacement,
        0.09428571428571428,
        1.0e-15,
        "spring_1d max displacement",
    );
    assert_close_abs(result.max_force, 1200.0, 1.0e-12, "spring_1d max force");
    assert_close_abs(
        result.nodes[1].ux,
        0.03428571428571429,
        1.0e-15,
        "spring_1d node-1 ux",
    );
    assert_close_abs(
        result.nodes[2].ux,
        0.09428571428571428,
        1.0e-15,
        "spring_1d node-2 ux",
    );
    assert_close_abs(
        result.elements[0].force,
        1200.0,
        1.0e-12,
        "spring_1d element-0 force",
    );
    assert_close_abs(
        result.elements[1].force,
        1199.9999999999998,
        1.0e-12,
        "spring_1d element-1 force",
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
fn accuracy_baseline_electrostatic_bar_1d_two_element_gradient() {
    let result = solve_electrostatic_bar_1d(&SolveElectrostaticBar1dRequest {
        nodes: vec![
            ElectrostaticBar1dNodeInput {
                id: "e0".to_string(),
                x: 0.0,
                fix_potential: true,
                potential: 12.0,
                charge_density: 0.0,
            },
            ElectrostaticBar1dNodeInput {
                id: "e1".to_string(),
                x: 1.0,
                fix_potential: false,
                potential: 0.0,
                charge_density: 0.0,
            },
            ElectrostaticBar1dNodeInput {
                id: "e2".to_string(),
                x: 2.0,
                fix_potential: true,
                potential: 4.0,
                charge_density: 0.0,
            },
        ],
        elements: vec![
            ElectrostaticBar1dElementInput {
                id: "ee0".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                permittivity: 3.0,
            },
            ElectrostaticBar1dElementInput {
                id: "ee1".to_string(),
                node_i: 1,
                node_j: 2,
                area: 0.01,
                permittivity: 3.0,
            },
        ],
    })
    .expect("electrostatic_bar_1d baseline should solve");

    assert_close_abs(
        result.max_potential,
        12.0,
        1.0e-12,
        "electrostatic_bar_1d max potential",
    );
    assert_close_abs(
        result.max_electric_field,
        4.0,
        1.0e-12,
        "electrostatic_bar_1d max electric field",
    );
    assert_close_abs(
        result.max_flux_density,
        12.0,
        1.0e-12,
        "electrostatic_bar_1d max flux density",
    );
    assert_close_abs(
        result.nodes[1].potential,
        8.0,
        1.0e-12,
        "electrostatic_bar_1d middle node potential",
    );
    assert_close_abs(
        result.elements[0].electric_field,
        4.0,
        1.0e-12,
        "electrostatic_bar_1d first element electric field",
    );
    assert_close_abs(
        result.elements[1].electric_field,
        4.0,
        1.0e-12,
        "electrostatic_bar_1d second element electric field",
    );
}
