use super::*;
use std::{cell::Cell, rc::Rc};

fn request(segments: usize, length: f64) -> Value {
    let mut value = model_with_segments(segments, length);
    value["mode_count"] = json!(20);
    value
}

fn check(result: &Value) {
    let roots = [
        (0, 3.017539808750544e-7),
        (1, 1.1842589597842897e-5),
        (2, 9.276402725565059e-5),
        (3, 3.557971963900987e-4),
        (4, 9.708758245053636e-4),
        (5, 2.1628755568757146e-3),
        (9, 1.9074331222057785e-2),
        (19, 3.252305299727169e-1),
    ];
    let modes = result["modes"].as_array().unwrap();
    assert_eq!(modes.len(), 20);
    assert_eq!(result["free_dofs"].as_array().unwrap().len(), 160);
    for (index, root) in roots {
        assert!(
            (modes[index]["eigenvalue_rad_s_squared"].as_f64().unwrap() / root - 1.0).abs() < 2e-8
        );
    }
    for (index, mode) in modes.iter().enumerate() {
        assert_eq!(mode["index"], json!(index));
        let shape = mode["shape"].as_array().unwrap();
        assert_eq!(shape.len(), 243);
        let mut norm = 0.0;
        for (dof, value) in shape.iter().enumerate() {
            let value = value.as_f64().unwrap();
            assert!(value.is_finite());
            if dof < 3 || dof % 3 == 0 {
                assert_eq!(value, 0.0);
            }
            norm += value * value;
        }
        assert!((norm - 1.0).abs() < 1e-10);
        assert!(
            (mode["natural_frequency_hz"].as_f64().unwrap() * mode["period_s"].as_f64().unwrap()
                - 1.0)
                .abs()
                < 1e-12
        );
    }
}

#[test]
fn headless_long_bending_matches_independent_low_and_higher_roots() {
    for length in [1.0, 1e14, 1e-10] {
        check(&planned_solve(false, request(80, length)).unwrap());
    }
}

#[test]
fn headless_compensated_modal_validation_cancels_then_replays() {
    let started = Rc::new(Cell::new(false));
    let products = Rc::new(Cell::new(0));
    let seen = products.clone();
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ModalIteration && point.completed_steps == 0 {
                started.set(true);
            }
            if started.get()
                && point.stage == SolverStage::SparseMatvec
                && point.completed_steps == 64
            {
                seen.set(seen.get() + 1);
                if seen.get() == 2 {
                    cancel.request_cancel();
                }
            }
        },
        || {
            let result = planned_solve(false, request(80, 1.0));
            assert!(
                result.is_err(),
                "Engine must not publish a cancelled refined spectrum"
            );
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    assert_eq!(products.get(), 2);
    check(&planned_solve(false, request(80, 1.0)).unwrap());
}

#[test]
fn headless_unresolved_long_bending_returns_error_not_partial_spectrum() {
    let error = planned_solve(false, request(128, 1.0)).unwrap_err();
    assert!(
        error.contains("refinement did not converge within 4 steps"),
        "{error}"
    );
    check(&planned_solve(false, request(80, 1.0)).unwrap());
}
