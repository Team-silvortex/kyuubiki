use kyuubiki_protocol::{SolveTruss2dRequest, TrussElementInput, TrussNodeInput};
use kyuubiki_solver::solve_truss_2d;

const TOL: f64 = 1.0e-8;

#[test]
fn truss_2d_matches_symmetric_two_bar_closed_form() {
    let half_span = 0.5;
    let height = 0.75;
    let load = -1000.0;
    let area = 0.01;
    let youngs_modulus = 70.0e9;
    let result = solve_truss_2d(&symmetric_request(
        half_span,
        height,
        load,
        area,
        youngs_modulus,
    ))
    .expect("symmetric two-bar truss should solve");

    let length = (half_span * half_span + height * height).sqrt();
    let sin_theta = height / length;
    let axial_force = load / (2.0 * sin_theta);
    let expected_uy = load * length / (2.0 * youngs_modulus * area * sin_theta * sin_theta);
    let expected_stress = axial_force / area;
    let expected_strain = expected_stress / youngs_modulus;
    let expected_energy = 2.0 * 0.5 * expected_stress * expected_strain * area * length;

    assert_close(result.nodes[0].ux, 0.0, 1.0e-12);
    assert_close(result.nodes[0].uy, 0.0, 1.0e-12);
    assert_close(result.nodes[1].ux, 0.0, 1.0e-12);
    assert_close(result.nodes[1].uy, 0.0, 1.0e-12);
    assert_close(result.nodes[2].ux, 0.0, TOL);
    assert_close(result.nodes[2].uy, expected_uy, TOL);
    assert_close(result.max_displacement, expected_uy.abs(), TOL);
    assert_close(result.max_stress, expected_stress.abs(), TOL);
    assert_close(result.total_strain_energy, expected_energy, TOL);

    for element in &result.elements {
        assert_close(element.length, length, TOL);
        assert_close(element.axial_force, axial_force, TOL);
        assert_close(element.stress, expected_stress, TOL);
        assert_close(element.strain, expected_strain, TOL);
        assert_close(
            element.strain_energy_density,
            0.5 * element.stress * element.strain,
            TOL,
        );
    }
}

fn symmetric_request(
    half_span: f64,
    height: f64,
    load_y: f64,
    area: f64,
    youngs_modulus: f64,
) -> SolveTruss2dRequest {
    SolveTruss2dRequest {
        nodes: vec![
            node("left", -half_span, 0.0, true, true, 0.0),
            node("right", half_span, 0.0, true, true, 0.0),
            node("apex", 0.0, height, false, false, load_y),
        ],
        elements: vec![
            element("left-web", 0, 2, area, youngs_modulus),
            element("right-web", 1, 2, area, youngs_modulus),
        ],
    }
}

fn node(id: &str, x: f64, y: f64, fix_x: bool, fix_y: bool, load_y: f64) -> TrussNodeInput {
    TrussNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x: 0.0,
        load_y,
    }
}

fn element(
    id: &str,
    node_i: usize,
    node_j: usize,
    area: f64,
    youngs_modulus: f64,
) -> TrussElementInput {
    TrussElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area,
        youngs_modulus,
    }
}

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance * scale,
        "expected {actual} to be close to {expected}",
    );
}
