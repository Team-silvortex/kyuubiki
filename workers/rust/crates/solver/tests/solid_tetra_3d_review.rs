use kyuubiki_protocol::{
    SolidTetra3dElementInput, SolidTetra3dNodeInput, SolveSolidTetra3dRequest,
};
use kyuubiki_solver::solve_solid_tetra_3d;

const TOL: f64 = 1.0e-10;

#[test]
fn solid_tetra_3d_review_checks_restrained_tip_load_response() {
    let result = solve_solid_tetra_3d(&restrained_tip_load_request())
        .expect("restrained solid tetra review fixture should solve");

    assert_close(result.total_volume, 1.0 / 6.0);
    assert!(result.max_displacement > 0.0);
    assert!(result.max_von_mises_stress > 0.0);
    assert!(result.total_strain_energy > 0.0);
    assert!(result.max_strain_energy_density > 0.0);

    for index in 0..3 {
        assert_close(result.nodes[index].ux, 0.0);
        assert_close(result.nodes[index].uy, 0.0);
        assert_close(result.nodes[index].uz, 0.0);
    }

    let loaded = &result.nodes[3];
    assert!(loaded.uz < 0.0, "loaded node should move with the z load");
    assert!(loaded.displacement_magnitude > 0.0);
    assert!(loaded.displacement_magnitude.is_finite());

    let element = &result.elements[0];
    assert_close(element.volume, 1.0 / 6.0);
    assert!(
        element.strain_z < 0.0,
        "compression should produce negative z strain"
    );
    assert!(
        element.stress_z < 0.0,
        "compression should produce negative z stress"
    );
    assert!(element.von_mises_stress > 0.0);
    assert!(element.von_mises_stress.is_finite());
    assert_close(
        element.strain_energy_density,
        element_energy_density(element),
    );
    assert_close(
        result.total_strain_energy,
        element.strain_energy_density * element.volume,
    );
}

#[test]
fn solid_tetra_3d_review_rejects_invalid_geometry_and_non_finite_material() {
    let mut request = restrained_tip_load_request();
    request.nodes[3].z = 0.0;
    let error = solve_solid_tetra_3d(&request).expect_err("flat tetra should be rejected");
    assert!(
        error.contains("zero volume") || error.contains("singular"),
        "unexpected error: {error}"
    );

    let mut request = restrained_tip_load_request();
    request.elements[0].youngs_modulus = f64::NAN;
    let error =
        solve_solid_tetra_3d(&request).expect_err("non-finite Young's modulus should be rejected");
    assert!(
        error.contains("youngs_modulus must be finite"),
        "unexpected error: {error}"
    );
}

fn restrained_tip_load_request() -> SolveSolidTetra3dRequest {
    SolveSolidTetra3dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, 0.0, true, 0.0),
            node("n1", 1.0, 0.0, 0.0, true, 0.0),
            node("n2", 0.0, 1.0, 0.0, true, 0.0),
            node("n3", 0.0, 0.0, 1.0, false, -1000.0),
        ],
        elements: vec![SolidTetra3dElementInput {
            id: "t0".to_string(),
            node_a: 0,
            node_b: 1,
            node_c: 2,
            node_d: 3,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
        }],
    }
}

fn node(id: &str, x: f64, y: f64, z: f64, fixed: bool, load_z: f64) -> SolidTetra3dNodeInput {
    SolidTetra3dNodeInput {
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
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}

fn element_energy_density(element: &kyuubiki_protocol::SolidTetra3dElementResult) -> f64 {
    0.5 * ((element.stress_x * element.strain_x)
        + (element.stress_y * element.strain_y)
        + (element.stress_z * element.strain_z)
        + (element.shear_xy * element.gamma_xy)
        + (element.shear_yz * element.gamma_yz)
        + (element.shear_zx * element.gamma_zx))
}
