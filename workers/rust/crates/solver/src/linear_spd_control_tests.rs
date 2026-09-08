use super::*;
use crate::linear_algebra::add_at;
use crate::linear_solver_profile::SpdPreconditioner;
use crate::solver_control::{SolverControl, with_solver_control, with_solver_observer};

const STAGES: [SolverStage; 9] = [
    SolverStage::PcgRhsScale,
    SolverStage::PcgRhsNormalize,
    SolverStage::PcgDirectionCopy,
    SolverStage::PcgDot,
    SolverStage::PcgNorm,
    SolverStage::PcgVectorUpdate,
    SolverStage::PcgResidualUpdate,
    SolverStage::PcgDirectionUpdate,
    SolverStage::PcgSolutionScale,
];

fn system(size: usize) -> (SparseMatrix, Vec<f64>) {
    let mut matrix = SparseMatrix::with_uniform_row_capacity(size, 4);
    for row in 0..size {
        add_at(&mut matrix, row, row, 4.0 + (row % 7) as f64);
        if row + 1 < size {
            add_at(&mut matrix, row, row + 1, -0.7);
            add_at(&mut matrix, row + 1, row, -0.7);
        }
    }
    if size > 17 {
        add_at(&mut matrix, 0, 17, 0.1);
        add_at(&mut matrix, 17, 0, 0.1);
    }
    let rhs = (0..size).map(|i| 1.0 + (i % 11) as f64).collect();
    (matrix, rhs)
}

pub(super) fn interrupted<T>(
    stage: SolverStage,
    steps: usize,
    operation: impl FnOnce() -> Result<T, String>,
) {
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == stage && point.completed_steps == steps as u64 {
                cancel.request_cancel();
            }
        },
        || {
            let result = operation();
            assert!(result.is_err(), "raw PCG operation ignored {stage:?}");
            result
        },
    );
    let error = result
        .err()
        .expect("cancellation must escape the control scope");
    assert!(error.starts_with("solver cancelled"), "{error}");
    assert!(control.was_interrupted());
    let point = control.last_checkpoint().unwrap();
    assert_eq!(point.stage, stage);
    assert_eq!(point.completed_steps, steps as u64);
}

fn cancel_solve(stage: SolverStage) {
    let (matrix, rhs) = system(2049);
    let options = SpdSolveOptions::default();
    let compressed = matrix.compress(SpdPreconditioner::Jacobi).unwrap();
    interrupted(stage, 1024, || {
        solve_spd_compressed(&compressed, &rhs, &matrix, &options)
    });
}

#[test]
fn pcg_rhs_scale_cancels_inside_the_reduction() {
    cancel_solve(SolverStage::PcgRhsScale);
}

#[test]
fn pcg_rhs_normalize_cancels_inside_the_vector() {
    cancel_solve(SolverStage::PcgRhsNormalize);
}

#[test]
fn pcg_direction_copy_cancels_inside_the_vector() {
    cancel_solve(SolverStage::PcgDirectionCopy);
}

#[test]
fn pcg_dot_cancels_inside_the_reduction() {
    cancel_solve(SolverStage::PcgDot);
}

#[test]
fn pcg_norm_cancels_inside_the_reduction() {
    cancel_solve(SolverStage::PcgNorm);
}

#[test]
fn pcg_vector_update_cancels_inside_the_vector() {
    cancel_solve(SolverStage::PcgVectorUpdate);
}

#[test]
fn pcg_residual_update_cancels_inside_the_vector() {
    cancel_solve(SolverStage::PcgResidualUpdate);
}

#[test]
fn pcg_direction_update_cancels_inside_the_vector() {
    cancel_solve(SolverStage::PcgDirectionUpdate);
}

#[test]
fn pcg_solution_scale_cancels_before_exporting_the_solution() {
    cancel_solve(SolverStage::PcgSolutionScale);
}

#[test]
fn cancelled_pcg_does_not_fallback_or_poison_any_preconditioner() {
    for preconditioner in [
        SpdPreconditioner::Jacobi,
        SpdPreconditioner::SymmetricGaussSeidel,
        SpdPreconditioner::IncompleteCholesky,
    ] {
        for size in [49, 2049] {
            let (matrix, rhs) = system(size);
            let options = SpdSolveOptions {
                preconditioner,
                progress_interval: None,
            };
            let compressed = matrix.compress(preconditioner).unwrap();
            let reference = solve_spd_compressed(&compressed, &rhs, &matrix, &options).unwrap();
            assert!(
                reference.iterations > 0,
                "must exercise PCG, not dense fallback"
            );
            for stage in STAGES {
                interrupted(stage, size.min(1024), || {
                    solve_spd_compressed(&compressed, &rhs, &matrix, &options)
                });
                let recovered = with_solver_control(&SolverControl::default(), || {
                    solve_spd_compressed(&compressed, &rhs, &matrix, &options)
                })
                .unwrap();
                assert_eq!(recovered.iterations, reference.iterations);
                assert_eq!(
                    recovered.residual_norm.to_bits(),
                    reference.residual_norm.to_bits()
                );
                assert_eq!(
                    recovered
                        .solution
                        .iter()
                        .map(|v| v.to_bits())
                        .collect::<Vec<_>>(),
                    reference
                        .solution
                        .iter()
                        .map(|v| v.to_bits())
                        .collect::<Vec<_>>()
                );
            }
        }
    }
}

#[test]
fn zero_rhs_scale_is_cancellable_before_the_zero_solution_shortcut() {
    for size in [0, 49, 2049] {
        let (matrix, _) = system(size);
        let options = SpdSolveOptions::default();
        let compressed = matrix.compress(options.preconditioner).unwrap();
        interrupted(SolverStage::PcgRhsScale, size.min(1024), || {
            solve_spd_compressed(&compressed, &vec![0.0; size], &matrix, &options)
        });
    }
}

#[test]
fn prepared_solver_propagates_vector_cancellation_without_regularized_retry() {
    let (matrix, rhs) = system(2049);
    let prepared = crate::linear_algebra::PreparedSpdSolver::factor(matrix).unwrap();
    let expected = prepared.solve(&rhs).unwrap();
    for stage in STAGES {
        interrupted(stage, 1024, || prepared.solve(&rhs));
        assert_eq!(prepared.solve(&rhs).unwrap(), expected);
    }
}
