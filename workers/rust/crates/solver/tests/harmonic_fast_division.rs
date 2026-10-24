use kyuubiki_protocol::SolveHarmonicSpring1dRequest;
use kyuubiki_solver::{solve_harmonic_spring_1d, solve_harmonic_spring_1d_owned};
use serde_json::json;

fn model() -> SolveHarmonicSpring1dRequest {
    serde_json::from_value(json!({
        "nodes":[
            {"id":"root", "x":0.0, "fix_x":true, "mass":1.0, "load_x":0.0},
            {"id":"tip", "x":1.0, "fix_x":false, "mass":1.0, "load_x":1.0}
        ],
        "elements":[{"id":"spring", "node_i":0, "node_j":1, "stiffness":3.0, "damping":3.0}],
        "frequencies_hz":[1.0/std::f64::consts::TAU]
    }))
    .unwrap()
}

#[test]
fn harmonic_fast_division_public_normal_damped_response_matches_rational_components() {
    for sign in [-1.0, 1.0] {
        for reversed in [false, true] {
            let mut input = model();
            input.nodes[1].load_x *= sign;
            if reversed {
                let spring = &mut input.elements[0];
                std::mem::swap(&mut spring.node_i, &mut spring.node_j);
            }
            let result = solve_harmonic_spring_1d(&input).unwrap();
            let expected = (2.0_f64 / 13.0).hypot(3.0 / 13.0);
            assert_eq!(result.max_displacement.to_bits(), expected.to_bits());
            assert!((result.max_force - 15.0_f64.hypot(3.0) / 13.0).abs() < 1e-14);
            assert_eq!(
                serde_json::to_value(solve_harmonic_spring_1d_owned(input).unwrap()).unwrap(),
                serde_json::to_value(result).unwrap()
            );
        }
    }
}

#[test]
fn harmonic_fast_division_late_frequency_range_failure_discards_the_normal_prefix() {
    let mut input = model();
    input.frequencies_hz.push(1e200);
    let error = solve_harmonic_spring_1d(&input).unwrap_err();
    assert!(
        error.contains("frequency 1") && error.contains("range"),
        "{error}"
    );
    assert_eq!(
        solve_harmonic_spring_1d_owned(input.clone()).unwrap_err(),
        error
    );
    input.frequencies_hz.pop();
    let result = solve_harmonic_spring_1d(&input).unwrap();
    assert_eq!(
        result.max_displacement.to_bits(),
        (2.0_f64 / 13.0).hypot(3.0 / 13.0).to_bits()
    );
}
