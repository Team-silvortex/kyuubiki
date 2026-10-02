use super::*;
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

fn operator(entries: &[(usize, usize, f64)], mass: &[f64]) -> SparseMassNormalizedOperator {
    let mut stiffness = SparseMatrix::new(mass.len());
    for &(row, column, value) in entries {
        add_at(&mut stiffness, row, column, value);
    }
    SparseMassNormalizedOperator::new(&stiffness, mass).unwrap()
}

fn assert_relative(actual: f64, expected: f64) {
    assert!(actual.is_finite(), "non-finite result: {actual}");
    assert!(
        (actual / expected - 1.0).abs() < 1e-14,
        "{actual:e} != {expected:e}"
    );
}

#[test]
fn sparse_products_retain_weak_couplings_under_mass_and_column_reordering() {
    for coupling in [-1e-300, -1e-220] {
        let entries = [
            (0, 0, 1e200),
            (1, 1, 1e-200),
            (0, 1, coupling),
            (1, 0, coupling),
        ];
        for order in [[0, 1], [1, 0]] {
            let permuted: Vec<_> = entries
                .iter()
                .map(|&(row, column, value)| (order[row], order[column], value))
                .collect();
            let mut mass = [0.0; 2];
            mass[order[0]] = 1e200;
            mass[order[1]] = 1e-200;
            let op = operator(&permuted, &mass);
            for column in 0..2 {
                let mut basis = [0.0; 2];
                basis[column] = 1.0;
                for product in [op.apply(&basis), op.apply_compensated(&basis)] {
                    let product = product.unwrap();
                    assert_relative(product[column], 1.0);
                    assert_relative(product[1 - column], coupling);
                }
            }
        }
    }
}

#[test]
fn sparse_products_rescue_representable_results_after_input_scaling_underflow() {
    let op = operator(&[(0, 0, 1e308)], &[1e200]);
    for value in [1e-250, -1e-250] {
        assert_eq!(value * op.inverse_mass_sqrt[0], 0.0);
        for result in [op.apply(&[value]), op.apply_compensated(&[value])] {
            assert_relative(result.unwrap()[0], value.signum() * 1e-142);
        }
    }
}

#[test]
fn sparse_products_rescue_representable_results_after_input_scaling_overflow() {
    let op = operator(&[(0, 0, 1e-300)], &[1e-300]);
    for value in [1e200, -1e200] {
        assert!(!(value * op.inverse_mass_sqrt[0]).is_finite());
        for result in [op.apply(&[value]), op.apply_compensated(&[value])] {
            assert_relative(result.unwrap()[0], value);
        }
    }
}

#[test]
fn sparse_products_rescue_representable_results_after_stiffness_product_overflow() {
    let op = operator(&[(0, 0, 1e308)], &[1e200]);
    for value in [1e200, -1e200] {
        let scaled = value * op.inverse_mass_sqrt[0];
        assert!(!(1e308 * scaled).is_finite());
        for result in [op.apply(&[value]), op.apply_compensated(&[value])] {
            assert_relative(result.unwrap()[0], value.signum() * 1e308);
        }
    }
}

#[test]
fn sparse_products_match_an_independent_reference_at_extreme_common_scales() {
    for scale in [1e-318, 1e-200, 1.0, 1e200] {
        let op = operator(
            &[
                (0, 0, 4.0 * scale),
                (0, 1, -scale),
                (1, 0, -scale),
                (1, 1, 4.0 * scale),
            ],
            &[scale; 2],
        );
        for result in [op.apply(&[1.25, -0.5]), op.apply_compensated(&[1.25, -0.5])] {
            let result = result.unwrap();
            assert_relative(result[0], 5.5);
            assert_relative(result[1], -3.25);
        }
    }
}

#[test]
fn range_safe_compensation_retains_fma_terms_after_extreme_common_scaling() {
    let epsilon = 2.0_f64.powi(-27);
    for scale in [2.0_f64.powi(-600), 2.0_f64.powi(600)] {
        let op = operator(
            &[
                (0, 0, (1.0 + epsilon) * scale),
                (0, 1, -scale),
                (1, 0, -scale),
                (1, 1, 2.0 * scale),
            ],
            &[scale; 2],
        );
        assert!(!op.staged_range_safe);
        let vector = [1.0 - epsilon, 1.0];
        assert_eq!(op.apply(&vector).unwrap()[0], 0.0);
        assert_eq!(
            op.apply_compensated(&vector).unwrap()[0],
            -epsilon * epsilon
        );
    }
}

#[test]
fn ordinary_matrix_routes_extreme_vectors_without_retaining_per_call_state() {
    let op = operator(&[(0, 0, 2.0)], &[0.5]);
    assert!(op.staged_range_safe);
    let minimum = f64::from_bits(1);
    assert_eq!(
        legacy_staged_product(&op, &[3.0 * minimum])[0].to_bits(),
        11
    );
    for input in [
        minimum,
        3.0 * minimum,
        -3.0 * minimum,
        2.0_f64.powi(-900),
        2.0_f64.powi(900),
    ] {
        for result in [op.apply(&[input]), op.apply_compensated(&[input])] {
            let value = result.unwrap()[0];
            if input.abs() < f64::MIN_POSITIVE {
                assert_eq!(value.to_bits(), (4.0 * input).to_bits());
            } else {
                assert_relative(value, 4.0 * input);
            }
        }
        assert!(
            op.staged_range_safe,
            "routing metadata cannot cache a particular vector"
        );
        assert_eq!(
            op.apply(&[1.0]).unwrap()[0].to_bits(),
            legacy_staged_product(&op, &[1.0])[0].to_bits()
        );
    }
}

#[test]
fn ordinary_product_output_checks_cancel_then_replay_without_rounding_changes() {
    let entries: Vec<_> = (0..129).map(|i| (i, i, 2.0)).collect();
    let op = operator(&entries, &[1.0; 129]);
    for occurrence in [1, 2, 3] {
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
                let result = op.apply(&[1.0; 129]);
                assert!(result.is_err());
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(seen.get(), occurrence);
        assert_eq!(op.apply(&[1.0; 129]).unwrap(), [2.0; 129]);
    }
}

#[test]
fn range_routing_preparation_cancels_in_mass_and_stiffness_scans_then_replays() {
    let size = 2050;
    let mut matrix = SparseMatrix::new(size);
    for row in 0..size {
        add_at(&mut matrix, row, row, 4.0);
        if row > 0 {
            add_at(&mut matrix, row - 1, row, -1.0);
            add_at(&mut matrix, row, row - 1, -1.0);
        }
    }
    for after_compression in [false, true] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let compressed = Rc::new(Cell::new(false));
        let seen = compressed.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::SparseCompress
                    && point.completed_steps == size as u64
                {
                    seen.set(true);
                }
                let step = if after_compression { 1024 } else { 64 };
                if point.stage == SolverStage::SparseDiagonalMagnitude
                    && point.completed_steps == step
                    && seen.get() == after_compression
                {
                    cancel.request_cancel();
                }
            },
            || {
                let result = SparseMassNormalizedOperator::new(&matrix, &vec![1.0; size]);
                assert!(
                    result.is_err(),
                    "preparation must stop before returning an operator"
                );
                result.map(|_| ())
            },
        )
        .expect_err("preparation must propagate cancellation");
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(compressed.get(), after_compression);
        let replay = SparseMassNormalizedOperator::new(&matrix, &vec![1.0; size]).unwrap();
        assert!(replay.staged_range_safe);
        let result = replay.apply(&vec![1.0; size]).unwrap();
        assert_eq!(result[0], 3.0);
        assert_eq!(result[size - 1], 3.0);
        assert!(result[1..size - 1].iter().all(|&value| value == 2.0));
    }
}

#[test]
fn sparse_products_reject_invalid_inputs_and_nonfinite_outputs_then_replay() {
    let unit = operator(&[(0, 0, 1.0)], &[1.0]);
    for vector in [vec![], vec![1.0, 1.0], vec![f64::NAN], vec![f64::INFINITY]] {
        assert!(unit.apply(&vector).is_err());
        assert!(unit.apply_compensated(&vector).is_err());
    }
    for op in [
        operator(&[(0, 0, 1.0)], &[1e-300]),
        operator(&[(0, 0, 1e200)], &[1.0]),
    ] {
        for result in [op.apply(&[1e200]), op.apply_compensated(&[1e200])] {
            assert!(result.is_err(), "a non-finite product cannot be successful");
        }
        assert!(op.apply(&[1e-200]).unwrap()[0].is_finite());
        assert!(op.apply_compensated(&[1e-200]).unwrap()[0].is_finite());
    }
    assert_eq!(unit.apply(&[2.0]).unwrap(), [2.0]);
    assert_eq!(unit.apply_compensated(&[2.0]).unwrap(), [2.0]);
}

#[test]
fn range_safe_products_cancel_during_inputs_and_rows_then_replay() {
    let entries: Vec<_> = (0..129).map(|i| (i, i, 2e-200)).collect();
    let op = operator(&entries, &[1e-200; 129]);
    for compensated in [false, true] {
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
                    let result = if compensated {
                        op.apply_compensated(&[1.0; 129])
                    } else {
                        op.apply(&[1.0; 129])
                    };
                    assert!(result.is_err(), "the product must stop inside the scope");
                    result
                },
            )
            .unwrap_err();
            assert!(error.contains("cancel"), "{error}");
            assert_eq!(seen.get(), occurrence);
            let replay = if compensated {
                op.apply_compensated(&[1.0; 129])
            } else {
                op.apply(&[1.0; 129])
            }
            .unwrap();
            for value in replay {
                assert_relative(value, 2.0);
            }
        }
    }
}

#[test]
fn range_safe_products_cancel_inside_a_wide_row_then_replay() {
    let size = 2050;
    let mut entries: Vec<_> = (0..size).map(|i| (i, i, 1e-200)).collect();
    entries.extend((1..size).map(|i| (0, i, 1e-200)));
    let op = operator(&entries, &vec![1e-200; size]);
    for compensated in [false, true] {
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
                let result = if compensated {
                    op.apply_compensated(&vec![1.0; size])
                } else {
                    op.apply(&vec![1.0; size])
                };
                assert!(result.is_err());
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        let replay = if compensated {
            op.apply_compensated(&vec![1.0; size])
        } else {
            op.apply(&vec![1.0; size])
        }
        .unwrap();
        assert_relative(replay[0], size as f64);
        for &value in &replay[1..] {
            assert_relative(value, 1.0);
        }
    }
}

fn legacy_staged_product(op: &SparseMassNormalizedOperator, vector: &[f64]) -> Vec<f64> {
    let scaled: Vec<_> = vector
        .iter()
        .zip(&op.inverse_mass_sqrt)
        .map(|(value, inverse)| value * inverse)
        .collect();
    let mut product = vec![0.0; vector.len()];
    op.stiffness
        .multiply_vector_into(&scaled, &mut product)
        .unwrap();
    product
        .into_iter()
        .zip(&op.inverse_mass_sqrt)
        .map(|(value, inverse)| value * inverse)
        .collect()
}

#[test]
fn ordinary_sparse_products_match_legacy_bits_and_record_bounded_debug_timings() {
    use std::{hint::black_box, time::Instant};

    const REPEATS: usize = 16;
    for size in [1024, 8192] {
        let mut entries: Vec<_> = (0..size).map(|i| (i, i, 4.0)).collect();
        for i in 1..size {
            entries.extend([(i - 1, i, -1.0), (i, i - 1, -1.0)]);
        }
        let mass: Vec<_> = (0..size).map(|i| 1.0 + (i % 5) as f64 * 0.25).collect();
        let vector: Vec<_> = (0..size).map(|i| ((i % 7) as f64 - 3.0) * 0.125).collect();
        let op = operator(&entries, &mass);
        assert!(op.staged_range_safe);
        let reference = legacy_staged_product(&op, &vector);
        let actual = op.apply(&vector).unwrap();
        for (value, expected) in actual.iter().zip(&reference) {
            assert_eq!(value.to_bits(), expected.to_bits());
        }
        let mut timings = [Vec::new(), Vec::new()];
        for sample in 0..5 {
            for index in if sample % 2 == 0 { [0, 1] } else { [1, 0] } {
                let start = Instant::now();
                for _ in 0..REPEATS {
                    if index == 0 {
                        black_box(legacy_staged_product(&op, black_box(&vector)));
                    } else {
                        black_box(op.apply(black_box(&vector)).unwrap());
                    }
                }
                timings[index].push(start.elapsed().as_micros() / REPEATS as u128);
            }
        }
        timings.iter_mut().for_each(|times| times.sort_unstable());
        eprintln!(
            "modal product debug: dofs={size}, nnz={}, legacy_median_us={}, checked_median_us={}, samples=5, repeats={REPEATS}",
            entries.len(),
            timings[0][2],
            timings[1][2]
        );
    }
}
