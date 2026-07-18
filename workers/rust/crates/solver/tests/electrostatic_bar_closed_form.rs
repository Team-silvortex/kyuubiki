use kyuubiki_protocol::{
    ElectrostaticBar1dElementInput, ElectrostaticBar1dNodeInput, SolveElectrostaticBar1dRequest,
    SolveElectrostaticBar1dResult,
};
use kyuubiki_solver::solve_electrostatic_bar_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn electrostatic_bar_1d_tracks_charge_and_permittivity_scaling() {
    let baseline = ElectrostaticCase {
        length: 2.0,
        area: 0.2,
        permittivity: 4.0e-9,
        charge: 8.0e-6,
    };
    let baseline_result =
        solve_electrostatic_bar_1d(&baseline.request()).expect("baseline electrostatic bar");
    assert_response(&baseline_result, baseline.expected());

    let charge_factor = 3.0;
    let charged = ElectrostaticCase {
        charge: baseline.charge * charge_factor,
        ..baseline
    };
    let charged_result =
        solve_electrostatic_bar_1d(&charged.request()).expect("charge-scaled electrostatic bar");
    assert_response(&charged_result, charged.expected());
    assert_close(
        charged_result.nodes[1].potential,
        baseline_result.nodes[1].potential * charge_factor,
    );
    assert_close(
        charged_result.elements[0].electric_field,
        baseline_result.elements[0].electric_field * charge_factor,
    );
    assert_close(
        charged_result.elements[0].electric_flux_density,
        baseline_result.elements[0].electric_flux_density * charge_factor,
    );
    assert_close(
        charged_result.total_stored_energy,
        baseline_result.total_stored_energy * charge_factor * charge_factor,
    );

    let permittivity_factor = 5.0;
    let dielectric = ElectrostaticCase {
        permittivity: baseline.permittivity * permittivity_factor,
        ..baseline
    };
    let dielectric_result = solve_electrostatic_bar_1d(&dielectric.request())
        .expect("permittivity-scaled electrostatic bar");
    assert_response(&dielectric_result, dielectric.expected());
    assert_close(
        dielectric_result.nodes[1].potential,
        baseline_result.nodes[1].potential / permittivity_factor,
    );
    assert_close(
        dielectric_result.elements[0].electric_field,
        baseline_result.elements[0].electric_field / permittivity_factor,
    );
    assert_close(
        dielectric_result.elements[0].electric_flux_density,
        baseline_result.elements[0].electric_flux_density,
    );
    assert_close(
        dielectric_result.total_stored_energy,
        baseline_result.total_stored_energy / permittivity_factor,
    );

    let area_factor = 2.0;
    let wider = ElectrostaticCase {
        area: baseline.area * area_factor,
        ..baseline
    };
    let wider_result =
        solve_electrostatic_bar_1d(&wider.request()).expect("area-scaled electrostatic bar");
    assert_response(&wider_result, wider.expected());
    assert_close(
        wider_result.nodes[1].potential,
        baseline_result.nodes[1].potential / area_factor,
    );
    assert_close(
        wider_result.elements[0].electric_field,
        baseline_result.elements[0].electric_field / area_factor,
    );
    assert_close(
        wider_result.elements[0].electric_flux_density,
        baseline_result.elements[0].electric_flux_density / area_factor,
    );
    assert_close(
        wider_result.total_stored_energy,
        baseline_result.total_stored_energy / area_factor,
    );

    let length_factor = 1.75;
    let longer = ElectrostaticCase {
        length: baseline.length * length_factor,
        ..baseline
    };
    let longer_result =
        solve_electrostatic_bar_1d(&longer.request()).expect("length-scaled electrostatic bar");
    assert_response(&longer_result, longer.expected());
    assert_close(
        longer_result.nodes[1].potential,
        baseline_result.nodes[1].potential * length_factor,
    );
    assert_close(
        longer_result.elements[0].electric_field,
        baseline_result.elements[0].electric_field,
    );
    assert_close(
        longer_result.elements[0].electric_flux_density,
        baseline_result.elements[0].electric_flux_density,
    );
    assert_close(
        longer_result.total_stored_energy,
        baseline_result.total_stored_energy * length_factor,
    );
}

#[derive(Clone, Copy)]
struct ElectrostaticCase {
    length: f64,
    area: f64,
    permittivity: f64,
    charge: f64,
}

impl ElectrostaticCase {
    fn request(self) -> SolveElectrostaticBar1dRequest {
        SolveElectrostaticBar1dRequest {
            nodes: vec![
                node("ground", 0.0, true, 0.0, 0.0),
                node("charged", self.length, false, 0.0, self.charge),
            ],
            elements: vec![ElectrostaticBar1dElementInput {
                id: "dielectric".to_string(),
                node_i: 0,
                node_j: 1,
                area: self.area,
                permittivity: self.permittivity,
            }],
        }
    }

    fn expected(self) -> ExpectedElectrostaticResponse {
        let capacitance = self.permittivity * self.area / self.length;
        let potential = self.charge / capacitance;
        let potential_gradient = potential / self.length;
        let electric_field = -potential_gradient;
        let electric_flux_density = self.permittivity * electric_field;
        let stored_energy =
            0.5 * self.permittivity * electric_field * electric_field * self.area * self.length;
        ExpectedElectrostaticResponse {
            potential,
            potential_gradient,
            electric_field,
            electric_flux_density,
            stored_energy,
        }
    }
}

struct ExpectedElectrostaticResponse {
    potential: f64,
    potential_gradient: f64,
    electric_field: f64,
    electric_flux_density: f64,
    stored_energy: f64,
}

fn node(
    id: &str,
    x: f64,
    fix_potential: bool,
    potential: f64,
    charge_density: f64,
) -> ElectrostaticBar1dNodeInput {
    ElectrostaticBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_potential,
        potential,
        charge_density,
    }
}

fn assert_response(
    result: &SolveElectrostaticBar1dResult,
    expected: ExpectedElectrostaticResponse,
) {
    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.nodes[0].potential, 0.0);
    assert_close(result.nodes[1].potential, expected.potential);
    assert_close(result.max_potential, expected.potential.abs());
    assert_close(result.max_electric_field, expected.electric_field.abs());
    assert_close(
        result.max_flux_density,
        expected.electric_flux_density.abs(),
    );
    assert_close(result.total_stored_energy, expected.stored_energy);
    assert_field_balance(result);
    assert_close(
        result.total_stored_energy,
        0.5 * result.nodes[1].potential * result.input.nodes[1].charge_density,
    );
    assert_close(
        result.elements[0].average_potential,
        expected.potential / 2.0,
    );
    assert_close(
        result.elements[0].potential_gradient,
        expected.potential_gradient,
    );
    assert_close(result.elements[0].electric_field, expected.electric_field);
    assert_close(
        result.elements[0].electric_flux_density,
        expected.electric_flux_density,
    );
    assert_close(result.elements[0].stored_energy, expected.stored_energy);
}

fn assert_field_balance(result: &SolveElectrostaticBar1dResult) {
    assert_eq!(result.nodes.len(), result.input.nodes.len());
    assert_eq!(result.elements.len(), result.input.elements.len());
    for (index, node) in result.nodes.iter().enumerate() {
        let input = &result.input.nodes[index];
        assert_eq!(node.index, index);
        assert_eq!(node.id, input.id);
        assert_close(node.x, input.x);
        assert_close(node.charge_density, input.charge_density);
    }

    let max_potential = result
        .nodes
        .iter()
        .map(|node| node.potential.abs())
        .fold(0.0_f64, f64::max);
    let max_electric_field = result
        .elements
        .iter()
        .map(|element| element.electric_field.abs())
        .fold(0.0_f64, f64::max);
    let max_flux_density = result
        .elements
        .iter()
        .map(|element| element.electric_flux_density.abs())
        .fold(0.0_f64, f64::max);
    let total_stored_energy = result
        .elements
        .iter()
        .map(|element| element.stored_energy)
        .sum::<f64>();
    assert_close(result.max_potential, max_potential);
    assert_close(result.max_electric_field, max_electric_field);
    assert_close(result.max_flux_density, max_flux_density);
    assert_close(result.total_stored_energy, total_stored_energy);

    for (index, element) in result.elements.iter().enumerate() {
        let input = &result.input.elements[index];
        assert_eq!(element.index, index);
        let node_i = &result.nodes[element.node_i];
        let node_j = &result.nodes[element.node_j];
        let expected_length = (node_j.x - node_i.x).abs();
        let expected_average_potential = 0.5 * (node_i.potential + node_j.potential);
        let expected_gradient = (node_j.potential - node_i.potential) / expected_length;
        let expected_stored_energy = 0.5
            * input.permittivity
            * element.electric_field
            * element.electric_field
            * input.area
            * element.length;
        assert_close(element.length, expected_length);
        assert_close(element.average_potential, expected_average_potential);
        assert_close(element.potential_gradient, expected_gradient);
        assert_close(element.electric_field, -element.potential_gradient);
        assert_close(
            element.electric_flux_density,
            input.permittivity * element.electric_field,
        );
        assert_close(element.stored_energy, expected_stored_energy);
        let nodal_charge = node_i.charge_density + node_j.charge_density;
        assert_close(
            element.electric_flux_density * input.area + nodal_charge,
            0.0,
        );
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
