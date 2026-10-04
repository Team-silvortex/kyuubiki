use kyuubiki_protocol::{SolveModalFrame3dRequest, SolveModalFrame3dResult};
use kyuubiki_solver::solve_modal_frame_3d;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc};

const ROW_LENGTH: usize = 129;
const ROW_STIFFNESS: f64 = 10_000.0;

#[path = "modal_connected_reliability/nonuniform.rs"]
mod nonuniform;

fn member(i: usize, j: usize, stiffness: f64, density: f64) -> Value {
    json!({"id":format!("member-{i}-{j}"), "node_i":i, "node_j":j,
        "area":1.0, "youngs_modulus":stiffness, "shear_modulus":0.4*stiffness,
        "torsion_constant":1.0/12.0, "moment_of_inertia_y":1.0/12.0,
        "moment_of_inertia_z":1.0/12.0, "density":density})
}

fn coupled_rows(rung: f64, contrast: f64, count: usize) -> SolveModalFrame3dRequest {
    coupled_rows_with_mass(ROW_LENGTH, [1.0, 1.0], rung, contrast, count)
}

fn coupled_rows_with_mass(
    row_length: usize,
    masses: [f64; 2],
    rung: f64,
    contrast: f64,
    count: usize,
) -> SolveModalFrame3dRequest {
    let mut nodes = Vec::new();
    let mut elements = Vec::new();
    for (row, mass) in masses.into_iter().enumerate() {
        for position in 0..row_length {
            let root = nodes.len();
            for tip in [false, true] {
                nodes.push(json!({"id":format!("node-{}", nodes.len()),
                    "x":f64::from(tip), "y":position as f64, "z":row as f64,
                    "fix_x":!tip, "fix_y":true, "fix_z":true,
                    "fix_rx":true, "fix_ry":true, "fix_rz":true,
                    "load_x":0.0, "load_y":0.0, "load_z":0.0,
                    "moment_x":0.0, "moment_y":0.0, "moment_z":0.0}));
            }
            let neighbors = usize::from(position > 0) + usize::from(position + 1 < row_length);
            // Account for couplers so all tips in a row have the prescribed mass.
            let density = 2.0 * mass - 0.1 * (neighbors + 1) as f64;
            let grounding = mass * (1.0 + if row == 0 { -contrast } else { contrast });
            elements.push(member(root, root + 1, grounding, density));
            if position > 0 {
                elements.push(member(root - 1, root + 1, ROW_STIFFNESS * mass, 0.1));
            }
            if row == 1 {
                elements.push(member(root + 1 - 2 * row_length, root + 1, rung, 0.1));
            }
        }
    }
    serde_json::from_value(json!({"nodes":nodes, "elements":elements, "mode_count":count})).unwrap()
}

fn roots(rung: f64, contrast: f64) -> [f64; 2] {
    let radius = rung.hypot(contrast);
    [1.0 + rung - radius, 1.0 + rung + radius]
}

fn check_low_shape(result: &SolveModalFrame3dResult, rung: f64, contrast: f64) {
    assert_eq!(result.modes.len(), 1);
    assert_eq!(result.free_dofs.len(), 2 * ROW_LENGTH);
    let mode = &result.modes[0];
    assert!((mode.eigenvalue_rad_s_squared / roots(rung, contrast)[0] - 1.0).abs() < 1e-8);
    assert_eq!(mode.shape.len(), 4 * ROW_LENGTH * 6);
    for (dof, value) in mode.shape.iter().enumerate() {
        assert!(value.is_finite());
        if !result.free_dofs.contains(&dof) {
            assert_eq!(*value, 0.0);
        }
    }
    let row_values: Vec<Vec<f64>> = (0..2)
        .map(|row| {
            (0..ROW_LENGTH)
                .map(|j| mode.shape[(2 * (row * ROW_LENGTH + j) + 1) * 6])
                .collect()
        })
        .collect();
    let averages: Vec<_> = row_values
        .iter()
        .map(|row| row.iter().sum::<f64>() / ROW_LENGTH as f64)
        .collect();
    let variation = row_values
        .iter()
        .zip(&averages)
        .flat_map(|(row, average)| row.iter().map(move |x| (x - average).powi(2)))
        .sum::<f64>()
        .sqrt();
    assert!(variation < 1e-6, "within-row variation={variation:e}");
    let lambda = mode.eigenvalue_rad_s_squared;
    let [a, b] = [averages[0], averages[1]];
    let residual = ((1.0 - contrast + rung - lambda) * a - rung * b)
        .hypot((1.0 + contrast + rung - lambda) * b - rung * a)
        / a.hypot(b);
    assert!(residual < 1e-6, "row-pair residual={residual:e}");
    assert!((mode.shape.iter().map(|x| x * x).sum::<f64>() - 1.0).abs() < 1e-10);
    assert!((mode.natural_frequency_hz * mode.period_s - 1.0).abs() < 1e-12);
}

#[test]
fn connected_weak_rungs_match_the_two_by_two_analytic_reference() {
    for rung in [1e-2, 1e-4, 1e-6] {
        let result = solve_modal_frame_3d(&coupled_rows(rung, 0.0, 2)).unwrap();
        assert_eq!(result.free_dofs.len(), 2 * ROW_LENGTH);
        assert_eq!(result.modes.len(), 2);
        for (mode, expected) in result.modes.iter().zip(roots(rung, 0.0)) {
            assert!((mode.eigenvalue_rad_s_squared / expected - 1.0).abs() < 1e-8);
        }
    }
}

#[test]
fn connected_weak_rungs_do_not_stall_the_sparse_single_mode_request() {
    let visited = Rc::new(Cell::new(false));
    let observed = visited.clone();
    let result = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::ModalIteration {
                observed.set(true);
            }
        },
        || solve_modal_frame_3d(&coupled_rows(1e-4, 0.0, 1)),
    )
    .unwrap();
    assert!(visited.get(), "must exercise the sparse inverse iteration");
    check_low_shape(&result, 1e-4, 0.0);
}

#[test]
fn connected_sparse_modes_follow_the_grounding_parameter_sweep() {
    for rung in [1e-2, 1e-4, 1e-6] {
        for contrast in [-0.25, -0.01, 0.0, 0.01, 0.25] {
            let single = solve_modal_frame_3d(&coupled_rows(rung, contrast, 1)).unwrap();
            check_low_shape(&single, rung, contrast);
            let dense = solve_modal_frame_3d(&coupled_rows(rung, contrast, 2)).unwrap();
            assert_eq!(dense.modes.len(), 2);
            for (mode, expected) in dense.modes.iter().zip(roots(rung, contrast)) {
                assert!((mode.eigenvalue_rad_s_squared / expected - 1.0).abs() < 1e-8);
            }
            assert!(
                (single.modes[0].eigenvalue_rad_s_squared
                    / dense.modes[0].eigenvalue_rad_s_squared
                    - 1.0)
                    .abs()
                    < 1e-8
            );
        }
    }
}

#[test]
fn connected_sparse_modes_preserve_common_material_scaling() {
    for scale in [1e-60, 1e60] {
        let mut input = coupled_rows(1e-4, 0.01, 1);
        for member in &mut input.elements {
            member.youngs_modulus *= scale;
            member.shear_modulus *= scale;
            member.density *= scale;
        }
        check_low_shape(&solve_modal_frame_3d(&input).unwrap(), 1e-4, 0.01);
    }
}

#[test]
fn connected_sparse_modes_are_invariant_to_node_and_member_reordering() {
    let mut input = coupled_rows(1e-4, 0.01, 1);
    let original = solve_modal_frame_3d(&input).unwrap();
    input.nodes.reverse();
    let last = input.nodes.len() - 1;
    for element in &mut input.elements {
        let (i, j) = (element.node_i, element.node_j);
        element.node_i = last - j;
        element.node_j = last - i;
    }
    input.elements.reverse();
    let reordered = solve_modal_frame_3d(&input).unwrap();
    let mut shape: Vec<_> = reordered.modes[0]
        .shape
        .chunks_exact(6)
        .rev()
        .flatten()
        .copied()
        .collect();
    let overlap: f64 = original.modes[0]
        .shape
        .iter()
        .zip(&shape)
        .map(|(a, b)| a * b)
        .sum();
    shape.iter_mut().for_each(|x| *x *= overlap.signum());
    let error = original.modes[0]
        .shape
        .iter()
        .zip(&shape)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(error < 1e-5, "reordered shape error={error:e}");
    assert!(
        (reordered.modes[0].eigenvalue_rad_s_squared / roots(1e-4, 0.01)[0] - 1.0).abs() < 1e-8
    );
}

#[test]
fn connected_sparse_projection_cancels_without_partial_success_and_replays() {
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ModalSweep {
                cancel.request_cancel();
            }
        },
        || {
            let result = solve_modal_frame_3d(&coupled_rows(1e-4, 0.01, 1));
            assert!(result.is_err());
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    check_low_shape(
        &solve_modal_frame_3d(&coupled_rows(1e-4, 0.01, 1)).unwrap(),
        1e-4,
        0.01,
    );
}
