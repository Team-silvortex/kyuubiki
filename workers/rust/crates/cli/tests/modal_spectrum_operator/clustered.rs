use super::{model, planned_solve};
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};

fn beam(axis: [f64; 3], split: f64, count: usize) -> Value {
    let mut input = model(true);
    input["nodes"].as_array_mut().unwrap().truncate(2);
    input["elements"].as_array_mut().unwrap().truncate(1);
    let norm = axis[0].hypot(axis[1]).hypot(axis[2]);
    for (key, value) in ["x", "y", "z"].into_iter().zip(axis) {
        input["nodes"][1][key] = json!(value / norm);
    }
    for key in ["fix_x", "fix_y", "fix_z", "fix_rx", "fix_ry", "fix_rz"] {
        input["nodes"][0][key] = json!(true);
        input["nodes"][1][key] = json!(false);
    }
    input["elements"][0]["youngs_modulus"] = json!(100.0);
    input["elements"][0]["shear_modulus"] = json!(40.0);
    input["elements"][0]["moment_of_inertia_y"] = json!(0.01 * (1.0 + split));
    input["mode_count"] = json!(count);
    input
}

fn reference_projector(axis: [f64; 3], high: bool) -> [[f64; 6]; 6] {
    let norm = axis[0].hypot(axis[1]).hypot(axis[2]);
    let n = axis.map(|x| x / norm);
    let beta = if high {
        -21.0_f64.sqrt() - 3.0
    } else {
        21.0_f64.sqrt() - 3.0
    };
    let norm = (0.5 + beta * beta / 24.0).sqrt();
    let a = 0.5_f64.sqrt() / norm;
    let b = beta / 24.0_f64.sqrt() / norm;
    let cross = [[0.0, -n[2], n[1]], [n[2], 0.0, -n[0]], [-n[1], n[0], 0.0]];
    let mut p = [[0.0; 6]; 6];
    for i in 0..3 {
        for j in 0..3 {
            let t = f64::from(i == j) - n[i] * n[j];
            p[i][j] = a * a * t;
            p[i + 3][j + 3] = b * b * t;
            p[i][j + 3] = -a * b * cross[i][j];
            p[i + 3][j] = a * b * cross[i][j];
        }
    }
    p
}

fn check_result(result: &Value, axis: [f64; 3], split: f64, count: usize) {
    let low = 60.0 - 12.0 * 21.0_f64.sqrt();
    let high = 60.0 + 12.0 * 21.0_f64.sqrt();
    let expected = [
        low,
        low * (1.0 + split),
        9.6,
        high,
        high * (1.0 + split),
        200.0,
    ];
    let modes = result["modes"].as_array().unwrap();
    assert_eq!(modes.len(), count);
    assert_eq!(result["free_dofs"], json!([6, 7, 8, 9, 10, 11]));
    let vectors: Vec<Vec<f64>> = modes
        .iter()
        .zip(expected)
        .enumerate()
        .map(|(i, (mode, root))| {
            assert_eq!(mode["index"], json!(i));
            let value = mode["eigenvalue_rad_s_squared"].as_f64().unwrap();
            assert!((value / root - 1.0).abs() < 1e-10);
            assert!((mode["participation_norm"].as_f64().unwrap() - 1.0).abs() < 1e-12);
            let shape = mode["shape"].as_array().unwrap();
            assert_eq!(shape.len(), 12);
            assert!(shape[..6].iter().all(|v| v.as_f64().unwrap() == 0.0));
            let mut v: Vec<_> = shape[6..]
                .iter()
                .enumerate()
                .map(|(j, x)| {
                    x.as_f64().unwrap()
                        * if j < 3 {
                            0.5_f64.sqrt()
                        } else {
                            1.0 / 24.0_f64.sqrt()
                        }
                })
                .collect();
            let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
            for x in &mut v {
                *x /= norm;
            }
            v
        })
        .collect();
    for (i, v) in vectors.iter().enumerate() {
        for w in vectors.iter().skip(i + 1) {
            assert!(v.iter().zip(w).map(|(a, b)| a * b).sum::<f64>().abs() < 1e-10);
        }
    }
    for (start, high) in [(0, false), (3, true)] {
        let reference = reference_projector(axis, high);
        for v in vectors.iter().skip(start).take(2) {
            for (row, component) in reference.iter().zip(v) {
                assert!(
                    (row.iter().zip(v).map(|(a, b)| a * b).sum::<f64>() - component).abs() < 1e-9
                );
            }
        }
        if count >= start + 2 {
            for (i, row) in reference.iter().enumerate() {
                for (j, entry) in row.iter().enumerate() {
                    let actual: f64 = vectors[start..start + 2].iter().map(|v| v[i] * v[j]).sum();
                    assert!((actual - entry).abs() < 1e-9);
                }
            }
        }
    }
}

#[test]
fn headless_repeated_and_split_modes_preserve_the_analytic_bending_subspaces() {
    for axis in [[1.0, 2.0, 3.0], [0.0, 0.0, 1.0]] {
        for split in [0.0, 1e-8] {
            let result = planned_solve(true, beam(axis, split, 6)).unwrap();
            check_result(&result, axis, split, 6);
        }
    }
}

#[test]
fn headless_cluster_truncation_keeps_the_requested_count_and_valid_directions() {
    let axis = [1.0, 2.0, 3.0];
    for count in 1..=6 {
        let result = planned_solve(true, beam(axis, 0.0, count)).unwrap();
        check_result(&result, axis, 0.0, count);
    }
}

#[test]
fn headless_cluster_validation_cancellation_replays_without_a_partial_spectrum() {
    let axis = [1.0, 2.0, 3.0];
    let input = beam(axis, 0.0, 6);
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ModalValidation && point.completed_steps == 1 {
                cancel.request_cancel();
            }
        },
        || {
            let result = planned_solve(true, input.clone());
            assert!(
                result.is_err(),
                "Engine must not return a partially validated cluster"
            );
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    let replay = planned_solve(true, input).unwrap();
    check_result(&replay, axis, 0.0, 6);
}
