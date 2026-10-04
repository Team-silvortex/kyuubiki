use super::*;
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::modal_math::jacobi_eigenpairs;
use crate::modal_sparse::reduce_sparse_modal_system;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

fn bending(segments: usize) -> ReducedSparseModalSystem {
    let size = 2 * segments;
    let mut matrix = SparseMatrix::new(size);
    let local = [
        [12.0, 6.0, -12.0, 6.0],
        [6.0, 4.0, -6.0, 2.0],
        [-12.0, -6.0, 12.0, -6.0],
        [6.0, 2.0, -6.0, 4.0],
    ];
    for e in 0..segments {
        for (i, row) in local.iter().enumerate() {
            for (j, &a) in row.iter().enumerate() {
                if 2 * e + i >= 2 && 2 * e + j >= 2 {
                    add_at(&mut matrix, 2 * e + i - 2, 2 * e + j - 2, a);
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
fn roundoff_resource_plan_rejects_dimensions_before_dense_preparation() {
    for size in [0, 1, 257, usize::MAX] {
        assert!(budget::Plan::new(size).is_err());
        assert!(!eligible(size));
    }
    let mut previous = (0, 0);
    for size in 2..=256 {
        let plan = budget::Plan::new(size).unwrap();
        assert!(plan.payload_bytes > previous.0 && plan.component_visits > previous.1);
        previous = (plan.payload_bytes, plan.component_visits);
    }
    assert_eq!(previous, (4_325_376, 335_544_320));
}

#[test]
fn roundoff_checked_product_budget_is_sticky_and_cancellable() {
    let mut checks = budget::Checks::new();
    for _ in 0..80 {
        checks.next().unwrap();
    }
    for _ in 0..3 {
        assert!(
            checks
                .next()
                .unwrap_err()
                .contains("checked-product budget")
        );
    }
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |p| {
            if p.stage == SolverStage::ModalRoundoffSearch {
                cancel.request_cancel();
            }
        },
        || budget::Checks::new().next(),
    )
    .unwrap_err();
    assert!(error.contains("modal_roundoff_search"), "{error}");
    budget::Checks::new().next().unwrap();
}

#[test]
fn roundoff_input_guards_precede_dense_allocation_and_reject_operator_mismatch() {
    let system = bending(129);
    let observed = Rc::new(Cell::new(0));
    let counts = observed.clone();
    with_solver_observer(
        &SolverControl::default(),
        move |p| {
            if p.stage == SolverStage::SparseMatrixScale {
                counts.set(counts.get() + 1);
            }
        },
        || {
            assert!(normalized(&system, 1.0, &[1.0; 2], 1e-8).is_err());
            assert!(physical(&system.operator, 1.0, &[1.0; 2], &[1.0; 2], 1e-8).is_err());
            Ok::<(), String>(())
        },
    )
    .unwrap();
    assert_eq!(
        observed.get(),
        0,
        "ineligible operator never materializes a dense matrix"
    );
    let small = bending(2);
    assert!(normalized(&small, 1.0, &[1.0; 2], 1e-8).is_err());
    assert!(physical(&small.operator, 1.0, &[1.0; 2], &[1.0; 2], 1e-8).is_err());
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(normalized(&small, value, &[1.0; 4], 1e-8).is_err());
        assert!(physical(&small.operator, value, &[1.0; 4], &small.mass, 1e-8).is_err());
    }
}

#[test]
fn roundoff_single_mode_recovery_reaches_original_gate_without_more_inverse_steps() {
    let system = bending(128);
    let seed = jacobi_eigenpairs(system.operator.dense_fallback_matrix().unwrap()).unwrap();
    let mut baseline = seed[..1].to_vec();
    super::super::refine_with_budget(&system, &mut baseline, 1e-8, 4).unwrap_err();
    let mut pairs = seed[..1].to_vec();
    let counts = Rc::new(Cell::new([0; 3]));
    let observed = counts.clone();
    with_solver_observer(
        &SolverControl::default(),
        move |p| {
            let mut c = observed.get();
            if p.stage == SolverStage::DenseFactor && p.completed_steps == 0 {
                c[0] += 1;
            }
            if p.stage == SolverStage::ModalRoundoffSearch {
                c[1] += 1;
            }
            if p.stage == SolverStage::ModalRoundoffValidate {
                c[2] += 1;
            }
            observed.set(c);
        },
        || super::super::refine_dense_modes(&system, &mut pairs, 1e-8),
    )
    .unwrap();
    let (value, vector) = &pairs[0];
    let actual = relative_residual(
        *value,
        vector,
        &system.operator.apply_compensated(vector).unwrap(),
    )
    .unwrap();
    println!(
        "production normalized first mode relative={actual:e} counts={:?}",
        counts.get()
    );
    assert!(actual <= 1e-8);
    assert_eq!(value.to_bits(), baseline[0].0.to_bits());
    assert_eq!(
        counts.get()[0],
        2,
        "one inverse plus one private Gram factor"
    );
    assert!((1..=80).contains(&counts.get()[1]));
    assert_eq!(counts.get()[2], 1);
}

#[test]
fn roundoff_multi_mode_path_still_rejects_the_unqualified_long_spectrum() {
    let system = bending(128);
    let seed = jacobi_eigenpairs(system.operator.dense_fallback_matrix().unwrap()).unwrap();
    let mut pairs = seed[..6].to_vec();
    let error = super::super::refine_dense_modes(&system, &mut pairs, 1e-8).unwrap_err();
    assert!(error.contains("within 4 steps"), "{error}");
}

#[test]
fn roundoff_admission_rechecks_and_cancels_before_committing_private_candidates() {
    let matrix = vec![vec![1.0, 0.0], vec![0.0, 2.0]];
    let seed = [1.0, 1.0];
    let mut calls = 0;
    let error = admit(matrix.clone(), &seed, 1e-8, |_| {
        calls += 1;
        Ok((if calls == 1 { 0.0 } else { 1.0 }, vec![0.0; 2]))
    })
    .unwrap_err();
    assert!(error.contains("final residual certificate"));
    assert_eq!(calls, 2);
    for stage in [
        SolverStage::ModalRoundoffPrepare,
        SolverStage::ModalRoundoffSearch,
        SolverStage::ModalRoundoffValidate,
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage {
                    cancel.request_cancel();
                }
            },
            || admit(matrix.clone(), &seed, 1e-8, |_| Ok((0.0, vec![0.0; 2]))),
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(seed, [1.0, 1.0]);
        let replay = admit(matrix.clone(), &seed, 1e-8, |_| Ok((0.0, vec![0.0; 2]))).unwrap();
        assert!(
            replay
                .iter()
                .zip(seed)
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
    }
}
