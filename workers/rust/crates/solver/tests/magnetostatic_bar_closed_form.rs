use kyuubiki_protocol::{
    MagnetostaticBar1dElementInput, MagnetostaticBar1dNodeInput, SolveMagnetostaticBar1dRequest,
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
        let expected = case.expected();
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
        assert_close(
            element.average_magnetic_potential,
            expected.magnetic_potential / 2.0,
        );
        assert_close(element.magnetic_potential_gradient, expected.gradient);
        assert_close(element.magnetic_field_strength, expected.field_strength);
        assert_close(element.magnetic_flux_density, expected.flux_density);
        assert_close(element.stored_energy, expected.stored_energy);
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
