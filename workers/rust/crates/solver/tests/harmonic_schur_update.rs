use kyuubiki_protocol::SolveHarmonicSpring1dRequest;
use kyuubiki_solver::{solve_harmonic_spring_1d, solve_harmonic_spring_1d_owned};
use serde_json::json;

fn model(w: f64) -> SolveHarmonicSpring1dRequest {
    let d = 2.0_f64.powi(-27);
    serde_json::from_value(json!({
        "nodes":[
            {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
            {"id":"middle","x":1.0,"fix_x":false,"mass":1.0,"load_x":0.0},
            {"id":"tip","x":2.0,"fix_x":false,"mass":1.0,"load_x":w}
        ],"elements":[
            {"id":"left","node_i":0,"node_j":1,"stiffness":d},
            {"id":"coupling","node_i":1,"node_j":2,"stiffness":1.0-d},
            {"id":"right","node_i":0,"node_j":2,"stiffness":w-d}
        ],"frequencies_hz":[0.0]
    }))
    .unwrap()
}

#[test]
fn harmonic_schur_update_public_three_spring_mode_matches_the_rational_stiffness_reference() {
    for (power, bits) in [
        (18, 0x3ff0000000010000),
        (20, 0x3ff0000000040000),
        (24, 0x3ff0000000400000),
    ] {
        for sign in [-1.0, 1.0] {
            for reversed in [false, true] {
                let mut input = model(2.0_f64.powi(-power));
                input.nodes[2].load_x *= sign;
                if reversed {
                    for e in &mut input.elements {
                        std::mem::swap(&mut e.node_i, &mut e.node_j);
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
                assert!(result.max_force.is_finite());
                assert_eq!(
                    serde_json::to_value(solve_harmonic_spring_1d_owned(input).unwrap()).unwrap(),
                    serde_json::to_value(result).unwrap()
                );
            }
        }
    }
}

#[test]
fn harmonic_schur_update_late_frequency_range_error_discards_the_corrected_mode_prefix() {
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
            / f64::from_bits(0x3ff0000000040000)
            - 1.0)
            .abs()
            <= 4.0 * f64::EPSILON
    );
}
