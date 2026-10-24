use super::*;
use serde_json::json;

fn contrast_request() -> SolveHarmonicSpring1dRequest {
    serde_json::from_value(json!({
        "nodes":[
            {"id":"root", "x":0.0, "fix_x":true, "load_x":7.0, "mass":1.0},
            {"id":"middle", "x":1.0, "fix_x":false, "load_x":0.0, "mass":1.0},
            {"id":"tip", "x":2.0, "fix_x":false, "load_x":1.0, "mass":1.0}
        ],
        "elements":[
            {"id":"hard", "node_i":0, "node_j":1, "stiffness":1e300},
            {"id":"soft", "node_i":1, "node_j":2, "stiffness":1e-300}
        ], "frequencies_hz":[0.0]
    }))
    .unwrap()
}

#[test]
fn harmonic_original_equations_reject_erased_incident_force() {
    let request = contrast_request();
    let displacement = [Complex::default(), Complex::default(), Complex::real(1e300)];
    let error =
        validate_harmonic_equilibrium(&request, 0.0, &displacement, &[None, Some(0), Some(1)])
            .unwrap_err();
    assert!(
        error.contains("backward-error") && error.contains("free row 0"),
        "{error}"
    );
}

#[test]
fn harmonic_original_equations_gate_real_solver_publication() {
    let error = solve_harmonic_spring_1d(&contrast_request()).unwrap_err();
    // This fixture now rejects at matrix preparation; the direct state control above still tests the equation gate.
    assert!(
        error.contains("frequency 0")
            && error.contains("prepared row 0 column 1")
            && error.contains("real coefficient normalization would lose numerical range"),
        "{error}"
    );
}

#[test]
fn harmonic_original_equations_reject_exact_support_drift() {
    let mut request = contrast_request();
    request.nodes[1].load_x = 1e300;
    request.nodes[2].load_x = 0.0;
    for drift in [
        Complex::real(f64::from_bits(1)),
        Complex {
            re: 0.0,
            im: f64::from_bits(1),
        },
    ] {
        let displacement = [drift, Complex::real(1.0), Complex::real(1.0)];
        let error =
            validate_harmonic_equilibrium(&request, 0.0, &displacement, &[None, Some(0), Some(1)])
                .unwrap_err();
        assert!(
            error.contains("support") && error.contains("node 0"),
            "{error}"
        );
    }
}

#[test]
fn harmonic_original_equations_check_isolated_fixed_unknowns() {
    let mut request = contrast_request();
    request.nodes[1].load_x = 1e300;
    request.nodes[2].load_x = 0.0;
    let mut isolated = request.nodes[0].clone();
    isolated.id = "isolated".into();
    isolated.x = 3.0;
    request.nodes.push(isolated);
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let displacement = [
            Complex::default(),
            Complex::real(1.0),
            Complex::real(1.0),
            Complex { re: 0.0, im: value },
        ];
        let error = validate_harmonic_equilibrium(
            &request,
            0.0,
            &displacement,
            &[None, Some(0), Some(1), None],
        )
        .unwrap_err();
        assert!(
            error.contains("non-finite") && error.contains("node 3"),
            "{error}"
        );
    }
}

#[test]
fn harmonic_original_equations_reject_bad_dimensions_and_mapping_without_panics() {
    let request = contrast_request();
    let state = [Complex::default(); 3];
    let map = [None, Some(0), Some(1)];
    for displacement in [&state[..2], &[Complex::default(); 4][..]] {
        assert!(
            validate_harmonic_equilibrium(&request, 0.0, displacement, &map)
                .unwrap_err()
                .contains("dimensions")
        );
    }
    for bad in [
        vec![None, Some(0)],
        vec![None, Some(0), Some(1), None],
        vec![None, None, Some(0)],
        vec![None, Some(0), Some(0)],
        vec![None, Some(1), Some(0)],
        vec![Some(0), Some(1), Some(2)],
        vec![None, Some(usize::MAX), Some(1)],
    ] {
        let error = validate_harmonic_equilibrium(&request, 0.0, &state, &bad).unwrap_err();
        assert!(
            error.contains("dimensions") || error.contains("reduction map"),
            "{error}"
        );
    }
}

#[test]
fn harmonic_original_equations_keep_complex_balance_and_loaded_supports() {
    for power in [-600, 0, 600] {
        let factor = 2.0_f64.powi(power);
        let mut request = contrast_request();
        request.nodes.truncate(2);
        request.elements.truncate(1);
        request.nodes[1].mass = factor;
        request.nodes[1].load_x = factor;
        request.elements[0].stiffness = 2.0 * factor;
        request.elements[0].damping = factor;
        let mut displacement = [
            Complex { re: -0.0, im: -0.0 },
            Complex { re: 0.5, im: -0.5 },
        ];
        validate_harmonic_equilibrium(&request, 1.0, &displacement, &[None, Some(0)]).unwrap();
        // Real equilibrium alone still holds; the imaginary equation does not.
        displacement[1] = Complex {
            re: 0.75,
            im: -0.25,
        };
        let error = validate_harmonic_equilibrium(&request, 1.0, &displacement, &[None, Some(0)])
            .unwrap_err();
        assert!(error.contains("backward-error"), "{error}");
    }
}

#[test]
fn harmonic_original_equations_preserve_representable_contrast_but_reject_lost_range() {
    for power in [500, 1000] {
        let mut request = contrast_request();
        request.elements[0].stiffness = 2.0_f64.powi(power);
        request.elements[1].stiffness = 2.0_f64.powi(-power);
        for sign in [-1.0, 1.0] {
            request.nodes[2].load_x = sign;
            let displacement = [
                Complex::default(),
                Complex::real(sign * 2.0_f64.powi(-power)),
                Complex::real(sign * 2.0_f64.powi(power)),
            ];
            let validation = validate_harmonic_equilibrium(
                &request,
                0.0,
                &displacement,
                &[None, Some(0), Some(1)],
            );
            if power == 500 {
                validation.unwrap();
                let result = solve_harmonic_spring_1d(&request).unwrap();
                assert_eq!(
                    result.frequencies[0].nodes[1].displacement_amplitude,
                    2.0_f64.powi(-power)
                );
            } else {
                assert!(validation.unwrap_err().contains("lose numerical range"));
            }
        }
    }
}

#[test]
fn harmonic_original_equations_cancel_state_and_residual_completion_then_replay() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    let mut request = contrast_request();
    request.elements[0].stiffness = 2.0;
    request.elements[1].stiffness = 1.0;
    let baseline = solve_harmonic_spring_1d(&request).unwrap();
    for (stage, completed) in [
        (SolverStage::ConstraintValidate, 0),
        (SolverStage::ConstraintValidate, 3),
        (SolverStage::ConstraintValidate, 6),
        (SolverStage::HarmonicResidual, 2),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == completed {
                    cancel.request_cancel();
                }
            },
            || solve_harmonic_spring_1d(&request),
        )
        .unwrap_err();
        assert!(
            control.was_interrupted() && error.contains(stage.as_str()),
            "{error}"
        );
        assert_eq!(
            serde_json::to_value(solve_harmonic_spring_1d(&request).unwrap()).unwrap(),
            serde_json::to_value(&baseline).unwrap()
        );
    }
}

#[test]
fn harmonic_original_equations_cancel_all_wide_scan_phases_then_replay() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    use std::{cell::Cell, rc::Rc};
    let size = 2051;
    let request: SolveHarmonicSpring1dRequest = serde_json::from_value(json!({
        "nodes":(0..size).map(|index| json!({
            "id":format!("n{index}"), "x":index, "fix_x":index != 1,
            "load_x":if index == 1 { 1.0 } else { 7.0 }, "mass":1.0
        })).collect::<Vec<_>>(),
        "elements":(0..size).filter(|&index| index != 1).map(|index| json!({
            "id":format!("s{index}"), "node_i":1, "node_j":index, "stiffness":1.0, "damping":0.5
        })).collect::<Vec<_>>(), "frequencies_hz":[1.0/(2.0*PI)]
    }))
    .unwrap();
    let baseline = serde_json::to_value(solve_harmonic_spring_1d(&request).unwrap()).unwrap();
    for phase in 1..=6 {
        let control = SolverControl::default();
        let cancel = control.clone();
        let hits = Rc::new(Cell::new(0));
        let observed = Rc::clone(&hits);
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::HarmonicResidual && point.completed_steps == 1024 {
                    observed.set(observed.get() + 1);
                    if observed.get() == phase {
                        cancel.request_cancel();
                    }
                }
            },
            || solve_harmonic_spring_1d(&request),
        )
        .unwrap_err();
        assert_eq!(hits.get(), phase);
        assert!(
            control.was_interrupted() && error.contains("harmonic_residual"),
            "{error}"
        );
        assert_eq!(
            serde_json::to_value(solve_harmonic_spring_1d(&request).unwrap()).unwrap(),
            baseline
        );
    }
}

#[test]
fn harmonic_frequency_range_original_gate_rejects_the_old_zero_inertia_solution() {
    for power in [-600, 600] {
        let mut request = contrast_request();
        request.nodes.truncate(2);
        request.elements.truncate(1);
        request.nodes[1].mass = 2.0_f64.powi(-3 * power / 2);
        request.nodes[1].load_x = 2.0_f64.powi(-power / 2);
        request.elements[0].stiffness = 2.0 * 2.0_f64.powi(power / 2);
        let mut displacement = [Complex::default(), Complex::real(2.0_f64.powi(-power))];
        validate_harmonic_equilibrium(
            &request,
            2.0_f64.powi(power),
            &displacement,
            &[None, Some(0)],
        )
        .unwrap();
        displacement[1].re *= 0.5;
        let error = validate_harmonic_equilibrium(
            &request,
            2.0_f64.powi(power),
            &displacement,
            &[None, Some(0)],
        )
        .unwrap_err();
        assert!(error.contains("backward-error"), "{error}");
    }
}
