use kyuubiki_protocol::SolveHarmonicSpring1dRequest;
use kyuubiki_solver::{solve_harmonic_spring_1d, solve_harmonic_spring_1d_owned};
use serde_json::json;

fn model() -> SolveHarmonicSpring1dRequest {
    serde_json::from_value(json!({
        "nodes":[
            {"id":"root", "x":0.0, "fix_x":true, "mass":1.0, "load_x":0.0},
            {"id":"tip", "x":1.0, "fix_x":false, "mass":1.0,
             "load_x":f64::from_bits((1023_u64 << 52) | 4)}
        ],
        "elements":[{"id":"spring", "node_i":0, "node_j":1,
            "stiffness":f64::from_bits((2046_u64 << 52) | 1)}],
        "frequencies_hz":[0.0]
    }))
    .unwrap()
}

#[test]
fn harmonic_quotient_rounding_public_operator_publishes_the_nearest_subnormal_response() {
    let expected = (1_u64 << 51) + 1;
    for sign in [-1.0, 1.0] {
        for reversed in [false, true] {
            let mut input = model();
            input.nodes[1].load_x *= sign;
            if reversed {
                let element = &mut input.elements[0];
                std::mem::swap(&mut element.node_i, &mut element.node_j);
            }
            let result = solve_harmonic_spring_1d(&input).unwrap();
            assert_eq!(
                result.frequencies[0].nodes[1]
                    .displacement_amplitude
                    .to_bits(),
                expected
            );
            assert!((result.max_force / input.nodes[1].load_x.abs() - 1.0).abs() < 1e-14);
            let owned = solve_harmonic_spring_1d_owned(input).unwrap();
            assert_eq!(
                serde_json::to_value(owned).unwrap(),
                serde_json::to_value(result).unwrap()
            );
        }
    }
}

#[test]
fn harmonic_quotient_rounding_late_singular_frequency_discards_the_rounded_prefix() {
    let mut input = model();
    input.nodes[1].mass = input.elements[0].stiffness;
    input.frequencies_hz.push(1.0 / std::f64::consts::TAU);
    let error = solve_harmonic_spring_1d(&input).unwrap_err();
    assert!(
        error.contains("frequency 1") && error.contains("singular"),
        "{error}"
    );
    assert_eq!(
        solve_harmonic_spring_1d_owned(input.clone()).unwrap_err(),
        error
    );
    input.frequencies_hz.truncate(1);
    let result = solve_harmonic_spring_1d(&input).unwrap();
    assert_eq!(
        result.frequencies[0].nodes[1]
            .displacement_amplitude
            .to_bits(),
        (1_u64 << 51) + 1
    );
}
