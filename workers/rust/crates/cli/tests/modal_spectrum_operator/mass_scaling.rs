use super::planned_solve;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};

const SEGMENTS: usize = 66;

#[path = "complete_bending.rs"]
mod complete_bending;

#[path = "longer_bending.rs"]
mod longer_bending;

#[path = "polished_bending.rs"]
mod polished_bending;

#[path = "json_round_trip.rs"]
mod json_round_trip;

#[path = "roundoff.rs"]
mod roundoff;

fn model(length: f64) -> Value {
    model_with_segments(SEGMENTS, length)
}

fn model_with_segments(segments: usize, length: f64) -> Value {
    let nodes: Vec<_> = (0..=segments)
        .map(|i| {
            json!({
                "id":format!("node-{i}"), "x":i as f64 * length, "y":0.0,
                "fix_x":true, "fix_y":i==0, "fix_rz":i==0,
                "load_x":0.0, "load_y":0.0, "moment_z":0.0
            })
        })
        .collect();
    let elements: Vec<_> = (0..segments)
        .map(|i| {
            json!({
                "id":format!("beam-{i}"), "node_i":i, "node_j":i+1,
                "area":1.0, "youngs_modulus":length.powi(3), "moment_of_inertia":1.0,
                "section_modulus":1.0, "density":length.recip()
            })
        })
        .collect();
    json!({"nodes":nodes, "elements":elements, "mode_count":1})
}

fn check(result: &Value, expected: f64) {
    let free = result["free_dofs"].as_array().unwrap();
    assert_eq!(free.len(), 2 * SEGMENTS);
    assert_eq!(result["modes"].as_array().unwrap().len(), 1);
    let mode = &result["modes"][0];
    let eigenvalue = mode["eigenvalue_rad_s_squared"].as_f64().unwrap();
    assert!(eigenvalue.is_finite() && eigenvalue > 0.0);
    assert!((eigenvalue / expected - 1.0).abs() < 1e-7);
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
        (mode["natural_frequency_hz"].as_f64().unwrap() * mode["period_s"].as_f64().unwrap() - 1.0)
            .abs()
            < 1e-12
    );
}

#[test]
fn headless_mass_coordinate_scaling_keeps_a_solvable_bending_mode_solvable() {
    let reference = planned_solve(false, model(1.0)).unwrap();
    let expected = reference["modes"][0]["eigenvalue_rad_s_squared"]
        .as_f64()
        .unwrap();
    check(&reference, expected);
    for length in [1e14, 1e-10] {
        check(&planned_solve(false, model(length)).unwrap(), expected);
    }
}

#[test]
fn headless_normalized_inverse_cancellation_returns_an_error_and_replays() {
    let reference = planned_solve(false, model(1.0)).unwrap();
    let expected = reference["modes"][0]["eigenvalue_rad_s_squared"]
        .as_f64()
        .unwrap();
    for stage in [
        SolverStage::SparseMatrixScale,
        SolverStage::DenseFactor,
        SolverStage::DenseSubstitution,
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == 64 {
                    cancel.request_cancel();
                }
            },
            || {
                let result = planned_solve(false, model(1e14));
                assert!(result.is_err(), "Engine must not publish a cancelled mode");
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        check(&planned_solve(false, model(1e14)).unwrap(), expected);
    }
}
