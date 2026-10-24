use super::*;
use serde_json::json;

fn model(sign: f64) -> SolveHarmonicSpring1dRequest {
    serde_json::from_value(json!({
        "nodes":[
            {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
            {"id":"tip","x":1.0,"fix_x":false,"mass":(1.0+f64::EPSILON)*2.0_f64.powi(-1000),"load_x":sign*0.5}
        ],"elements":[
            {"id":"spring","node_i":0,"node_j":1,"stiffness":1.0,"damping":0.5}
        ],"frequencies_hz":[1.0/std::f64::consts::TAU]
    })).unwrap()
}

#[test]
fn harmonic_residual_enclosure_accepts_the_independent_response_despite_tiny_alignment_loss() {
    for sign in [-1.0, 1.0] {
        let input = model(sign);
        let values = [
            Complex::default(),
            Complex {
                re: sign * 0.4,
                im: -sign * 0.2,
            },
        ];
        validate_harmonic_equilibrium(&input, 1.0, &values, &[None, Some(0)]).unwrap();
    }
}

#[test]
fn harmonic_residual_enclosure_rejects_forged_amplitude_phase_support_and_reduction_state() {
    for sign in [-1.0, 1.0] {
        let input = model(sign);
        for wrong in [
            Complex {
                re: sign * 0.8,
                im: -sign * 0.4,
            },
            Complex {
                re: sign * 0.4,
                im: sign * 0.2,
            },
        ] {
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
        let good = Complex {
            re: sign * 0.4,
            im: -sign * 0.2,
        };
        assert!(
            validate_harmonic_equilibrium(
                &input,
                1.0,
                &[Complex::real(1.0), good],
                &[None, Some(0)]
            )
            .unwrap_err()
            .contains("support")
        );
        assert!(
            validate_harmonic_equilibrium(
                &input,
                1.0,
                &[Complex::default(), good],
                &[None, Some(1)]
            )
            .unwrap_err()
            .contains("reduction map")
        );
        validate_harmonic_equilibrium(&input, 1.0, &[Complex::default(), good], &[None, Some(0)])
            .unwrap();
    }
}

#[test]
fn harmonic_residual_enclosure_ordinary_rows_keep_the_existing_storage_and_no_replay_path() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    use std::{cell::Cell, rc::Rc};
    for (power, expected) in [(-900, 1), (-1000, 2)] {
        let mut input = model(1.0);
        input.nodes[1].mass = (1.0 + f64::EPSILON) * 2.0_f64.powi(power);
        let seen = Rc::new(Cell::new(0));
        let events = seen.clone();
        with_solver_observer(
            &SolverControl::default(),
            move |p| {
                if p.stage == SolverStage::HarmonicResidual && p.completed_steps == 0 {
                    events.set(events.get() + 1);
                }
            },
            || {
                validate_harmonic_equilibrium(
                    &input,
                    1.0,
                    &[Complex::default(), Complex { re: 0.4, im: -0.2 }],
                    &[None, Some(0)],
                )
            },
        )
        .unwrap();
        assert_eq!(seen.get(), expected);
    }
    assert!(std::mem::size_of::<Equation>() <= 56);
}
