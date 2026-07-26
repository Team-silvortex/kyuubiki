use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dMonotonicBilinearMaterialInput, Frame2dNodeInput,
    Frame2dStabilityKinematics, SolveBucklingFrame2dRequest, SolveFrame2dMaterialPDeltaRequest,
    SolveFrame2dPDeltaRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::{solve_frame_2d_material_p_delta, solve_frame_2d_p_delta};

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

    assert!(
        result.stability_result.converged,
        "steps={:?}",
        result.stability_result.steps
    );
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
fn material_contract_rejects_duplicate_unknown_and_invalid_control_assignments() {
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
    assert!(error.contains("load control only"));
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

#[test]
fn cyclic_schedule_tracks_reversal_residual_strain_and_accumulated_plasticity() {
    let factors = [1.3, 0.0, -1.3, 0.0, 1.3];
    let mut request = request(1.3);
    request.stability.maximum_load_factor = None;
    request.stability.load_steps = None;
    request.load_factor_schedule = Some(factors.to_vec());

    let result =
        solve_frame_2d_material_p_delta(&request).expect("cyclic material column should converge");

    assert!(
        result.stability_result.converged,
        "steps={:?}",
        result.stability_result.steps
    );
    assert_eq!(result.material_history.len(), factors.len());
    let plastic_modulus = YOUNGS_MODULUS * HARDENING_RATIO / (1.0 - HARDENING_RATIO);
    let plastic_strain_amplitude = (0.3 * YIELD_STRENGTH) / plastic_modulus;
    let expected_plastic_strains = [
        -plastic_strain_amplitude,
        -plastic_strain_amplitude,
        plastic_strain_amplitude,
        plastic_strain_amplitude,
        -plastic_strain_amplitude,
    ];
    let expected_equivalent = [
        plastic_strain_amplitude,
        plastic_strain_amplitude,
        3.0 * plastic_strain_amplitude,
        3.0 * plastic_strain_amplitude,
        5.0 * plastic_strain_amplitude,
    ];

    for (index, history) in result.material_history.iter().enumerate() {
        assert!(history.converged);
        assert_relative(history.load_factor, factors[index], 1.0e-12);
        assert_relative(history.achieved_load_factor, factors[index], 1.0e-12);
        let expected_stress = -factors[index] * YIELD_STRENGTH;
        for state in &history.material_states {
            assert_absolute(state.axial_stress, expected_stress, 2.0);
            assert_relative(
                state.plastic_strain,
                expected_plastic_strains[index],
                2.0e-7,
            );
            assert_relative(
                state.backstress,
                plastic_modulus * expected_plastic_strains[index],
                2.0e-7,
            );
            assert_relative(
                state.equivalent_plastic_strain,
                expected_equivalent[index],
                2.0e-7,
            );
        }
    }
    let increments = result
        .stability_result
        .steps
        .iter()
        .map(|step| step.load_factor_increment.unwrap())
        .collect::<Vec<_>>();
    assert_eq!(increments, vec![1.3, -1.3, -1.3, 1.3, 1.3]);
}

#[test]
fn cyclic_schedule_contract_rejects_ambiguous_and_malformed_paths() {
    let mut ambiguous = request(1.3);
    ambiguous.load_factor_schedule = Some(vec![1.3, 0.0, -1.3]);
    let error = solve_frame_2d_material_p_delta(&ambiguous)
        .expect_err("mixed generated and explicit paths must be rejected");
    assert!(error.contains("cannot be combined"));

    let mut malformed = request(1.3);
    malformed.stability.maximum_load_factor = None;
    malformed.stability.load_steps = None;
    malformed.load_factor_schedule = Some(vec![1.0, 1.0]);
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("duplicate path points must be rejected");
    assert!(error.contains("duplicates the previous factor"));

    malformed.load_factor_schedule = Some(vec![f64::NAN]);
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("non-finite path points must be rejected");
    assert!(error.contains("must be finite"));

    malformed.load_factor_schedule = Some(vec![0.0, 0.1]);
    solve_frame_2d_material_p_delta(&malformed)
        .expect("an explicit initial zero-load observation must be accepted");
}

#[test]
fn self_equilibrated_initial_stress_is_visible_at_zero_load() {
    let request = residual_stress_request();
    let result = solve_frame_2d_material_p_delta(&request)
        .expect("balanced parallel residual stresses should converge");

    assert!(result.stability_result.converged);
    assert_eq!(result.material_history.len(), 2);
    let initial = &result.material_history[0];
    assert_eq!(initial.load_factor, 0.0);
    assert_eq!(initial.achieved_load_factor, 0.0);
    assert!(initial.converged);
    assert_eq!(initial.material_states.len(), 4);
    for (index, state) in initial.material_states.iter().enumerate() {
        let expected = if index % 2 == 0 {
            0.2 * YIELD_STRENGTH
        } else {
            -0.2 * YIELD_STRENGTH
        };
        assert_absolute(state.initial_axial_stress, expected, 1.0e-9);
        assert_absolute(state.axial_stress, expected, 1.0e-6);
        assert_eq!(state.plastic_strain, 0.0);
        assert_eq!(state.equivalent_plastic_strain, 0.0);
        assert_eq!(state.tangent_modulus, YOUNGS_MODULUS);
    }
}

#[test]
fn initial_stress_contract_rejects_unbalanced_and_over_yield_states() {
    let mut unbalanced = request(0.2);
    unbalanced.materials[0].initial_axial_stress = 0.2 * YIELD_STRENGTH;
    let error = solve_frame_2d_material_p_delta(&unbalanced)
        .expect_err("a free residual force must not define a hidden preload");
    assert!(error.contains("not self-equilibrated on free DOFs"));

    let mut over_yield = request(0.2);
    over_yield.materials[0].initial_axial_stress = 1.01 * YIELD_STRENGTH;
    let error = solve_frame_2d_material_p_delta(&over_yield)
        .expect_err("an initially inadmissible material state must be rejected");
    assert!(error.contains("must remain within yield_strength"));
}

#[test]
fn self_equilibrated_initial_stress_shifts_the_buckling_baseline() {
    let compression =
        solve_frame_2d_material_p_delta(&prestressed_chain_request(-0.05 * YIELD_STRENGTH))
            .expect("self-equilibrated compression should retain a buckling baseline");
    let neutral_request = prestressed_chain_request(0.0);
    let neutral = solve_frame_2d_material_p_delta(&neutral_request)
        .expect("unstressed chain should retain a buckling baseline");
    let elastic = solve_frame_2d_p_delta(&neutral_request.stability)
        .expect("zero initial stress should retain the ordinary elastic baseline");
    let tension =
        solve_frame_2d_material_p_delta(&prestressed_chain_request(0.05 * YIELD_STRENGTH))
            .expect("self-equilibrated tension should retain a buckling baseline");

    let compressed_factor = compression
        .stability_result
        .buckling_result
        .minimum_load_factor;
    let neutral_factor = neutral.stability_result.buckling_result.minimum_load_factor;
    let tensioned_factor = tension.stability_result.buckling_result.minimum_load_factor;
    assert_relative(
        neutral_factor,
        elastic.buckling_result.minimum_load_factor,
        1.0e-12,
    );
    assert!(
        compressed_factor < neutral_factor && neutral_factor < tensioned_factor,
        "compressed={compressed_factor:.12e}, neutral={neutral_factor:.12e}, \
         tensioned={tensioned_factor:.12e}"
    );
    eprintln!(
        "initial-stress buckling factors: compression={compressed_factor:.9e}, \
         neutral={neutral_factor:.9e}, tension={tensioned_factor:.9e}"
    );
    assert_relative(
        0.5 * (compressed_factor + tensioned_factor),
        neutral_factor,
        5.0e-5,
    );
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
            initial_axial_stress: 0.0,
            section_library: None,
            section_fibers: Vec::new(),
            fiber_materials: Vec::new(),
            residual_stress_template: None,
            longitudinal_integration_points: 2,
            adaptive_longitudinal_integration: false,
            longitudinal_integration_tolerance: 1.0e-3,
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
        load_factor_schedule: None,
    }
}

fn residual_stress_request() -> SolveFrame2dMaterialPDeltaRequest {
    let nodes = vec![
        Frame2dNodeInput {
            id: "base".into(),
            x: 0.0,
            y: 0.0,
            fix_x: true,
            fix_y: true,
            fix_rz: false,
            load_x: 0.0,
            load_y: 0.0,
            moment_z: 0.0,
        },
        Frame2dNodeInput {
            id: "mid".into(),
            x: 0.0,
            y: LENGTH / 2.0,
            fix_x: false,
            fix_y: false,
            fix_rz: false,
            load_x: 0.0,
            load_y: 0.0,
            moment_z: 0.0,
        },
        Frame2dNodeInput {
            id: "top".into(),
            x: 0.0,
            y: LENGTH,
            fix_x: true,
            fix_y: false,
            fix_rz: false,
            load_x: 0.0,
            load_y: -REFERENCE_FORCE,
            moment_z: 0.0,
        },
    ];
    let elements = (0..2)
        .flat_map(|segment| {
            (0..2).map(move |parallel| Frame2dElementInput {
                id: format!("e{segment}-{parallel}"),
                node_i: segment,
                node_j: segment + 1,
                area: AREA,
                youngs_modulus: YOUNGS_MODULUS,
                moment_of_inertia: 0.05,
                section_modulus: 0.1,
            })
        })
        .collect::<Vec<_>>();
    let materials = elements
        .iter()
        .enumerate()
        .map(|(index, element)| Frame2dMonotonicBilinearMaterialInput {
            element_id: element.id.clone(),
            yield_strength: YIELD_STRENGTH,
            hardening_ratio: HARDENING_RATIO,
            initial_axial_stress: if index % 2 == 0 {
                0.2 * YIELD_STRENGTH
            } else {
                -0.2 * YIELD_STRENGTH
            },
            section_library: None,
            section_fibers: Vec::new(),
            fiber_materials: Vec::new(),
            residual_stress_template: None,
            longitudinal_integration_points: 2,
            adaptive_longitudinal_integration: false,
            longitudinal_integration_tolerance: 1.0e-3,
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
            maximum_load_factor: None,
            load_steps: None,
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
        load_factor_schedule: Some(vec![0.0, 0.2]),
    }
}

fn prestressed_chain_request(initial_axial_stress: f64) -> SolveFrame2dMaterialPDeltaRequest {
    let nodes = vec![
        Frame2dNodeInput {
            id: "base".into(),
            x: 0.0,
            y: 0.0,
            fix_x: true,
            fix_y: true,
            fix_rz: true,
            load_x: 0.0,
            load_y: 0.0,
            moment_z: 0.0,
        },
        Frame2dNodeInput {
            id: "joint".into(),
            x: 0.0,
            y: LENGTH / 2.0,
            fix_x: false,
            fix_y: false,
            fix_rz: false,
            load_x: 0.0,
            load_y: -REFERENCE_FORCE,
            moment_z: 0.0,
        },
        Frame2dNodeInput {
            id: "top".into(),
            x: 0.0,
            y: LENGTH,
            fix_x: true,
            fix_y: true,
            fix_rz: true,
            load_x: 0.0,
            load_y: 0.0,
            moment_z: 0.0,
        },
    ];
    let elements = (0..2)
        .map(|index| Frame2dElementInput {
            id: format!("prestressed-e{index}"),
            node_i: index,
            node_j: index + 1,
            area: AREA,
            youngs_modulus: YOUNGS_MODULUS,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.0e-4,
        })
        .collect::<Vec<_>>();
    let materials = elements
        .iter()
        .map(|element| Frame2dMonotonicBilinearMaterialInput {
            element_id: element.id.clone(),
            yield_strength: YIELD_STRENGTH,
            hardening_ratio: HARDENING_RATIO,
            initial_axial_stress,
            section_library: None,
            section_fibers: Vec::new(),
            fiber_materials: Vec::new(),
            residual_stress_template: None,
            longitudinal_integration_points: 2,
            adaptive_longitudinal_integration: false,
            longitudinal_integration_tolerance: 1.0e-3,
        })
        .collect();
    SolveFrame2dMaterialPDeltaRequest {
        stability: SolveFrame2dPDeltaRequest {
            buckling: SolveBucklingFrame2dRequest {
                frame: SolveFrame2dRequest { nodes, elements },
                mode_count: Some(1),
            },
            imperfection_amplitude: 1.0e-12,
            kinematics: Frame2dStabilityKinematics::Corotational,
            path_control: Default::default(),
            imperfection_shape: None,
            imperfection_mode_index: Some(0),
            maximum_load_factor: Some(1.0e-3),
            load_steps: Some(1),
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
        load_factor_schedule: None,
    }
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0e-14);
    assert!(
        relative <= tolerance,
        "actual={actual:.12e}, expected={expected:.12e}, relative={relative:.12e}"
    );
}

fn assert_absolute(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.12e}, expected={expected:.12e}, tolerance={tolerance:.12e}"
    );
}
