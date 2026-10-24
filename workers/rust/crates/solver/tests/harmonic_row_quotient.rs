use kyuubiki_protocol::SolveHarmonicSpring1dRequest;
use kyuubiki_solver::{solve_harmonic_spring_1d, solve_harmonic_spring_1d_owned};
use serde_json::json;

fn model(power: i32, scale: f64, sign: f64, reversed: bool) -> SolveHarmonicSpring1dRequest {
    let weak = 2.0_f64.powi(-power);
    serde_json::from_value(json!({
        "nodes":[
            {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
            {"id":"middle","x":1.0,"fix_x":false,"mass":1.0,"load_x":sign*scale*(-1.0+weak)},
            {"id":"tip","x":2.0,"fix_x":false,"mass":1.0,"load_x":sign*scale}
        ], "elements":[
            {"id":"left","node_i":usize::from(reversed),"node_j":usize::from(!reversed),"stiffness":scale},
            {"id":"right","node_i":if reversed {2}else{1},"node_j":if reversed {1}else{2},"stiffness":scale}
        ], "frequencies_hz":[0.0]
    })).unwrap()
}

#[test]
fn harmonic_row_quotient_public_balanced_load_replay_preserves_analytical_nodes_and_forces() {
    for power in [30, 40, 48] {
        for scale_power in [-500, 0, 500] {
            let scale = 2.0_f64.powi(scale_power);
            for sign in [-1.0, 1.0] {
                for reversed in [false, true] {
                    let input = model(power, scale, sign, reversed);
                    let actual = solve_harmonic_spring_1d(&input).unwrap();
                    let weak = 2.0_f64.powi(-power);
                    let nodes = &actual.frequencies[0].nodes;
                    assert_eq!(nodes[1].displacement_amplitude.to_bits(), weak.to_bits());
                    assert_eq!(
                        nodes[2].displacement_amplitude.to_bits(),
                        (1.0 + weak).to_bits()
                    );
                    assert_eq!(
                        actual.frequencies[0].elements[0].force_amplitude.to_bits(),
                        (scale * weak).to_bits()
                    );
                    assert_eq!(
                        actual.frequencies[0].elements[1].force_amplitude.to_bits(),
                        scale.to_bits()
                    );
                    assert_eq!(
                        serde_json::to_value(solve_harmonic_spring_1d_owned(input).unwrap())
                            .unwrap(),
                        serde_json::to_value(actual).unwrap()
                    );
                }
            }
        }
    }
}

#[test]
fn harmonic_row_quotient_late_frequency_failure_cannot_return_the_balanced_prefix() {
    let mut input = model(40, 1.0, 1.0, false);
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
    assert_eq!(
        solve_harmonic_spring_1d(&input).unwrap().frequencies[0].nodes[1].displacement_amplitude,
        2.0_f64.powi(-40)
    );
}
