use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dMonotonicBilinearMaterialInput, Frame2dNodeInput,
    Frame2dSectionFiberInput, Frame2dStabilityKinematics, SolveBucklingFrame2dRequest,
    SolveFrame2dMaterialPDeltaRequest, SolveFrame2dPDeltaRequest, SolveFrame2dRequest,
};
use kyuubiki_solver::solve_frame_2d_material_p_delta;

const LENGTH: f64 = 4.0;
const AREA: f64 = 0.01;
const INERTIA: f64 = 5.0e-4;
const YOUNGS_MODULUS: f64 = 200.0e9;
const YIELD_STRENGTH: f64 = 250.0e6;
const HARDENING_RATIO: f64 = 0.05;
const REFERENCE_FORCE: f64 = AREA * YIELD_STRENGTH;
const REFERENCE_MOMENT: f64 = 0.8 * YIELD_STRENGTH * INERTIA / 0.3;

#[test]
fn fiber_section_recovers_the_axial_bilinear_reference() {
    let maximum_factor = 1.2;
    let result = solve_frame_2d_material_p_delta(&request(maximum_factor, 0.0, [0.0; 4]))
        .expect("fiber-section axial path should converge");

    assert!(
        result.stability_result.converged,
        "steps={:#?}",
        result.stability_result.steps
    );
    assert_eq!(result.material_states.len(), 2);
    assert_eq!(result.yielded_element_count, 2);
    let expected_strain = -(YIELD_STRENGTH / YOUNGS_MODULUS
        + (maximum_factor - 1.0) * YIELD_STRENGTH / (HARDENING_RATIO * YOUNGS_MODULUS));
    let top_y = result.stability_result.final_displacements[7];
    assert_relative(top_y, expected_strain * LENGTH, 3.0e-6);
    for state in &result.material_states {
        assert_relative(state.axial_strain, expected_strain, 3.0e-6);
        assert_relative(
            state.section_axial_force.unwrap(),
            -maximum_factor * REFERENCE_FORCE,
            3.0e-6,
        );
        assert_absolute(state.section_end_moment_i.unwrap(), 0.0, 0.1);
        assert_absolute(state.section_end_moment_j.unwrap(), 0.0, 0.1);
        assert_eq!(state.fiber_point_count, 8);
        assert_eq!(state.yielded_fiber_point_count, 8);
        assert!(state.max_fiber_equivalent_plastic_strain > 0.0);
        assert_relative(
            state.tangent_modulus,
            HARDENING_RATIO * YOUNGS_MODULUS,
            1.0e-12,
        );
    }
}

#[test]
fn combined_axial_bending_load_yields_only_part_of_the_section() {
    let result = solve_frame_2d_material_p_delta(&request(0.6, REFERENCE_MOMENT, [0.0; 4]))
        .expect("combined fiber-section path should converge");

    assert!(
        result.stability_result.converged,
        "steps={:#?}",
        result.stability_result.steps
    );
    assert!(result.material_states.iter().any(|state| {
        state.yielded_fiber_point_count > 0
            && state.yielded_fiber_point_count < state.fiber_point_count
            && state.max_fiber_equivalent_plastic_strain > 0.0
    }));
    assert!(
        result
            .material_states
            .iter()
            .any(|state| state.section_end_moment_i.unwrap().abs() > 1.0)
    );
}

#[test]
fn distributed_residual_stress_is_visible_without_section_resultants() {
    let mut request = request(
        0.1,
        0.0,
        [
            -0.2 * YIELD_STRENGTH,
            0.2 * YIELD_STRENGTH,
            0.2 * YIELD_STRENGTH,
            -0.2 * YIELD_STRENGTH,
        ],
    );
    request.stability.maximum_load_factor = None;
    request.stability.load_steps = None;
    request.load_factor_schedule = Some(vec![0.0, 0.1]);
    let result = solve_frame_2d_material_p_delta(&request)
        .expect("self-equilibrated fiber residual stress should converge");

    let initial = &result.material_history[0];
    assert_eq!(initial.load_factor, 0.0);
    for state in &initial.material_states {
        assert_absolute(state.initial_axial_stress, 0.0, 1.0e-8);
        assert_absolute(state.section_axial_force.unwrap(), 0.0, 1.0e-8);
        assert_absolute(state.section_end_moment_i.unwrap(), 0.0, 1.0e-8);
        assert_absolute(state.section_end_moment_j.unwrap(), 0.0, 1.0e-8);
        assert_eq!(state.fiber_point_count, 8);
        assert_eq!(state.yielded_fiber_point_count, 0);
    }
}

#[test]
fn fiber_section_contract_rejects_inconsistent_geometry_and_stress_sources() {
    let mut malformed = request(0.1, 0.0, [0.0; 4]);
    malformed.materials[0].section_fibers[0].area *= 0.5;
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("fiber area mismatch must be rejected");
    assert!(error.contains("fiber area must match"));

    let mut malformed = request(0.1, 0.0, [0.0; 4]);
    malformed.materials[0].section_fibers[0].y += 0.01;
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("off-center fibers must be rejected");
    assert!(error.contains("centered at y=0"));

    let mut malformed = request(0.1, 0.0, [0.0; 4]);
    malformed.materials[0].initial_axial_stress = 1.0;
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("uniform and distributed residual stress sources must not be mixed");
    assert!(error.contains("cannot combine uniform"));

    let unbalanced_stress = 0.2 * YIELD_STRENGTH;
    let unbalanced = request(0.1, 0.0, [unbalanced_stress; 4]);
    let error = solve_frame_2d_material_p_delta(&unbalanced)
        .expect_err("unbalanced fiber residual stress must be rejected");
    assert!(error.contains("not self-equilibrated"));
}

fn request(
    maximum_load_factor: f64,
    reference_moment: f64,
    initial_stresses: [f64; 4],
) -> SolveFrame2dMaterialPDeltaRequest {
    let nodes = vec![
        node("base", 0.0, true, 0.0, 0.0),
        node("mid", LENGTH / 2.0, false, 0.0, 0.0),
        node("top", LENGTH, false, -REFERENCE_FORCE, reference_moment),
    ];
    let elements = (0..2)
        .map(|index| Frame2dElementInput {
            id: format!("fiber-e{index}"),
            node_i: index,
            node_j: index + 1,
            area: AREA,
            youngs_modulus: YOUNGS_MODULUS,
            moment_of_inertia: INERTIA,
            section_modulus: INERTIA / 0.3,
        })
        .collect::<Vec<_>>();
    let materials = elements
        .iter()
        .map(|element| Frame2dMonotonicBilinearMaterialInput {
            element_id: element.id.clone(),
            yield_strength: YIELD_STRENGTH,
            hardening_ratio: HARDENING_RATIO,
            initial_axial_stress: 0.0,
            section_fibers: [-0.3, -0.1, 0.1, 0.3]
                .into_iter()
                .zip(initial_stresses)
                .map(|(y, initial_axial_stress)| Frame2dSectionFiberInput {
                    y,
                    area: AREA / 4.0,
                    initial_axial_stress,
                })
                .collect(),
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

fn node(id: &str, y: f64, fixed: bool, load_y: f64, moment_z: f64) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.into(),
        x: 0.0,
        y,
        fix_x: fixed,
        fix_y: fixed,
        fix_rz: fixed,
        load_x: 0.0,
        load_y,
        moment_z,
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
