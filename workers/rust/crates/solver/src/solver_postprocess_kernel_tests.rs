use super::*;
use crate::solver_postprocess_tests::interrupted;
use std::cell::Cell;

const LENGTHS: [usize; 11] = [0, 1, 63, 64, 65, 127, 128, 129, 1023, 1024, 1025];

#[test]
fn collection_polls_entry_interior_and_final_blocks_without_visiting_the_tail() {
    for size in LENGTHS {
        for stage in [SolverStage::ResultNodes, SolverStage::ResultElements] {
            for steps in [0, size.min(64), size] {
                let visited = Cell::new(0);
                interrupted(stage, steps, || {
                    collect_results(
                        stage,
                        (0..size).map(|i| {
                            visited.set(visited.get() + 1);
                            i
                        }),
                    )
                });
                assert_eq!(visited.get(), steps);
            }
            assert_eq!(
                collect_results(stage, 0..size).unwrap(),
                (0..size).collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn cancelled_collection_drops_every_constructed_result() {
    struct Item<'a>(&'a Cell<usize>);
    impl Drop for Item<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let drops = Cell::new(0);
    interrupted(SolverStage::ResultElements, 64, || {
        collect_results(SolverStage::ResultElements, (0..129).map(|_| Item(&drops)))
    });
    assert_eq!(drops.get(), 64);
}

#[test]
fn summary_polls_each_block_without_returning_a_partial_accumulator() {
    for size in LENGTHS {
        for stage in [
            SolverStage::ResultNodeSummary,
            SolverStage::ResultElementSummary,
            SolverStage::ResultTotals,
            SolverStage::ResultRhsNorm,
        ] {
            for steps in [0, size.min(64), size] {
                let visited = Cell::new(0);
                interrupted(stage, steps, || {
                    fold_results(stage, 0..size, 0usize, |sum, value| {
                        visited.set(visited.get() + 1);
                        sum + value
                    })
                });
                assert_eq!(visited.get(), steps);
            }
        }
    }
}

fn same_bits(actual: f64, expected: f64) {
    assert!(
        actual.to_bits() == expected.to_bits() || actual.is_nan() && expected.is_nan(),
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn summaries_preserve_sequential_rounding_signed_zero_and_nonfinite_behavior() {
    for size in LENGTHS {
        let values: Vec<_> = (0..size)
            .map(|i| [1e16, 1.0, -1e16, -0.0, 1e-200][i % 5])
            .collect();
        for tail in [0.0, -0.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            let mut values = values.clone();
            if let Some(last) = values.last_mut() {
                *last = tail;
            }
            same_bits(
                fold_results(
                    SolverStage::ResultTotals,
                    values.iter().copied(),
                    -0.0,
                    |a, b| a + b,
                )
                .unwrap(),
                values.iter().copied().sum::<f64>(),
            );
            same_bits(
                max_results(SolverStage::ResultNodeSummary, &values, |v| *v).unwrap(),
                values.iter().copied().fold(0.0_f64, f64::max),
            );
            same_bits(
                fold_results(
                    SolverStage::ResultRhsNorm,
                    values.iter().map(|v| v * v),
                    -0.0,
                    |a, b| a + b,
                )
                .unwrap()
                .sqrt(),
                values.iter().map(|v| v * v).sum::<f64>().sqrt(),
            );
        }
    }
}

#[test]
fn solution_restoration_retains_prescribed_and_reduced_order() {
    for size in LENGTHS {
        let prescribed: Vec<_> = (0..size).map(|i| (2 * i, -(i as f64))).collect();
        let free: Vec<_> = (0..size).map(|i| 2 * (size - i) - 1).collect();
        let reduced: Vec<_> = (0..size).map(|i| i as f64 * 0.125).collect();
        let mut reference = vec![0.0; size * 2];
        for &(i, value) in &prescribed {
            reference[i] = value;
        }
        for (i, &dof) in free.iter().enumerate() {
            reference[dof] = reduced[i];
        }
        let result = restore_solution(size * 2, &prescribed, &free, &reduced).unwrap();
        for (actual, expected) in result.iter().zip(reference) {
            same_bits(*actual, expected);
        }
        for stage in [SolverStage::ResultPrescribed, SolverStage::ResultFreeDofs] {
            for steps in [0, size.min(64), size] {
                interrupted(stage, steps, || {
                    restore_solution(size * 2, &prescribed, &free, &reduced)
                });
            }
        }
    }
}

#[path = "solver_postprocess_benchmark.rs"]
mod benchmark;

#[test]
fn maximum_slice_polling_preserves_projection_order_and_special_values() {
    for size in LENGTHS {
        for stage in [
            SolverStage::ResultNodeSummary,
            SolverStage::ResultElementSummary,
        ] {
            let values = vec![0.0; size];
            for steps in [0, size.min(1024), size] {
                let visited = Cell::new(0);
                interrupted(stage, steps, || {
                    max_results(stage, &values, |v| {
                        visited.set(visited.get() + 1);
                        *v
                    })
                });
                assert_eq!(visited.get(), steps);
            }
            for value in [0.0, -0.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                let values = vec![value; size];
                same_bits(
                    max_results(stage, &values, |v| *v).unwrap(),
                    values.iter().copied().fold(0.0_f64, f64::max),
                );
            }
        }
    }
}
