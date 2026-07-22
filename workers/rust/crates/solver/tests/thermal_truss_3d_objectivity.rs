use kyuubiki_protocol::{
    SolveThermalTruss3dRequest, ThermalTruss3dElementInput, ThermalTruss3dNodeInput,
};
use kyuubiki_solver::solve_thermal_truss_3d;

const REL_TOL: f64 = 1.0e-9;
const ABS_TOL: f64 = 1.0e-11;

#[test]
fn thermal_truss_3d_preserves_coupled_response_under_rigid_rotation() {
    let baseline_request = heated_loaded_tripod();
    let baseline =
        solve_thermal_truss_3d(&baseline_request).expect("baseline heated tripod should solve");

    for angle in [std::f64::consts::FRAC_PI_6, -std::f64::consts::FRAC_PI_4] {
        let rotated_request = rotate_request_about_y(baseline_request.clone(), angle);
        let rotated = solve_thermal_truss_3d(&rotated_request)
            .expect("rigidly rotated heated tripod should solve");
        let sine = angle.sin();
        let cosine = angle.cos();
        let baseline_apex = &baseline.nodes[3];
        let rotated_apex = &rotated.nodes[3];

        assert_close(
            rotated_apex.ux,
            cosine * baseline_apex.ux + sine * baseline_apex.uz,
            "rotated apex ux",
        );
        assert_close(rotated_apex.uy, baseline_apex.uy, "rotated apex uy");
        assert_close(
            rotated_apex.uz,
            -sine * baseline_apex.ux + cosine * baseline_apex.uz,
            "rotated apex uz",
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

fn heated_loaded_tripod() -> SolveThermalTruss3dRequest {
    let radius = 0.62;
    let height = 0.91;
    let root_three_over_two = 3.0_f64.sqrt() * 0.5;
    let area = 0.011;
    let youngs_modulus = 73.0e9;
    let thermal_expansion = 11.5e-6;

    SolveThermalTruss3dRequest {
        nodes: vec![
            node("base-a", radius, 0.0, 0.0, true, 18.0, 0.0),
            node(
                "base-b",
                -0.5 * radius,
                root_three_over_two * radius,
                0.0,
                true,
                24.0,
                0.0,
            ),
            node(
                "base-c",
                -0.5 * radius,
                -root_three_over_two * radius,
                0.0,
                true,
                30.0,
                0.0,
            ),
            node("apex", 0.0, 0.0, height, false, 52.0, -1450.0),
        ],
        elements: vec![
            element("leg-a", 0, 3, area, youngs_modulus, thermal_expansion),
            element("leg-b", 1, 3, area, youngs_modulus, thermal_expansion),
            element("leg-c", 2, 3, area, youngs_modulus, thermal_expansion),
        ],
    }
}

#[allow(clippy::too_many_arguments)]
fn node(
    id: &str,
    x: f64,
    y: f64,
    z: f64,
    fixed: bool,
    temperature_delta: f64,
    load_z: f64,
) -> ThermalTruss3dNodeInput {
    ThermalTruss3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        load_x: 0.0,
        load_y: 0.0,
        load_z,
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
) -> ThermalTruss3dElementInput {
    ThermalTruss3dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area,
        youngs_modulus,
        thermal_expansion,
    }
}

fn rotate_request_about_y(
    mut request: SolveThermalTruss3dRequest,
    angle: f64,
) -> SolveThermalTruss3dRequest {
    let sine = angle.sin();
    let cosine = angle.cos();
    for node in &mut request.nodes {
        (node.x, node.z) = (
            cosine * node.x + sine * node.z,
            -sine * node.x + cosine * node.z,
        );
        (node.load_x, node.load_z) = (
            cosine * node.load_x + sine * node.load_z,
            -sine * node.load_x + cosine * node.load_z,
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
