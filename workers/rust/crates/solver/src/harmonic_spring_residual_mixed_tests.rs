use super::*;
use serde_json::json;

fn mixed() -> (SolveHarmonicSpring1dRequest, Vec<Complex>) {
    let mut input = json!({
        "nodes":[{"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0}],
        "elements":[],"frequencies_hz":[1.0/std::f64::consts::TAU]
    });
    let mut values = vec![Complex::default()];
    for (index, power) in [0, 500, -20, -500, 50].into_iter().enumerate() {
        let replay = index % 2 == 0;
        let scale = 2.0_f64.powi(power);
        let mass = if replay {
            (1.0 + f64::EPSILON) * 2.0_f64.powi(-1000) * scale
        } else {
            scale
        };
        input["nodes"].as_array_mut().unwrap().push(json!({
            "id":format!("node-{index}"),"x":(index+1) as f64,
            "fix_x":false,"mass":mass,"load_x":if replay {0.5*scale} else {scale}
        }));
        input["elements"].as_array_mut().unwrap().push(json!({
            "id":format!("ground-{index}"),"node_i":0,"node_j":index+1,
            "stiffness":if replay {scale} else {2.0*scale},
            "damping":if replay {0.5*scale} else {scale}
        }));
        values.push(if replay {
            Complex { re: 0.4, im: -0.2 }
        } else {
            Complex { re: 0.5, im: -0.5 }
        });
    }
    (serde_json::from_value(input).unwrap(), values)
}

#[test]
fn harmonic_residual_mixed_rows_reject_each_ordinary_or_replayed_forgery_independently() {
    let (mut input, values) = mixed();
    let map = [None, Some(0), Some(1), Some(2), Some(3), Some(4)];
    for reversed in [false, true] {
        if reversed {
            input.elements.reverse();
            for element in &mut input.elements {
                std::mem::swap(&mut element.node_i, &mut element.node_j);
            }
        }
        validate_harmonic_equilibrium(&input, 1.0, &values, &map).unwrap();
        for node in 1..values.len() {
            let mut forged = values.clone();
            forged[node].re *= 2.0;
            let error = validate_harmonic_equilibrium(&input, 1.0, &forged, &map).unwrap_err();
            assert!(error.contains(&format!("free row {}", node - 1)), "{error}");
            assert!(error.contains("backward-error"), "{error}");
            assert_eq!(error.contains("enclosure"), node % 2 == 1, "{error}");
        }
        validate_harmonic_equilibrium(&input, 1.0, &values, &map).unwrap();
    }
}

#[test]
fn harmonic_residual_many_parallel_terms_certify_the_same_independent_response() {
    for power in [0, 10, 16] {
        let count = 1_usize << power;
        let stiffness = 2.0_f64.powi(-power);
        let mut input: SolveHarmonicSpring1dRequest = serde_json::from_value(json!({
            "nodes":[
                {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
                {"id":"tip","x":1.0,"fix_x":false,
                 "mass":(1.0+f64::EPSILON)*2.0_f64.powi(-1000),"load_x":0.5}
            ],"elements":[],"frequencies_hz":[1.0/std::f64::consts::TAU]
        }))
        .unwrap();
        let element = serde_json::from_value(json!({
            "id":"parallel","node_i":0,"node_j":1,
            "stiffness":stiffness,"damping":0.5*stiffness
        }))
        .unwrap();
        input.elements = vec![element; count];
        validate_harmonic_equilibrium(
            &input,
            1.0,
            &[Complex::default(), Complex { re: 0.4, im: -0.2 }],
            &[None, Some(0)],
        )
        .unwrap_or_else(|error| panic!("{count} parallel terms: {error}"));
        for wrong in [Complex { re: 0.8, im: -0.4 }, Complex { re: 0.4, im: 0.2 }] {
            let error = validate_harmonic_equilibrium(
                &input,
                1.0,
                &[Complex::default(), wrong],
                &[None, Some(0)],
            )
            .unwrap_err();
            assert!(
                error.contains("backward-error") && error.contains("free row 0"),
                "{error}"
            );
        }
    }
}
