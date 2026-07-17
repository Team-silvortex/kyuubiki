use kyuubiki_protocol::{
    SolveThermalFrame2dRequest, ThermalFrame2dElementInput, ThermalFrame2dNodeInput,
};
use kyuubiki_solver::solve_thermal_frame_2d;

const TOL: f64 = 1.0e-10;

#[test]
fn thermal_frame_2d_matches_restrained_uniform_temperature_closed_form() {
    let length = 1.8;
    let area = 0.018;
    let youngs_modulus = 210.0e9;
    let thermal_expansion = 12.0e-6;
    let temperature_delta = 32.0;
    let result = solve_thermal_frame_2d(&restrained_member(
        length,
        area,
        youngs_modulus,
        thermal_expansion,
        temperature_delta,
    ))
    .expect("restrained thermal frame 2d should solve");

    let expected_thermal_strain = thermal_expansion * temperature_delta;
    let expected_mechanical_strain = -expected_thermal_strain;
    let expected_axial_force = youngs_modulus * area * expected_thermal_strain;
    let expected_axial_stress = youngs_modulus * expected_thermal_strain;
    let expected_energy = 0.5 * youngs_modulus * area * expected_thermal_strain.powi(2) * length;

    for node in &result.nodes {
        assert_close(node.ux, 0.0);
        assert_close(node.uy, 0.0);
        assert_close(node.rz, 0.0);
        assert_close(node.displacement_magnitude, 0.0);
        assert_close(node.temperature_delta, temperature_delta);
    }

    let element = &result.elements[0];
    assert_close(element.length, length);
    assert_close(element.average_temperature_delta, temperature_delta);
    assert_close(element.thermal_strain, expected_thermal_strain);
    assert_close(element.mechanical_strain, expected_mechanical_strain);
    assert_close(element.total_strain, 0.0);
    assert_close(element.temperature_gradient_y, 0.0);
    assert_close(element.thermal_curvature, 0.0);
    assert_close(element.axial_force_i, expected_axial_force);
    assert_close(element.axial_force_j, -expected_axial_force);
    assert_close(element.shear_force_i, 0.0);
    assert_close(element.shear_force_j, 0.0);
    assert_close(element.moment_i, 0.0);
    assert_close(element.moment_j, 0.0);
    assert_close(element.axial_stress, expected_axial_stress);
    assert_close(element.max_bending_stress, 0.0);
    assert_close(element.max_combined_stress, expected_axial_stress);
    assert_close(element.strain_energy, expected_energy);

    assert_close(result.max_displacement, 0.0);
    assert_close(result.max_rotation, 0.0);
    assert_close(result.max_moment, 0.0);
    assert_close(result.max_axial_force, expected_axial_force);
    assert_close(result.max_temperature_delta, temperature_delta);
    assert_close(result.max_temperature_gradient, 0.0);
    assert_close(result.total_strain_energy, expected_energy);
}

#[test]
fn thermal_frame_2d_tracks_temperature_area_and_modulus_scaling() {
    let length = 1.7;
    let area = 0.016;
    let youngs_modulus = 205.0e9;
    let thermal_expansion = 11.0e-6;
    let temperature_delta = 30.0;
    let baseline = solve_thermal_frame_2d(&restrained_member(
        length,
        area,
        youngs_modulus,
        thermal_expansion,
        temperature_delta,
    ))
    .expect("baseline restrained thermal frame 2d should solve");

    let temperature_scale = 1.5;
    let hotter = solve_thermal_frame_2d(&restrained_member(
        length,
        area,
        youngs_modulus,
        thermal_expansion,
        temperature_delta * temperature_scale,
    ))
    .expect("temperature-scaled thermal frame 2d should solve");
    assert_close(
        hotter.elements[0].thermal_strain / baseline.elements[0].thermal_strain,
        temperature_scale,
    );
    assert_close(
        hotter.elements[0].axial_force_i / baseline.elements[0].axial_force_i,
        temperature_scale,
    );
    assert_close(
        hotter.elements[0].axial_stress / baseline.elements[0].axial_stress,
        temperature_scale,
    );
    assert_close(
        hotter.total_strain_energy / baseline.total_strain_energy,
        temperature_scale * temperature_scale,
    );

    let area_scale = 1.7;
    let wider = solve_thermal_frame_2d(&restrained_member(
        length,
        area * area_scale,
        youngs_modulus,
        thermal_expansion,
        temperature_delta,
    ))
    .expect("area-scaled thermal frame 2d should solve");
    assert_close(wider.elements[0].axial_stress, baseline.elements[0].axial_stress);
    assert_close(
        wider.elements[0].axial_force_i / baseline.elements[0].axial_force_i,
        area_scale,
    );
    assert_close(
        wider.total_strain_energy / baseline.total_strain_energy,
        area_scale,
    );

    let modulus_scale = 1.25;
    let stiffer = solve_thermal_frame_2d(&restrained_member(
        length,
        area,
        youngs_modulus * modulus_scale,
        thermal_expansion,
        temperature_delta,
    ))
    .expect("modulus-scaled thermal frame 2d should solve");
    assert_close(
        stiffer.elements[0].thermal_strain,
        baseline.elements[0].thermal_strain,
    );
    assert_close(
        stiffer.elements[0].axial_force_i / baseline.elements[0].axial_force_i,
        modulus_scale,
    );
    assert_close(
        stiffer.elements[0].axial_stress / baseline.elements[0].axial_stress,
        modulus_scale,
    );
    assert_close(
        stiffer.total_strain_energy / baseline.total_strain_energy,
        modulus_scale,
    );
}

fn restrained_member(
    length: f64,
    area: f64,
    youngs_modulus: f64,
    thermal_expansion: f64,
    temperature_delta: f64,
) -> SolveThermalFrame2dRequest {
    SolveThermalFrame2dRequest {
        nodes: vec![
            node("left", 0.0, 0.0, temperature_delta),
            node("right", length, 0.0, temperature_delta),
        ],
        elements: vec![ThermalFrame2dElementInput {
            id: "member".to_string(),
            node_i: 0,
            node_j: 1,
            area,
            youngs_modulus,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
            thermal_expansion,
            section_depth: 0.3,
            temperature_gradient_y: 0.0,
        }],
    }
}

fn node(id: &str, x: f64, y: f64, temperature_delta: f64) -> ThermalFrame2dNodeInput {
    ThermalFrame2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x: true,
        fix_y: true,
        fix_rz: true,
        load_x: 0.0,
        load_y: 0.0,
        moment_z: 0.0,
        temperature_delta,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
