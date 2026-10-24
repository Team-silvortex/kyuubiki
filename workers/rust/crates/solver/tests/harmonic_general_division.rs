use kyuubiki_protocol::SolveHarmonicSpring1dRequest;
use kyuubiki_solver::{solve_harmonic_spring_1d, solve_harmonic_spring_1d_owned};
use serde_json::json;

fn model() -> SolveHarmonicSpring1dRequest {
    serde_json::from_value(json!({
        "nodes":[
            {"id":"root", "x":0.0, "fix_x":true, "mass":1.0, "load_x":0.0},
            {"id":"tip", "x":1.0, "fix_x":false, "mass":1.0, "load_x":1.0}
        ],
        "elements":[{"id":"spring", "node_i":0, "node_j":1,
            "stiffness":f64::from_bits((2046_u64 << 52) | 2),
            "damping":f64::from_bits((2046_u64 << 52) | 7)}],
        "frequencies_hz":[1.0/std::f64::consts::TAU]
    }))
    .unwrap()
}

#[test]
fn harmonic_general_division_public_damped_response_keeps_the_quantized_amplitude() {
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
                1_592_262_918_131_441
            );
            assert!((result.max_force - 1.0).abs() < 1e-14);
            assert_eq!(
                serde_json::to_value(solve_harmonic_spring_1d_owned(input).unwrap()).unwrap(),
                serde_json::to_value(result).unwrap()
            );
        }
    }
}

#[test]
fn harmonic_general_division_late_frequency_range_error_discards_the_successful_prefix() {
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
    assert_eq!(result.max_displacement.to_bits(), 1_592_262_918_131_441);
}
