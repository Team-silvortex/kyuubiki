use super::*;
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::modal_math::jacobi_eigenpairs;
use crate::modal_sparse::reduce_sparse_modal_system;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

#[path = "modal_frame_conditioning_tests.rs"]
mod conditioning;

fn bending(segments: usize) -> ReducedSparseModalSystem {
    let size = 2 * segments;
    let mut matrix = SparseMatrix::new(size);
    let element = [
        [12.0, 6.0, -12.0, 6.0],
        [6.0, 4.0, -6.0, 2.0],
        [-12.0, -6.0, 12.0, -6.0],
        [6.0, 2.0, -6.0, 4.0],
    ];
    for segment in 0..segments {
        for (row, values) in element.iter().enumerate() {
            for (column, &value) in values.iter().enumerate() {
                let (row, column) = (2 * segment + row, 2 * segment + column);
                if row >= 2 && column >= 2 {
                    add_at(&mut matrix, row - 2, column - 2, value);
                }
            }
        }
    }
    let mass: Vec<_> = (0..size)
        .map(|i| {
            (if i % 2 == 0 { 1.0 } else { 1.0 / 12.0 }) * if i >= size - 2 { 0.5 } else { 1.0 }
        })
        .collect();
    reduce_sparse_modal_system(&matrix, &mass, &[]).unwrap()
}

#[test]
fn slender_bending_refinement_never_replaces_a_better_physical_residual() {
    let system = bending(128);
    let seed = jacobi_eigenpairs(system.operator.dense_fallback_matrix().unwrap()).unwrap();
    let mut previous = f64::INFINITY;
    let mut retained = None;
    for budget in 0..=4 {
        let mut pairs = seed[..6].to_vec();
        let counts = Rc::new(Cell::new([0; 4]));
        let observed = counts.clone();
        let outcome = with_solver_observer(
            &SolverControl::default(),
            move |point| {
                if point.completed_steps == 0 {
                    let mut counts = observed.get();
                    match point.stage {
                        SolverStage::LinearPrepare => counts[0] += 1,
                        SolverStage::DenseFactor => counts[1] += 1,
                        SolverStage::ModalValidation => counts[2] += 1,
                        SolverStage::SparseMatvec => counts[3] += 1,
                        _ => {}
                    }
                    observed.set(counts);
                }
            },
            || refine_with_budget(&system, &mut pairs, 1e-8, budget),
        );
        let applied = system.operator.apply_compensated(&pairs[0].1).unwrap();
        let relative = relative_residual(pairs[0].0, &pairs[0].1, &applied).unwrap();
        assert!(
            relative <= previous,
            "budget={budget}, previous={previous:e}, relative={relative:e}"
        );
        if budget == 1 {
            assert!(
                relative < previous * 0.01,
                "the first correction must still improve"
            );
        }
        if budget > 1 && relative == previous {
            assert_eq!(
                Some(&pairs[0]),
                retained.as_ref(),
                "rejected pairs stay intact"
            );
        }
        retained = Some(pairs[0].clone());
        previous = relative;
        let error = outcome.unwrap_err();
        assert!(error.contains(&format!("within {budget} steps")), "{error}");
        let preparations = usize::from(budget > 0);
        assert_eq!(
            counts.get(),
            [preparations, preparations, budget, 1 + 3 * budget]
        );
    }
}
