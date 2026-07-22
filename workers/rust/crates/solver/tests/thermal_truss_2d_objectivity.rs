use kyuubiki_protocol::{
    SolveThermalTruss2dRequest, ThermalTruss2dElementInput, ThermalTruss2dNodeInput,
};
use kyuubiki_solver::solve_thermal_truss_2d;

const REL_TOL: f64 = 1.0e-9;
const ABS_TOL: f64 = 1.0e-11;

#[test]
fn thermal_truss_2d_preserves_coupled_response_under_rigid_rotation() {
    let baseline_request = heated_loaded_triangle();
    let baseline =
        solve_thermal_truss_2d(&baseline_request).expect("baseline heated triangle should solve");

    for angle in [std::f64::consts::FRAC_PI_6, -std::f64::consts::FRAC_PI_4] {
        let rotated_request = rotate_request(baseline_request.clone(), angle);
        let rotated = solve_thermal_truss_2d(&rotated_request)
            .expect("rigidly rotated heated triangle should solve");
        let sine = angle.sin();
        let cosine = angle.cos();
        let baseline_apex = &baseline.nodes[2];
        let rotated_apex = &rotated.nodes[2];

        assert_close(
            rotated_apex.ux,
            cosine * baseline_apex.ux - sine * baseline_apex.uy,
            "rotated apex ux",
        );
        assert_close(
            rotated_apex.uy,
            sine * baseline_apex.ux + cosine * baseline_apex.uy,
            "rotated apex uy",
        );
        assert_close(
            rotated.max_displacement,
            baseline.max_displacement,
            "rotation-invariant max displacement",
        );
        assert_close(
            rotated.max_stress,
            baseline.max_stress,
            "rotation-invariant max stress",
        );
        assert_close(
            rotated.max_axial_force,
            baseline.max_axial_force,
            "rotation-invariant max axial force",
        );
        assert_close(
            rotated.max_temperature_delta,
            baseline.max_temperature_delta,
            "rotation-invariant max temperature",
        );
        assert_close(
            rotated.max_strain_energy_density,
            baseline.max_strain_energy_density,
            "rotation-invariant energy density",
        );
        assert_close(
            rotated.total_strain_energy,
            baseline.total_strain_energy,
            "rotation-invariant total energy",
        );

        for (baseline_element, rotated_element) in baseline.elements.iter().zip(&rotated.elements) {
            assert_eq!(rotated_element.id, baseline_element.id);
            assert_close(
                rotated_element.length,
                baseline_element.length,
                "rotation-invariant member length",
            );
            assert_close(
                rotated_element.average_temperature_delta,
                baseline_element.average_temperature_delta,
                "rotation-invariant average temperature",
            );
            assert_close(
                rotated_element.thermal_strain,
                baseline_element.thermal_strain,
                "rotation-invariant thermal strain",
            );
            assert_close(
                rotated_element.mechanical_strain,
                baseline_element.mechanical_strain,
                "rotation-invariant mechanical strain",
            );
            assert_close(
                rotated_element.total_strain,
                baseline_element.total_strain,
                "rotation-invariant total strain",
            );
            assert_close(
                rotated_element.stress,
                baseline_element.stress,
                "rotation-invariant member stress",
            );
            assert_close(
                rotated_element.axial_force,
                baseline_element.axial_force,
                "rotation-invariant member force",
            );
            assert_close(
                rotated_element.strain_energy_density,
                baseline_element.strain_energy_density,
                "rotation-invariant member energy density",
            );
        }
    }
}

fn heated_loaded_triangle() -> SolveThermalTruss2dRequest {
    let area = 0.012;
    let youngs_modulus = 69.0e9;
    let thermal_expansion = 10.8e-6;

    SolveThermalTruss2dRequest {
        nodes: vec![
            node("base-left", -0.58, 0.0, true, 20.0, 0.0),
            node("base-right", 0.58, 0.0, true, 28.0, 0.0),
            node("apex", 0.0, 0.86, false, 51.0, -1320.0),
        ],
        elements: vec![
            element("left-leg", 0, 2, area, youngs_modulus, thermal_expansion),
            element("right-leg", 1, 2, area, youngs_modulus, thermal_expansion),
        ],
    }
}

fn node(
    id: &str,
    x: f64,
    y: f64,
    fixed: bool,
    temperature_delta: f64,
    load_y: f64,
) -> ThermalTruss2dNodeInput {
    ThermalTruss2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x: fixed,
        fix_y: fixed,
        load_x: 0.0,
        load_y,
        temperature_delta,
    }
}

fn element(
    id: &str,
    node_i: usize,
    node_j: usize,
    area: f64,
    youngs_modulus: f64,
    thermal_expansion: f64,
) -> ThermalTruss2dElementInput {
    ThermalTruss2dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area,
        youngs_modulus,
        thermal_expansion,
    }
}

fn rotate_request(
    mut request: SolveThermalTruss2dRequest,
    angle: f64,
) -> SolveThermalTruss2dRequest {
    let sine = angle.sin();
    let cosine = angle.cos();
    for node in &mut request.nodes {
        (node.x, node.y) = (
            cosine * node.x - sine * node.y,
            sine * node.x + cosine * node.y,
        );
        (node.load_x, node.load_y) = (
            cosine * node.load_x - sine * node.load_y,
            sine * node.load_x + cosine * node.load_y,
        );
    }
    request
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    let tolerance = ABS_TOL.max(REL_TOL * expected.abs().max(1.0));
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {actual} to be close to {expected} within {tolerance}",
    );
}
