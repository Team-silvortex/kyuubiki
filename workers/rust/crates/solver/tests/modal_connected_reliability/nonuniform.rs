use super::*;

fn reference(masses: [f64; 2], rung: f64, contrast: f64) -> [f64; 2] {
    let a = 1.0 - contrast + rung / masses[0];
    let d = 1.0 + contrast + rung / masses[1];
    let b = rung / (masses[0].sqrt() * masses[1].sqrt());
    let center = 0.5 * (a + d);
    let radius = (0.5 * (a - d)).hypot(b);
    [center - radius, center + radius]
}

fn check(
    result: &SolveModalFrame3dResult,
    row_length: usize,
    masses: [f64; 2],
    rung: f64,
    contrast: f64,
) {
    assert_eq!(result.free_dofs.len(), 2 * row_length);
    assert_eq!(result.modes.len(), 1);
    let mode = &result.modes[0];
    assert!(
        (mode.eigenvalue_rad_s_squared / reference(masses, rung, contrast)[0] - 1.0).abs() < 1e-8
    );
    assert_eq!(mode.shape.len(), 4 * row_length * 6);
    let mut average = [0.0; 2];
    for (row, value) in average.iter_mut().enumerate() {
        *value = (0..row_length)
            .map(|j| mode.shape[(2 * (row * row_length + j) + 1) * 6])
            .sum::<f64>()
            / row_length as f64;
    }
    let mut variation = 0.0;
    let mut norm = 0.0;
    for (node, values) in mode.shape.chunks_exact(6).enumerate() {
        for (dof, value) in values.iter().enumerate() {
            assert!(value.is_finite());
            norm += value * value;
            if node % 2 == 0 || dof != 0 {
                assert_eq!(*value, 0.0);
            }
        }
        if node % 2 == 1 {
            variation += (values[0] - average[node / (2 * row_length)]).powi(2);
        }
    }
    assert!(variation.sqrt() < 1e-6);
    assert!((norm - 1.0).abs() < 1e-10);
    let a = average[0] * masses[0].sqrt();
    let b = average[1] * masses[1].sqrt();
    let coupling = rung / (masses[0].sqrt() * masses[1].sqrt());
    let lambda = mode.eigenvalue_rad_s_squared;
    let residual = ((1.0 - contrast + rung / masses[0] - lambda) * a - coupling * b)
        .hypot((1.0 + contrast + rung / masses[1] - lambda) * b - coupling * a)
        / a.hypot(b);
    assert!(residual < 1e-6, "mass-weighted row residual={residual:e}");
}

#[test]
fn unequal_row_masses_preserve_sparse_dense_and_analytic_low_modes() {
    for masses in [[1.0, 4.0], [4.0, 1.0], [0.5, 10.0]] {
        for contrast in [0.0, 0.01] {
            let single =
                solve_modal_frame_3d(&coupled_rows_with_mass(67, masses, 1e-4, contrast, 1))
                    .unwrap();
            check(&single, 67, masses, 1e-4, contrast);
            let dense =
                solve_modal_frame_3d(&coupled_rows_with_mass(67, masses, 1e-4, contrast, 2))
                    .unwrap();
            assert_eq!(dense.modes.len(), 2);
            for (mode, expected) in dense.modes.iter().zip(reference(masses, 1e-4, contrast)) {
                assert!((mode.eigenvalue_rad_s_squared / expected - 1.0).abs() < 1e-8);
            }
        }
    }
}

#[test]
fn connected_unequal_masses_use_one_ic0_factor_for_all_sparse_inverse_steps() {
    let counts = Rc::new(Cell::new([0; 4]));
    let observed = counts.clone();
    let input = coupled_rows_with_mass(513, [1.0, 4.0], 1e-4, 0.0, 1);
    let result = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.completed_steps == 0 {
                let mut values = observed.get();
                match point.stage {
                    SolverStage::IncompleteCholeskyFactor => values[0] += 1,
                    SolverStage::SparseIteration => values[1] += 1,
                    SolverStage::DenseFactor => values[2] += 1,
                    SolverStage::ModalIteration => values[3] += 1,
                    _ => (),
                }
                observed.set(values);
            }
        },
        || solve_modal_frame_3d(&input),
    )
    .unwrap();
    assert_eq!(counts.get()[0], 1, "one reusable IC(0) factor is required");
    assert!(
        counts.get()[1] > 1,
        "must exercise multiple inner PCG solves"
    );
    assert_eq!(counts.get()[2], 0, "must not use a dense fallback");
    assert_eq!(counts.get()[3], 2, "both modal probes must execute");
    check(&result, 513, [1.0, 4.0], 1e-4, 0.0);
}

#[test]
fn mass_normalization_and_sparse_factor_cancellation_replay_without_partial_state() {
    let input = coupled_rows_with_mass(513, [1.0, 4.0], 1e-4, 0.01, 1);
    for (stage, boundary) in [
        (SolverStage::SparseMatrixScale, 64),
        (SolverStage::IncompleteCholeskyFactor, 64),
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
                let result = solve_modal_frame_3d(&input);
                assert!(result.is_err());
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        check(
            &solve_modal_frame_3d(&input).unwrap(),
            513,
            [1.0, 4.0],
            1e-4,
            0.01,
        );
    }
}
