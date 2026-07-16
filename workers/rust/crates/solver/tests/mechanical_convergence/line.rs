use super::common::assert_close;
use kyuubiki_protocol::SolveBarRequest;
use kyuubiki_solver::solve_bar_1d;

#[test]
fn axial_bar_refinement_is_closed_form_invariant() {
    let base = SolveBarRequest {
        length: 3.5,
        area: 0.018,
        youngs_modulus: 205.0e9,
        elements: 1,
        tip_force: 3_250.0,
    };
    let expected_tip = base.tip_force * base.length / (base.youngs_modulus * base.area);
    let expected_stress = base.tip_force / base.area;
    let expected_energy = 0.5 * base.tip_force * expected_tip;

    for elements in [1_usize, 2, 4, 8, 16, 32] {
        let request = SolveBarRequest { elements, ..base };
        let result = solve_bar_1d(&request).expect("refined axial bar should solve");

        assert_close(result.tip_displacement, expected_tip, "tip displacement");
        assert_close(result.max_displacement, expected_tip, "max displacement");
        assert_close(result.max_stress, expected_stress, "max stress");
        assert_close(result.reaction_force, -base.tip_force, "reaction force");
        assert_close(result.total_strain_energy, expected_energy, "strain energy");

        let expected_element_length = base.length / elements as f64;
        for element in &result.elements {
            assert_close(
                element.x2 - element.x1,
                expected_element_length,
                "element length",
            );
            assert_close(element.stress, expected_stress, "element stress");
            assert_close(element.axial_force, base.tip_force, "element axial force");
        }
    }
}

#[test]
fn axial_bar_linear_perturbations_preserve_expected_scaling() {
    let base = SolveBarRequest {
        length: 2.0,
        area: 0.012,
        youngs_modulus: 190.0e9,
        elements: 12,
        tip_force: 2_400.0,
    };
    let baseline = solve_bar_1d(&base).expect("baseline bar should solve");

    let doubled_force = solve_bar_1d(&SolveBarRequest {
        tip_force: base.tip_force * 2.0,
        ..base
    })
    .expect("force perturbation should solve");
    assert_close(
        doubled_force.tip_displacement / baseline.tip_displacement,
        2.0,
        "force to displacement scaling",
    );
    assert_close(
        doubled_force.max_stress / baseline.max_stress,
        2.0,
        "force to stress scaling",
    );
    assert_close(
        doubled_force.total_strain_energy / baseline.total_strain_energy,
        4.0,
        "force to energy scaling",
    );

    let doubled_area = solve_bar_1d(&SolveBarRequest {
        area: base.area * 2.0,
        ..base
    })
    .expect("area perturbation should solve");
    assert_close(
        doubled_area.tip_displacement / baseline.tip_displacement,
        0.5,
        "area to displacement scaling",
    );
    assert_close(
        doubled_area.max_stress / baseline.max_stress,
        0.5,
        "area to stress scaling",
    );
    assert_close(
        doubled_area.total_strain_energy / baseline.total_strain_energy,
        0.5,
        "area to energy scaling",
    );
}
