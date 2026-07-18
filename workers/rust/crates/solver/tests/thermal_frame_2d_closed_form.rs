use kyuubiki_protocol::{
    SolveThermalFrame2dRequest, SolveThermalFrame2dResult, ThermalFrame2dElementInput,
    ThermalFrame2dNodeInput,
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

    for (index, node) in result.nodes.iter().enumerate() {
        assert_eq!(node.index, index);
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
    assert_member_energy_balance(&result.elements, result.total_strain_energy);
    assert_thermal_frame_summary(&result);
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
    assert_member_energy_balance(&baseline.elements, baseline.total_strain_energy);
    assert_thermal_frame_summary(&baseline);

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
    assert_member_energy_balance(&hotter.elements, hotter.total_strain_energy);
    assert_thermal_frame_summary(&hotter);

    let expansion_scale = 1.3;
    let expanded = solve_thermal_frame_2d(&restrained_member(
        length,
        area,
        youngs_modulus,
        thermal_expansion * expansion_scale,
        temperature_delta,
    ))
    .expect("thermal-expansion-scaled thermal frame 2d should solve");
    assert_close(
        expanded.elements[0].thermal_strain / baseline.elements[0].thermal_strain,
        expansion_scale,
    );
    assert_close(
        expanded.elements[0].axial_force_i / baseline.elements[0].axial_force_i,
        expansion_scale,
    );
    assert_close(
        expanded.elements[0].axial_stress / baseline.elements[0].axial_stress,
        expansion_scale,
    );
    assert_close(
        expanded.total_strain_energy / baseline.total_strain_energy,
        expansion_scale * expansion_scale,
    );
    assert_member_energy_balance(&expanded.elements, expanded.total_strain_energy);
    assert_thermal_frame_summary(&expanded);

    let area_scale = 1.7;
    let wider = solve_thermal_frame_2d(&restrained_member(
        length,
        area * area_scale,
        youngs_modulus,
        thermal_expansion,
        temperature_delta,
    ))
    .expect("area-scaled thermal frame 2d should solve");
    assert_close(
        wider.elements[0].axial_stress,
        baseline.elements[0].axial_stress,
    );
    assert_close(
        wider.elements[0].axial_force_i / baseline.elements[0].axial_force_i,
        area_scale,
    );
    assert_close(
        wider.total_strain_energy / baseline.total_strain_energy,
        area_scale,
    );
    assert_member_energy_balance(&wider.elements, wider.total_strain_energy);
    assert_thermal_frame_summary(&wider);

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
    assert_member_energy_balance(&stiffer.elements, stiffer.total_strain_energy);
    assert_thermal_frame_summary(&stiffer);

    let length_scale = 1.45;
    let longer = solve_thermal_frame_2d(&restrained_member(
        length * length_scale,
        area,
        youngs_modulus,
        thermal_expansion,
        temperature_delta,
    ))
    .expect("length-scaled thermal frame 2d should solve");
    assert_close(
        longer.elements[0].thermal_strain,
        baseline.elements[0].thermal_strain,
    );
    assert_close(
        longer.elements[0].axial_stress,
        baseline.elements[0].axial_stress,
    );
    assert_close(
        longer.elements[0].axial_force_i,
        baseline.elements[0].axial_force_i,
    );
    assert_close(
        longer.total_strain_energy / baseline.total_strain_energy,
        length_scale,
    );
    assert_member_energy_balance(&longer.elements, longer.total_strain_energy);
    assert_thermal_frame_summary(&longer);
}

fn assert_thermal_frame_summary(result: &SolveThermalFrame2dResult) {
    let mut max_moment = 0.0_f64;
    let mut max_stress = 0.0_f64;
    let mut max_axial_force = 0.0_f64;
    let mut max_temperature_gradient = 0.0_f64;
    let mut total_strain_energy = 0.0_f64;
    assert_eq!(result.nodes.len(), result.input.nodes.len());
    assert_eq!(result.elements.len(), result.input.elements.len());
    for (index, node) in result.nodes.iter().enumerate() {
        let input = &result.input.nodes[index];
        assert_eq!(node.index, index);
        assert_eq!(node.id, input.id);
        assert_close(node.x, input.x);
        assert_close(node.y, input.y);
        assert_close(node.temperature_delta, input.temperature_delta);
        assert_close(
            node.displacement_magnitude,
            (node.ux * node.ux + node.uy * node.uy).sqrt(),
        );
    }

    for (index, element) in result.elements.iter().enumerate() {
        let input = &result.input.elements[index];
        assert_eq!(element.index, index);
        let average_temperature_delta = (result.nodes[element.node_i].temperature_delta
            + result.nodes[element.node_j].temperature_delta)
            / 2.0;
        assert_close(element.average_temperature_delta, average_temperature_delta);
        assert_close(
            element.thermal_strain,
            input.thermal_expansion * average_temperature_delta,
        );
        assert_close(
            element.mechanical_strain,
            element.total_strain - element.thermal_strain,
        );
        assert_close(
            element.thermal_curvature,
            input.thermal_expansion * element.temperature_gradient_y / input.section_depth,
        );
        assert_close(
            element.axial_stress,
            -input.youngs_modulus * element.mechanical_strain,
        );
        assert_close(element.axial_force_i, element.axial_stress * input.area);
        assert_close(element.axial_force_j, -element.axial_force_i);
        assert_close(
            element.max_combined_stress,
            element.axial_stress.abs() + element.max_bending_stress.abs(),
        );
        max_moment = max_moment.max(element.moment_i.abs());
        max_moment = max_moment.max(element.moment_j.abs());
        max_stress = max_stress.max(element.max_combined_stress.abs());
        max_axial_force = max_axial_force.max(element.axial_force_i.abs());
        max_axial_force = max_axial_force.max(element.axial_force_j.abs());
        max_temperature_gradient =
            max_temperature_gradient.max(element.temperature_gradient_y.abs());
        total_strain_energy += element.strain_energy;
    }

    assert_close(
        result.max_displacement,
        result
            .nodes
            .iter()
            .map(|node| node.displacement_magnitude)
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        result.max_rotation,
        result
            .nodes
            .iter()
            .map(|node| node.rz.abs())
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        result.max_temperature_delta,
        result
            .nodes
            .iter()
            .map(|node| node.temperature_delta.abs())
            .fold(0.0_f64, f64::max),
    );
    assert_close(result.max_moment, max_moment);
    assert_close(result.max_stress, max_stress);
    assert_close(result.max_axial_force, max_axial_force);
    assert_close(result.max_temperature_gradient, max_temperature_gradient);
    assert_close(result.total_strain_energy, total_strain_energy);
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

fn assert_member_energy_balance(
    elements: &[kyuubiki_protocol::ThermalFrame2dElementResult],
    total_strain_energy: f64,
) {
    assert_close(
        total_strain_energy,
        elements.iter().map(|element| element.strain_energy).sum(),
    );
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
