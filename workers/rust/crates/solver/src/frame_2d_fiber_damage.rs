use kyuubiki_protocol::Frame2dFiberDamageInput;

use crate::frame_2d_material_p_delta::{BilinearResponse, Frame2dMaterialPointHistory};

#[derive(Debug, Clone, Copy)]
pub(crate) struct CompiledFrame2dFiberDamage {
    onset: f64,
    failure: f64,
    maximum: f64,
}

pub(crate) fn compile_fiber_damage(
    element_id: &str,
    material_id: &str,
    input: Option<&Frame2dFiberDamageInput>,
) -> Result<Option<CompiledFrame2dFiberDamage>, String> {
    let Some(input) = input else {
        return Ok(None);
    };
    if !(input.onset_equivalent_plastic_strain.is_finite()
        && input.onset_equivalent_plastic_strain >= 0.0)
    {
        return Err(error(
            element_id,
            material_id,
            "onset_equivalent_plastic_strain must be finite and nonnegative",
        ));
    }
    if !(input.failure_equivalent_plastic_strain.is_finite()
        && input.failure_equivalent_plastic_strain > input.onset_equivalent_plastic_strain)
    {
        return Err(error(
            element_id,
            material_id,
            "failure_equivalent_plastic_strain must be finite and greater than onset",
        ));
    }
    if !(input.maximum_damage.is_finite()
        && input.maximum_damage > 0.0
        && input.maximum_damage <= 0.99)
    {
        return Err(error(
            element_id,
            material_id,
            "maximum_damage must be finite and in (0, 0.99]",
        ));
    }
    Ok(Some(CompiledFrame2dFiberDamage {
        onset: input.onset_equivalent_plastic_strain,
        failure: input.failure_equivalent_plastic_strain,
        maximum: input.maximum_damage,
    }))
}

pub(crate) fn apply_fiber_damage(
    damage: Option<&CompiledFrame2dFiberDamage>,
    mut effective: BilinearResponse,
    committed: &Frame2dMaterialPointHistory,
    hardening_ratio: f64,
) -> BilinearResponse {
    let Some(damage) = damage else {
        return effective;
    };
    let equivalent_plastic_strain = effective.history.equivalent_plastic_strain;
    let target = damage.maximum
        * ((equivalent_plastic_strain - damage.onset) / (damage.failure - damage.onset))
            .clamp(0.0, 1.0);
    let updated_damage = committed.damage.max(target);
    let active_damage_growth = updated_damage > committed.damage
        && equivalent_plastic_strain > damage.onset
        && equivalent_plastic_strain < damage.failure;
    let plastic_increment = effective.history.plastic_strain - committed.plastic_strain;
    let kappa_gradient = if active_damage_growth && plastic_increment != 0.0 {
        plastic_increment.signum() * (1.0 - hardening_ratio)
    } else {
        0.0
    };
    let damage_gradient = if kappa_gradient == 0.0 {
        0.0
    } else {
        damage.maximum / (damage.failure - damage.onset) * kappa_gradient
    };
    let degradation = 1.0 - updated_damage;
    effective.tangent_modulus =
        degradation * effective.tangent_modulus - effective.stress * damage_gradient;
    effective.stress *= degradation;
    effective.history.damage = updated_damage;
    effective.history.tangent_modulus = effective.tangent_modulus;
    effective
}

fn error(element_id: &str, material_id: &str, detail: &str) -> String {
    format!("frame 2d material '{element_id}' fiber material '{material_id}' damage {detail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame_2d_material_p_delta::CompiledFrame2dPointMaterial;

    #[test]
    fn active_damage_tangent_matches_a_central_difference() {
        let material = point_material();
        let committed = Frame2dMaterialPointHistory::default();
        let strain = 0.30;
        let step = 1.0e-7;
        let response = material.response(strain, &committed, 0.0);
        let plus = material.response(strain + step, &committed, 0.0);
        let minus = material.response(strain - step, &committed, 0.0);
        let numerical = (plus.stress - minus.stress) / (2.0 * step);

        assert!(response.history.damage > 0.0);
        assert!((response.tangent_modulus - numerical).abs() < 1.0e-5);
    }

    #[test]
    fn unloading_freezes_damage_and_trial_history_is_rollback_safe() {
        let material = point_material();
        let committed = Frame2dMaterialPointHistory::default();
        let loaded = material.response(0.30, &committed, 0.0);
        let trial = material.response(0.50, &loaded.history, 0.0);
        let unloaded = material.response(0.20, &loaded.history, 0.0);

        assert!(trial.history.damage > loaded.history.damage);
        assert_eq!(
            loaded.history.damage,
            material.response(0.30, &committed, 0.0).history.damage
        );
        assert_eq!(unloaded.history.damage, loaded.history.damage);
        assert!(
            (unloaded.tangent_modulus - (1.0 - loaded.history.damage) * 1_000.0).abs() < 1.0e-10
        );
    }

    fn point_material() -> CompiledFrame2dPointMaterial {
        CompiledFrame2dPointMaterial {
            youngs_modulus: 1_000.0,
            yield_strength: 100.0,
            hardening_ratio: 0.1,
            damage: Some(CompiledFrame2dFiberDamage {
                onset: 0.05,
                failure: 0.25,
                maximum: 0.4,
            }),
        }
    }
}
