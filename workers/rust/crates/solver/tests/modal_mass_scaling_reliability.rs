use kyuubiki_protocol::{SolveModalFrame2dRequest, SolveModalFrame2dResult};
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{solve_modal_frame_2d, solve_modal_frame_2d_owned};
use serde_json::json;
use std::{cell::Cell, rc::Rc};

const SEGMENTS: usize = 66;

#[path = "modal_mass_scaling_reliability/complete.rs"]
mod complete;

#[path = "modal_mass_scaling_reliability/longer.rs"]
mod longer;

#[path = "modal_mass_scaling_reliability/polished.rs"]
mod polished;

#[path = "support/modal_published_reference.rs"]
mod published_reference;

#[path = "modal_mass_scaling_reliability/published.rs"]
mod published;

#[path = "modal_mass_scaling_reliability/json_round_trip.rs"]
mod json_round_trip;

#[path = "modal_mass_scaling_reliability/normalization.rs"]
mod normalization;

#[path = "modal_mass_scaling_reliability/control.rs"]
mod control;

#[path = "modal_mass_scaling_reliability/roundoff.rs"]
mod roundoff;

fn bending_chain(length: f64, count: usize) -> SolveModalFrame2dRequest {
    bending_chain_with_segments(SEGMENTS, length, count)
}

fn bending_chain_with_segments(
    segments: usize,
    length: f64,
    count: usize,
) -> SolveModalFrame2dRequest {
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
    serde_json::from_value(json!({"nodes":nodes, "elements":elements, "mode_count":count})).unwrap()
}

fn shape_in_common_coordinates(result: &SolveModalFrame2dResult, length: f64) -> Vec<f64> {
    assert_eq!(result.free_dofs.len(), 2 * SEGMENTS);
    assert_eq!(result.modes.len(), 1);
    assert!((result.total_mass / SEGMENTS as f64 - 1.0).abs() < 1e-12);
    let mode = &result.modes[0];
    assert_eq!(mode.shape.len(), 3 * (SEGMENTS + 1));
    assert!((mode.natural_frequency_hz * mode.period_s - 1.0).abs() < 1e-12);
    let mut shape = Vec::with_capacity(mode.shape.len());
    for (dof, value) in mode.shape.iter().enumerate() {
        assert!(value.is_finite());
        if dof < 3 || dof % 3 == 0 {
            assert_eq!(*value, 0.0);
        }
        // K(L) = D K(1) D and M(L) = D M(1) D, with D = diag(1, L) per tip.
        shape.push(if dof % 3 == 2 { value * length } else { *value });
    }
    let norm = mode.shape.iter().map(|v| v * v).sum::<f64>();
    assert!((norm - 1.0).abs() < 1e-10);
    let common_norm = shape.iter().map(|v| v * v).sum::<f64>().sqrt();
    assert!(common_norm.is_finite() && common_norm > 0.0);
    shape.iter_mut().for_each(|v| *v /= common_norm);
    shape
}

fn check_equivalent(
    result: &SolveModalFrame2dResult,
    length: f64,
    reference: &SolveModalFrame2dResult,
) {
    let actual = shape_in_common_coordinates(result, length);
    let expected = shape_in_common_coordinates(reference, 1.0);
    let ratio =
        result.modes[0].eigenvalue_rad_s_squared / reference.modes[0].eigenvalue_rad_s_squared;
    assert!(
        (ratio - 1.0).abs() < 1e-7,
        "length={length:e}, ratio={ratio:e}"
    );
    let dot = actual
        .iter()
        .zip(&expected)
        .map(|(a, b)| a * b)
        .sum::<f64>();
    let error = actual
        .iter()
        .zip(expected)
        .map(|(a, b)| (a - dot.signum() * b).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(
        error < 1e-6,
        "length={length:e}, common-coordinate shape error={error:e}"
    );
}

#[test]
fn connected_bending_mode_does_not_become_singular_from_coordinate_scale() {
    let reference = solve_modal_frame_2d(&bending_chain(1.0, 1)).unwrap();
    for length in [1.0, 1e14, 1e-10] {
        let result = solve_modal_frame_2d(&bending_chain(length, 1)).unwrap();
        check_equivalent(&result, length, &reference);
    }
}

#[test]
fn bending_inverse_is_prepared_once_per_call_not_shared_between_models() {
    let counts = Rc::new(Cell::new([0; 2]));
    let observed = counts.clone();
    let reference = solve_modal_frame_2d(&bending_chain(1.0, 1)).unwrap();
    with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.completed_steps == 0 {
                let mut values = observed.get();
                match point.stage {
                    SolverStage::DenseFactor => values[0] += 1,
                    SolverStage::ModalIteration => values[1] += 1,
                    _ => (),
                }
                observed.set(values);
            }
        },
        || {
            for length in [1e14, 1e-10] {
                let result = solve_modal_frame_2d_owned(bending_chain(length, 1))?;
                check_equivalent(&result, length, &reference);
            }
            Ok::<(), String>(())
        },
    )
    .unwrap();
    assert_eq!(
        counts.get(),
        [2, 2],
        "one inverse factor and one dense refinement sequence per bounded call"
    );
}

#[test]
fn normalized_bending_preparation_and_substitution_cancel_without_partial_modes() {
    let reference = solve_modal_frame_2d(&bending_chain(1.0, 1)).unwrap();
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
                let result = solve_modal_frame_2d_owned(bending_chain(1e14, 1));
                assert!(
                    result.is_err(),
                    "cancellation must interrupt the solve itself"
                );
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        let replay = solve_modal_frame_2d_owned(bending_chain(1e14, 1)).unwrap();
        check_equivalent(&replay, 1e14, &reference);
    }
}
