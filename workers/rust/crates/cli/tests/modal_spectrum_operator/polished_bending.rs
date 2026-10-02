use super::*;
use std::{cell::Cell, rc::Rc};

#[path = "../../../solver/tests/support/modal_published_reference.rs"]
mod published_reference;

const ROOTS_96: [f64; 6] = [
    1.4553085447431156e-7,
    5.712730198911023e-6,
    4.47606463734338e-5,
    1.7174163867530644e-4,
    4.688419676264506e-4,
    1.045002999787545e-3,
];
const ROOTS_100: [f64; 6] = [
    1.236075027143164e-7,
    4.852330686025576e-6,
    3.8021072825043554e-5,
    1.458920056234989e-4,
    3.983_054_168_160_999e-4,
    8.878653418209562e-4,
];

pub(super) fn request(segments: usize, length: f64, space: bool) -> Value {
    let mut model = model_with_segments(segments, length);
    model["mode_count"] = json!(if space { 6 } else { 20 });
    if space {
        for node in model["nodes"].as_array_mut().unwrap() {
            node["z"] = json!(0.0);
            node["fix_z"] = node["fix_y"].clone();
            node["fix_ry"] = node["fix_rz"].clone();
            node["fix_rx"] = json!(true);
            for key in ["load_z", "moment_x", "moment_y"] {
                node[key] = json!(0.0);
            }
        }
        for element in model["elements"].as_array_mut().unwrap() {
            element["shear_modulus"] = json!(0.4 * length.powi(3));
            for key in [
                "moment_of_inertia_y",
                "moment_of_inertia_z",
                "torsion_constant",
            ] {
                element[key] = json!(1.0);
            }
            element.as_object_mut().unwrap().remove("moment_of_inertia");
            element.as_object_mut().unwrap().remove("section_modulus");
        }
    }
    model
}

pub(super) fn check(result: &Value, segments: usize, length: f64, space: bool) {
    let roots = if segments == 96 { ROOTS_96 } else { ROOTS_100 };
    let modes = result["modes"].as_array().unwrap();
    assert_eq!(modes.len(), if space { 6 } else { 20 });
    assert_eq!(
        result["free_dofs"].as_array().unwrap().len(),
        segments * if space { 4 } else { 2 }
    );
    let stride = if space { 6 } else { 3 };
    for (index, mode) in modes.iter().enumerate() {
        let reference = if space { index / 2 } else { index };
        if reference < roots.len() {
            assert!(
                (mode["eigenvalue_rad_s_squared"].as_f64().unwrap() / roots[reference] - 1.0).abs()
                    < 2e-8
            );
        }
        assert_eq!(mode["index"], json!(index));
        let shape = mode["shape"].as_array().unwrap();
        assert_eq!(shape.len(), stride * (segments + 1));
        let mut norm = 0.0;
        for (dof, value) in shape.iter().enumerate() {
            let value = value.as_f64().unwrap();
            assert!(value.is_finite());
            if dof < stride || dof % stride == 0 || (space && dof % stride == 3) {
                assert_eq!(value, 0.0);
            }
            norm += value * value;
        }
        assert!((norm - 1.0).abs() < 1e-10);
        if length == 1.0 {
            let physical: Vec<_> = shape.iter().map(|v| v.as_f64().unwrap()).collect();
            let relative = published_reference::unit_bending_residual(
                segments,
                mode["eigenvalue_rad_s_squared"].as_f64().unwrap(),
                &physical,
                space,
            );
            assert!(
                relative <= 1e-8,
                "headless published mode {index}: {relative:e}"
            );
        }
        assert!(
            (mode["natural_frequency_hz"].as_f64().unwrap() * mode["period_s"].as_f64().unwrap()
                - 1.0)
                .abs()
                < 1e-12
        );
    }
    if !space {
        let higher = if segments == 96 {
            [9.241836936192287e-3, 1.5949592609683072e-1]
        } else {
            [7.856_089_927_861_69e-3, 1.3587583669754427e-1]
        };
        for (index, root) in [9, 19].into_iter().zip(higher) {
            assert!(
                (modes[index]["eigenvalue_rad_s_squared"].as_f64().unwrap() / root - 1.0).abs()
                    < 2e-8
            );
        }
    }
}

#[test]
fn headless_polished_bending_matches_low_and_high_roots_at_multiple_scales() {
    for segments in [96, 100] {
        for length in [1.0, 1e14, 1e-10] {
            check(
                &planned_solve(false, request(segments, length, false)).unwrap(),
                segments,
                length,
                false,
            );
        }
    }
}

#[test]
fn headless_polished_spatial_bending_preserves_repeated_frequencies_and_shape_contracts() {
    for length in [1.0, 1e14, 1e-10] {
        check(
            &planned_solve(true, request(100, length, true)).unwrap(),
            100,
            length,
            true,
        );
    }
}

#[test]
fn headless_polish_cancels_inside_the_extra_product_without_partial_results_and_replays() {
    let started = Rc::new(Cell::new(false));
    let seen = started.clone();
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ModalValidation && point.completed_steps == 0 {
                seen.set(true);
            }
            if seen.get() && point.stage == SolverStage::SparseMatvec && point.completed_steps == 64
            {
                cancel.request_cancel();
            }
        },
        || {
            let result = planned_solve(false, request(100, 1.0, false));
            assert!(result.is_err());
            result
        },
    )
    .unwrap_err();
    assert!(started.get());
    assert!(error.contains("cancel"));
    check(
        &planned_solve(false, request(100, 1.0, false)).unwrap(),
        100,
        1.0,
        false,
    );
}

#[test]
fn headless_published_shape_cancellation_leaks_no_partial_payload_and_replays() {
    for stage in [SolverStage::SparseMatvec, SolverStage::ResidualValidate] {
        let output = Rc::new(Cell::new(false));
        let repair = Rc::new(Cell::new(false));
        let seen = Rc::new(Cell::new(false));
        let (out, polish, hit) = (output.clone(), repair.clone(), seen.clone());
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ResultTotals {
                    out.set(true);
                }
                if out.get() && point.stage == SolverStage::ModalValidation {
                    polish.set(true);
                }
                if out.get()
                    && (stage == SolverStage::SparseMatvec || polish.get())
                    && point.stage == stage
                    && point.completed_steps == 64
                {
                    hit.set(true);
                    cancel.request_cancel();
                }
            },
            || {
                let result = planned_solve(false, request(100, 1.0, false));
                assert!(
                    result.is_err(),
                    "Engine must not serialize cancelled published modes"
                );
                result
            },
        )
        .unwrap_err();
        assert!(seen.get());
        assert!(error.contains("cancel"), "{error}");
        check(
            &planned_solve(false, request(100, 1.0, false)).unwrap(),
            100,
            1.0,
            false,
        );
    }
}
