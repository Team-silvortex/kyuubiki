use std::collections::{HashMap, HashSet};

use crate::frame_2d_corotational_element::element_deformation;
use crate::frame_2d_fiber_section::{committed_effective_axial_tangent, section_response};
use crate::frame_2d_p_delta::solve_frame_2d_p_delta_with_materials;
use kyuubiki_protocol::{
    Frame2dBilinearKinematicMaterialInput, Frame2dElementInput, Frame2dMaterialStateResult,
    Frame2dMaterialStepResult, Frame2dStabilityKinematics, Frame2dStabilityPathControl,
    SolveFrame2dMaterialPDeltaRequest, SolveFrame2dMaterialPDeltaResult,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct CompiledFrame2dFiber {
    pub(crate) y: f64,
    pub(crate) area: f64,
    pub(crate) initial_axial_stress: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct CompiledFrame2dMaterial {
    pub(crate) yield_strength: f64,
    pub(crate) hardening_ratio: f64,
    pub(crate) initial_axial_stress: f64,
    pub(crate) section_fibers: Vec<CompiledFrame2dFiber>,
    pub(crate) longitudinal_integration_points: usize,
    pub(crate) adaptive_longitudinal_integration: bool,
    pub(crate) longitudinal_integration_tolerance: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Frame2dMaterialPointHistory {
    pub(crate) plastic_strain: f64,
    pub(crate) backstress: f64,
    pub(crate) equivalent_plastic_strain: f64,
    pub(crate) tangent_modulus: f64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Frame2dMaterialHistory {
    pub(crate) point: Frame2dMaterialPointHistory,
    pub(crate) fiber_points: Vec<Frame2dMaterialPointHistory>,
    pub(crate) active_longitudinal_integration_points: usize,
    pub(crate) longitudinal_integration_error: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BilinearResponse {
    pub(crate) stress: f64,
    pub(crate) tangent_modulus: f64,
    pub(crate) history: Frame2dMaterialPointHistory,
}

impl CompiledFrame2dMaterial {
    pub(crate) fn has_initial_stress(&self) -> bool {
        self.initial_axial_stress != 0.0
            || self
                .section_fibers
                .iter()
                .any(|fiber| fiber.initial_axial_stress != 0.0)
    }

    pub(crate) fn response(
        &self,
        youngs_modulus: f64,
        strain: f64,
        committed: &Frame2dMaterialPointHistory,
        initial_axial_stress: f64,
    ) -> BilinearResponse {
        let trial_stress =
            initial_axial_stress + youngs_modulus * (strain - committed.plastic_strain);
        let relative_trial = trial_stress - committed.backstress;
        let yield_excess = relative_trial.abs() - self.yield_strength;
        if yield_excess <= self.yield_strength * 1.0e-12 {
            let mut history = *committed;
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
            history: Frame2dMaterialPointHistory {
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
    let (stability_result, committed_states, history_steps) =
        solve_frame_2d_p_delta_with_materials(
            &request.stability,
            &compiled,
            request.load_factor_schedule.as_deref(),
        )?;
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
    let material_history = stability_result
        .steps
        .iter()
        .zip(&history_steps)
        .map(|(step, histories)| {
            Ok(Frame2dMaterialStepResult {
                step: step.step,
                load_factor: step.load_factor,
                achieved_load_factor: step.achieved_load_factor.unwrap_or(step.load_factor),
                converged: step.converged,
                material_states: material_state_results(
                    &positions,
                    &frame.elements,
                    &step.displacements,
                    &compiled,
                    histories,
                )?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let material_states = material_state_results(
        &positions,
        &frame.elements,
        &stability_result.final_displacements,
        &compiled,
        &committed_states,
    )?;
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
        material_history,
    })
}

fn material_state_results(
    positions: &[(f64, f64)],
    elements: &[Frame2dElementInput],
    displacement: &[f64],
    materials: &[Option<CompiledFrame2dMaterial>],
    histories: &[Frame2dMaterialHistory],
) -> Result<Vec<Frame2dMaterialStateResult>, String> {
    elements
        .iter()
        .enumerate()
        .filter_map(|(element_index, element)| {
            materials
                .get(element_index)
                .and_then(Option::as_ref)
                .map(|material| (element_index, element, material))
        })
        .map(|(element_index, element, material)| {
            let history = histories.get(element_index).cloned().unwrap_or_default();
            let deformation = element_deformation(positions, element, displacement)?;
            let section = section_response(
                Some(material),
                element.youngs_modulus,
                element.area,
                element.moment_of_inertia,
                deformation.length,
                deformation.extension,
                deformation.phi_i,
                deformation.phi_j,
                &history,
            );
            let tangent_modulus = committed_effective_axial_tangent(
                material,
                element.youngs_modulus,
                element.area,
                &history,
            );
            Ok(Frame2dMaterialStateResult {
                element_index,
                element_id: element.id.clone(),
                axial_strain: deformation.axial_strain,
                axial_stress: section.average_stress,
                initial_axial_stress: section.average_initial_stress,
                plastic_strain: section.average_plastic_strain,
                backstress: section.average_backstress,
                equivalent_plastic_strain: section.max_equivalent_plastic_strain,
                tangent_modulus,
                yielded: section.max_equivalent_plastic_strain > 0.0,
                section_axial_force: Some(section.axial_force),
                section_end_moment_i: Some(section.moment_i),
                section_end_moment_j: Some(section.moment_j),
                fiber_point_count: section.fiber_point_count,
                evaluated_fiber_point_count: section.evaluated_fiber_point_count,
                yielded_fiber_point_count: section.yielded_fiber_point_count,
                max_fiber_equivalent_plastic_strain: section.max_equivalent_plastic_strain,
                active_longitudinal_integration_points: section
                    .active_longitudinal_integration_points,
                longitudinal_integration_error: section.longitudinal_integration_error,
            })
        })
        .collect()
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
        let Some(material) = materials.get(element_index).and_then(Option::as_ref) else {
            continue;
        };
        let deformation = element_deformation(positions, element, displacement)?;
        updated[element_index] = section_response(
            Some(material),
            element.youngs_modulus,
            element.area,
            element.moment_of_inertia,
            deformation.length,
            deformation.extension,
            deformation.phi_i,
            deformation.phi_j,
            &updated[element_index],
        )
        .history;
    }
    Ok(updated)
}

fn validate_control_contract(request: &SolveFrame2dMaterialPDeltaRequest) -> Result<(), String> {
    if request.stability.kinematics != Frame2dStabilityKinematics::Corotational {
        return Err("frame 2d material p-delta requires corotational kinematics".into());
    }
    if request.stability.path_control != Frame2dStabilityPathControl::LoadControl {
        return Err("frame 2d material p-delta supports load control only".into());
    }
    if request.materials.is_empty() {
        return Err("frame 2d material p-delta requires at least one material assignment".into());
    }
    if let Some(schedule) = &request.load_factor_schedule {
        if request.stability.maximum_load_factor.is_some() || request.stability.load_steps.is_some()
        {
            return Err(
                "frame 2d material load_factor_schedule cannot be combined with maximum_load_factor or load_steps"
                    .into(),
            );
        }
        if schedule.is_empty() || schedule.len() > 256 {
            return Err(
                "frame 2d material load_factor_schedule must contain between 1 and 256 points"
                    .into(),
            );
        }
        let mut previous = 0.0;
        for (index, &factor) in schedule.iter().enumerate() {
            if !factor.is_finite() {
                return Err(format!(
                    "frame 2d material load_factor_schedule[{index}] must be finite"
                ));
            }
            if index > 0 && factor == previous {
                return Err(format!(
                    "frame 2d material load_factor_schedule[{index}] duplicates the previous factor"
                ));
            }
            previous = factor;
        }
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
        validate_material(material, &elements[element_index])?;
        compiled[element_index] = Some(CompiledFrame2dMaterial {
            yield_strength: material.yield_strength,
            hardening_ratio: material.hardening_ratio,
            initial_axial_stress: material.initial_axial_stress,
            section_fibers: material
                .section_fibers
                .iter()
                .map(|fiber| CompiledFrame2dFiber {
                    y: fiber.y,
                    area: fiber.area,
                    initial_axial_stress: fiber.initial_axial_stress,
                })
                .collect(),
            longitudinal_integration_points: material.longitudinal_integration_points,
            adaptive_longitudinal_integration: material.adaptive_longitudinal_integration,
            longitudinal_integration_tolerance: material.longitudinal_integration_tolerance,
        });
    }
    Ok(compiled)
}

fn validate_material(
    material: &Frame2dBilinearKinematicMaterialInput,
    element: &Frame2dElementInput,
) -> Result<(), String> {
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
    if !material.initial_axial_stress.is_finite() {
        return Err(format!(
            "frame 2d material '{}' initial_axial_stress must be finite",
            material.element_id
        ));
    }
    if material.initial_axial_stress.abs() > material.yield_strength {
        return Err(format!(
            "frame 2d material '{}' initial_axial_stress must remain within yield_strength",
            material.element_id
        ));
    }
    validate_section_fibers(material, element)?;
    Ok(())
}

fn validate_section_fibers(
    material: &Frame2dBilinearKinematicMaterialInput,
    element: &Frame2dElementInput,
) -> Result<(), String> {
    if material.section_fibers.is_empty() {
        if material.longitudinal_integration_points != 2
            || material.adaptive_longitudinal_integration
        {
            return Err(format!(
                "frame 2d material '{}' longitudinal integration controls require section_fibers",
                material.element_id
            ));
        }
        return Ok(());
    }
    if !(2..=4).contains(&material.longitudinal_integration_points) {
        return Err(format!(
            "frame 2d material '{}' longitudinal_integration_points must be between 2 and 4",
            material.element_id
        ));
    }
    if !(material.longitudinal_integration_tolerance.is_finite()
        && material.longitudinal_integration_tolerance > 0.0
        && material.longitudinal_integration_tolerance <= 0.25)
    {
        return Err(format!(
            "frame 2d material '{}' longitudinal_integration_tolerance must be finite and in (0, 0.25]",
            material.element_id
        ));
    }
    if !(2..=32).contains(&material.section_fibers.len()) {
        return Err(format!(
            "frame 2d material '{}' section_fibers must contain between 2 and 32 fibers",
            material.element_id
        ));
    }
    if material.initial_axial_stress != 0.0 {
        return Err(format!(
            "frame 2d material '{}' cannot combine uniform initial_axial_stress with section_fibers",
            material.element_id
        ));
    }
    let mut area = 0.0;
    let mut first_moment = 0.0;
    let mut inertia = 0.0;
    let mut max_y = 0.0_f64;
    for (index, fiber) in material.section_fibers.iter().enumerate() {
        if !(fiber.y.is_finite() && fiber.area.is_finite() && fiber.area > 0.0) {
            return Err(format!(
                "frame 2d material '{}' section_fibers[{index}] requires finite y and positive finite area",
                material.element_id
            ));
        }
        if !fiber.initial_axial_stress.is_finite()
            || fiber.initial_axial_stress.abs() > material.yield_strength
        {
            return Err(format!(
                "frame 2d material '{}' section_fibers[{index}] initial_axial_stress must be finite and remain within yield_strength",
                material.element_id
            ));
        }
        area += fiber.area;
        first_moment += fiber.area * fiber.y;
        inertia += fiber.area * fiber.y * fiber.y;
        max_y = max_y.max(fiber.y.abs());
    }
    let area_tolerance = element.area.abs().max(f64::MIN_POSITIVE) * 1.0e-8;
    if (area - element.area).abs() > area_tolerance {
        return Err(format!(
            "frame 2d material '{}' section fiber area must match element area",
            material.element_id
        ));
    }
    let centroid_tolerance = element.area * max_y.max(1.0e-12) * 1.0e-8;
    if first_moment.abs() > centroid_tolerance {
        return Err(format!(
            "frame 2d material '{}' section fibers must be centered at y=0",
            material.element_id
        ));
    }
    let inertia_tolerance = element.moment_of_inertia.abs().max(f64::MIN_POSITIVE) * 1.0e-8;
    if (inertia - element.moment_of_inertia).abs() > inertia_tolerance {
        return Err(format!(
            "frame 2d material '{}' section fiber inertia must match element moment_of_inertia",
            material.element_id
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CompiledFrame2dMaterial, Frame2dMaterialPointHistory};

    #[test]
    fn bilinear_response_is_continuous_and_reports_plastic_strain() {
        let material = CompiledFrame2dMaterial {
            yield_strength: 250.0,
            hardening_ratio: 0.1,
            initial_axial_stress: 0.0,
            section_fibers: Vec::new(),
            longitudinal_integration_points: 2,
            adaptive_longitudinal_integration: false,
            longitudinal_integration_tolerance: 1.0e-3,
        };
        let initial = Frame2dMaterialPointHistory::default();
        let elastic = material.response(1_000.0, 0.2, &initial, 0.0);
        let yield_point = material.response(1_000.0, 0.25, &initial, 0.0);
        let plastic = material.response(1_000.0, -0.5, &initial, 0.0);

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
            initial_axial_stress: 0.0,
            section_fibers: Vec::new(),
            longitudinal_integration_points: 2,
            adaptive_longitudinal_integration: false,
            longitudinal_integration_tolerance: 1.0e-3,
        };
        let initial = Frame2dMaterialPointHistory::default();
        let loaded = material.response(1_000.0, 0.5, &initial, 0.0);
        let rejected_trial = material.response(1_000.0, 0.8, &loaded.history, 0.0);
        let unloaded = material.response(1_000.0, 0.0, &loaded.history, 0.0);
        let reversed = material.response(1_000.0, -0.5, &loaded.history, 0.0);

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

    #[test]
    fn initial_stress_is_an_observable_elastic_offset() {
        let material = CompiledFrame2dMaterial {
            yield_strength: 250.0,
            hardening_ratio: 0.1,
            initial_axial_stress: 50.0,
            section_fibers: Vec::new(),
            longitudinal_integration_points: 2,
            adaptive_longitudinal_integration: false,
            longitudinal_integration_tolerance: 1.0e-3,
        };

        let initial = material.response(
            1_000.0,
            0.0,
            &Frame2dMaterialPointHistory::default(),
            material.initial_axial_stress,
        );

        assert_eq!(initial.stress, 50.0);
        assert_eq!(initial.tangent_modulus, 1_000.0);
        assert_eq!(initial.history.plastic_strain, 0.0);
        assert_eq!(initial.history.equivalent_plastic_strain, 0.0);
    }
}
