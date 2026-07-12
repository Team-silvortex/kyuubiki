use kyuubiki_protocol::{
    HeatBar1dNodeInput, SolveTransientHeatBar1dRequest, TransientHeatBar1dElementInput,
};
use kyuubiki_solver::solve_transient_heat_bar_1d;

#[test]
fn transient_heat_bar_1d_rejects_non_finite_time_step_and_node_values() {
    let mut request = transient_heat_request();
    request.time_step = f64::NAN;
    let error =
        solve_transient_heat_bar_1d(&request).expect_err("non-finite time step should be rejected");
    assert!(
        error.contains("time_step must be positive"),
        "unexpected time-step error: {error}"
    );

    let mut request = transient_heat_request();
    request.nodes[1].heat_load = f64::INFINITY;
    let error =
        solve_transient_heat_bar_1d(&request).expect_err("non-finite heat load should be rejected");
    assert!(
        error.contains("heat_load must be finite"),
        "unexpected heat-load error: {error}"
    );
}

#[test]
fn transient_heat_bar_1d_rejects_invalid_element_geometry_and_materials() {
    let mut request = transient_heat_request();
    request.nodes[1].x = request.nodes[0].x;
    let error = solve_transient_heat_bar_1d(&request)
        .expect_err("zero-length transient heat element should be rejected");
    assert!(
        error.contains("non-zero length"),
        "unexpected zero-length error: {error}"
    );

    let mut request = transient_heat_request();
    request.elements[0].conductivity = f64::NAN;
    let error =
        solve_transient_heat_bar_1d(&request).expect_err("NaN conductivity should be rejected");
    assert!(
        error.contains("positive area and conductivity"),
        "unexpected conductivity error: {error}"
    );

    let mut request = transient_heat_request();
    request.elements[0].specific_heat = 0.0;
    let error =
        solve_transient_heat_bar_1d(&request).expect_err("zero specific heat should be rejected");
    assert!(
        error.contains("positive density and specific_heat"),
        "unexpected specific heat error: {error}"
    );
}

#[test]
fn transient_heat_bar_1d_rejects_missing_node_and_unheated_capacity_island() {
    let mut request = transient_heat_request();
    request.elements[0].node_j = 99;
    let error = solve_transient_heat_bar_1d(&request).expect_err("missing node should be rejected");
    assert!(
        error.contains("references missing node 99"),
        "unexpected missing-node error: {error}"
    );

    let mut request = transient_heat_request();
    request.elements.remove(1);
    let error =
        solve_transient_heat_bar_1d(&request).expect_err("capacity island should be rejected");
    assert!(
        error.contains("every node must receive positive heat capacity"),
        "unexpected capacity island error: {error}"
    );
}

fn transient_heat_request() -> SolveTransientHeatBar1dRequest {
    SolveTransientHeatBar1dRequest {
        nodes: vec![
            node("hot", 0.0, true, 100.0, 0.0),
            node("mid", 0.5, false, 20.0, 1.0),
            node("cold", 1.0, true, 0.0, 0.0),
        ],
        elements: vec![
            element("e0", 0, 1, 0.01, 45.0, 7800.0, 500.0),
            element("e1", 1, 2, 0.01, 45.0, 7800.0, 500.0),
        ],
        time_step: 0.1,
        steps: 4,
    }
}

fn node(
    id: &str,
    x: f64,
    fix_temperature: bool,
    temperature: f64,
    heat_load: f64,
) -> HeatBar1dNodeInput {
    HeatBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_temperature,
        temperature,
        heat_load,
    }
}

fn element(
    id: &str,
    node_i: usize,
    node_j: usize,
    area: f64,
    conductivity: f64,
    density: f64,
    specific_heat: f64,
) -> TransientHeatBar1dElementInput {
    TransientHeatBar1dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area,
        conductivity,
        density,
        specific_heat,
    }
}
