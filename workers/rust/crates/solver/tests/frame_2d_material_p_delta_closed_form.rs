use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dMonotonicBilinearMaterialInput, Frame2dNodeInput,
    Frame2dStabilityKinematics, SolveBucklingFrame2dRequest, SolveFrame2dMaterialPDeltaRequest,
    SolveFrame2dPDeltaRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::solve_frame_2d_material_p_delta;

const ELEMENT_COUNT: usize = 16;
const LENGTH: f64 = 4.0;
const AREA: f64 = 0.01;
const YOUNGS_MODULUS: f64 = 200.0e9;
const YIELD_STRENGTH: f64 = 250.0e6;
const HARDENING_RATIO: f64 = 0.05;
const REFERENCE_FORCE: f64 = AREA * YIELD_STRENGTH;

#[test]
fn monotonic_column_tracks_the_external_bilinear_axial_reference() {
    let maximum_factor = 1.3;
    let result = solve_frame_2d_material_p_delta(&request(maximum_factor))
        .expect("monotonic material column should converge");

    assert!(result.stability_result.converged);
    assert_eq!(result.stability_result.steps.len(), 13);
    assert_eq!(result.material_states.len(), ELEMENT_COUNT);
    assert_eq!(result.yielded_element_count, ELEMENT_COUNT);

    let expected_strain = -(YIELD_STRENGTH / YOUNGS_MODULUS
        + (maximum_factor - 1.0) * YIELD_STRENGTH / (HARDENING_RATIO * YOUNGS_MODULUS));
    let expected_stress = -maximum_factor * YIELD_STRENGTH;
    let expected_signed_plastic_strain = expected_strain - expected_stress / YOUNGS_MODULUS;
    let expected_plastic_strain = expected_signed_plastic_strain.abs();
    let plastic_modulus = YOUNGS_MODULUS * HARDENING_RATIO / (1.0 - HARDENING_RATIO);
    let expected_backstress = plastic_modulus * expected_signed_plastic_strain;
    let top_y = result.stability_result.final_displacements[ELEMENT_COUNT * 3 + 1];
    assert_relative(top_y, expected_strain * LENGTH, 2.0e-7);
    assert_relative(
        result.max_equivalent_plastic_strain,
        expected_plastic_strain,
        2.0e-7,
    );
    for state in &result.material_states {
        assert_relative(state.axial_strain, expected_strain, 2.0e-7);
        assert_relative(state.axial_stress, expected_stress, 2.0e-7);
        assert_relative(state.plastic_strain, expected_signed_plastic_strain, 2.0e-7);
        assert_relative(state.backstress, expected_backstress, 2.0e-7);
        assert_relative(
            state.equivalent_plastic_strain,
            expected_plastic_strain,
            2.0e-7,
        );
        assert_eq!(state.tangent_modulus, HARDENING_RATIO * YOUNGS_MODULUS);
        assert!(state.yielded);
    }
}

#[test]
fn material_contract_rejects_duplicate_unknown_and_non_monotonic_assignments() {
    let mut duplicate = request(0.8);
    duplicate.materials.push(duplicate.materials[0].clone());
    let error = solve_frame_2d_material_p_delta(&duplicate)
        .expect_err("duplicate assignments must be rejected");
    assert!(error.contains("duplicate material assignments"));

    let mut unknown = request(0.8);
    unknown.materials[0].element_id = "missing".into();
    let error = solve_frame_2d_material_p_delta(&unknown)
        .expect_err("unknown element assignments must be rejected");
    assert!(error.contains("unknown element"));

    let mut duplicate_ids = request(0.8);
    duplicate_ids.stability.buckling.frame.elements[1].id = "e0".into();
    let error = solve_frame_2d_material_p_delta(&duplicate_ids)
        .expect_err("ambiguous element IDs must be rejected");
    assert!(error.contains("requires unique element IDs"));

    let mut arc_length = request(0.8);
    arc_length.stability.path_control = kyuubiki_protocol::Frame2dStabilityPathControl::ArcLength;
    let error = solve_frame_2d_material_p_delta(&arc_length)
        .expect_err("history-free material contract must reject arc length");
    assert!(error.contains("monotonic load control only"));
}

#[test]
fn failed_newton_attempt_does_not_commit_material_history() {
    let mut request = request(1.3);
    request.stability.max_iterations = Some(1);
    request.stability.max_step_cutbacks = Some(0);

    let result = solve_frame_2d_material_p_delta(&request)
        .expect("failed material equilibrium should remain inspectable");

    assert!(!result.stability_result.converged);
    assert_eq!(result.stability_result.steps.len(), 1);
    assert_eq!(
        result.stability_result.steps[0].failure_reason,
        Some(kyuubiki_protocol::Frame2dPDeltaFailureReason::CutbackLimitExhausted)
    );
    assert_eq!(
        result.stability_result.steps[0].achieved_load_factor,
        Some(0.0)
    );
    assert_eq!(result.yielded_element_count, 0);
    assert_eq!(result.max_equivalent_plastic_strain, 0.0);
    for state in &result.material_states {
        assert_eq!(state.plastic_strain, 0.0);
        assert_eq!(state.backstress, 0.0);
        assert_eq!(state.equivalent_plastic_strain, 0.0);
    }
}

fn request(maximum_load_factor: f64) -> SolveFrame2dMaterialPDeltaRequest {
    let segment = LENGTH / ELEMENT_COUNT as f64;
    let nodes = (0..=ELEMENT_COUNT)
        .map(|index| Frame2dNodeInput {
            id: format!("n{index}"),
            x: 0.0,
            y: index as f64 * segment,
            fix_x: index == 0 || index == ELEMENT_COUNT,
            fix_y: index == 0,
            fix_rz: false,
            load_x: 0.0,
            load_y: if index == ELEMENT_COUNT {
                -REFERENCE_FORCE
            } else {
                0.0
            },
            moment_z: 0.0,
        })
        .collect::<Vec<_>>();
    let elements = (0..ELEMENT_COUNT)
        .map(|index| Frame2dElementInput {
            id: format!("e{index}"),
            node_i: index,
            node_j: index + 1,
            area: AREA,
            youngs_modulus: YOUNGS_MODULUS,
            moment_of_inertia: 0.1,
            section_modulus: 0.1,
        })
        .collect::<Vec<_>>();
    let materials = elements
        .iter()
        .map(|element| Frame2dMonotonicBilinearMaterialInput {
            element_id: element.id.clone(),
            yield_strength: YIELD_STRENGTH,
            hardening_ratio: HARDENING_RATIO,
        })
        .collect();
    SolveFrame2dMaterialPDeltaRequest {
        stability: SolveFrame2dPDeltaRequest {
            buckling: SolveBucklingFrame2dRequest {
                frame: SolveFrame2dRequest { nodes, elements },
                mode_count: Some(1),
            },
            imperfection_amplitude: 1.0e-8,
            kinematics: Frame2dStabilityKinematics::Corotational,
            path_control: Default::default(),
            imperfection_shape: None,
            imperfection_mode_index: Some(0),
            maximum_load_factor: Some(maximum_load_factor),
            load_steps: Some(13),
            max_iterations: Some(64),
            tolerance: Some(1.0e-10),
            max_step_cutbacks: Some(12),
            arc_length_radius: None,
            arc_length_load_scale: None,
            arc_length_target_iterations: None,
            tangent_transition_refinement_steps: None,
            branch_switch: Default::default(),
            branch_switch_amplitude: None,
            branch_switch_mode_count: None,
            branch_switch_pairwise_combinations: false,
            branch_switch_mode_weights: None,
            branch_switch_subspace_sample_count: None,
            branch_switch_subspace_refinement_levels: None,
            branch_continuation_steps: None,
            branch_continuation_radius: None,
            branch_continuation_min_radius_ratio: None,
            continuation_state: None,
        },
        materials,
    }
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0e-14);
    assert!(
        relative <= tolerance,
        "actual={actual:.12e}, expected={expected:.12e}, relative={relative:.12e}"
    );
}
