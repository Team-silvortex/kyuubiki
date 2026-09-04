use kyuubiki_protocol::{
    SolveTransientSpring1dRequest, TransientSpring1dElementInput, TransientSpring1dNodeInput,
};
use kyuubiki_solver::solve_transient_spring_1d;

#[test]
fn transient_spring_1d_rejects_non_finite_time_step_and_node_state() {
    let mut request = transient_spring_request();
    request.time_step = f64::NAN;
    let error =
        solve_transient_spring_1d(&request).expect_err("non-finite time step should be rejected");
    assert!(
        error.contains("positive time_step"),
        "unexpected time-step error: {error}"
    );

    let mut request = transient_spring_request();
    request.nodes[1].initial_velocity = f64::INFINITY;
    let error = solve_transient_spring_1d(&request)
        .expect_err("non-finite initial velocity should be rejected");
    assert!(
        error.contains("finite coordinates, load, initial state, and positive mass"),
        "unexpected node-state error: {error}"
    );
}

#[test]
fn transient_spring_1d_rejects_invalid_element_connectivity_and_materials() {
    let mut request = transient_spring_request();
    request.elements[0].node_j = 9;
    let error = solve_transient_spring_1d(&request).expect_err("missing node should be rejected");
    assert!(
        error.contains("references missing node 9"),
        "unexpected missing-node error: {error}"
    );

    let mut request = transient_spring_request();
    request.elements[0].stiffness = f64::NAN;
    let error = solve_transient_spring_1d(&request).expect_err("NaN stiffness should be rejected");
    assert!(
        error.contains("valid connectivity, stiffness, and damping"),
        "unexpected stiffness error: {error}"
    );

    let mut request = transient_spring_request();
    request.elements[0].damping = -1.0;
    let error =
        solve_transient_spring_1d(&request).expect_err("negative damping should be rejected");
    assert!(
        error.contains("valid connectivity, stiffness, and damping"),
        "unexpected damping error: {error}"
    );

    let mut request = transient_spring_request();
    request.nodes[1].x = request.nodes[0].x;
    let error =
        solve_transient_spring_1d(&request).expect_err("zero-length element should be rejected");
    assert!(
        error.contains("length must be positive"),
        "unexpected zero-length error: {error}"
    );
}

#[test]
fn transient_spring_1d_rejects_non_finite_derived_dynamics() {
    let mut request = transient_spring_request();
    request.time_step = f64::MIN_POSITIVE;
    let error = solve_transient_spring_1d(&request)
        .expect_err("an overflowing Newmark coefficient should be rejected");
    assert!(
        error.contains("non-finite Newmark coefficients"),
        "unexpected Newmark coefficient error: {error}"
    );

    let mut request = transient_spring_request();
    request.nodes[1].mass = f64::MIN_POSITIVE;
    request.nodes[1].load_x = f64::MAX;
    let error = solve_transient_spring_1d(&request)
        .expect_err("an overflowing initial acceleration should be rejected");
    assert!(
        error.contains("initial acceleration became non-finite"),
        "unexpected acceleration error: {error}"
    );
}

#[test]
fn transient_spring_1d_handles_a_large_sparse_chain() {
    const NODE_COUNT: usize = 10_000;
    let nodes = (0..NODE_COUNT)
        .map(|index| {
            node(
                &format!("n{index}"),
                index as f64,
                index == 0,
                if index + 1 == NODE_COUNT { 1.0 } else { 0.0 },
                1.0,
                0.0,
                0.0,
            )
        })
        .collect();
    let elements = (0..NODE_COUNT - 1)
        .map(|index| TransientSpring1dElementInput {
            id: format!("e{index}"),
            node_i: index,
            node_j: index + 1,
            stiffness: 10.0,
            damping: 0.1,
        })
        .collect();
    let result = solve_transient_spring_1d(&SolveTransientSpring1dRequest {
        nodes,
        elements,
        time_step: 0.01,
        steps: 2,
    })
    .expect("large transient chain should stay on the sparse prepared path");

    assert_eq!(result.nodes.len(), NODE_COUNT);
    assert_eq!(result.history.len(), 3);
    assert!(result.nodes[NODE_COUNT - 1].ux.is_finite());
}

fn transient_spring_request() -> SolveTransientSpring1dRequest {
    SolveTransientSpring1dRequest {
        nodes: vec![
            node("fixed", 0.0, true, 0.0, 1.0, 0.0, 0.0),
            node("tip", 1.0, false, 10.0, 2.0, 0.0, 0.0),
        ],
        elements: vec![TransientSpring1dElementInput {
            id: "s0".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: 100.0,
            damping: 0.5,
        }],
        time_step: 0.01,
        steps: 10,
    }
}

fn node(
    id: &str,
    x: f64,
    fix_x: bool,
    load_x: f64,
    mass: f64,
    initial_displacement: f64,
    initial_velocity: f64,
) -> TransientSpring1dNodeInput {
    TransientSpring1dNodeInput {
        id: id.to_string(),
        x,
        fix_x,
        load_x,
        mass,
        initial_displacement,
        initial_velocity,
    }
}
