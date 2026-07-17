use kyuubiki_protocol::{
    SolveThermalBeam1dRequest, ThermalBeam1dElementInput, ThermalBeam1dNodeInput,
};
use kyuubiki_solver::solve_thermal_beam_1d;

const TOL: f64 = 1.0e-10;

#[test]
fn thermal_beam_1d_matches_free_curvature_closed_form() {
    let length = 2.4;
    let thermal_expansion = 12.0e-6;
    let temperature_gradient_y = 45.0;
    let section_depth = 0.3;
    let result = solve_thermal_beam_1d(&request(
        length,
        thermal_expansion,
        temperature_gradient_y,
        section_depth,
    ))
    .expect("thermal beam closed-form fixture should solve");

    let curvature = thermal_expansion * temperature_gradient_y / section_depth;
    let expected_tip_rotation = curvature * length;
    let expected_tip_displacement = 0.5 * curvature * length * length;
    assert_close(result.nodes[0].uy, 0.0);
    assert_close(result.nodes[0].rz, 0.0);
    assert_close(result.nodes[1].uy, expected_tip_displacement);
    assert_close(result.nodes[1].rz, expected_tip_rotation);
    assert_close(result.max_displacement, expected_tip_displacement);
    assert_close(result.max_rotation, expected_tip_rotation);
    assert_close(result.max_temperature_gradient, temperature_gradient_y);
    assert!(result.max_moment < 1.0e-8);
    assert!(result.max_stress < 1.0e-5);
    assert!(result.total_strain_energy.abs() < 1.0e-12);

    let element = &result.elements[0];
    assert_close(element.length, length);
    assert_close(element.temperature_gradient_y, temperature_gradient_y);
    assert_close(element.thermal_curvature, curvature);
    assert!(element.shear_force_i.abs() < 1.0e-12);
    assert!(element.shear_force_j.abs() < 1.0e-12);
    assert!(element.moment_i.abs() < 1.0e-8);
    assert!(element.moment_j.abs() < 1.0e-8);
}

#[test]
fn thermal_beam_1d_reports_zero_response_for_zero_gradient() {
    let result = solve_thermal_beam_1d(&request(1.75, 10.0e-6, 0.0, 0.25))
        .expect("zero-gradient thermal beam fixture should solve");

    assert_close(result.max_displacement, 0.0);
    assert_close(result.max_rotation, 0.0);
    assert_close(result.max_moment, 0.0);
    assert_close(result.max_stress, 0.0);
    assert_close(result.max_temperature_gradient, 0.0);
    assert_close(result.total_strain_energy, 0.0);
    for node in &result.nodes {
        assert_close(node.uy, 0.0);
        assert_close(node.rz, 0.0);
    }
    let element = &result.elements[0];
    assert_close(element.thermal_curvature, 0.0);
    assert_close(element.shear_force_i, 0.0);
    assert_close(element.shear_force_j, 0.0);
    assert_close(element.moment_i, 0.0);
    assert_close(element.moment_j, 0.0);
}

#[test]
fn thermal_beam_1d_tracks_gradient_and_section_depth_scaling() {
    let length = 2.1;
    let thermal_expansion = 11.0e-6;
    let gradient = 36.0;
    let section_depth = 0.28;
    let baseline =
        solve_thermal_beam_1d(&request(length, thermal_expansion, gradient, section_depth))
            .expect("baseline free-curvature thermal beam should solve");

    let gradient_scale = 1.6;
    let hotter = solve_thermal_beam_1d(&request(
        length,
        thermal_expansion,
        gradient * gradient_scale,
        section_depth,
    ))
    .expect("gradient-scaled thermal beam should solve");
    assert_close(
        hotter.elements[0].thermal_curvature / baseline.elements[0].thermal_curvature,
        gradient_scale,
    );
    assert_close(hotter.nodes[1].rz / baseline.nodes[1].rz, gradient_scale);
    assert_close(hotter.nodes[1].uy / baseline.nodes[1].uy, gradient_scale);
    assert!(hotter.max_moment < 1.0e-8);
    assert!(hotter.total_strain_energy.abs() < 1.0e-12);

    let expansion_scale = 1.25;
    let expanded = solve_thermal_beam_1d(&request(
        length,
        thermal_expansion * expansion_scale,
        gradient,
        section_depth,
    ))
    .expect("thermal-expansion-scaled thermal beam should solve");
    assert_close(
        expanded.elements[0].thermal_curvature / baseline.elements[0].thermal_curvature,
        expansion_scale,
    );
    assert_close(expanded.nodes[1].rz / baseline.nodes[1].rz, expansion_scale);
    assert_close(expanded.nodes[1].uy / baseline.nodes[1].uy, expansion_scale);
    assert!(expanded.max_moment < 1.0e-8);
    assert!(expanded.total_strain_energy.abs() < 1.0e-12);

    let depth_scale = 1.4;
    let deeper = solve_thermal_beam_1d(&request(
        length,
        thermal_expansion,
        gradient,
        section_depth * depth_scale,
    ))
    .expect("depth-scaled thermal beam should solve");
    assert_close(
        deeper.elements[0].thermal_curvature / baseline.elements[0].thermal_curvature,
        1.0 / depth_scale,
    );
    assert_close(deeper.nodes[1].rz / baseline.nodes[1].rz, 1.0 / depth_scale);
    assert_close(deeper.nodes[1].uy / baseline.nodes[1].uy, 1.0 / depth_scale);
    assert!(deeper.max_moment < 1.0e-8);
    assert!(deeper.total_strain_energy.abs() < 1.0e-12);
}

fn request(
    length: f64,
    thermal_expansion: f64,
    temperature_gradient_y: f64,
    section_depth: f64,
) -> SolveThermalBeam1dRequest {
    SolveThermalBeam1dRequest {
        nodes: vec![
            node("fixed", 0.0, true, true),
            node("free", length, false, false),
        ],
        elements: vec![ThermalBeam1dElementInput {
            id: "thermal-beam".to_string(),
            node_i: 0,
            node_j: 1,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 0.00012,
            section_modulus: 0.0011,
            thermal_expansion,
            section_depth,
            distributed_load_y: 0.0,
            temperature_gradient_y,
        }],
    }
}

fn node(id: &str, x: f64, fix_y: bool, fix_rz: bool) -> ThermalBeam1dNodeInput {
    ThermalBeam1dNodeInput {
        id: id.to_string(),
        x,
        fix_y,
        fix_rz,
        load_y: 0.0,
        moment_z: 0.0,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
