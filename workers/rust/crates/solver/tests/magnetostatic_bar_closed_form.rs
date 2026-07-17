use kyuubiki_protocol::{
    MagnetostaticBar1dElementInput, MagnetostaticBar1dNodeInput, SolveMagnetostaticBar1dRequest,
    SolveMagnetostaticBar1dResult,
};
use kyuubiki_solver::solve_magnetostatic_bar_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn magnetostatic_bar_1d_matches_closed_form_permeance_scaling() {
    for case in [
        MagneticCase {
            length: 1.5,
            area: 0.2,
            permeability: 4.0e-7 * std::f64::consts::PI,
            source: 2.0e-6,
        },
        MagneticCase {
            length: 3.0,
            area: 0.4,
            permeability: 8.0e-7 * std::f64::consts::PI,
            source: 5.0e-6,
        },
    ] {
        let result =
            solve_magnetostatic_bar_1d(&case.request()).expect("closed-form magnetic case");
        assert_case(&result, case.expected());
    }
}

#[test]
fn magnetostatic_bar_1d_reports_zero_energy_for_zero_source() {
    let case = MagneticCase {
        length: 2.0,
        area: 0.3,
        permeability: 4.0e-7 * std::f64::consts::PI,
        source: 0.0,
    };
    let result = solve_magnetostatic_bar_1d(&case.request()).expect("zero-source magnetic case");

    assert_close(result.max_magnetic_potential, 0.0);
    assert_close(result.max_magnetic_field_strength, 0.0);
    assert_close(result.max_flux_density, 0.0);
    assert_close(result.total_stored_energy, 0.0);
    assert_case(&result, case.expected());
}

#[test]
fn magnetostatic_bar_1d_tracks_source_permeability_and_area_scaling() {
    let baseline = MagneticCase {
        length: 2.25,
        area: 0.15,
        permeability: 4.0e-7 * std::f64::consts::PI,
        source: 3.0e-6,
    };
    let baseline_result =
        solve_magnetostatic_bar_1d(&baseline.request()).expect("baseline magnetic scaling case");

    let source_factor = 2.5;
    let sourced = MagneticCase {
        source: baseline.source * source_factor,
        ..baseline
    };
    let sourced_result =
        solve_magnetostatic_bar_1d(&sourced.request()).expect("source-scaled magnetic case");
    assert_case(&sourced_result, sourced.expected());
    assert_close(
        sourced_result.nodes[1].magnetic_potential,
        baseline_result.nodes[1].magnetic_potential * source_factor,
    );
    assert_close(
        sourced_result.elements[0].magnetic_field_strength,
        baseline_result.elements[0].magnetic_field_strength * source_factor,
    );
    assert_close(
        sourced_result.elements[0].magnetic_flux_density,
        baseline_result.elements[0].magnetic_flux_density * source_factor,
    );
    assert_close(
        sourced_result.total_stored_energy,
        baseline_result.total_stored_energy * source_factor * source_factor,
    );

    let permeability_factor = 4.0;
    let permeable = MagneticCase {
        permeability: baseline.permeability * permeability_factor,
        ..baseline
    };
    let permeable_result = solve_magnetostatic_bar_1d(&permeable.request())
        .expect("permeability-scaled magnetic case");
    assert_case(&permeable_result, permeable.expected());
    assert_close(
        permeable_result.nodes[1].magnetic_potential,
        baseline_result.nodes[1].magnetic_potential / permeability_factor,
    );
    assert_close(
        permeable_result.elements[0].magnetic_field_strength,
        baseline_result.elements[0].magnetic_field_strength / permeability_factor,
    );
    assert_close(
        permeable_result.elements[0].magnetic_flux_density,
        baseline_result.elements[0].magnetic_flux_density,
    );
    assert_close(
        permeable_result.total_stored_energy,
        baseline_result.total_stored_energy / permeability_factor,
    );

    let area_factor = 3.0;
    let wider = MagneticCase {
        area: baseline.area * area_factor,
        ..baseline
    };
    let wider_result =
        solve_magnetostatic_bar_1d(&wider.request()).expect("area-scaled magnetic case");
    assert_case(&wider_result, wider.expected());
    assert_close(
        wider_result.nodes[1].magnetic_potential,
        baseline_result.nodes[1].magnetic_potential / area_factor,
    );
    assert_close(
        wider_result.elements[0].magnetic_field_strength,
        baseline_result.elements[0].magnetic_field_strength / area_factor,
    );
    assert_close(
        wider_result.elements[0].magnetic_flux_density,
        baseline_result.elements[0].magnetic_flux_density / area_factor,
    );
    assert_close(
        wider_result.total_stored_energy,
        baseline_result.total_stored_energy / area_factor,
    );

    let length_factor = 1.6;
    let longer = MagneticCase {
        length: baseline.length * length_factor,
        ..baseline
    };
    let longer_result =
        solve_magnetostatic_bar_1d(&longer.request()).expect("length-scaled magnetic case");
    assert_case(&longer_result, longer.expected());
    assert_close(
        longer_result.nodes[1].magnetic_potential,
        baseline_result.nodes[1].magnetic_potential * length_factor,
    );
    assert_close(
        longer_result.elements[0].magnetic_field_strength,
        baseline_result.elements[0].magnetic_field_strength,
    );
    assert_close(
        longer_result.elements[0].magnetic_flux_density,
        baseline_result.elements[0].magnetic_flux_density,
    );
    assert_close(
        longer_result.total_stored_energy,
        baseline_result.total_stored_energy * length_factor,
    );
}

#[derive(Clone, Copy)]
struct MagneticCase {
    length: f64,
    area: f64,
    permeability: f64,
    source: f64,
}

impl MagneticCase {
    fn request(self) -> SolveMagnetostaticBar1dRequest {
        SolveMagnetostaticBar1dRequest {
            nodes: vec![
                node("ground", 0.0, true, 0.0, 0.0),
                node("source", self.length, false, 0.0, self.source),
            ],
            elements: vec![MagnetostaticBar1dElementInput {
                id: "core".to_string(),
                node_i: 0,
                node_j: 1,
                area: self.area,
                permeability: self.permeability,
            }],
        }
    }

    fn expected(self) -> ExpectedMagneticResponse {
        let permeance = self.permeability * self.area / self.length;
        let magnetic_potential = self.source / permeance;
        let gradient = magnetic_potential / self.length;
        let field_strength = -gradient;
        let flux_density = self.permeability * field_strength;
        let stored_energy =
            0.5 * self.permeability * field_strength * field_strength * self.area * self.length;
        ExpectedMagneticResponse {
            magnetic_potential,
            gradient,
            field_strength,
            flux_density,
            stored_energy,
        }
    }
}

struct ExpectedMagneticResponse {
    magnetic_potential: f64,
    gradient: f64,
    field_strength: f64,
    flux_density: f64,
    stored_energy: f64,
}

fn assert_case(result: &SolveMagnetostaticBar1dResult, expected: ExpectedMagneticResponse) {
    let element = &result.elements[0];
    assert_close(result.nodes[0].magnetic_potential, 0.0);
    assert_close(
        result.nodes[1].magnetic_potential,
        expected.magnetic_potential,
    );
    assert_close(
        result.max_magnetic_potential,
        expected.magnetic_potential.abs(),
    );
    assert_close(
        result.max_magnetic_field_strength,
        expected.field_strength.abs(),
    );
    assert_close(result.max_flux_density, expected.flux_density.abs());
    assert_close(result.total_stored_energy, expected.stored_energy);
    assert_field_balance(result);
    assert_close(
        result.total_stored_energy,
        0.5 * result.nodes[1].magnetic_potential * result.input.nodes[1].magnetomotive_source,
    );
    assert_close(
        element.average_magnetic_potential,
        expected.magnetic_potential / 2.0,
    );
    assert_close(element.magnetic_potential_gradient, expected.gradient);
    assert_close(element.magnetic_field_strength, expected.field_strength);
    assert_close(element.magnetic_flux_density, expected.flux_density);
    assert_close(element.stored_energy, expected.stored_energy);
}

fn assert_field_balance(result: &SolveMagnetostaticBar1dResult) {
    for node in &result.nodes {
        let input = &result.input.nodes[node.index];
        assert_eq!(node.id, input.id);
        assert_close(node.x, input.x);
        assert_close(node.magnetomotive_source, input.magnetomotive_source);
    }

    let max_magnetic_potential = result
        .nodes
        .iter()
        .map(|node| node.magnetic_potential.abs())
        .fold(0.0_f64, f64::max);
    let max_field_strength = result
        .elements
        .iter()
        .map(|element| element.magnetic_field_strength.abs())
        .fold(0.0_f64, f64::max);
    let max_flux_density = result
        .elements
        .iter()
        .map(|element| element.magnetic_flux_density.abs())
        .fold(0.0_f64, f64::max);
    let total_stored_energy = result
        .elements
        .iter()
        .map(|element| element.stored_energy)
        .sum::<f64>();
    assert_close(result.max_magnetic_potential, max_magnetic_potential);
    assert_close(result.max_magnetic_field_strength, max_field_strength);
    assert_close(result.max_flux_density, max_flux_density);
    assert_close(result.total_stored_energy, total_stored_energy);

    for element in &result.elements {
        let input = &result.input.elements[element.index];
        let node_i = &result.nodes[element.node_i];
        let node_j = &result.nodes[element.node_j];
        let expected_length = (node_j.x - node_i.x).abs();
        let expected_average_potential =
            0.5 * (node_i.magnetic_potential + node_j.magnetic_potential);
        let expected_gradient =
            (node_j.magnetic_potential - node_i.magnetic_potential) / expected_length;
        let expected_stored_energy = 0.5
            * input.permeability
            * element.magnetic_field_strength
            * element.magnetic_field_strength
            * input.area
            * element.length;
        assert_close(element.length, expected_length);
        assert_close(
            element.average_magnetic_potential,
            expected_average_potential,
        );
        assert_close(element.magnetic_potential_gradient, expected_gradient);
        assert_close(
            element.magnetic_field_strength,
            -element.magnetic_potential_gradient,
        );
        assert_close(
            element.magnetic_flux_density,
            input.permeability * element.magnetic_field_strength,
        );
        assert_close(element.stored_energy, expected_stored_energy);
        let nodal_source = node_i.magnetomotive_source + node_j.magnetomotive_source;
        assert_close(
            element.magnetic_flux_density * input.area + nodal_source,
            0.0,
        );
    }
}

fn node(
    id: &str,
    x: f64,
    fix_magnetic_potential: bool,
    magnetic_potential: f64,
    magnetomotive_source: f64,
) -> MagnetostaticBar1dNodeInput {
    MagnetostaticBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_magnetic_potential,
        magnetic_potential,
        magnetomotive_source,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
