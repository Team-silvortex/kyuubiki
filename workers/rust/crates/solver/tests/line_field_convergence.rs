use kyuubiki_protocol::{
    ElectrostaticBar1dElementInput, ElectrostaticBar1dNodeInput, HeatBar1dElementInput,
    HeatBar1dNodeInput, SolveBarRequest, SolveElectrostaticBar1dRequest, SolveHeatBar1dRequest,
    SolveThermalBar1dRequest, ThermalBar1dElementInput, ThermalBar1dNodeInput,
};
use kyuubiki_solver::{
    solve_bar_1d, solve_electrostatic_bar_1d, solve_heat_bar_1d, solve_thermal_bar_1d,
};

const TOL: f64 = 1.0e-10;

#[test]
fn axial_bar_linear_field_is_refinement_invariant() {
    let length = 3.2;
    let area = 0.014;
    let youngs_modulus = 205.0e9;
    let tip_force = 4_200.0;
    let expected_tip = tip_force * length / (youngs_modulus * area);
    let expected_strain = expected_tip / length;
    let expected_stress = youngs_modulus * expected_strain;
    let expected_energy_density = 0.5 * expected_stress * expected_strain;
    let expected_total_energy = 0.5 * tip_force * expected_tip;

    for elements in [1_usize, 2, 4, 8, 16] {
        let result = solve_bar_1d(&SolveBarRequest {
            length,
            area,
            youngs_modulus,
            elements,
            tip_force,
        })
        .expect("refined axial bar should solve");

        assert_eq!(result.nodes.len(), elements + 1);
        assert_eq!(result.elements.len(), elements);
        assert_close(result.tip_displacement, expected_tip, "bar tip");
        assert_close(result.reaction_force, -tip_force, "bar reaction");
        assert_close(
            result.total_strain_energy,
            expected_total_energy,
            "bar energy",
        );
        for node in &result.nodes {
            let expected = expected_tip * node.x / length;
            assert_close(node.displacement, expected, "bar displacement field");
        }
        for element in &result.elements {
            assert_close(element.strain, expected_strain, "bar strain");
            assert_close(element.stress, expected_stress, "bar stress");
            assert_close(element.axial_force, tip_force, "bar axial force");
            assert_close(
                element.strain_energy_density,
                expected_energy_density,
                "bar energy density",
            );
        }
    }
}

#[test]
fn thermal_bar_restrained_uniform_rise_is_refinement_invariant() {
    let length = 2.4;
    let area = 0.011;
    let youngs_modulus = 210.0e9;
    let alpha = 11.5e-6;
    let temperature_delta = 42.0;
    let thermal_strain = alpha * temperature_delta;
    let stress = -youngs_modulus * thermal_strain;
    let axial_force = stress * area;
    let energy_density = 0.5 * stress * -thermal_strain;
    let total_energy = energy_density * area * length;

    for elements in [1_usize, 2, 4, 8, 16] {
        let result = solve_thermal_bar_1d(&thermal_mesh(
            elements,
            length,
            area,
            youngs_modulus,
            alpha,
            temperature_delta,
        ))
        .expect("refined thermal bar should solve");

        assert_eq!(result.nodes.len(), elements + 1);
        assert_eq!(result.elements.len(), elements);
        assert_close(result.max_displacement, 0.0, "thermal displacement");
        assert_close(result.max_stress, stress.abs(), "thermal max stress");
        assert_close(result.max_axial_force, axial_force.abs(), "thermal force");
        assert_close(result.total_strain_energy, total_energy, "thermal energy");
        for node in &result.nodes {
            assert_close(node.ux, 0.0, "thermal ux");
            assert_close(node.temperature_delta, temperature_delta, "thermal dt");
        }
        for element in &result.elements {
            assert_close(element.thermal_strain, thermal_strain, "thermal strain");
            assert_close(
                element.mechanical_strain,
                -thermal_strain,
                "mechanical strain",
            );
            assert_close(element.total_strain, 0.0, "total strain");
            assert_close(element.stress, stress, "thermal stress");
            assert_close(element.axial_force, axial_force, "thermal axial force");
            assert_close(
                element.strain_energy_density,
                energy_density,
                "thermal energy density",
            );
        }
    }
}

#[test]
fn heat_bar_linear_temperature_field_is_refinement_invariant() {
    let length = 2.5;
    let area = 0.2;
    let conductivity = 28.0;
    let left_temperature = 120.0;
    let right_temperature = 40.0;
    let gradient = (right_temperature - left_temperature) / length;
    let heat_flux = -conductivity * gradient;

    for elements in [1_usize, 2, 4, 8, 16] {
        let result = solve_heat_bar_1d(&heat_mesh(
            elements,
            length,
            area,
            conductivity,
            left_temperature,
            right_temperature,
        ))
        .expect("refined heat bar should solve");

        assert_eq!(result.nodes.len(), elements + 1);
        assert_eq!(result.elements.len(), elements);
        assert_close(
            result.max_temperature,
            left_temperature,
            "heat max temperature",
        );
        assert_close(result.max_heat_flux, heat_flux.abs(), "heat max flux");
        for node in &result.nodes {
            let expected = left_temperature + gradient * node.x;
            assert_close(node.temperature, expected, "heat temperature");
        }
        for element in &result.elements {
            assert_close(element.temperature_gradient, gradient, "heat gradient");
            assert_close(element.heat_flux, heat_flux, "heat flux");
            assert_close(
                element.average_temperature,
                0.5 * (result.nodes[element.node_i].temperature
                    + result.nodes[element.node_j].temperature),
                "heat average",
            );
        }
    }
}

#[test]
fn electrostatic_bar_linear_potential_field_is_refinement_invariant() {
    let length = 1.8;
    let area = 0.16;
    let permittivity = 3.2e-9;
    let left_potential = 0.0;
    let right_potential = 12.0;
    let gradient = (right_potential - left_potential) / length;
    let electric_field: f64 = -gradient;
    let flux_density = permittivity * electric_field;
    let total_energy = 0.5 * permittivity * electric_field.powi(2) * area * length;

    for elements in [1_usize, 2, 4, 8, 16] {
        let result = solve_electrostatic_bar_1d(&electrostatic_mesh(
            elements,
            length,
            area,
            permittivity,
            left_potential,
            right_potential,
        ))
        .expect("refined electrostatic bar should solve");

        assert_eq!(result.nodes.len(), elements + 1);
        assert_eq!(result.elements.len(), elements);
        assert_close(
            result.max_potential,
            right_potential,
            "electrostatic potential",
        );
        assert_close(
            result.max_electric_field,
            electric_field.abs(),
            "electric field",
        );
        assert_close(result.max_flux_density, flux_density.abs(), "flux density");
        assert_close(result.total_stored_energy, total_energy, "stored energy");
        for node in &result.nodes {
            let expected = left_potential + gradient * node.x;
            assert_close(node.potential, expected, "potential field");
        }
        for element in &result.elements {
            assert_close(element.potential_gradient, gradient, "potential gradient");
            assert_close(element.electric_field, electric_field, "electric field");
            assert_close(element.electric_flux_density, flux_density, "flux density");
        }
    }
}

fn thermal_mesh(
    elements: usize,
    length: f64,
    area: f64,
    youngs_modulus: f64,
    alpha: f64,
    temperature_delta: f64,
) -> SolveThermalBar1dRequest {
    SolveThermalBar1dRequest {
        nodes: (0..=elements)
            .map(|index| ThermalBar1dNodeInput {
                id: format!("n-{index}"),
                x: length * index as f64 / elements as f64,
                fix_x: index == 0 || index == elements,
                load_x: 0.0,
                temperature_delta,
            })
            .collect(),
        elements: (0..elements)
            .map(|index| ThermalBar1dElementInput {
                id: format!("e-{index}"),
                node_i: index,
                node_j: index + 1,
                area,
                youngs_modulus,
                thermal_expansion: alpha,
            })
            .collect(),
    }
}

fn heat_mesh(
    elements: usize,
    length: f64,
    area: f64,
    conductivity: f64,
    left_temperature: f64,
    right_temperature: f64,
) -> SolveHeatBar1dRequest {
    SolveHeatBar1dRequest {
        nodes: (0..=elements)
            .map(|index| {
                let fixed = index == 0 || index == elements;
                HeatBar1dNodeInput {
                    id: format!("n-{index}"),
                    x: length * index as f64 / elements as f64,
                    fix_temperature: fixed,
                    temperature: if index == elements {
                        right_temperature
                    } else {
                        left_temperature
                    },
                    heat_load: 0.0,
                }
            })
            .collect(),
        elements: (0..elements)
            .map(|index| HeatBar1dElementInput {
                id: format!("e-{index}"),
                node_i: index,
                node_j: index + 1,
                area,
                conductivity,
            })
            .collect(),
    }
}

fn electrostatic_mesh(
    elements: usize,
    length: f64,
    area: f64,
    permittivity: f64,
    left_potential: f64,
    right_potential: f64,
) -> SolveElectrostaticBar1dRequest {
    SolveElectrostaticBar1dRequest {
        nodes: (0..=elements)
            .map(|index| {
                let fixed = index == 0 || index == elements;
                ElectrostaticBar1dNodeInput {
                    id: format!("n-{index}"),
                    x: length * index as f64 / elements as f64,
                    fix_potential: fixed,
                    potential: if index == elements {
                        right_potential
                    } else {
                        left_potential
                    },
                    charge_density: 0.0,
                }
            })
            .collect(),
        elements: (0..elements)
            .map(|index| ElectrostaticBar1dElementInput {
                id: format!("e-{index}"),
                node_i: index,
                node_j: index + 1,
                area,
                permittivity,
            })
            .collect(),
    }
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "{label}: expected {actual} to be close to {expected}",
    );
}
