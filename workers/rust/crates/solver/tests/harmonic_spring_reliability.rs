use kyuubiki_protocol::{
    SolveHarmonicSpring1dRequest, TransientSpring1dElementInput, TransientSpring1dNodeInput,
};
use kyuubiki_solver::solve_harmonic_spring_1d;

#[test]
fn harmonic_spring_1d_rejects_non_finite_frequency_and_node_state() {
    let mut request = harmonic_spring_request();
    request.frequencies_hz[0] = f64::NAN;
    let error =
        solve_harmonic_spring_1d(&request).expect_err("non-finite frequency should be rejected");
    assert!(
        error.contains("frequency 0 must be non-negative and finite"),
        "unexpected frequency error: {error}"
    );

    let mut request = harmonic_spring_request();
    request.nodes[1].initial_displacement = f64::INFINITY;
    let error =
        solve_harmonic_spring_1d(&request).expect_err("non-finite node state should be rejected");
    assert!(
        error.contains("finite coordinates, load, initial state, and positive mass"),
        "unexpected node-state error: {error}"
    );
}

#[test]
fn harmonic_spring_1d_rejects_invalid_element_and_degenerate_length() {
    let mut request = harmonic_spring_request();
    request.elements[0].node_j = 9;
    let error = solve_harmonic_spring_1d(&request).expect_err("missing node should be rejected");
    assert!(
        error.contains("references missing node 9"),
        "unexpected missing-node error: {error}"
    );

    let mut request = harmonic_spring_request();
    request.elements[0].damping = -1.0;
    let error =
        solve_harmonic_spring_1d(&request).expect_err("negative damping should be rejected");
    assert!(
        error.contains("valid connectivity, stiffness, and damping"),
        "unexpected damping error: {error}"
    );

    let mut request = harmonic_spring_request();
    request.nodes[1].x = request.nodes[0].x;
    let error =
        solve_harmonic_spring_1d(&request).expect_err("zero-length element should be rejected");
    assert!(
        error.contains("length must be positive"),
        "unexpected zero-length error: {error}"
    );
}

#[test]
fn harmonic_spring_1d_is_invariant_to_common_dynamic_scale() {
    let baseline = solve_harmonic_spring_1d(&harmonic_spring_request())
        .expect("baseline harmonic system should solve");
    let baseline_frequency = &baseline.frequencies[0];

    for factor in [1.0e-300, 1.0e300] {
        let mut request = harmonic_spring_request();
        for node in &mut request.nodes {
            node.mass *= factor;
            node.load_x *= factor;
        }
        for element in &mut request.elements {
            element.stiffness *= factor;
            element.damping *= factor;
        }
        let scaled = solve_harmonic_spring_1d(&request)
            .expect("commonly scaled harmonic system should remain solvable");
        let scaled_frequency = &scaled.frequencies[0];

        assert_relative(
            scaled_frequency.nodes[1].displacement_amplitude,
            baseline_frequency.nodes[1].displacement_amplitude,
        );
        assert_relative(
            scaled_frequency.nodes[1].velocity_amplitude,
            baseline_frequency.nodes[1].velocity_amplitude,
        );
        assert_relative(
            scaled_frequency.elements[0].force_amplitude,
            baseline_frequency.elements[0].force_amplitude * factor,
        );
    }
}

#[test]
fn harmonic_spring_1d_reports_an_unrestrained_static_mode() {
    let mut request = harmonic_spring_request();
    request.nodes[0].fix_x = false;
    request.frequencies_hz = vec![0.0];

    let error = solve_harmonic_spring_1d(&request)
        .expect_err("unrestrained zero-frequency system should be singular");
    assert!(error.contains("dynamic stiffness is singular"));
}

#[test]
fn harmonic_spring_1d_rejects_unsafe_dense_allocation_before_solving() {
    let node_count = 2_050;
    let request = SolveHarmonicSpring1dRequest {
        nodes: (0..node_count)
            .map(|index| {
                node(
                    &format!("n{index}"),
                    index as f64,
                    index == 0,
                    (index + 1 == node_count) as u8 as f64,
                    1.0,
                    0.0,
                    0.0,
                )
            })
            .collect(),
        elements: (0..node_count - 1)
            .map(|index| TransientSpring1dElementInput {
                id: format!("s{index}"),
                node_i: index,
                node_j: index + 1,
                stiffness: 1.0,
                damping: 0.0,
            })
            .collect(),
        frequencies_hz: vec![1.0],
    };

    let error = solve_harmonic_spring_1d(&request)
        .expect_err("oversized dense harmonic system should be rejected before allocation");
    assert!(error.contains("2049 free degrees of freedom"));
    assert!(error.contains("supports at most 2048"));
}

fn harmonic_spring_request() -> SolveHarmonicSpring1dRequest {
    SolveHarmonicSpring1dRequest {
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
        frequencies_hz: vec![2.0],
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

fn assert_relative(actual: f64, expected: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(f64::MIN_POSITIVE);
    assert!(
        relative < 1.0e-10,
        "expected {actual:.16e} to match {expected:.16e} relatively, error={relative:.6e}"
    );
}
