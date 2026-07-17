use kyuubiki_protocol::{
    SolveThermalBar1dRequest, SolveThermalBar1dResult, ThermalBar1dElementInput,
    ThermalBar1dElementResult, ThermalBar1dNodeInput,
};
use kyuubiki_solver::solve_thermal_bar_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn thermal_bar_1d_tracks_restrained_uniform_rise_scaling() {
    let baseline_case = ThermalBarCase {
        length: 1.4,
        area: 0.012,
        youngs_modulus: 210.0e9,
        thermal_expansion: 12.0e-6,
        temperature_delta: 35.0,
    };
    let baseline = solve_thermal_bar_1d(&baseline_case.request()).expect("baseline thermal bar");
    assert_response(&baseline, baseline_case.expected());

    let temperature_scale = 1.6;
    let heated_case = ThermalBarCase {
        temperature_delta: baseline_case.temperature_delta * temperature_scale,
        ..baseline_case
    };
    let heated = solve_thermal_bar_1d(&heated_case.request()).expect("temperature-scaled bar");
    assert_response(&heated, heated_case.expected());
    assert_close(heated.max_stress / baseline.max_stress, temperature_scale);
    assert_close(
        heated.max_axial_force / baseline.max_axial_force,
        temperature_scale,
    );
    assert_close(
        heated.total_strain_energy / baseline.total_strain_energy,
        temperature_scale * temperature_scale,
    );

    let expansion_scale = 1.4;
    let expanded_case = ThermalBarCase {
        thermal_expansion: baseline_case.thermal_expansion * expansion_scale,
        ..baseline_case
    };
    let expanded = solve_thermal_bar_1d(&expanded_case.request()).expect("alpha-scaled bar");
    assert_response(&expanded, expanded_case.expected());
    assert_close(expanded.max_stress / baseline.max_stress, expansion_scale);
    assert_close(
        expanded.total_strain_energy / baseline.total_strain_energy,
        expansion_scale * expansion_scale,
    );

    let modulus_scale = 1.8;
    let stiffer_case = ThermalBarCase {
        youngs_modulus: baseline_case.youngs_modulus * modulus_scale,
        ..baseline_case
    };
    let stiffer = solve_thermal_bar_1d(&stiffer_case.request()).expect("modulus-scaled bar");
    assert_response(&stiffer, stiffer_case.expected());
    assert_close(stiffer.max_stress / baseline.max_stress, modulus_scale);
    assert_close(
        stiffer.total_strain_energy / baseline.total_strain_energy,
        modulus_scale,
    );

    let area_scale = 2.2;
    let wider_case = ThermalBarCase {
        area: baseline_case.area * area_scale,
        ..baseline_case
    };
    let wider = solve_thermal_bar_1d(&wider_case.request()).expect("area-scaled bar");
    assert_response(&wider, wider_case.expected());
    assert_close(wider.max_stress, baseline.max_stress);
    assert_close(wider.max_axial_force / baseline.max_axial_force, area_scale);
    assert_close(
        wider.total_strain_energy / baseline.total_strain_energy,
        area_scale,
    );

    let length_scale = 1.7;
    let longer_case = ThermalBarCase {
        length: baseline_case.length * length_scale,
        ..baseline_case
    };
    let longer = solve_thermal_bar_1d(&longer_case.request()).expect("length-scaled bar");
    assert_response(&longer, longer_case.expected());
    assert_close(longer.max_stress, baseline.max_stress);
    assert_close(longer.max_axial_force, baseline.max_axial_force);
    assert_close(
        longer.total_strain_energy / baseline.total_strain_energy,
        length_scale,
    );
}

#[derive(Clone, Copy)]
struct ThermalBarCase {
    length: f64,
    area: f64,
    youngs_modulus: f64,
    thermal_expansion: f64,
    temperature_delta: f64,
}

impl ThermalBarCase {
    fn request(self) -> SolveThermalBar1dRequest {
        SolveThermalBar1dRequest {
            nodes: vec![
                node("fixed-left", 0.0, true, 0.0, self.temperature_delta),
                node(
                    "fixed-right",
                    self.length,
                    true,
                    0.0,
                    self.temperature_delta,
                ),
            ],
            elements: vec![ThermalBar1dElementInput {
                id: "bar".to_string(),
                node_i: 0,
                node_j: 1,
                area: self.area,
                youngs_modulus: self.youngs_modulus,
                thermal_expansion: self.thermal_expansion,
            }],
        }
    }

    fn expected(self) -> ExpectedThermalBarResponse {
        let thermal_strain = self.thermal_expansion * self.temperature_delta;
        let mechanical_strain = -thermal_strain;
        let stress = self.youngs_modulus * mechanical_strain;
        let axial_force = stress * self.area;
        let strain_energy_density = 0.5 * stress * mechanical_strain;
        ExpectedThermalBarResponse {
            thermal_strain,
            mechanical_strain,
            total_strain: 0.0,
            stress,
            axial_force,
            strain_energy_density,
            total_strain_energy: strain_energy_density * self.area * self.length,
        }
    }
}

struct ExpectedThermalBarResponse {
    thermal_strain: f64,
    mechanical_strain: f64,
    total_strain: f64,
    stress: f64,
    axial_force: f64,
    strain_energy_density: f64,
    total_strain_energy: f64,
}

fn node(
    id: &str,
    x: f64,
    fix_x: bool,
    load_x: f64,
    temperature_delta: f64,
) -> ThermalBar1dNodeInput {
    ThermalBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_x,
        load_x,
        temperature_delta,
    }
}

fn assert_response(result: &SolveThermalBar1dResult, expected: ExpectedThermalBarResponse) {
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.nodes[0].ux, 0.0);
    assert_close(result.nodes[1].ux, 0.0);
    assert_close(result.max_displacement, 0.0);
    assert_close(result.max_stress, expected.stress.abs());
    assert_close(result.max_axial_force, expected.axial_force.abs());
    assert_close(
        result.max_strain_energy_density,
        expected.strain_energy_density.abs(),
    );
    assert_close(result.total_strain_energy, expected.total_strain_energy);
    assert_thermal_bar_summary(result);

    let element = &result.elements[0];
    assert_close(element.thermal_strain, expected.thermal_strain);
    assert_close(element.mechanical_strain, expected.mechanical_strain);
    assert_close(element.total_strain, expected.total_strain);
    assert_close(element.stress, expected.stress);
    assert_close(element.axial_force, expected.axial_force);
    assert_close(
        element.strain_energy_density,
        expected.strain_energy_density,
    );
    assert!(element.stress <= 0.0);
}

fn assert_thermal_bar_summary(result: &SolveThermalBar1dResult) {
    for node in &result.nodes {
        let input = &result.input.nodes[node.index];
        assert_eq!(node.id, input.id);
        assert_close(node.x, input.x);
        assert_close(node.temperature_delta, input.temperature_delta);
    }

    let max_displacement = result
        .nodes
        .iter()
        .map(|node| node.ux.abs())
        .fold(0.0_f64, f64::max);
    let max_temperature_delta = result
        .nodes
        .iter()
        .map(|node| node.temperature_delta.abs())
        .fold(0.0_f64, f64::max);
    assert_close(result.max_displacement, max_displacement);
    assert_close(result.max_temperature_delta, max_temperature_delta);

    let mut max_stress = 0.0_f64;
    let mut max_axial_force = 0.0_f64;
    let mut max_energy_density = 0.0_f64;
    let mut total_energy = 0.0_f64;
    for element in &result.elements {
        let input = &result.input.elements[element.index];
        assert_element_law(result, element);
        max_stress = max_stress.max(element.stress.abs());
        max_axial_force = max_axial_force.max(element.axial_force.abs());
        max_energy_density = max_energy_density.max(element.strain_energy_density.abs());
        total_energy += element.strain_energy_density * input.area * element.length;
    }
    assert_close(result.max_stress, max_stress);
    assert_close(result.max_axial_force, max_axial_force);
    assert_close(result.max_strain_energy_density, max_energy_density);
    assert_close(result.total_strain_energy, total_energy);
}

fn assert_element_law(result: &SolveThermalBar1dResult, element: &ThermalBar1dElementResult) {
    let input = &result.input.elements[element.index];
    let node_i = &result.nodes[element.node_i];
    let node_j = &result.nodes[element.node_j];
    let expected_length = (node_j.x - node_i.x).abs();
    let expected_average_temperature = 0.5 * (node_i.temperature_delta + node_j.temperature_delta);
    let expected_total_strain = (node_j.ux - node_i.ux) / expected_length;
    let expected_thermal_strain = input.thermal_expansion * expected_average_temperature;
    let expected_mechanical_strain = expected_total_strain - expected_thermal_strain;
    let expected_stress = input.youngs_modulus * expected_mechanical_strain;
    let expected_axial_force = expected_stress * input.area;
    let expected_energy_density = 0.5 * expected_stress * expected_mechanical_strain;
    assert_close(element.length, expected_length);
    assert_close(
        element.average_temperature_delta,
        expected_average_temperature,
    );
    assert_close(element.total_strain, expected_total_strain);
    assert_close(element.thermal_strain, expected_thermal_strain);
    assert_close(element.mechanical_strain, expected_mechanical_strain);
    assert_close(element.stress, expected_stress);
    assert_close(element.axial_force, expected_axial_force);
    assert_close(element.strain_energy_density, expected_energy_density);
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
