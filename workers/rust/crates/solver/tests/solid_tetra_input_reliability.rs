use kyuubiki_protocol::{
    SolidTetra3dElementInput, SolidTetra3dNodeInput, SolveSolidTetra3dRequest,
};
use kyuubiki_solver::solve_solid_tetra_3d;

#[test]
fn solid_tetra_3d_rejects_non_finite_node_coordinates_and_loads() {
    let mut request = tetra_request();
    request.nodes[3].z = f64::NAN;
    let error = solve_solid_tetra_3d(&request).expect_err("NaN coordinate should be rejected");
    assert!(
        error.contains("coordinates must be finite"),
        "unexpected coordinate error: {error}"
    );

    let mut request = tetra_request();
    request.nodes[3].load_z = f64::INFINITY;
    let error = solve_solid_tetra_3d(&request).expect_err("infinite load should be rejected");
    assert!(
        error.contains("loads must be finite"),
        "unexpected load error: {error}"
    );
}

#[test]
fn solid_tetra_3d_rejects_invalid_topology_and_material_bounds() {
    let mut request = tetra_request();
    request.elements[0].node_d = 9;
    let error = solve_solid_tetra_3d(&request).expect_err("missing node should be rejected");
    assert!(
        error.contains("references missing node 9"),
        "unexpected missing-node error: {error}"
    );

    let mut request = tetra_request();
    request.elements[0].node_d = request.elements[0].node_a;
    let error = solve_solid_tetra_3d(&request).expect_err("duplicate node should be rejected");
    assert!(
        error.contains("four distinct nodes"),
        "unexpected duplicate-node error: {error}"
    );

    let mut request = tetra_request();
    request.elements[0].poisson_ratio = 0.5;
    let error = solve_solid_tetra_3d(&request).expect_err("invalid poisson ratio should fail");
    assert!(
        error.contains("poisson_ratio in (-1, 0.5)"),
        "unexpected poisson ratio error: {error}"
    );

    let mut request = tetra_request();
    request.elements[0].poisson_ratio = f64::NAN;
    let error = solve_solid_tetra_3d(&request).expect_err("NaN poisson ratio should fail");
    assert!(
        error.contains("poisson_ratio must be finite"),
        "unexpected poisson finite error: {error}"
    );
}

#[test]
fn solid_tetra_3d_rejects_zero_volume_and_invalid_stiffness() {
    let mut request = tetra_request();
    request.nodes[3].z = 0.0;
    let error = solve_solid_tetra_3d(&request).expect_err("zero-volume tetra should be rejected");
    assert!(
        error.contains("zero volume"),
        "unexpected zero-volume error: {error}"
    );

    let mut request = tetra_request();
    request.elements[0].youngs_modulus = 0.0;
    let error =
        solve_solid_tetra_3d(&request).expect_err("zero Young's modulus should be rejected");
    assert!(
        error.contains("positive youngs_modulus"),
        "unexpected Young's modulus error: {error}"
    );
}

fn tetra_request() -> SolveSolidTetra3dRequest {
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
