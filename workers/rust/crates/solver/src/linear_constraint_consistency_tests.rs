use super::{SparseMatrix, add_at, reduce_sparse_system_with_prescribed, solve_spd_system};
use crate::chain_tridiagonal::solve_path_with_prescribed;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

type Solution = Result<Vec<f64>, String>;

fn solve_both(prescribed: &[(usize, f64)]) -> (Solution, Solution) {
    (solve_chain(prescribed), solve_sparse(prescribed))
}

fn solve_chain(prescribed: &[(usize, f64)]) -> Solution {
    let edges = [(0, 1), (1, 2)];
    solve_path_with_prescribed(
        3,
        &edges,
        |&(first, second)| (first, second),
        |_| Ok([[1.0, -1.0], [-1.0, 1.0]]),
        &[0.0; 3],
        prescribed,
    )
    .unwrap()
}

fn solve_sparse(prescribed: &[(usize, f64)]) -> Solution {
    let edges = [(0, 1), (1, 2)];
    let mut matrix = SparseMatrix::new(3);
    for (first, second) in edges {
        for (row, col, value) in [
            (first, first, 1.0),
            (first, second, -1.0),
            (second, first, -1.0),
            (second, second, 1.0),
        ] {
            add_at(&mut matrix, row, col, value);
        }
    }
    reduce_sparse_system_with_prescribed(&matrix, &[0.0; 3], prescribed).and_then(
        |(reduced, rhs, free)| {
            let solved = solve_spd_system(&reduced, &rhs)?;
            let mut values = vec![0.0; 3];
            for &(index, value) in prescribed {
                values[index] = value;
            }
            for (&index, &value) in free.iter().zip(&solved) {
                values[index] = value;
            }
            Ok(values)
        },
    )
}

#[test]
fn prescribed_consistency_rejects_distinct_duplicate_values_at_every_scale_and_order() {
    for (first, second) in [
        (0.0, f64::from_bits(1)),
        (-f64::from_bits(1), f64::from_bits(1)),
        (
            f64::MIN_POSITIVE,
            f64::from_bits(f64::MIN_POSITIVE.to_bits() + 1),
        ),
        (1.0, f64::from_bits(1.0_f64.to_bits() + 1)),
        (-1.0, -f64::from_bits(1.0_f64.to_bits() + 1)),
        (f64::from_bits(f64::MAX.to_bits() - 1), f64::MAX),
    ] {
        for (left, right) in [(first, second), (second, first)] {
            let (chain, sparse) = solve_both(&[(0, left), (0, right), (1, 0.0), (2, 0.0)]);
            assert!(chain.unwrap_err().contains("conflicting prescribed values"));
            assert!(
                sparse
                    .unwrap_err()
                    .contains("conflicting prescribed values")
            );
        }
    }
}

#[test]
fn prescribed_consistency_keeps_equal_repeats_and_signed_zeros_idempotent() {
    for value in [
        0.0,
        -0.0,
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        1.0,
        -1.0,
        2.0_f64.powi(500),
        -2.0_f64.powi(500),
    ] {
        let (chain, sparse) = solve_both(&[(1, value), (1, value)]);
        assert_eq!(chain.unwrap(), [value; 3]);
        assert_eq!(sparse.unwrap(), [value; 3]);
        let (chain, sparse) = solve_both(&[(1, value)]);
        assert_eq!(chain.unwrap(), [value; 3]);
        assert_eq!(sparse.unwrap(), [value; 3]);
    }
    for zeros in [[0.0, -0.0], [-0.0, 0.0]] {
        let (chain, sparse) = solve_both(&[(1, zeros[0]), (1, zeros[1])]);
        assert_eq!(chain.unwrap(), [0.0; 3]);
        assert_eq!(sparse.unwrap(), [0.0; 3]);
    }
    for value in [f64::MAX, -f64::MAX] {
        let (chain, sparse) = solve_both(&[(0, value), (1, value), (2, value), (1, value)]);
        assert_eq!(chain.unwrap(), [value; 3]);
        assert_eq!(sparse.unwrap(), [value; 3]);
    }
}

#[test]
fn prescribed_consistency_keeps_distinct_values_on_different_dofs_independent() {
    let next = f64::from_bits(1.0_f64.to_bits() + 1);
    let (chain, sparse) = solve_both(&[(0, 1.0), (2, next), (0, 1.0), (2, next)]);
    let chain = chain.unwrap();
    let sparse = sparse.unwrap();
    assert_eq!(chain, sparse);
    assert_eq!(chain[0].to_bits(), 1.0_f64.to_bits());
    assert_eq!(chain[2].to_bits(), next.to_bits());
    let quantum = f64::from_bits(1);
    let (chain, sparse) = solve_both(&[(0, 0.0), (2, 2.0 * quantum)]);
    assert_eq!(chain.unwrap(), [0.0, quantum, 2.0 * quantum]);
    assert_eq!(sparse.unwrap(), [0.0, quantum, 2.0 * quantum]);
}

#[test]
fn prescribed_consistency_rejects_invalid_values_even_in_repeated_assignments() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for values in [[0.0, invalid], [invalid, 0.0], [invalid, invalid]] {
            let (chain, sparse) = solve_both(&[(1, values[0]), (1, values[1])]);
            assert!(chain.unwrap_err().contains("non-finite prescribed value"));
            assert!(sparse.unwrap_err().contains("non-finite prescribed value"));
        }
    }
    let (chain, sparse) = solve_both(&[(3, 0.0), (3, 0.0)]);
    assert!(chain.unwrap_err().contains("out-of-range prescribed value"));
    assert!(
        sparse
            .unwrap_err()
            .contains("out-of-range prescribed value")
    );
}

#[test]
fn prescribed_consistency_cancellation_keeps_long_equal_repeats_reusable() {
    let prescribed = vec![(1, 1.0); 128];
    for solve in [solve_chain as fn(&[(usize, f64)]) -> Solution, solve_sparse] {
        for step in [0, 64, 128] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let result = with_solver_observer(
                &control,
                move |point| {
                    if point.stage == SolverStage::ConstraintIndex && point.completed_steps == step
                    {
                        cancel.request_cancel();
                    }
                },
                || solve(&prescribed),
            );
            assert!(result.unwrap_err().contains("cancelled"));
            assert!(control.was_interrupted());
            assert_eq!(solve(&prescribed).unwrap(), [1.0; 3]);
        }
    }
}
