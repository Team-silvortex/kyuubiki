use crate::linear_algebra::SparseMatrix;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

pub(super) fn element_masses(
    id: &str,
    density: f64,
    area: f64,
    length: f64,
) -> Result<[f64; 3], String> {
    let total = positive_product([density, area, length]);
    let translational = total / 2.0;
    let rotary = positive_product([total, length, length, 1.0 / 24.0]);
    for (field, value) in [
        ("element mass", total),
        ("nodal translational mass", translational),
        ("nodal rotary mass", rotary),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!(
                "modal frame element {id}: {field} is not representable"
            ));
        }
    }
    Ok([total, translational, rotary])
}

pub(super) fn positive_product<const N: usize>(mut factors: [f64; N]) -> f64 {
    factors.sort_unstable_by(f64::total_cmp);
    let (mut left, mut right) = (0, N);
    let mut product = 1.0;
    // Alternate small/large factors according to the running magnitude, so
    // representable products do not first overflow or underflow unnecessarily.
    while left < right {
        if product < 1.0 {
            right -= 1;
            product *= factors[right];
        } else {
            product *= factors[left];
            left += 1;
        }
    }
    product
}

pub(super) fn validate_element_stiffness<const N: usize>(
    id: &str,
    space: &str,
    matrix: &[[f64; N]; N],
) -> Result<(), String> {
    if matrix.iter().flatten().any(|value| !value.is_finite())
        || (0..N).any(|i| matrix[i][i] <= 0.0)
    {
        return Err(format!(
            "modal frame element {id}: {space} stiffness is not representable"
        ));
    }
    Ok(())
}

pub(super) fn validate_assembled_system(
    stiffness: &SparseMatrix,
    mass: &[f64],
    total_mass: f64,
) -> Result<(), String> {
    if !total_mass.is_finite() || total_mass <= 0.0 {
        return Err("modal frame total mass must be finite and positive".into());
    }
    checkpoint(SolverStage::SparseValidateMatrix, 0)?;
    // Validate the complete model before restraints can discard invalid rows.
    for (dof, &value) in mass.iter().enumerate() {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!(
                "modal frame assembled mass at dof {dof} is not representable"
            ));
        }
        if stiffness
            .row_entries(dof)
            .iter()
            .any(|(_, value)| !value.is_finite())
        {
            return Err(format!(
                "modal frame assembled stiffness at dof {dof} is not representable"
            ));
        }
        checkpoint_chunk(SolverStage::SparseValidateMatrix, dof + 1, mass.len())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{element_masses, positive_product};

    #[test]
    fn balanced_positive_products_keep_representable_extreme_factors() {
        for factors in [
            [1.0e-250, 1.0e200, 1.0e100, 1.0e-50],
            [1.0e250, 1.0e200, 1.0e-200, 1.0e-250],
            [1.0e-200, 1.0e-200, 1.0e200, 1.0e200],
        ] {
            assert!((positive_product(factors) - 1.0).abs() < 1.0e-14);
        }
        assert_eq!(positive_product([1.0e-200, 1.0e-200]), 0.0);
        assert_eq!(positive_product([1.0e200, 1.0e200]), f64::INFINITY);
    }

    #[test]
    fn lumped_rotary_mass_divides_before_avoidable_overflow() {
        let [total, translation, rotation] = element_masses("beam", 1.0e307, 1.0, 3.0).unwrap();
        for (actual, expected) in [
            (total, 3.0e307),
            (translation, 1.5e307),
            (rotation, 1.125e307),
        ] {
            assert!((actual / expected - 1.0).abs() < 1.0e-14);
        }
    }

    #[test]
    fn lost_half_nodal_mass_is_rejected_even_when_total_is_positive() {
        let error = element_masses("beam", f64::from_bits(1), 1.0, 1.0).unwrap_err();
        assert!(error.contains("nodal translational mass"), "{error}");
    }
}
