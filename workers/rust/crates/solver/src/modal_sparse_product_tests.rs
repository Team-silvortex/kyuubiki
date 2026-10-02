use super::*;
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

fn operator(entries: &[(usize, usize, f64)], mass: &[f64]) -> SparseMassNormalizedOperator {
    let mut matrix = SparseMatrix::new(mass.len());
    for &(row, column, value) in entries {
        add_at(&mut matrix, row, column, value);
    }
    SparseMassNormalizedOperator::new(&matrix, mass).unwrap()
}

#[test]
fn compensated_modal_sums_retain_small_terms_under_column_reordering() {
    let entries = [
        (0, 0, 1e16),
        (0, 1, 1.0),
        (0, 2, -1e16),
        (1, 0, 1.0),
        (1, 1, 2.0),
        (2, 0, -1e16),
        (2, 2, 2e16),
    ];
    let original = operator(&entries, &[1.0; 3]);
    assert_eq!(original.apply(&[1.0; 3]).unwrap()[0], 0.0);
    for order in [[0, 1, 2], [0, 2, 1], [2, 1, 0]] {
        let permuted: Vec<_> = entries
            .iter()
            .map(|&(r, c, v)| (order[r], order[c], v))
            .collect();
        let result = operator(&permuted, &[1.0; 3])
            .apply_compensated(&[1.0; 3])
            .unwrap();
        assert_eq!(result[order[0]], 1.0);
        assert_eq!(result[order[1]], 3.0);
        assert_eq!(result[order[2]], 1e16);
    }
}

#[test]
fn compensated_modal_products_retain_fma_roundoff() {
    let epsilon = 2.0_f64.powi(-27);
    let operator = operator(
        &[
            (0, 0, 1.0 + epsilon),
            (0, 1, -1.0),
            (1, 0, -1.0),
            (1, 1, 2.0),
        ],
        &[1.0; 2],
    );
    let vector = [1.0 - epsilon, 1.0];
    assert_eq!(operator.apply(&vector).unwrap()[0], 0.0);
    assert_eq!(
        operator.apply_compensated(&vector).unwrap()[0],
        -epsilon * epsilon
    );
}

#[test]
fn compensated_modal_products_retain_mass_scaling_roundoff() {
    let inverse = 0.5_f64.sqrt().recip();
    let input = 1.0 + 2.0_f64.powi(-27);
    let high = input * inverse;
    let operator = operator(
        &[
            (0, 0, 1.0),
            (0, 1, -high),
            (1, 0, -high),
            (1, 1, high * high + 1.0),
        ],
        &[0.5, 1.0],
    );
    let expected = input.mul_add(inverse, -high) * inverse;
    assert_ne!(expected, 0.0);
    assert_eq!(operator.apply(&[input, 1.0]).unwrap()[0], 0.0);
    assert_eq!(
        operator.apply_compensated(&[input, 1.0]).unwrap()[0],
        expected
    );
}

#[test]
fn compensated_modal_products_reject_invalid_inputs_and_overflow() {
    let unit = operator(&[(0, 0, 1.0)], &[1.0]);
    for vector in [vec![], vec![1.0, 1.0], vec![f64::NAN], vec![f64::INFINITY]] {
        assert!(unit.apply_compensated(&vector).is_err());
    }
    let scaled = operator(&[(0, 0, 1.0)], &[1e-300]);
    assert!(
        scaled
            .apply_compensated(&[1e200])
            .unwrap_err()
            .contains("stiffness product")
    );
    let stiff = operator(&[(0, 0, 1e200)], &[1.0]);
    assert!(
        stiff
            .apply_compensated(&[1e200])
            .unwrap_err()
            .contains("stiffness product")
    );
    assert_eq!(unit.apply_compensated(&[2.0]).unwrap(), [2.0]);
}

#[test]
fn compensated_modal_scaling_and_rows_cancel_inside_the_product_then_replay() {
    let entries: Vec<_> = (0..129).map(|i| (i, i, 2.0)).collect();
    let operator = operator(&entries, &[1.0; 129]);
    for occurrence in [1, 2] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let seen = Rc::new(Cell::new(0));
        let observed = seen.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::SparseMatvec && point.completed_steps == 64 {
                    observed.set(observed.get() + 1);
                    if observed.get() == occurrence {
                        cancel.request_cancel();
                    }
                }
            },
            || {
                let result = operator.apply_compensated(&[1.0; 129]);
                assert!(
                    result.is_err(),
                    "the product must stop before returning success"
                );
                result
            },
        )
        .unwrap_err();
        assert_eq!(seen.get(), occurrence);
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(operator.apply_compensated(&[1.0; 129]).unwrap(), [2.0; 129]);
    }
}

#[test]
fn compensated_modal_wide_row_cancels_without_resetting_the_accumulator() {
    let mut entries: Vec<_> = (0..2050).map(|i| (i, i, 1.0)).collect();
    entries.extend((1..2050).map(|i| (0, i, 1.0)));
    let operator = operator(&entries, &vec![1.0; 2050]);
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::SparseMatvecRow && point.completed_steps == 1024 {
                cancel.request_cancel();
            }
        },
        || {
            let result = operator.apply_compensated(&vec![1.0; 2050]);
            assert!(result.is_err());
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    let result = operator.apply_compensated(&vec![1.0; 2050]).unwrap();
    assert_eq!(result[0], 2050.0);
    assert!(result[1..].iter().all(|v| *v == 1.0));
}
