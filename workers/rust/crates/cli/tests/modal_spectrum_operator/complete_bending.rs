use super::*;

fn request(length: f64) -> Value {
    let mut value = model(length);
    value["mode_count"] = json!(6);
    value
}

fn check_complete(result: &Value) {
    let roots = [
        6.513218799205997e-7,
        2.5553097863515013e-5,
        2.0007498276160384e-4,
        7.669658416882003e-4,
        2.0914454533768654e-3,
        4.655568785074286e-3,
    ];
    let modes = result["modes"].as_array().unwrap();
    assert_eq!(modes.len(), 6);
    assert_eq!(result["free_dofs"].as_array().unwrap().len(), 2 * SEGMENTS);
    for (index, (mode, root)) in modes.iter().zip(roots).enumerate() {
        assert_eq!(mode["index"], json!(index));
        assert!((mode["eigenvalue_rad_s_squared"].as_f64().unwrap() / root - 1.0).abs() < 2e-8);
        let shape = mode["shape"].as_array().unwrap();
        assert_eq!(shape.len(), 3 * (SEGMENTS + 1));
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
fn headless_complete_bending_spectrum_matches_independent_discrete_roots() {
    for length in [1.0, 1e14, 1e-10] {
        check_complete(&planned_solve(false, request(length)).unwrap());
    }
}

#[test]
fn headless_complete_bending_refinement_cancels_and_replays_without_partial_spectrum() {
    for (stage, boundary) in [
        (SolverStage::DenseFactor, 64),
        (SolverStage::ModalIteration, 1),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == boundary {
                    cancel.request_cancel();
                }
            },
            || {
                let result = planned_solve(false, request(1.0));
                assert!(
                    result.is_err(),
                    "Engine must not publish a partially refined spectrum"
                );
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        check_complete(&planned_solve(false, request(1.0)).unwrap());
    }
}
