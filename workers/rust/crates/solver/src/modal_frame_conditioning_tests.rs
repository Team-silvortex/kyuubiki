use super::*;

#[path = "modal_frame_conditioning_reference.rs"]
mod reference;

use reference::{BendingReference, Wide};

#[path = "modal_frame_representable_tests.rs"]
mod representable;

#[test]
fn long_bending_budget_sweep_checks_residuals_against_a_wide_oracle() {
    let system = bending(128);
    let oracle = BendingReference::new(128);
    let (reference_value, reference_vector) = oracle.first_mode();
    assert!(oracle.residual(reference_value, &reference_vector) < 1e-20);
    let seed = jacobi_eigenpairs(system.operator.dense_fallback_matrix().unwrap()).unwrap();
    let mut previous = f64::INFINITY;
    let mut seed_direction_error = 0.0;
    for budget in [0, 1, 4, 8, 16, 32, 64] {
        let mut pairs = seed[..1].to_vec();
        let outcome = refine_with_budget(&system, &mut pairs, 1e-8, budget);
        let (value, vector) = &pairs[0];
        let applied = system.operator.apply_compensated(vector).unwrap();
        let residual = relative_residual(*value, vector, &applied).unwrap();
        let wide_vector: Vec<_> = vector.iter().map(|&v| Wide::from(v)).collect();
        let wide_residual = oracle.residual(Wide::from(*value), &wide_vector);
        let direction_error = oracle.direction_error(&wide_vector, &reference_vector);
        let root_error = (*value / reference_value.rounded() - 1.0).abs();
        println!(
            "budget={budget} success={} value={value:.17e} residual={residual:.17e} wide={wide_residual:.17e} direction={direction_error:.17e} root={root_error:.17e}",
            outcome.is_ok()
        );
        assert!(residual <= previous);
        assert!((wide_residual / residual - 1.0).abs() < 1e-6);
        assert_eq!(outcome.is_ok(), residual <= 1e-8);
        if let Err(error) = &outcome {
            assert!(error.contains(&format!("within {budget} steps")), "{error}");
        }
        if budget == 0 {
            seed_direction_error = direction_error;
        } else {
            assert!(root_error < 1e-14);
            assert!(direction_error < 0.1 * seed_direction_error);
        }
        previous = residual;
    }
}

#[test]
fn wide_bending_mode_compares_internal_and_rounded_representation_residuals() {
    for segments in [64, 80, 100, 128] {
        let system = bending(segments);
        let oracle = BendingReference::new(segments);
        let (value, vector) = oracle.first_mode();
        let wide_residual = oracle.residual(value, &vector);
        let rounded: Vec<_> = vector.iter().map(|v| v.rounded()).collect();
        let rounded_wide: Vec<_> = rounded.iter().map(|&v| Wide::from(v)).collect();
        let rounded_residual = oracle.residual(Wide::from(value.rounded()), &rounded_wide);
        let applied = system.operator.apply_compensated(&rounded).unwrap();
        let production_residual = relative_residual(value.rounded(), &rounded, &applied).unwrap();
        let rounded_matrix = oracle.rounded_matrix_residual(value, &vector);
        println!(
            "segments={segments} oracle value={:.17e} wide={wide_residual:.17e} rounded={rounded_residual:.17e} production={production_residual:.17e} rounded_matrix={rounded_matrix:.17e}",
            value.rounded()
        );
        assert!(wide_residual < 1e-20);
        assert!((production_residual / rounded_residual - 1.0).abs() < 1e-6);
        assert!(rounded_residual > wide_residual * 1e10);
        if segments == 128 {
            assert!(rounded_residual > 1e-8);
        }
    }
}
