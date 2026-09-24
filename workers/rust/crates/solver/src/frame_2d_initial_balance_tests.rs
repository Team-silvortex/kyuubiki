use super::validate_initial_force_balance as check_balance;
use crate::frame_2d_corotational_element::assemble_initial_material_forces;
use crate::frame_2d_material_p_delta::{CompiledFrame2dMaterial, Frame2dMaterialHistory};
use kyuubiki_protocol::Frame2dElementInput;

#[test]
fn initial_balance_uses_relative_local_contributions_without_an_absolute_floor() {
    for scale in [1e-300, 1e-200, 1.0, 1e200, 1e300] {
        let scales = [scale, scale, 0.0];
        check_balance(&[scale * 1e-12, -scale * 1e-12, 0.0], &scales, &[]).unwrap();
        let error = check_balance(&[scale * 1e-6, 0.0, 0.0], &scales, &[]).unwrap_err();
        assert!(error.contains("node 0 translation"), "{error}");
        assert!(check_balance(&[scale, 0.0, 0.0], &[0.0; 3], &[]).is_err());
    }
    check_balance(&[0.0; 3], &[0.0; 3], &[]).unwrap();
}

#[test]
fn initial_moment_is_not_scaled_by_a_large_translational_force() {
    for moment_scale in [1e-250, 1.0, 1e250] {
        let error = check_balance(
            &[0.0, 0.0, moment_scale * 1e-4],
            &[1e300, 1e300, moment_scale],
            &[],
        )
        .unwrap_err();
        assert!(error.contains("node 0 rotation"), "{error}");
    }
}

#[test]
fn initial_balance_does_not_borrow_scale_from_another_node() {
    let error = check_balance(
        &[0.0, 0.0, 0.0, 1e-200, 0.0, 0.0],
        &[1e300, 1e300, 1e300, 1e-200, 1e-200, 0.0],
        &[],
    )
    .unwrap_err();
    assert!(error.contains("node 1 translation"), "{error}");
}

#[test]
fn initial_support_reactions_are_not_free_equations() {
    let force = [4.0, -3.0, 2.0];
    let scales = [5.0, 5.0, 2.0];
    check_balance(&force, &scales, &[0, 1, 2]).unwrap();
    for constrained in [&[1, 2][..], &[0, 2], &[0, 1]] {
        assert!(check_balance(&force, &scales, constrained).is_err());
    }
}

#[test]
fn initial_balance_rejects_nonfinite_and_negative_scales_even_at_supports() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = check_balance(&[value, 0.0, 0.0], &[1.0; 3], &[0, 1, 2]).unwrap_err();
        assert!(error.contains("non-finite"), "{error}");
        assert!(check_balance(&[0.0; 3], &[value, 1.0, 1.0], &[0, 1, 2]).is_err());
    }
    assert!(check_balance(&[0.0; 3], &[1.0, -1.0, 1.0], &[]).is_err());
}

#[test]
fn initial_balance_rejects_malformed_dimensions_and_constraints_without_panicking() {
    assert!(check_balance(&[0.0; 3], &[1.0; 2], &[]).is_err());
    assert!(check_balance(&[0.0; 2], &[1.0; 2], &[]).is_err());
    assert!(check_balance(&[0.0; 3], &[1.0; 3], &[usize::MAX]).is_err());
}

#[test]
fn initial_balance_preserves_translation_rotation_and_length_unit_classification() {
    for angle in [0.0_f64, 0.27, 1.8, 3.5] {
        for length_scale in [1e-12, 1.0, 1e12] {
            let scales = [1.0, 1.0, length_scale];
            for (residual, accepted) in [(1e-12, true), (1e-6, false)] {
                let force = [residual * angle.cos(), residual * angle.sin(), 0.0];
                assert_eq!(check_balance(&force, &scales, &[]).is_ok(), accepted);
                let moment = [0.0, 0.0, residual * length_scale];
                assert_eq!(check_balance(&moment, &scales, &[]).is_ok(), accepted);
            }
        }
    }
}

#[test]
fn initial_assembly_rejects_overflow_in_force_or_absolute_contribution_sums() {
    let elements = (0..2)
        .map(|index| Frame2dElementInput {
            id: format!("e{index}"),
            node_i: 0,
            node_j: 1,
            area: 10.0,
            youngs_modulus: 1e4,
            moment_of_inertia: 1.0,
            section_modulus: 1.0,
        })
        .collect::<Vec<_>>();
    for sign in [1.0, -1.0] {
        let materials = [1.0, sign].map(|sign| {
            Some(CompiledFrame2dMaterial {
                yield_strength: 1e308,
                hardening_ratio: 0.05,
                initial_axial_stress: sign * 1e307,
                section_fibers: vec![],
                fiber_material_ids: vec![],
                longitudinal_integration_points: 2,
                adaptive_longitudinal_integration: false,
                longitudinal_integration_tolerance: 1e-3,
            })
        });
        let error = assemble_initial_material_forces(
            &[(0.0, 0.0), (1.0, 0.0)],
            &elements,
            &[0.0; 6],
            &materials,
            &vec![Frame2dMaterialHistory::default(); 2],
        )
        .unwrap_err();
        assert!(
            error.contains("element 'e1'") && error.contains("non-finite"),
            "{error}"
        );
    }
}
