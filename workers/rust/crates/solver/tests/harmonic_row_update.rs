use kyuubiki_protocol::SolveHarmonicSpring1dRequest;
use kyuubiki_solver::{solve_harmonic_spring_1d, solve_harmonic_spring_1d_owned};
use serde_json::json;

fn model(weak: f64) -> SolveHarmonicSpring1dRequest {
    let delta = 2.0_f64.powi(-27);
    serde_json::from_value(json!({
        "nodes":[
            {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
            {"id":"middle","x":1.0,"fix_x":false,"mass":1.0,"load_x":2.0*(1.0-delta)},
            {"id":"tip","x":2.0,"fix_x":false,"mass":1.0,"load_x":-1.0+weak}
        ],
        "elements":[
            {"id":"support","node_i":0,"node_j":1,"stiffness":1.0-delta},
            {"id":"coupling","node_i":1,"node_j":2,"stiffness":1.0+delta}
        ],"frequencies_hz":[0.0]
    }))
    .unwrap()
}

#[test]
fn harmonic_row_update_public_two_spring_weak_response_matches_an_independent_closed_form() {
    // Rational references for 2*(weak-delta^2)/(1-delta^2), rounded once.
    for (power, bits) in [
        (18, 0x3edffffffffe0000),
        (20, 0x3ebffffffff80000),
        (24, 0x3e7fffffff800000),
    ] {
        for sign in [-1.0, 1.0] {
            for reversed in [false, true] {
                let mut input = model(2.0_f64.powi(-power));
                for node in &mut input.nodes {
                    node.load_x *= sign;
                }
                if reversed {
                    for element in &mut input.elements {
                        std::mem::swap(&mut element.node_i, &mut element.node_j);
                    }
                }
                let result = solve_harmonic_spring_1d(&input).unwrap();
                let expected = f64::from_bits(bits);
                assert!(
                    (result.frequencies[0].nodes[2].displacement_amplitude / expected - 1.0).abs()
                        <= 4.0 * f64::EPSILON,
                    "{:?} != {expected}",
                    result.frequencies[0].nodes[2]
                );
                assert_eq!(
                    serde_json::to_value(solve_harmonic_spring_1d_owned(input).unwrap()).unwrap(),
                    serde_json::to_value(result).unwrap()
                );
            }
        }
    }
}

#[test]
fn harmonic_row_update_late_frequency_failure_discards_the_corrected_weak_prefix() {
    let mut input = model(2.0_f64.powi(-20));
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
    assert!(
        (result.frequencies[0].nodes[2].displacement_amplitude
            / f64::from_bits(0x3ebffffffff80000)
            - 1.0)
            .abs()
            <= 4.0 * f64::EPSILON
    );
}
