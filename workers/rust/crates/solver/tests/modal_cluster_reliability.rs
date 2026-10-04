use kyuubiki_protocol::{SolveModalFrame3dRequest, SolveModalFrame3dResult};
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{solve_modal_frame_3d, solve_modal_frame_3d_owned};
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc};

fn unit(axis: [f64; 3]) -> [f64; 3] {
    let norm = axis[0].hypot(axis[1]).hypot(axis[2]);
    axis.map(|value| value / norm)
}

fn beam(
    axis: [f64; 3],
    length: f64,
    scale: f64,
    split: f64,
    count: usize,
) -> SolveModalFrame3dRequest {
    let axis = unit(axis);
    let nodes: Vec<Value> = (0..2)
        .map(|i| {
            let t = i as f64 * length;
            let fixed = i == 0;
            json!({"id":format!("node{i}"), "x":t*axis[0], "y":t*axis[1], "z":t*axis[2],
            "fix_x":fixed, "fix_y":fixed, "fix_z":fixed,
            "fix_rx":fixed, "fix_ry":fixed, "fix_rz":fixed,
            "load_x":0.0, "load_y":0.0, "load_z":0.0,
            "moment_x":0.0, "moment_y":0.0, "moment_z":0.0})
        })
        .collect();
    serde_json::from_value(json!({"nodes":nodes, "elements":[{
        "id":"beam", "node_i":0, "node_j":1, "area":1.0,
        "youngs_modulus":100.0*scale, "shear_modulus":40.0*scale,
        "torsion_constant":0.01, "moment_of_inertia_y":0.01*(1.0+split),
        "moment_of_inertia_z":0.01, "density":scale
    }], "mode_count":count}))
    .unwrap()
}

fn roots(length: f64, split: f64) -> Vec<f64> {
    let low = (60.0 - 12.0 * 21.0_f64.sqrt()) / length.powi(4);
    let high = (60.0 + 12.0 * 21.0_f64.sqrt()) / length.powi(4);
    let mut values = vec![
        low,
        low * (1.0 + split),
        9.6 / length.powi(4),
        high,
        high * (1.0 + split),
        200.0 / length.powi(2),
    ];
    values.sort_by(f64::total_cmp);
    values
}

fn weighted_modes(result: &SolveModalFrame3dResult, length: f64, tip: usize) -> Vec<Vec<f64>> {
    result
        .modes
        .iter()
        .map(|mode| {
            assert_eq!(mode.shape.len(), 12);
            for (dof, value) in mode.shape.iter().enumerate() {
                assert!(value.is_finite());
                if dof / 6 != tip {
                    assert_eq!(*value, 0.0);
                }
            }
            let mut v: Vec<_> = mode.shape[tip * 6..tip * 6 + 6]
                .iter()
                .enumerate()
                .map(|(i, value)| {
                    value
                        * if i < 3 {
                            0.5_f64.sqrt()
                        } else {
                            length / 24.0_f64.sqrt()
                        }
                })
                .collect();
            let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
            for x in &mut v {
                *x /= norm;
            }
            v
        })
        .collect()
}

fn projector(vectors: &[Vec<f64>]) -> Vec<Vec<f64>> {
    (0..6)
        .map(|i| {
            (0..6)
                .map(|j| vectors.iter().map(|v| v[i] * v[j]).sum())
                .collect()
        })
        .collect()
}

fn analytic_bending_projector(axis: [f64; 3], high: bool) -> Vec<Vec<f64>> {
    let n = unit(axis);
    let beta = if high {
        -21.0_f64.sqrt() - 3.0
    } else {
        21.0_f64.sqrt() - 3.0
    };
    let norm = (0.5 + beta * beta / 24.0).sqrt();
    let a = 0.5_f64.sqrt() / norm;
    let b = beta / 24.0_f64.sqrt() / norm;
    let cross = [[0.0, -n[2], n[1]], [n[2], 0.0, -n[0]], [-n[1], n[0], 0.0]];
    let mut p = vec![vec![0.0; 6]; 6];
    for i in 0..3 {
        for j in 0..3 {
            let transverse = f64::from(i == j) - n[i] * n[j];
            p[i][j] = a * a * transverse;
            p[i + 3][j + 3] = b * b * transverse;
            p[i][j + 3] = -a * b * cross[i][j];
            p[i + 3][j] = a * b * cross[i][j];
        }
    }
    p
}

fn assert_projector(actual: &[Vec<f64>], expected: &[Vec<f64>]) {
    let error = actual
        .iter()
        .flatten()
        .zip(expected.iter().flatten())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(error < 1e-9, "projector error={error:e}");
}

fn check_complete(
    result: &SolveModalFrame3dResult,
    axis: [f64; 3],
    length: f64,
    split: f64,
    tip: usize,
) {
    assert_eq!(result.modes.len(), 6);
    let reference = roots(length, split);
    for (index, (mode, root)) in result.modes.iter().zip(reference).enumerate() {
        assert_eq!(mode.index, index);
        assert!((mode.eigenvalue_rad_s_squared / root - 1.0).abs() < 1e-10);
        assert!((mode.natural_frequency_hz * mode.period_s - 1.0).abs() < 1e-12);
        assert!((mode.participation_norm - 1.0).abs() < 1e-12);
    }
    let vectors = weighted_modes(result, length, tip);
    for (i, v) in vectors.iter().enumerate() {
        for w in vectors.iter().skip(i + 1) {
            assert!(v.iter().zip(w).map(|(a, b)| a * b).sum::<f64>().abs() < 1e-10);
        }
    }
    for high in [false, true] {
        let eigenvalue =
            (60.0 + if high { 12.0 } else { -12.0 } * 21.0_f64.sqrt()) / length.powi(4);
        let cluster: Vec<_> = result
            .modes
            .iter()
            .zip(&vectors)
            .filter(|(mode, _)| (mode.eigenvalue_rad_s_squared / eigenvalue - 1.0).abs() < 1e-5)
            .map(|(_, v)| v.clone())
            .collect();
        assert_eq!(cluster.len(), 2);
        assert_projector(
            &projector(&cluster),
            &analytic_bending_projector(axis, high),
        );
    }
}

#[test]
fn spatial_repeated_bending_modes_match_analytic_subspaces_under_rotation() {
    for axis in [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 2.0, 3.0],
        [-3.0, 2.0, -1.0],
    ] {
        for length in [0.5, 1.0, 2.0] {
            let result = solve_modal_frame_3d(&beam(axis, length, 1.0, 0.0, 6)).unwrap();
            check_complete(&result, axis, length, 0.0, 1);
        }
    }
}

#[test]
fn spatial_near_repeated_bending_modes_retain_splitting_and_mass_orthogonality() {
    for split in [1e-6, 1e-8] {
        for axis in [[1.0, 0.0, 0.0], [1.0, 2.0, 3.0], [0.0, 0.0, 1.0]] {
            let result = solve_modal_frame_3d(&beam(axis, 1.0, 1.0, split, 6)).unwrap();
            check_complete(&result, axis, 1.0, split, 1);
            let gap = result.modes[1].eigenvalue_rad_s_squared
                / result.modes[0].eigenvalue_rad_s_squared
                - 1.0;
            assert!(
                (gap / split - 1.0).abs() < 1e-4,
                "gap={gap:e}, split={split:e}"
            );
        }
    }
}

#[test]
fn spatial_repeated_subspaces_survive_material_rescaling_and_reversed_topology() {
    let axis = [1.0, 2.0, 3.0];
    for scale in [1e-200, 1.0, 1e200] {
        let mut input = beam(axis, 1.0, scale, 0.0, 6);
        let reference = solve_modal_frame_3d(&input).unwrap();
        check_complete(&reference, axis, 1.0, 0.0, 1);
        input.nodes.reverse();
        let reversed = solve_modal_frame_3d_owned(input).unwrap();
        check_complete(&reversed, axis, 1.0, 0.0, 0);
    }
}

#[test]
fn truncating_a_repeated_cluster_returns_a_member_not_a_unique_direction() {
    let axis = [1.0, 2.0, 3.0];
    for count in [1, 2, 3, 4, 5, 6] {
        let result = solve_modal_frame_3d(&beam(axis, 1.0, 1.0, 0.0, count)).unwrap();
        assert_eq!(result.modes.len(), count);
        let vectors = weighted_modes(&result, 1.0, 1);
        let expected = analytic_bending_projector(axis, false);
        for v in vectors.iter().take(2) {
            for (row, component) in expected.iter().zip(v) {
                assert!(
                    (row.iter().zip(v).map(|(a, b)| a * b).sum::<f64>() - component).abs() < 1e-10
                );
            }
        }
        if count >= 2 {
            assert_projector(&projector(&vectors[..2]), &expected);
        }
    }
}

#[test]
fn bending_cluster_subspaces_survive_the_implicit_local_axis_reference_switch() {
    for vertical in [0.9 - 1e-12, 0.9, 0.9 + 1e-12, 1.0] {
        let axis = [(1.0_f64 - vertical * vertical).sqrt(), 0.0, vertical];
        for split in [0.0, 1e-8] {
            let result = solve_modal_frame_3d(&beam(axis, 1.0, 1.0, split, 6)).unwrap();
            check_complete(&result, axis, 1.0, split, 1);
        }
    }
}

#[test]
fn repeated_cluster_validation_cancellation_cannot_publish_a_partial_spectrum() {
    let axis = [1.0, 2.0, 3.0];
    let input = beam(axis, 1.0, 1.0, 0.0, 6);
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
            let result = solve_modal_frame_3d(&input);
            assert!(
                result.is_err(),
                "partial validation must not return a successful spectrum"
            );
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    check_complete(&solve_modal_frame_3d(&input).unwrap(), axis, 1.0, 0.0, 1);
}

#[test]
fn sparse_single_mode_lies_in_the_complete_repeated_low_mode_subspace() {
    let axis = [1.0, 2.0, 3.0];
    let mut input = beam(axis, 1.0, 1.0, 0.0, 96);
    let template = input.clone();
    input.nodes.clear();
    input.elements.clear();
    // 48 beams give 288 free DOFs, beyond the 256-DOF single-mode dense fallback.
    for copy in 0..48 {
        for node in &template.nodes {
            let mut node = node.clone();
            node.id = format!("{}-{copy}", node.id);
            node.x += copy as f64 * 3.0;
            input.nodes.push(node);
        }
        let mut element = template.elements[0].clone();
        element.id = format!("beam-{copy}");
        element.node_i = copy * 2;
        element.node_j = copy * 2 + 1;
        input.elements.push(element);
    }
    let complete = solve_modal_frame_3d(&input).unwrap();
    assert_eq!(complete.free_dofs.len(), 288);
    assert_eq!(complete.modes.len(), 96);
    input.mode_count = Some(1);
    let saw_iteration = Rc::new(Cell::new(false));
    let observed = Rc::clone(&saw_iteration);
    let single = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::ModalIteration {
                observed.set(true);
            }
        },
        || solve_modal_frame_3d(&input),
    )
    .unwrap();
    assert!(
        saw_iteration.get(),
        "the single-mode request must exercise sparse iteration"
    );
    assert_eq!(single.modes.len(), 1);
    let expected = roots(1.0, 0.0)[0];
    let whiten = |shape: &[f64]| {
        let mut vector: Vec<_> = complete
            .free_dofs
            .iter()
            .map(|dof| {
                shape[*dof]
                    * if dof % 6 < 3 {
                        0.5_f64.sqrt()
                    } else {
                        1.0 / 24.0_f64.sqrt()
                    }
            })
            .collect();
        let norm = vector.iter().map(|x| x * x).sum::<f64>().sqrt();
        for value in &mut vector {
            *value /= norm;
        }
        vector
    };
    let basis: Vec<_> = complete
        .modes
        .iter()
        .map(|mode| {
            assert!((mode.eigenvalue_rad_s_squared / expected - 1.0).abs() < 1e-10);
            whiten(&mode.shape)
        })
        .collect();
    let local = analytic_bending_projector(axis, false);
    for row in 0..288 {
        for column in 0..288 {
            let actual: f64 = basis.iter().map(|v| v[row] * v[column]).sum();
            let reference = if row / 6 == column / 6 {
                local[row % 6][column % 6]
            } else {
                0.0
            };
            assert!((actual - reference).abs() < 1e-9);
        }
    }
    let first = &single.modes[0];
    assert!((first.eigenvalue_rad_s_squared / expected - 1.0).abs() < 1e-8);
    let vector = whiten(&first.shape);
    let mut projected = vec![0.0; 288];
    for mode in basis {
        let coefficient: f64 = mode.iter().zip(&vector).map(|(a, b)| a * b).sum();
        for (value, basis_value) in projected.iter_mut().zip(mode) {
            *value += coefficient * basis_value;
        }
    }
    let error = vector
        .iter()
        .zip(projected)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(error < 1e-5, "sparse mode subspace error={error:e}");
}
