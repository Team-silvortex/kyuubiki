use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dFiberDamageInput, Frame2dFiberMaterialInput,
    Frame2dMonotonicBilinearMaterialInput, Frame2dNodeInput, Frame2dResidualStressTemplateInput,
    Frame2dSectionFiberInput, Frame2dSectionLayerInput, Frame2dSectionLibraryInput,
    Frame2dStabilityKinematics, SolveBucklingFrame2dRequest, SolveFrame2dMaterialPDeltaRequest,
    SolveFrame2dPDeltaRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::solve_frame_2d_material_p_delta;

const LENGTH: f64 = 4.0;
const AREA: f64 = 0.01;
const INERTIA: f64 = 5.0e-4;
const EFFECTIVE_YOUNGS_MODULUS: f64 = 135.0e9;
const REFERENCE_FORCE: f64 = 2.5e6;

#[test]
fn uniformly_distributed_composite_recovers_the_transformed_elastic_reference() {
    let load_factor = 0.4;
    let result = solve_frame_2d_material_p_delta(&request(load_factor))
        .expect("elastic composite path should converge");
    let expected_force = -load_factor * REFERENCE_FORCE;
    let expected_strain = expected_force / (EFFECTIVE_YOUNGS_MODULUS * AREA);

    assert!(result.stability_result.converged);
    assert_eq!(result.yielded_element_count, 0);
    assert_relative(
        result.stability_result.final_displacements[7],
        expected_strain * LENGTH,
        2.0e-5,
    );
    for state in &result.material_states {
        assert_eq!(state.fiber_material_ids, ["soft", "stiff"]);
        assert_eq!(state.fiber_point_count, 16);
        assert_eq!(state.yielded_fiber_point_count, 0);
        assert_relative(state.section_axial_force.unwrap(), expected_force, 2.0e-5);
        assert_relative(state.tangent_modulus, EFFECTIVE_YOUNGS_MODULUS, 1.0e-12);
    }
}

#[test]
fn composite_section_yields_the_soft_phase_before_the_stiff_phase() {
    let result =
        solve_frame_2d_material_p_delta(&request(0.9)).expect("composite path should converge");

    assert!(result.stability_result.converged);
    assert_eq!(result.yielded_element_count, 2);
    assert!(result.material_states.iter().all(|state| {
        state.fiber_material_ids == ["soft", "stiff"]
            && state.yielded_fiber_point_count > 0
            && state.yielded_fiber_point_count < state.fiber_point_count
            && state.max_fiber_equivalent_plastic_strain > 0.0
            && state.tangent_modulus < EFFECTIVE_YOUNGS_MODULUS
            && state.tangent_modulus > 90.0e9
    }));
}

#[test]
fn phase_local_damage_degrades_only_the_soft_composite_fibers() {
    let baseline =
        solve_frame_2d_material_p_delta(&request(0.9)).expect("baseline path should converge");
    let mut damaged_request = request(0.9);
    for material in &mut damaged_request.materials {
        material.fiber_materials[0].damage = Some(Frame2dFiberDamageInput {
            onset_equivalent_plastic_strain: 1.0e-5,
            failure_equivalent_plastic_strain: 0.02,
            maximum_damage: 0.2,
        });
    }
    let damaged = solve_frame_2d_material_p_delta(&damaged_request)
        .expect("phase-local damage path should converge");

    assert!(damaged.stability_result.converged);
    for (damaged_state, baseline_state) in damaged
        .material_states
        .iter()
        .zip(&baseline.material_states)
    {
        assert!(damaged_state.max_fiber_damage > 0.0);
        assert!(damaged_state.max_fiber_damage < 0.2);
        assert!(damaged_state.damaged_fiber_point_count > 0);
        assert!(damaged_state.damaged_fiber_point_count < damaged_state.fiber_point_count);
        assert!(damaged_state.tangent_modulus < baseline_state.tangent_modulus);
    }
    assert!(
        damaged.stability_result.final_displacements[7].abs()
            > baseline.stability_result.final_displacements[7].abs()
    );
}

#[test]
fn layered_composite_executes_material_ids_through_the_full_solver() {
    let mut request = request(0.2);
    let depth = (12.0 * INERTIA / AREA).sqrt();
    let width = AREA / depth;
    for material in &mut request.materials {
        material.section_fibers.clear();
        material.section_library = Some(Frame2dSectionLibraryInput::Layered {
            layers: vec![
                Frame2dSectionLayerInput {
                    y_min: -0.5 * depth,
                    y_max: 0.0,
                    width,
                    fiber_count: 4,
                    material_id: Some("soft".into()),
                },
                Frame2dSectionLayerInput {
                    y_min: 0.0,
                    y_max: 0.5 * depth,
                    width,
                    fiber_count: 4,
                    material_id: Some("stiff".into()),
                },
            ],
        });
        for definition in &mut material.fiber_materials {
            definition.youngs_modulus = EFFECTIVE_YOUNGS_MODULUS;
        }
    }

    let result = solve_frame_2d_material_p_delta(&request)
        .expect("layered composite material path should converge");
    assert!(result.stability_result.converged);
    assert!(result.material_states.iter().all(|state| {
        state.fiber_material_ids == ["soft", "stiff"]
            && state.fiber_point_count == 16
            && state.yielded_fiber_point_count == 0
    }));
}

#[test]
fn composite_material_catalog_rejects_ambiguous_and_unused_definitions() {
    let mut malformed = request(0.2);
    malformed.materials[0].fiber_materials[1].id = "soft".into();
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("duplicate fiber material IDs must be rejected");
    assert!(error.contains("is duplicated"));

    let mut malformed = request(0.2);
    malformed.materials[0].section_fibers[0].material_id = Some("missing".into());
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("unknown fiber material IDs must be rejected");
    assert!(error.contains("references unknown"));

    let mut malformed = request(0.2);
    for fiber in &mut malformed.materials[0].section_fibers {
        fiber.material_id = Some("soft".into());
    }
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("unused fiber material definitions must be rejected");
    assert!(error.contains("is not referenced"));

    let mut malformed = request(0.2);
    malformed.materials[0].section_fibers[0].initial_axial_stress = 110.0e6;
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("fiber initial stress must respect its referenced material");
    assert!(error.contains("remain within yield_strength"));

    let mut malformed = request(0.2);
    malformed.materials[0].fiber_materials[0].damage = Some(Frame2dFiberDamageInput {
        onset_equivalent_plastic_strain: 0.02,
        failure_equivalent_plastic_strain: 0.01,
        maximum_damage: 0.2,
    });
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("reversed damage thresholds must be rejected");
    assert!(error.contains("greater than onset"));
}

#[test]
fn residual_stress_template_executes_as_a_visible_self_equilibrated_field() {
    let mut request = request(0.2);
    let depth = (12.0 * INERTIA / AREA).sqrt();
    let width = AREA / depth;
    for material in &mut request.materials {
        material.section_fibers.clear();
        material.fiber_materials.clear();
        material.section_library = Some(Frame2dSectionLibraryInput::Rectangle {
            width,
            depth,
            fiber_count: 8,
        });
        material.residual_stress_template = Some(
            Frame2dResidualStressTemplateInput::SelfEquilibratedQuadratic {
                peak_stress: 50.0e6,
            },
        );
    }
    request.stability.maximum_load_factor = None;
    request.stability.load_steps = None;
    request.load_factor_schedule = Some(vec![0.0, 0.2]);

    let result = solve_frame_2d_material_p_delta(&request)
        .expect("self-equilibrated residual stress template should execute");
    let initial = &result.material_history[0];
    assert_eq!(initial.load_factor, 0.0);
    for state in &initial.material_states {
        assert_relative(
            state
                .max_fiber_initial_axial_stress
                .unwrap()
                .abs()
                .max(state.min_fiber_initial_axial_stress.unwrap().abs()),
            50.0e6,
            1.0e-12,
        );
        assert!(state.min_fiber_initial_axial_stress.unwrap() < 0.0);
        assert!(state.max_fiber_initial_axial_stress.unwrap() > 0.0);
        assert!(state.section_axial_force.unwrap().abs() < 1.0e-8);
        assert!(state.section_end_moment_i.unwrap().abs() < 1.0e-8);
        assert!(state.section_end_moment_j.unwrap().abs() < 1.0e-8);
    }

    request.materials[0].residual_stress_template = Some(
        Frame2dResidualStressTemplateInput::SelfEquilibratedQuadratic {
            peak_stress: 600.0e6,
        },
    );
    let error = solve_frame_2d_material_p_delta(&request)
        .expect_err("template stresses outside the phase yield surface must be rejected");
    assert!(error.contains("remain within yield_strength"));
}

fn request(maximum_load_factor: f64) -> SolveFrame2dMaterialPDeltaRequest {
    let nodes = vec![
        node("base", 0.0, true, 0.0),
        node("mid", LENGTH / 2.0, false, 0.0),
        node("top", LENGTH, false, -REFERENCE_FORCE),
    ];
    let elements = (0..2)
        .map(|index| Frame2dElementInput {
            id: format!("composite-e{index}"),
            node_i: index,
            node_j: index + 1,
            area: AREA,
            youngs_modulus: EFFECTIVE_YOUNGS_MODULUS,
            moment_of_inertia: INERTIA,
            section_modulus: INERTIA / 0.3,
        })
        .collect::<Vec<_>>();
    let materials = elements
        .iter()
        .map(|element| Frame2dMonotonicBilinearMaterialInput {
            element_id: element.id.clone(),
            yield_strength: 500.0e6,
            hardening_ratio: 0.02,
            initial_axial_stress: 0.0,
            section_library: None,
            section_fibers: composite_fibers(),
            fiber_materials: composite_materials(),
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
            tolerance: Some(1.0e-9),
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

fn composite_fibers() -> Vec<Frame2dSectionFiberInput> {
    [-0.3, -0.1, 0.1, 0.3]
        .into_iter()
        .flat_map(|y| {
            ["soft", "stiff"].map(|material_id| Frame2dSectionFiberInput {
                y,
                area: AREA / 8.0,
                initial_axial_stress: 0.0,
                material_id: Some(material_id.into()),
            })
        })
        .collect()
}

fn composite_materials() -> Vec<Frame2dFiberMaterialInput> {
    vec![
        Frame2dFiberMaterialInput {
            id: "soft".into(),
            youngs_modulus: 70.0e9,
            yield_strength: 100.0e6,
            hardening_ratio: 0.02,
            damage: None,
        },
        Frame2dFiberMaterialInput {
            id: "stiff".into(),
            youngs_modulus: 200.0e9,
            yield_strength: 500.0e6,
            hardening_ratio: 0.02,
            damage: None,
        },
    ]
}

fn node(id: &str, y: f64, fixed: bool, load_y: f64) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.into(),
        x: 0.0,
        y,
        fix_x: fixed,
        fix_y: fixed,
        fix_rz: fixed,
        load_x: 0.0,
        load_y,
        moment_z: 0.0,
    }
}

fn assert_relative(actual: f64, expected: f64, tolerance: f64) {
    let relative = (actual - expected).abs() / expected.abs().max(1.0e-14);
    assert!(
        relative <= tolerance,
        "actual={actual:.12e}, expected={expected:.12e}, relative={relative:.12e}"
    );
}
