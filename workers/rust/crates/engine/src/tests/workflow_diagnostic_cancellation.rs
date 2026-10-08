use crate::workflow_diagnostic_samples::Samples;
use crate::workflow_executor::run_extract_operator;
use kyuubiki_solver::solver_control::{SolverControl, with_solver_control, with_solver_observer};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};

fn extract(payload: Value) -> Result<Value, String> {
    run_extract_operator("extract.transport_result_diagnostics", payload, Value::Null)
}

#[test]
fn diagnostic_scan_cancellation_stops_before_late_corruption_and_fresh_call_recovers() {
    let healthy = json!({"nodes":(0..257).map(|i| json!({"concentration":i as f64 / 256.0,"source":0.0})).collect::<Vec<_>>(),"elements":[]});
    let mut corrupt = healthy.clone();
    corrupt["nodes"][128]["concentration"] = Value::Null;
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage.as_str() == "result_diagnostics" && point.completed_steps == 64 {
                cancel.request_cancel();
            }
        },
        || extract(corrupt),
    );
    assert!(
        result
            .unwrap_err()
            .contains("cancelled at result_diagnostics after 64 steps")
    );
    assert!(control.was_interrupted());
    let fresh = SolverControl::default();
    let result = with_solver_control(&fresh, || extract(healthy)).unwrap();
    assert_eq!(result["transport_concentration_max"], 1.0);
    assert!(!fresh.was_interrupted());
}

#[test]
fn stabilization_scan_cancellation_does_not_publish_earlier_physical_peaks() {
    let healthy = json!({"input":{"scheme":"upwind"},"nodes":[{"concentration":0.0},{"concentration":1.0}],
        "elements":(0..129).map(|i| json!({"id":format!("e{i}"),"total_flux":3.0,
            "stabilization":{"artificial_diffusivity":1.0,"stabilization_flux":-1.0,"numerical_flux":2.0}})).collect::<Vec<_>>()});
    let mut corrupt = healthy.clone();
    corrupt["elements"][100]["stabilization"]["numerical_flux"] = Value::Null;
    let control = SolverControl::default();
    let cancel = control.clone();
    let passes = Rc::new(RefCell::new(0));
    let seen = passes.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage.as_str() == "result_diagnostics" && point.completed_steps == 64 {
                *seen.borrow_mut() += 1;
                // Four physical element scans precede the stabilization scan.
                if *seen.borrow() == 5 {
                    cancel.request_cancel();
                }
            }
        },
        || extract(corrupt),
    );
    assert!(
        result
            .unwrap_err()
            .contains("cancelled at result_diagnostics after 64 steps")
    );
    assert_eq!(*passes.borrow(), 5);
    let fresh = extract(healthy).unwrap();
    assert_eq!(fresh["transport_total_flux_peak"], 3.0);
    assert_eq!(fresh["transport_numerical_flux_peak"], 2.0);
}

#[test]
fn diagnostic_safe_points_cover_empty_short_chunked_and_final_scans() {
    for length in [0, 1, 63, 64, 65, 129] {
        let payload = json!({"nodes":vec![json!({"value":1.0});length]});
        let samples = Samples::fixed(payload.as_object().unwrap(), "nodes").unwrap();
        let points = Rc::new(RefCell::new(Vec::new()));
        let seen = points.clone();
        let control = SolverControl::default();
        let present = with_solver_observer(
            &control,
            move |point| {
                assert_eq!(point.stage.as_str(), "result_diagnostics");
                seen.borrow_mut().push(point.completed_steps);
            },
            || {
                samples.scan(
                    "value",
                    false,
                    |row, _, _| Ok(row["value"].as_f64()),
                    |_, _| Ok(()),
                )
            },
        )
        .unwrap();
        let expected = (0..=length)
            .filter(|i| i % 64 == 0 || *i == length)
            .map(|i| i as u64)
            .collect::<Vec<_>>();
        assert_eq!(*points.borrow(), expected, "length={length}");
        assert_eq!(present, length > 0);
    }
}

#[test]
fn diagnostic_error_before_next_safe_point_is_not_relabelled_as_cancellation() {
    let payload = json!({"nodes":[{}, {}, {}, {}]});
    let samples = Samples::fixed(payload.as_object().unwrap(), "nodes").unwrap();
    let control = SolverControl::default();
    let mut observed = 0;
    let result = with_solver_control(&control, || {
        samples.scan(
            "value",
            false,
            |_, _, index| {
                if index == 3 {
                    Err("invalid original sample".into())
                } else {
                    Ok(Some(1.0))
                }
            },
            |_, _| {
                observed += 1;
                if observed == 3 {
                    control.request_cancel();
                }
                Ok(())
            },
        )
    });
    assert_eq!(result.unwrap_err(), "invalid original sample");
    assert_eq!(observed, 3);
    assert!(!control.was_interrupted());
    assert!(extract(json!({"nodes":[{"concentration":1.0}],"elements":[]})).is_ok());
}
