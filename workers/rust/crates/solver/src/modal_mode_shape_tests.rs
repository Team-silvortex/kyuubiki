use crate::modal_frame_spectrum::checked_mode_shape;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

fn close(actual: f64, expected: f64) {
    assert!(actual.is_finite());
    assert!(
        (actual - expected).abs() < 2.0e-14,
        "{actual:.17e} != {expected:.17e}"
    );
}

#[test]
fn inactive_tiny_mass_cannot_reduce_active_mode_shape_precision() {
    let (separate, _) = checked_mode_shape(&[0.6, 0.8], &[1e308; 2], &[0, 1], 2).unwrap();
    let (combined, norm) = checked_mode_shape(
        &[0.0, 0.6, 0.8],
        &[f64::from_bits(1), 1e308, 1e308],
        &[0, 1, 2],
        3,
    )
    .unwrap();
    assert_eq!(combined[0], 0.0);
    close(norm, 1.0);
    for (actual, expected) in combined[1..].iter().zip(separate) {
        close(*actual, expected);
    }
}

#[test]
fn recovered_mode_shape_is_invariant_under_finite_vector_rescaling() {
    for scale in [1e-300, 1.0, 1e300, -1e-300, -1e300] {
        let (shape, norm) =
            checked_mode_shape(&[0.6 * scale, 0.8 * scale], &[1e-300; 2], &[0, 1], 2).unwrap();
        close(norm, 1.0);
        close(shape[0], 0.6 * scale.signum());
        close(shape[1], 0.8 * scale.signum());
    }
}

fn power(exponent: i32) -> f64 {
    f64::from_bits(((exponent + 1023) as u64) << 52)
}

#[test]
fn mass_weighting_compensates_extreme_vector_scale_before_normalization() {
    let (shape, norm) = checked_mode_shape(
        &[power(-500), -power(500)],
        &[power(-1000), power(1000)],
        &[0, 1],
        2,
    )
    .unwrap();
    close(shape[0], std::f64::consts::FRAC_1_SQRT_2);
    close(shape[1], -std::f64::consts::FRAC_1_SQRT_2);
    close(norm, 1.0);
}

#[test]
fn subnormal_eigenvectors_recover_a_nonzero_unit_shape() {
    let tiny = f64::from_bits(1);
    let (shape, norm) =
        checked_mode_shape(&[3.0 * tiny, 4.0 * tiny], &[1e308; 2], &[0, 1], 2).unwrap();
    close(shape[0], 0.6);
    close(shape[1], 0.8);
    close(norm, 1.0);
}

#[test]
fn weighted_shapes_match_exact_power_of_two_references_across_range() {
    for vector_exponent in [-1000, -500, 0, 500, 1000] {
        for mass_exponent in [-1000, -500, 0, 500, 1000] {
            let (shape, norm) = checked_mode_shape(
                &[power(vector_exponent), -power(vector_exponent - 1)],
                &[power(mass_exponent), power(mass_exponent + 2)],
                &[0, 1],
                2,
            )
            .unwrap();
            close(shape[0], 4.0 / 17.0_f64.sqrt());
            close(shape[1], -1.0 / 17.0_f64.sqrt());
            close(norm, 1.0);
        }
    }
}

#[test]
fn representable_subnormal_shape_components_round_only_at_the_output_scale() {
    for vector_scale in [1.0, power(500)] {
        for (mass_exponent, mantissa, bits) in [
            (148, 1.0, 1),
            (148, 1.5, 2),
            (150, 1.0, 0),
            (150, 1.5, 1),
            (152, 1.0, 0),
            (152, 1.5, 0),
        ] {
            let (shape, norm) = checked_mode_shape(
                &[vector_scale, mantissa * power(-1000) * vector_scale],
                &[1.0, power(mass_exponent)],
                &[0, 1],
                2,
            )
            .unwrap();
            assert_eq!(shape[0], 1.0);
            assert_eq!(shape[1].to_bits(), bits);
            assert_eq!(norm, 1.0);
        }
    }
}

#[test]
fn unsorted_free_dof_mapping_preserves_constrained_zeros() {
    let (shape, norm) =
        checked_mode_shape(&[0.6, -0.8], &[0.0, 4.0, 0.0, 4.0, 0.0], &[3, 1], 5).unwrap();
    for index in [0, 2, 4] {
        assert_eq!(shape[index], 0.0);
    }
    close(shape[3], 0.6);
    close(shape[1], -0.8);
    close(norm, 1.0);
}

#[test]
fn invalid_shape_inputs_fail_without_panicking_or_partial_results() {
    let invalid = [
        (vec![], vec![], vec![], 0),
        (vec![], vec![1.0], vec![0], 1),
        (vec![1.0, 2.0], vec![1.0], vec![0], 1),
        (vec![1.0], vec![], vec![0], 1),
        (vec![1.0], vec![1.0; 2], vec![0], 1),
        (vec![1.0], vec![1.0], vec![1], 1),
        (vec![1.0], vec![1.0], vec![usize::MAX], 1),
        (vec![1.0, 2.0], vec![1.0; 2], vec![0, 0], 2),
        (vec![0.0, -0.0], vec![1.0; 2], vec![0, 1], 2),
        (vec![f64::NAN], vec![1.0], vec![0], 1),
        (vec![f64::INFINITY], vec![1.0], vec![0], 1),
        (vec![1.0], vec![0.0], vec![0], 1),
        (vec![1.0], vec![-1.0], vec![0], 1),
        (vec![1.0], vec![f64::NAN], vec![0], 1),
        (vec![1.0], vec![f64::INFINITY], vec![0], 1),
        (vec![1.0, 0.0], vec![1.0, 0.0], vec![0, 1], 2),
    ];
    for (vector, mass, free, count) in invalid {
        let result = std::panic::catch_unwind(|| checked_mode_shape(&vector, &mass, &free, count));
        assert!(result.expect("invalid mapping must not panic").is_err());
    }
    assert!(checked_mode_shape(&[1.0], &[1.0], &[0], 1).is_ok());
}

#[test]
fn mode_shape_recovery_stages_cancel_in_chunks_and_replay() {
    let vector = vec![1.0; 128];
    let mass = vec![1.0; 128];
    let free: Vec<_> = (0..128).collect();
    for stage in [
        SolverStage::ResultFreeDofs,
        SolverStage::ResultNodes,
        SolverStage::ResultNodeSummary,
        SolverStage::ResultTotals,
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == 64 {
                    cancel.request_cancel();
                }
            },
            || {
                let result = checked_mode_shape(&vector, &mass, &free, 128);
                assert!(result.is_err(), "recovery must cancel before scope exit");
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(control.last_checkpoint().unwrap().stage, stage);
        assert_eq!(control.last_checkpoint().unwrap().completed_steps, 64);
        close(
            checked_mode_shape(&vector, &mass, &free, 128).unwrap().1,
            1.0,
        );
    }
}
