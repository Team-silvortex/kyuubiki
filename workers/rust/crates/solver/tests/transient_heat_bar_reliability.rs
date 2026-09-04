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
fn transient_heat_bar_1d_samples_history_and_rejects_zero_stride() {
    let mut request = transient_heat_request();
    request.steps = 5;
    request.history_stride = Some(2);
    let result = solve_transient_heat_bar_1d(&request).expect("sampled history should solve");
    let recorded_steps = result
        .history
        .iter()
        .map(|frame| frame.step)
        .collect::<Vec<_>>();
    assert_eq!(recorded_steps, vec![0, 2, 4, 5]);

    request.history_stride = Some(0);
    let error = solve_transient_heat_bar_1d(&request).expect_err("zero stride should fail");
    assert!(error.contains("history_stride must be positive"));
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

#[test]
fn transient_heat_bar_1d_rejects_non_finite_derived_coefficients() {
    let mut request = transient_heat_request();
    request.time_step = f64::MIN_POSITIVE;
    let error = solve_transient_heat_bar_1d(&request)
        .expect_err("an overflowing capacity rate should be rejected");
    assert!(
        error.contains("non-finite capacity rate"),
        "unexpected capacity-rate error: {error}"
    );

    let mut request = transient_heat_request();
    request.elements[0].density = f64::MAX;
    request.elements[0].specific_heat = f64::MAX;
    let error = solve_transient_heat_bar_1d(&request)
        .expect_err("an overflowing element capacity should be rejected");
    assert!(
        error.contains("finite positive capacity and conductance"),
        "unexpected derived-coefficient error: {error}"
    );
}

#[test]
fn transient_heat_bar_1d_handles_a_large_prepared_chain() {
    const NODE_COUNT: usize = 10_000;
    let path = (0..NODE_COUNT)
        .step_by(2)
        .chain((1..NODE_COUNT).step_by(2))
        .collect::<Vec<_>>();
    let mut positions = vec![0_usize; NODE_COUNT];
    for (position, &node_index) in path.iter().enumerate() {
        positions[node_index] = position;
    }
    let nodes = (0..NODE_COUNT)
        .map(|index| {
            let position = positions[index];
            let fixed = position == 0 || position == NODE_COUNT / 2 || position + 1 == NODE_COUNT;
            let temperature = if position == 0 {
                100.0
            } else if position == NODE_COUNT / 2 {
                60.0
            } else {
                20.0
            };
            node(
                &format!("n{index}"),
                position as f64,
                fixed,
                temperature,
                0.0,
            )
        })
        .collect();
    let elements = path
        .windows(2)
        .enumerate()
        .map(|(index, edge)| {
            element(
                &format!("e{index}"),
                edge[0],
                edge[1],
                0.01,
                45.0,
                7800.0,
                500.0,
            )
        })
        .collect();
    let result = solve_transient_heat_bar_1d(&SolveTransientHeatBar1dRequest {
        nodes,
        elements,
        time_step: 0.1,
        steps: 2,
        history_stride: None,
    })
    .expect("large transient heat chain should reuse its prepared system");

    assert_eq!(result.nodes.len(), NODE_COUNT);
    assert_eq!(result.history.len(), 3);
    assert_eq!(result.nodes[path[0]].temperature, 100.0);
    assert_eq!(result.nodes[path[NODE_COUNT / 2]].temperature, 60.0);
    assert_eq!(result.nodes[*path.last().unwrap()].temperature, 20.0);
    assert!(result.nodes[NODE_COUNT / 2].temperature.is_finite());
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
        history_stride: None,
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
