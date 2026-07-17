use kyuubiki_protocol::SolveBarRequest;
use kyuubiki_solver::solve_bar_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn bar_1d_tracks_load_area_and_modulus_scaling() {
    let baseline_case = BarCase {
        length: 2.8,
        area: 0.016,
        youngs_modulus: 205.0e9,
        elements: 6,
        tip_force: 3_600.0,
    };
    let baseline = solve_bar_1d(&baseline_case.request()).expect("baseline axial bar");
    assert_response(&baseline, baseline_case.expected());

    let load_scale = 1.7;
    let loaded_case = BarCase {
        tip_force: baseline_case.tip_force * load_scale,
        ..baseline_case
    };
    let loaded = solve_bar_1d(&loaded_case.request()).expect("load-scaled axial bar");
    assert_response(&loaded, loaded_case.expected());
    assert_close(
        loaded.tip_displacement / baseline.tip_displacement,
        load_scale,
    );
    assert_close(loaded.max_stress / baseline.max_stress, load_scale);
    assert_close(loaded.reaction_force / baseline.reaction_force, load_scale);
    assert_close(
        loaded.total_strain_energy / baseline.total_strain_energy,
        load_scale * load_scale,
    );

    let area_scale = 2.5;
    let wider_case = BarCase {
        area: baseline_case.area * area_scale,
        ..baseline_case
    };
    let wider = solve_bar_1d(&wider_case.request()).expect("area-scaled axial bar");
    assert_response(&wider, wider_case.expected());
    assert_close(
        wider.tip_displacement / baseline.tip_displacement,
        1.0 / area_scale,
    );
    assert_close(wider.max_stress / baseline.max_stress, 1.0 / area_scale);
    assert_close(
        wider.elements[0].axial_force,
        baseline.elements[0].axial_force,
    );
    assert_close(
        wider.total_strain_energy / baseline.total_strain_energy,
        1.0 / area_scale,
    );

    let modulus_scale = 1.9;
    let stiffer_case = BarCase {
        youngs_modulus: baseline_case.youngs_modulus * modulus_scale,
        ..baseline_case
    };
    let stiffer = solve_bar_1d(&stiffer_case.request()).expect("modulus-scaled axial bar");
    assert_response(&stiffer, stiffer_case.expected());
    assert_close(
        stiffer.tip_displacement / baseline.tip_displacement,
        1.0 / modulus_scale,
    );
    assert_close(stiffer.max_stress, baseline.max_stress);
    assert_close(
        stiffer.elements[0].axial_force,
        baseline.elements[0].axial_force,
    );
    assert_close(
        stiffer.total_strain_energy / baseline.total_strain_energy,
        1.0 / modulus_scale,
    );

    let length_scale = 1.6;
    let longer_case = BarCase {
        length: baseline_case.length * length_scale,
        ..baseline_case
    };
    let longer = solve_bar_1d(&longer_case.request()).expect("length-scaled axial bar");
    assert_response(&longer, longer_case.expected());
    assert_close(
        longer.tip_displacement / baseline.tip_displacement,
        length_scale,
    );
    assert_close(longer.max_stress, baseline.max_stress);
    assert_close(
        longer.max_strain_energy_density,
        baseline.max_strain_energy_density,
    );
    assert_close(
        longer.elements[0].axial_force,
        baseline.elements[0].axial_force,
    );
    assert_close(
        longer.total_strain_energy / baseline.total_strain_energy,
        length_scale,
    );
}

#[derive(Clone, Copy)]
struct BarCase {
    length: f64,
    area: f64,
    youngs_modulus: f64,
    elements: usize,
    tip_force: f64,
}

impl BarCase {
    fn request(self) -> SolveBarRequest {
        SolveBarRequest {
            length: self.length,
            area: self.area,
            youngs_modulus: self.youngs_modulus,
            elements: self.elements,
            tip_force: self.tip_force,
        }
    }

    fn expected(self) -> ExpectedBarResponse {
        let tip_displacement = self.tip_force * self.length / (self.youngs_modulus * self.area);
        let stress = self.tip_force / self.area;
        let strain = stress / self.youngs_modulus;
        let strain_energy_density = 0.5 * stress * strain;
        ExpectedBarResponse {
            tip_displacement,
            stress,
            strain,
            strain_energy_density,
            axial_force: self.tip_force,
            reaction_force: -self.tip_force,
            total_strain_energy: 0.5 * self.tip_force * tip_displacement,
        }
    }
}

struct ExpectedBarResponse {
    tip_displacement: f64,
    stress: f64,
    strain: f64,
    strain_energy_density: f64,
    axial_force: f64,
    reaction_force: f64,
    total_strain_energy: f64,
}

fn assert_response(result: &kyuubiki_protocol::SolveBarResult, expected: ExpectedBarResponse) {
    assert_close(result.tip_displacement, expected.tip_displacement);
    assert_close(result.max_displacement, expected.tip_displacement.abs());
    assert_close(result.max_stress, expected.stress.abs());
    assert_close(result.reaction_force, expected.reaction_force);
    assert_close(result.total_strain_energy, expected.total_strain_energy);
    assert_energy_balance(result);
    assert_close(
        result.max_strain_energy_density,
        expected.strain_energy_density.abs(),
    );

    for element in &result.elements {
        assert_close(element.strain, expected.strain);
        assert_close(element.stress, expected.stress);
        assert_close(element.axial_force, expected.axial_force);
        assert_close(
            element.strain_energy_density,
            expected.strain_energy_density,
        );
    }
}

fn assert_energy_balance(result: &kyuubiki_protocol::SolveBarResult) {
    let total_from_elements = result
        .elements
        .iter()
        .map(|element| {
            element.strain_energy_density * result.input.area * (element.x2 - element.x1).abs()
        })
        .sum::<f64>();
    assert_close(result.total_strain_energy, total_from_elements);
    assert_close(
        result.max_strain_energy_density,
        result
            .elements
            .iter()
            .map(|element| element.strain_energy_density.abs())
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        result.total_strain_energy,
        0.5 * result.input.tip_force * result.tip_displacement,
    );
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
