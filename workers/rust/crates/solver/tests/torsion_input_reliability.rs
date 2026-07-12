use kyuubiki_protocol::{SolveTorsion1dRequest, Torsion1dElementInput, Torsion1dNodeInput};
use kyuubiki_solver::solve_torsion_1d;

#[test]
fn torsion_1d_rejects_missing_support_and_non_finite_node_inputs() {
    let mut request = torsion_request();
    for node in &mut request.nodes {
        node.fix_rz = false;
    }
    let error = solve_torsion_1d(&request).expect_err("missing support should be rejected");
    assert!(
        error.contains("rotational support"),
        "unexpected support error: {error}"
    );

    let mut request = torsion_request();
    request.nodes[1].torque_z = f64::NAN;
    let error = solve_torsion_1d(&request).expect_err("NaN torque should be rejected");
    assert!(
        error.contains("invalid torque"),
        "unexpected torque error: {error}"
    );
}

#[test]
fn torsion_1d_rejects_invalid_element_topology_and_properties() {
    let mut request = torsion_request();
    request.elements[0].node_j = 9;
    let error = solve_torsion_1d(&request).expect_err("out-of-range node should be rejected");
    assert!(
        error.contains("out-of-range node"),
        "unexpected topology error: {error}"
    );

    let mut request = torsion_request();
    request.nodes[1].x = request.nodes[0].x;
    let error = solve_torsion_1d(&request).expect_err("zero-length element should be rejected");
    assert!(
        error.contains("length must be positive"),
        "unexpected length error: {error}"
    );

    let mut request = torsion_request();
    request.elements[0].section_modulus = 0.0;
    let error = solve_torsion_1d(&request).expect_err("zero section modulus should be rejected");
    assert!(
        error.contains("section_modulus must be positive"),
        "unexpected section modulus error: {error}"
    );
}

fn torsion_request() -> SolveTorsion1dRequest {
    SolveTorsion1dRequest {
        nodes: vec![
            node("fixed", 0.0, true, 0.0),
            node("tip", 1.5, false, 2500.0),
        ],
        elements: vec![Torsion1dElementInput {
            id: "shaft".to_string(),
            node_i: 0,
            node_j: 1,
            shear_modulus: 79.0e9,
            polar_moment: 1.8e-6,
            section_modulus: 1.2e-4,
        }],
    }
}

fn node(id: &str, x: f64, fix_rz: bool, torque_z: f64) -> Torsion1dNodeInput {
    Torsion1dNodeInput {
        id: id.to_string(),
        x,
        fix_rz,
        torque_z,
    }
}
