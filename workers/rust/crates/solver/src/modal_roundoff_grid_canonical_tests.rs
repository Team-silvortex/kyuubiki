use super::super::super::{
    hybrid_tests::check_published, triangular_grid_robustness_tests::Fixture,
};
use super::super::tests::shuffled;
use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};

#[test]
fn triangular_grid_canonical_beams_keep_identical_candidates_under_shuffled_numbering() {
    for segments in [80, 100, 128] {
        for step in [1.0, 1e14, 1e-10] {
            let fixture = Fixture::with_segments(segments, step);
            let size = fixture.shape.len();
            let mut reference: Option<Vec<f64>> = None;
            for shuffle in [0, 7, 19, 41, 113] {
                let permutation = if shuffle == 0 {
                    (0..size).collect()
                } else {
                    shuffled(size, shuffle)
                };
                let matrix: Vec<Vec<_>> = permutation
                    .iter()
                    .map(|&i| {
                        permutation
                            .iter()
                            .map(|&j| fixture.directions[i][j])
                            .collect()
                    })
                    .collect();
                let mass: Vec<_> = permutation
                    .iter()
                    .map(|&i| fixture.system.mass[i])
                    .collect();
                for sign in [1.0, -1.0] {
                    let seed: Vec<_> = permutation
                        .iter()
                        .map(|&i| sign * fixture.shape[i])
                        .collect();
                    let restore_original = |v: &[f64]| {
                        let mut original = vec![0.0; size];
                        for (&i, &a) in permutation.iter().zip(v) {
                            original[i] = a;
                        }
                        original
                    };
                    let mut calls = 0;
                    let accepted = search_canonical_unit_shape(&matrix, &seed, &mass, 1e-8, |v| {
                        calls += 1;
                        let (relative, residual) = fixture.checked(&restore_original(v))?;
                        Ok((relative, permutation.iter().map(|&i| residual[i]).collect()))
                    })
                    .unwrap();
                    assert_eq!(calls, accepted.usage.certificates);
                    assert!(calls <= super::super::MAX_TOTAL_CERTIFICATES);
                    assert!(accepted.usage.attempts <= 3);
                    let plan = Plan::canonical(size).unwrap();
                    assert_eq!(
                        accepted.usage.component_visits,
                        plan.preflight_visits + accepted.usage.attempts * plan.fit_visits
                    );
                    let original = restore_original(&accepted.shape);
                    check_published(segments, step, fixture.value, &original);
                    let positive: Vec<_> = original.iter().map(|v| sign * v).collect();
                    if let Some(reference) = &reference {
                        assert!(
                            positive
                                .iter()
                                .zip(reference)
                                .all(|(a, b)| a.to_bits() == b.to_bits()),
                            "canonical bits changed for {segments}/{step:e}/{shuffle}/{sign}"
                        );
                    } else {
                        reference = Some(positive);
                    }
                    println!(
                        "canonical portfolio segments={segments} step={step:e} shuffle={shuffle} sign={sign} calls={calls} order={:?} relative={:e}",
                        accepted.order,
                        fixture.checked(&original).unwrap().0
                    );
                }
            }
        }
    }
}

#[test]
fn triangular_grid_canonical_ties_faults_and_final_cancellation_fail_closed() {
    let plan = Plan::canonical(256).unwrap();
    assert_eq!(plan.payload_bytes, 6_914_048);
    assert_eq!(plan.component_visits, 1_009_090_560);
    for bad in [0, 1, 257, usize::MAX] {
        assert!(Plan::canonical(bad).is_err());
    }
    let matrix: Vec<Vec<_>> = (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    let seed = [3.0_f64.sqrt().recip(); 3];
    let error = search_canonical_unit_shape(&matrix, &seed, &[1.0; 3], 1e-8, |_| unreachable!())
        .unwrap_err();
    assert!(error.contains("ambiguous coordinate signatures"), "{error}");
    let seed = [
        1.0 / 14.0_f64.sqrt(),
        2.0 / 14.0_f64.sqrt(),
        3.0 / 14.0_f64.sqrt(),
    ];
    let mut calls = 0;
    let error = search_canonical_unit_shape(&matrix, &seed, &[1.0; 3], 1e-8, |_| {
        calls += 1;
        Err("original operator fault".into())
    })
    .unwrap_err();
    assert_eq!(error, "original operator fault");
    assert_eq!(calls, 1);
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |p| {
            if p.stage == SolverStage::ModalRoundoffValidate && p.completed_steps == 5 {
                cancel.request_cancel();
            }
        },
        || {
            search_canonical_unit_shape(&matrix, &seed, &[1.0; 3], 1e-8, |_| {
                Ok((0.0, vec![0.0; 3]))
            })
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    assert_eq!(
        search_canonical_unit_shape(&matrix, &seed, &[1.0; 3], 1e-8, |_| Ok((0.0, vec![0.0; 3])))
            .unwrap()
            .shape,
        seed
    );
}

#[test]
fn triangular_grid_canonical_malformed_original_receipts_never_retry_or_publish() {
    let matrix: Vec<Vec<_>> = (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    let seed = [
        1.0 / 14.0_f64.sqrt(),
        2.0 / 14.0_f64.sqrt(),
        3.0 / 14.0_f64.sqrt(),
    ];
    for receipt in [
        (0.0, vec![0.0; 2]),
        (f64::NAN, vec![0.0; 3]),
        (-1.0, vec![0.0; 3]),
        (0.0, vec![f64::INFINITY; 3]),
    ] {
        let mut calls = 0;
        assert!(
            search_canonical_unit_shape(&matrix, &seed, &[1.0; 3], 1e-8, |_| {
                calls += 1;
                Ok(receipt.clone())
            })
            .is_err()
        );
        assert_eq!(calls, 1);
    }
    let mut calls = 0;
    let error = search_canonical_unit_shape(&matrix, &seed, &[1.0; 3], 1e-8, |_| {
        calls += 1;
        Ok((if calls < 3 { 0.0 } else { 1.0 }, vec![0.0; 3]))
    })
    .unwrap_err();
    assert!(
        error.contains("lost its final physical certificate"),
        "{error}"
    );
    assert_eq!(calls, 3);

    let control = SolverControl::default();
    let cancel = control.clone();
    let mut calls = 0;
    let error = with_solver_observer(
        &control,
        |_| {},
        || {
            search_canonical_unit_shape(&matrix, &seed, &[1.0; 3], 1e-8, |_| {
                calls += 1;
                cancel.request_cancel();
                Ok((0.0, vec![0.0; 3]))
            })
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    assert_eq!(calls, 1);
    assert_eq!(
        search_canonical_unit_shape(&matrix, &seed, &[1.0; 3], 1e-8, |_| Ok((0.0, vec![0.0; 3])))
            .unwrap()
            .shape,
        seed
    );
}
