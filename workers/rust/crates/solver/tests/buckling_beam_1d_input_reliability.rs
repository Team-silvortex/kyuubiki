use kyuubiki_protocol::{
    BucklingBeam1dElementInput, BucklingBeam1dNodeInput, SolveBucklingBeam1dRequest,
};
use kyuubiki_solver::solve_buckling_beam_1d;

#[test]
fn rejects_invalid_topology_and_non_positive_properties() {
    let mut request = valid_request();
    request.elements[0].node_j = 9;
    assert!(
        solve_buckling_beam_1d(&request)
            .unwrap_err()
            .contains("topology")
    );

    let mut request = valid_request();
    request.elements[0].moment_of_inertia = 0.0;
    assert!(
        solve_buckling_beam_1d(&request)
            .unwrap_err()
            .contains("moment_of_inertia")
    );

    let mut request = valid_request();
    request.elements[0].reference_compressive_force = -1.0;
    assert!(
        solve_buckling_beam_1d(&request)
            .unwrap_err()
            .contains("reference_compressive_force")
    );
}

#[test]
fn rejects_non_finite_geometry_and_underconstrained_models() {
    let mut request = valid_request();
    request.nodes[1].x = f64::NAN;
    assert!(
        solve_buckling_beam_1d(&request)
            .unwrap_err()
            .contains("finite")
    );

    let mut request = valid_request();
    request.nodes[1].fix_y = false;
    assert!(
        solve_buckling_beam_1d(&request)
            .unwrap_err()
            .contains("restrain at least two")
    );
}

#[test]
fn rejects_ambiguous_node_and_element_ids() {
    let mut request = valid_request();
    request.nodes[1].id = request.nodes[0].id.clone();
    assert!(
        solve_buckling_beam_1d(&request)
            .unwrap_err()
            .contains("node ids")
    );

    let mut request = valid_request();
    request.elements.push(request.elements[0].clone());
    assert!(
        solve_buckling_beam_1d(&request)
            .unwrap_err()
            .contains("element ids")
    );
}

fn valid_request() -> SolveBucklingBeam1dRequest {
    SolveBucklingBeam1dRequest {
        nodes: vec![node("left", 0.0, true), node("right", 2.0, true)],
        elements: vec![BucklingBeam1dElementInput {
            id: "column".to_string(),
            node_i: 0,
            node_j: 1,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            reference_compressive_force: 100_000.0,
        }],
        mode_count: Some(1),
    }
}

fn node(id: &str, x: f64, fix_y: bool) -> BucklingBeam1dNodeInput {
    BucklingBeam1dNodeInput {
        id: id.to_string(),
        x,
        fix_y,
        fix_rz: false,
    }
}
