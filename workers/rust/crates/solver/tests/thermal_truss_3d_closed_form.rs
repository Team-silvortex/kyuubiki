use kyuubiki_protocol::{
    SolveThermalTruss3dRequest, ThermalTruss3dElementInput, ThermalTruss3dElementResult,
    ThermalTruss3dNodeInput,
};
use kyuubiki_solver::solve_thermal_truss_3d;

const TOL: f64 = 1.0e-10;

#[test]
fn thermal_truss_3d_matches_restrained_uniform_temperature_closed_form() {
    let area = 0.012;
    let youngs_modulus = 70.0e9;
    let thermal_expansion = 11.0e-6;
    let temperature_delta = 45.0;
    let result = solve_thermal_truss_3d(&restrained_triangle(
        area,
        youngs_modulus,
        thermal_expansion,
        temperature_delta,
    ))
    .expect("restrained thermal truss should solve");

    let expected_thermal_strain = thermal_expansion * temperature_delta;
    let expected_mechanical_strain = -expected_thermal_strain;
    let expected_stress = youngs_modulus * expected_mechanical_strain;
    let expected_axial_force = expected_stress * area;
    let expected_energy_density = 0.5 * expected_stress * expected_mechanical_strain;

    for node in &result.nodes {
        assert_close(node.ux, 0.0);
        assert_close(node.uy, 0.0);
        assert_close(node.uz, 0.0);
        assert_close(node.temperature_delta, temperature_delta);
    }

    for element in &result.elements {
        assert_close(element.average_temperature_delta, temperature_delta);
        assert_close(element.thermal_strain, expected_thermal_strain);
        assert_close(element.total_strain, 0.0);
        assert_close(element.mechanical_strain, expected_mechanical_strain);
        assert_close(element.stress, expected_stress);
        assert_close(element.axial_force, expected_axial_force);
        assert_close(element.strain_energy_density, expected_energy_density);
    }

    assert_close(result.max_displacement, 0.0);
    assert_close(result.max_stress, expected_stress.abs());
    assert_close(result.max_axial_force, expected_axial_force.abs());
    assert_close(result.max_temperature_delta, temperature_delta);
    assert_close(result.max_strain_energy_density, expected_energy_density);
    assert_close(
        result.total_strain_energy,
        total_strain_energy(&result.elements, area),
    );
}

#[test]
fn thermal_truss_3d_tracks_temperature_and_area_scaling() {
    let area = 0.013;
    let youngs_modulus = 72.0e9;
    let thermal_expansion = 10.0e-6;
    let temperature_delta = 42.0;
    let baseline = solve_thermal_truss_3d(&restrained_triangle(
        area,
        youngs_modulus,
        thermal_expansion,
        temperature_delta,
    ))
    .expect("baseline restrained thermal truss 3d should solve");

    let temperature_scale = 1.4;
    let hotter = solve_thermal_truss_3d(&restrained_triangle(
        area,
        youngs_modulus,
        thermal_expansion,
        temperature_delta * temperature_scale,
    ))
    .expect("temperature-scaled thermal truss 3d should solve");
    assert_close(
        hotter.elements[0].thermal_strain / baseline.elements[0].thermal_strain,
        temperature_scale,
    );
    assert_close(
        hotter.elements[0].stress / baseline.elements[0].stress,
        temperature_scale,
    );
    assert_close(
        hotter.elements[0].axial_force / baseline.elements[0].axial_force,
        temperature_scale,
    );
    assert_close(
        hotter.total_strain_energy / baseline.total_strain_energy,
        temperature_scale * temperature_scale,
    );

    let expansion_scale = 1.25;
    let expanded = solve_thermal_truss_3d(&restrained_triangle(
        area,
        youngs_modulus,
        thermal_expansion * expansion_scale,
        temperature_delta,
    ))
    .expect("thermal-expansion-scaled thermal truss 3d should solve");
    assert_close(
        expanded.elements[0].thermal_strain / baseline.elements[0].thermal_strain,
        expansion_scale,
    );
    assert_close(
        expanded.elements[0].stress / baseline.elements[0].stress,
        expansion_scale,
    );
    assert_close(
        expanded.elements[0].axial_force / baseline.elements[0].axial_force,
        expansion_scale,
    );
    assert_close(
        expanded.total_strain_energy / baseline.total_strain_energy,
        expansion_scale * expansion_scale,
    );

    let modulus_scale = 1.3;
    let stiffer = solve_thermal_truss_3d(&restrained_triangle(
        area,
        youngs_modulus * modulus_scale,
        thermal_expansion,
        temperature_delta,
    ))
    .expect("modulus-scaled thermal truss 3d should solve");
    assert_close(
        stiffer.elements[0].thermal_strain,
        baseline.elements[0].thermal_strain,
    );
    assert_close(
        stiffer.elements[0].stress / baseline.elements[0].stress,
        modulus_scale,
    );
    assert_close(
        stiffer.elements[0].axial_force / baseline.elements[0].axial_force,
        modulus_scale,
    );
    assert_close(
        stiffer.total_strain_energy / baseline.total_strain_energy,
        modulus_scale,
    );

    let area_scale = 1.7;
    let larger = solve_thermal_truss_3d(&restrained_triangle(
        area * area_scale,
        youngs_modulus,
        thermal_expansion,
        temperature_delta,
    ))
    .expect("area-scaled thermal truss 3d should solve");
    assert_close(larger.elements[0].stress, baseline.elements[0].stress);
    assert_close(
        larger.elements[0].strain_energy_density,
        baseline.elements[0].strain_energy_density,
    );
    assert_close(
        larger.elements[0].axial_force / baseline.elements[0].axial_force,
        area_scale,
    );
    assert_close(
        larger.total_strain_energy / baseline.total_strain_energy,
        area_scale,
    );

    let geometry_scale = 1.5;
    let longer = solve_thermal_truss_3d(&restrained_triangle_scaled(
        geometry_scale,
        area,
        youngs_modulus,
        thermal_expansion,
        temperature_delta,
    ))
    .expect("geometry-scaled thermal truss 3d should solve");
    assert_close(
        longer.elements[0].length / baseline.elements[0].length,
        geometry_scale,
    );
    assert_close(
        longer.elements[0].thermal_strain,
        baseline.elements[0].thermal_strain,
    );
    assert_close(longer.elements[0].stress, baseline.elements[0].stress);
    assert_close(
        longer.elements[0].strain_energy_density,
        baseline.elements[0].strain_energy_density,
    );
    assert_close(
        longer.elements[0].axial_force,
        baseline.elements[0].axial_force,
    );
    assert_close(
        longer.total_strain_energy / baseline.total_strain_energy,
        geometry_scale,
    );
}

fn restrained_triangle(
    area: f64,
    youngs_modulus: f64,
    thermal_expansion: f64,
    temperature_delta: f64,
) -> SolveThermalTruss3dRequest {
    restrained_triangle_scaled(
        1.0,
        area,
        youngs_modulus,
        thermal_expansion,
        temperature_delta,
    )
}

fn restrained_triangle_scaled(
    geometry_scale: f64,
    area: f64,
    youngs_modulus: f64,
    thermal_expansion: f64,
    temperature_delta: f64,
) -> SolveThermalTruss3dRequest {
    SolveThermalTruss3dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, 0.0, temperature_delta),
            node("n1", 1.0 * geometry_scale, 0.0, 0.0, temperature_delta),
            node(
                "n2",
                0.0,
                1.0 * geometry_scale,
                1.0 * geometry_scale,
                temperature_delta,
            ),
        ],
        elements: vec![
            element("edge-01", 0, 1, area, youngs_modulus, thermal_expansion),
            element("edge-12", 1, 2, area, youngs_modulus, thermal_expansion),
            element("edge-20", 2, 0, area, youngs_modulus, thermal_expansion),
        ],
    }
}

fn node(id: &str, x: f64, y: f64, z: f64, temperature_delta: f64) -> ThermalTruss3dNodeInput {
    ThermalTruss3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x: true,
        fix_y: true,
        fix_z: true,
        load_x: 0.0,
        load_y: 0.0,
        load_z: 0.0,
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

fn total_strain_energy(elements: &[ThermalTruss3dElementResult], area: f64) -> f64 {
    elements
        .iter()
        .map(|element| element.strain_energy_density * area * element.length)
        .sum()
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
