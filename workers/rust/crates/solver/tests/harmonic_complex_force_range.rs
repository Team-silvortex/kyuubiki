use kyuubiki_protocol::SolveHarmonicSpring1dRequest;
use kyuubiki_solver::{solve_harmonic_spring_1d, solve_harmonic_spring_1d_owned};
use serde_json::json;
use std::f64::consts::TAU;

fn quantum_force_model() -> SolveHarmonicSpring1dRequest {
    serde_json::from_value(json!({
        "nodes":[
            {"id":"root", "x":0.0, "fix_x":true, "mass":1.0, "load_x":0.0},
            {"id":"tip", "x":1.0, "fix_x":false, "mass":2.0_f64.powi(-700), "load_x":f64::from_bits(1)}
        ],
        "elements":[{"id":"spring", "node_i":0, "node_j":1, "stiffness":2.0_f64.powi(-600), "damping":2.0_f64.powi(-600)}],
        "frequencies_hz":[1.0/TAU]
    })).unwrap()
}

#[test]
fn harmonic_complex_force_range_publishes_a_nonzero_quantum_force() {
    for sign in [-1.0, 1.0] {
        for reversed in [false, true] {
            let mut input = quantum_force_model();
            input.nodes[1].load_x *= sign;
            if reversed {
                input.elements[0].node_i = 1;
                input.elements[0].node_j = 0;
            }
            let result = solve_harmonic_spring_1d(&input).unwrap();
            assert_eq!(result.max_force, f64::from_bits(1));
            assert_eq!(
                result.frequencies[0].elements[0].force_amplitude,
                f64::from_bits(1)
            );
            let phase = if sign > 0.0 { -45.0 } else { 135.0 };
            assert!((result.frequencies[0].nodes[1].displacement_phase_deg - phase).abs() < 1e-12);
            assert_eq!(
                serde_json::to_value(solve_harmonic_spring_1d_owned(input).unwrap()).unwrap(),
                serde_json::to_value(result).unwrap()
            );
        }
    }
}

#[test]
fn harmonic_complex_force_range_rejects_unrepresentable_per_element_forces() {
    let mut input = quantum_force_model();
    input.elements = (0..4)
        .map(|index| {
            let mut element = input.elements[0].clone();
            element.id = format!("parallel{index}");
            element
        })
        .collect();
    let error = solve_harmonic_spring_1d(&input).unwrap_err();
    assert!(
        error.contains("element 0 force") && error.contains("numerical range"),
        "{error}"
    );
    assert_eq!(
        solve_harmonic_spring_1d(&quantum_force_model())
            .unwrap()
            .max_force,
        f64::from_bits(1)
    );
}

#[test]
fn harmonic_complex_force_range_reports_unrepresentable_extension_separately() {
    let input: SolveHarmonicSpring1dRequest = serde_json::from_value(json!({
        "nodes":[
            {"id":"root", "x":0.0, "fix_x":true, "mass":1.0, "load_x":0.0},
            {"id":"left", "x":1.0, "fix_x":false, "mass":1.0, "load_x":4.0},
            {"id":"right", "x":2.0, "fix_x":false, "mass":1.0, "load_x":-4.0}
        ], "elements":[
            {"id":"left-ground", "node_i":0, "node_j":1, "stiffness":2e-308},
            {"id":"right-ground", "node_i":0, "node_j":2, "stiffness":2e-308},
            {"id":"link", "node_i":1, "node_j":2, "stiffness":1e-308}
        ], "frequencies_hz":[0.0]
    }))
    .unwrap();
    let error = solve_harmonic_spring_1d(&input).unwrap_err();
    assert!(
        error.contains("element 2 extension amplitude") && error.contains("finite numeric range"),
        "{error}"
    );
}
