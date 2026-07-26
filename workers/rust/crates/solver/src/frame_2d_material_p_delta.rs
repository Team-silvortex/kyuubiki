use std::collections::{HashMap, HashSet};

use crate::frame_2d_corotational_element::element_axial_strain;
use crate::frame_2d_p_delta::solve_frame_2d_p_delta_with_materials;
use kyuubiki_protocol::{
    Frame2dMaterialStateResult, Frame2dMonotonicBilinearMaterialInput, Frame2dStabilityKinematics,
    Frame2dStabilityPathControl, SolveFrame2dMaterialPDeltaRequest,
    SolveFrame2dMaterialPDeltaResult,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct CompiledFrame2dMaterial {
    pub(crate) yield_strength: f64,
    pub(crate) hardening_ratio: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Frame2dMaterialHistory {
    pub(crate) plastic_strain: f64,
    pub(crate) backstress: f64,
    pub(crate) equivalent_plastic_strain: f64,
    pub(crate) tangent_modulus: f64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BilinearResponse {
    pub(crate) stress: f64,
    pub(crate) tangent_modulus: f64,
    pub(crate) history: Frame2dMaterialHistory,
}

impl CompiledFrame2dMaterial {
    pub(crate) fn response(
        self,
        youngs_modulus: f64,
        strain: f64,
        committed: Frame2dMaterialHistory,
    ) -> BilinearResponse {
        let trial_stress = youngs_modulus * (strain - committed.plastic_strain);
        let relative_trial = trial_stress - committed.backstress;
        let yield_excess = relative_trial.abs() - self.yield_strength;
        if yield_excess <= self.yield_strength * 1.0e-12 {
            let mut history = committed;
            history.tangent_modulus = youngs_modulus;
            return BilinearResponse {
                stress: trial_stress,
                tangent_modulus: youngs_modulus,
                history,
            };
        }
        let plastic_modulus = youngs_modulus * self.hardening_ratio
            / (1.0 - self.hardening_ratio).max(f64::MIN_POSITIVE);
        let plastic_increment = yield_excess / (youngs_modulus + plastic_modulus);
        let direction = relative_trial.signum();
        let plastic_strain = committed.plastic_strain + plastic_increment * direction;
        let backstress = committed.backstress + plastic_modulus * plastic_increment * direction;
        let tangent_modulus = youngs_modulus * plastic_modulus / (youngs_modulus + plastic_modulus);
        let stress = trial_stress - youngs_modulus * plastic_increment * direction;
        BilinearResponse {
            stress,
            tangent_modulus,
            history: Frame2dMaterialHistory {
                plastic_strain,
                backstress,
                equivalent_plastic_strain: committed.equivalent_plastic_strain + plastic_increment,
                tangent_modulus,
            },
        }
    }
}

pub fn solve_frame_2d_material_p_delta(
    request: &SolveFrame2dMaterialPDeltaRequest,
) -> Result<SolveFrame2dMaterialPDeltaResult, String> {
    validate_control_contract(request)?;
    let compiled = compile_materials(request)?;
    let (stability_result, committed_states) =
        solve_frame_2d_p_delta_with_materials(&request.stability, &compiled)?;
    let frame = &request.stability.buckling.frame;
    let positions = frame
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            (
                node.x + stability_result.initial_imperfection_shape[index * 3],
                node.y + stability_result.initial_imperfection_shape[index * 3 + 1],
            )
        })
        .collect::<Vec<_>>();
    let material_states = frame
        .elements
        .iter()
        .enumerate()
        .filter_map(|(element_index, element)| {
            compiled[element_index].map(|material| {
                (
                    element_index,
                    element,
                    material,
                    committed_states[element_index],
                )
            })
        })
        .map(|(element_index, element, _material, history)| {
            let axial_strain =
                element_axial_strain(&positions, element, &stability_result.final_displacements)?;
            Ok(Frame2dMaterialStateResult {
                element_index,
                element_id: element.id.clone(),
                axial_strain,
                axial_stress: element.youngs_modulus * (axial_strain - history.plastic_strain),
                plastic_strain: history.plastic_strain,
                backstress: history.backstress,
                equivalent_plastic_strain: history.equivalent_plastic_strain,
                tangent_modulus: history.tangent_modulus,
                yielded: history.equivalent_plastic_strain > 0.0,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let yielded_element_count = material_states.iter().filter(|state| state.yielded).count();
    let max_equivalent_plastic_strain = material_states
        .iter()
        .map(|state| state.equivalent_plastic_strain)
        .fold(0.0_f64, f64::max);
    Ok(SolveFrame2dMaterialPDeltaResult {
        input: request.clone(),
        stability_result,
        material_states,
        yielded_element_count,
        max_equivalent_plastic_strain,
    })
}

pub(crate) fn update_material_histories(
    positions: &[(f64, f64)],
    elements: &[kyuubiki_protocol::Frame2dElementInput],
    displacement: &[f64],
    materials: &[Option<CompiledFrame2dMaterial>],
    committed: &[Frame2dMaterialHistory],
) -> Result<Vec<Frame2dMaterialHistory>, String> {
    let mut updated = committed.to_vec();
    updated.resize(elements.len(), Frame2dMaterialHistory::default());
    for (element_index, element) in elements.iter().enumerate() {
        let Some(material) = materials.get(element_index).copied().flatten() else {
            continue;
        };
        let strain = element_axial_strain(positions, element, displacement)?;
        updated[element_index] = material
            .response(element.youngs_modulus, strain, updated[element_index])
            .history;
    }
    Ok(updated)
}

fn validate_control_contract(request: &SolveFrame2dMaterialPDeltaRequest) -> Result<(), String> {
    if request.stability.kinematics != Frame2dStabilityKinematics::Corotational {
        return Err("frame 2d material p-delta requires corotational kinematics".into());
    }
    if request.stability.path_control != Frame2dStabilityPathControl::LoadControl {
        return Err(
            "frame 2d material p-delta currently supports monotonic load control only".into(),
        );
    }
    if request.materials.is_empty() {
        return Err("frame 2d material p-delta requires at least one material assignment".into());
    }
    Ok(())
}

fn compile_materials(
    request: &SolveFrame2dMaterialPDeltaRequest,
) -> Result<Vec<Option<CompiledFrame2dMaterial>>, String> {
    let elements = &request.stability.buckling.frame.elements;
    let mut element_indices = HashMap::new();
    for (index, element) in elements.iter().enumerate() {
        if element_indices.insert(element.id.as_str(), index).is_some() {
            return Err(format!(
                "frame 2d material p-delta requires unique element IDs; '{}' is duplicated",
                element.id
            ));
        }
    }
    let mut assigned = HashSet::new();
    let mut compiled = vec![None; elements.len()];
    for material in &request.materials {
        validate_material(material)?;
        let Some(&element_index) = element_indices.get(material.element_id.as_str()) else {
            return Err(format!(
                "frame 2d material assignment references unknown element '{}'",
                material.element_id
            ));
        };
        if !assigned.insert(element_index) {
            return Err(format!(
                "frame 2d element '{}' has duplicate material assignments",
                material.element_id
            ));
        }
        compiled[element_index] = Some(CompiledFrame2dMaterial {
            yield_strength: material.yield_strength,
            hardening_ratio: material.hardening_ratio,
        });
    }
    Ok(compiled)
}

fn validate_material(material: &Frame2dMonotonicBilinearMaterialInput) -> Result<(), String> {
    if !(material.yield_strength.is_finite() && material.yield_strength > 0.0) {
        return Err(format!(
            "frame 2d material '{}' yield_strength must be positive and finite",
            material.element_id
        ));
    }
    if !(material.hardening_ratio.is_finite() && (0.0..1.0).contains(&material.hardening_ratio)) {
        return Err(format!(
            "frame 2d material '{}' hardening_ratio must be at least zero and less than one",
            material.element_id
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CompiledFrame2dMaterial, Frame2dMaterialHistory};

    #[test]
    fn bilinear_response_is_continuous_and_reports_plastic_strain() {
        let material = CompiledFrame2dMaterial {
            yield_strength: 250.0,
            hardening_ratio: 0.1,
        };
        let initial = Frame2dMaterialHistory::default();
        let elastic = material.response(1_000.0, 0.2, initial);
        let yield_point = material.response(1_000.0, 0.25, initial);
        let plastic = material.response(1_000.0, -0.5, initial);

        assert_eq!(elastic.stress, 200.0);
        assert_eq!(yield_point.stress, 250.0);
        assert_eq!(yield_point.history.equivalent_plastic_strain, 0.0);
        assert_eq!(plastic.stress, -275.0);
        assert_eq!(plastic.tangent_modulus, 100.0);
        assert!((plastic.history.plastic_strain + 0.225).abs() < 1.0e-12);
        assert!((plastic.history.backstress + 25.0).abs() < 1.0e-12);
        assert!((plastic.history.equivalent_plastic_strain - 0.225).abs() < 1.0e-12);
    }

    #[test]
    fn return_mapping_is_reversal_aware_and_trial_states_are_rollback_safe() {
        let material = CompiledFrame2dMaterial {
            yield_strength: 250.0,
            hardening_ratio: 0.1,
        };
        let loaded = material.response(1_000.0, 0.5, Frame2dMaterialHistory::default());
        let rejected_trial = material.response(1_000.0, 0.8, loaded.history);
        let unloaded = material.response(1_000.0, 0.0, loaded.history);
        let reversed = material.response(1_000.0, -0.5, loaded.history);

        assert_eq!(loaded.stress, 275.0);
        assert!(rejected_trial.history.equivalent_plastic_strain > 0.225);
        assert_eq!(unloaded.stress, -225.0);
        assert_eq!(unloaded.tangent_modulus, 1_000.0);
        assert_eq!(
            unloaded.history.plastic_strain,
            loaded.history.plastic_strain
        );
        assert_eq!(reversed.stress, -275.0);
        assert!((reversed.history.plastic_strain + 0.225).abs() < 1.0e-12);
        assert!((reversed.history.equivalent_plastic_strain - 0.675).abs() < 1.0e-12);
    }
}
