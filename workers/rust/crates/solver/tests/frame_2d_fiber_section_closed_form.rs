use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dMonotonicBilinearMaterialInput, Frame2dNodeInput,
    Frame2dSectionFiberInput, Frame2dSectionLayerInput, Frame2dSectionLibraryInput,
    Frame2dSectionVertexInput, Frame2dStabilityKinematics, SolveBucklingFrame2dRequest,
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
fn rectangle_section_library_executes_through_the_material_solver() {
    let mut request = request(0.4, 0.5 * REFERENCE_MOMENT, [0.0; 4]);
    let depth = (12.0 * INERTIA / AREA).sqrt();
    for material in &mut request.materials {
        material.section_fibers.clear();
        material.section_library = Some(Frame2dSectionLibraryInput::Rectangle {
            width: AREA / depth,
            depth,
            fiber_count: 8,
        });
    }

    let result = solve_frame_2d_material_p_delta(&request)
        .expect("rectangle section-library path should converge");

    assert!(result.stability_result.converged);
    assert!(result.material_states.iter().all(|state| {
        state.fiber_point_count == 16
            && state.evaluated_fiber_point_count == 16
            && state.section_axial_force.is_some()
            && state.section_end_moment_i.is_some()
            && state.section_end_moment_j.is_some()
    }));
}

#[test]
fn i_section_library_executes_through_the_material_solver() {
    let mut request = request(0.2, 0.25 * REFERENCE_MOMENT, [0.0; 4]);
    let depth = 0.6_f64;
    let flange_width = 0.24_f64;
    let flange_thickness = 0.04_f64;
    let web_thickness = 0.02_f64;
    let web_depth = depth - 2.0 * flange_thickness;
    let area = 2.0 * flange_width * flange_thickness + web_thickness * web_depth;
    let inertia =
        (flange_width * depth.powi(3) - (flange_width - web_thickness) * web_depth.powi(3)) / 12.0;
    for element in &mut request.stability.buckling.frame.elements {
        element.area = area;
        element.moment_of_inertia = inertia;
        element.section_modulus = inertia / (0.5 * depth);
    }
    for material in &mut request.materials {
        material.section_fibers.clear();
        material.section_library = Some(Frame2dSectionLibraryInput::ISection {
            depth,
            flange_width,
            flange_thickness,
            web_thickness,
            fibers_per_flange: 4,
            web_fiber_count: 8,
        });
    }

    let result = solve_frame_2d_material_p_delta(&request)
        .expect("i-section section-library path should converge");

    assert!(result.stability_result.converged);
    assert!(result.material_states.iter().all(|state| {
        state.fiber_point_count == 32
            && state.evaluated_fiber_point_count == 32
            && state.section_axial_force.is_some()
            && state.section_end_moment_i.is_some()
            && state.section_end_moment_j.is_some()
    }));
}

#[test]
fn circular_box_and_t_section_libraries_execute_through_the_material_solver() {
    let radius = 0.1_f64;
    let circle = (
        "circular",
        Frame2dSectionLibraryInput::Circular {
            radius,
            fiber_count: 12,
        },
        std::f64::consts::PI * radius.powi(2),
        std::f64::consts::PI * radius.powi(4) / 4.0,
        12,
    );
    let (box_width, box_depth, wall) = (0.2_f64, 0.3_f64, 0.02_f64);
    let (inner_width, inner_depth) = (box_width - 2.0 * wall, box_depth - 2.0 * wall);
    let hollow_box = (
        "hollow_box",
        Frame2dSectionLibraryInput::HollowBox {
            width: box_width,
            depth: box_depth,
            wall_thickness: wall,
            fibers_per_flange: 3,
            web_fiber_count: 6,
        },
        box_width * box_depth - inner_width * inner_depth,
        (box_width * box_depth.powi(3) - inner_width * inner_depth.powi(3)) / 12.0,
        12,
    );
    let (depth, flange_width, flange_thickness, web_thickness) =
        (0.3_f64, 0.2_f64, 0.03_f64, 0.02_f64);
    let web_depth = depth - flange_thickness;
    let web_area = web_thickness * web_depth;
    let flange_area = flange_width * flange_thickness;
    let t_area = web_area + flange_area;
    let web_y = -0.5 * flange_thickness;
    let flange_y = 0.5 * (depth - flange_thickness);
    let centroid = (web_area * web_y + flange_area * flange_y) / t_area;
    let t_inertia = web_thickness * web_depth.powi(3) / 12.0
        + web_area * (web_y - centroid).powi(2)
        + flange_width * flange_thickness.powi(3) / 12.0
        + flange_area * (flange_y - centroid).powi(2);
    let t_section = (
        "t_section",
        Frame2dSectionLibraryInput::TSection {
            depth,
            flange_width,
            flange_thickness,
            web_thickness,
            flange_fiber_count: 4,
            web_fiber_count: 8,
        },
        t_area,
        t_inertia,
        12,
    );
    let layers = vec![
        Frame2dSectionLayerInput {
            y_min: -0.3,
            y_max: -0.1,
            width: 0.05,
            fiber_count: 4,
        },
        Frame2dSectionLayerInput {
            y_min: -0.1,
            y_max: 0.15,
            width: 0.02,
            fiber_count: 5,
        },
        Frame2dSectionLayerInput {
            y_min: 0.15,
            y_max: 0.3,
            width: 0.1,
            fiber_count: 3,
        },
    ];
    let layered_area = layers
        .iter()
        .map(|layer| layer.width * (layer.y_max - layer.y_min))
        .sum::<f64>();
    let layered_centroid = layers
        .iter()
        .map(|layer| layer.width * (layer.y_max - layer.y_min) * 0.5 * (layer.y_min + layer.y_max))
        .sum::<f64>()
        / layered_area;
    let layered_inertia = layers
        .iter()
        .map(|layer| {
            let layer_depth = layer.y_max - layer.y_min;
            let layer_area = layer.width * layer_depth;
            let layer_y = 0.5 * (layer.y_min + layer.y_max);
            layer.width * layer_depth.powi(3) / 12.0
                + layer_area * (layer_y - layered_centroid).powi(2)
        })
        .sum::<f64>();
    let layered = (
        "layered",
        Frame2dSectionLibraryInput::Layered { layers },
        layered_area,
        layered_inertia,
        12,
    );
    let polygon_area = 0.05_f64 * 0.4 + 0.15 * 0.1;
    let polygon_centroid = 0.15_f64 * 0.1 * 0.15 / polygon_area;
    let polygon_inertia = 0.05 * 0.4_f64.powi(3) / 12.0
        + 0.15 * 0.1_f64.powi(3) / 12.0
        + 0.15 * 0.1 * 0.15_f64.powi(2)
        - polygon_area * polygon_centroid.powi(2);
    let polygon = (
        "polygon",
        Frame2dSectionLibraryInput::Polygon {
            vertices: vec![
                Frame2dSectionVertexInput { y: -0.2, z: 0.0 },
                Frame2dSectionVertexInput { y: -0.2, z: 0.05 },
                Frame2dSectionVertexInput { y: 0.1, z: 0.05 },
                Frame2dSectionVertexInput { y: 0.1, z: 0.2 },
                Frame2dSectionVertexInput { y: 0.2, z: 0.2 },
                Frame2dSectionVertexInput { y: 0.2, z: 0.0 },
            ],
            fiber_count: 16,
        },
        polygon_area,
        polygon_inertia,
        16,
    );

    for (name, section, area, inertia, fiber_count) in
        [circle, hollow_box, t_section, layered, polygon]
    {
        let mut request = request(0.1, 0.1 * REFERENCE_MOMENT, [0.0; 4]);
        for element in &mut request.stability.buckling.frame.elements {
            element.area = area;
            element.moment_of_inertia = inertia;
            element.section_modulus = inertia / 0.2;
        }
        for material in &mut request.materials {
            material.section_fibers.clear();
            material.section_library = Some(section.clone());
        }

        let result = solve_frame_2d_material_p_delta(&request)
            .unwrap_or_else(|error| panic!("{name} section-library path failed: {error}"));

        assert!(result.stability_result.converged, "{name}");
        assert!(
            result.material_states.iter().all(|state| {
                state.fiber_point_count == fiber_count * 2
                    && state.evaluated_fiber_point_count == fiber_count * 2
            }),
            "{name}: {:#?}",
            result.material_states
        );
    }
}

#[test]
fn combined_axial_bending_load_yields_only_part_of_the_section() {
    let mut request = request(0.6, REFERENCE_MOMENT, [0.0; 4]);
    for material in &mut request.materials {
        material.longitudinal_integration_points = 4;
        material.adaptive_longitudinal_integration = true;
        material.longitudinal_integration_tolerance = 1.0e-10;
    }
    let result = solve_frame_2d_material_p_delta(&request)
        .expect("combined fiber-section path should converge");

    assert!(
        result.stability_result.converged,
        "steps={:#?}",
        result.stability_result.steps
    );
    assert!(
        result.material_states.iter().any(|state| {
            state.evaluated_fiber_point_count == 116
                && [2, 3, 4, 8, 12].contains(&state.active_longitudinal_integration_points)
                && state.longitudinal_integration_error.is_some()
                && state.yielded_fiber_point_count > 0
                && state.yielded_fiber_point_count < state.fiber_point_count
                && state.max_fiber_equivalent_plastic_strain > 0.0
        }),
        "states={:#?}",
        result.material_states
    );
    assert!(
        result.material_history.iter().all(|step| {
            step.material_states.iter().all(|state| {
                state.evaluated_fiber_point_count == 116
                    && [2, 3, 4, 8, 12].contains(&state.active_longitudinal_integration_points)
                    && state.longitudinal_integration_error.is_some()
            })
        }),
        "history={:#?}",
        result.material_history
    );
    assert!(
        result
            .material_states
            .iter()
            .any(|state| state.section_end_moment_i.unwrap().abs() > 1.0)
    );
}

#[test]
fn cyclic_axial_bending_path_reverses_moment_and_accumulates_fiber_plasticity() {
    let mut request = request(0.6, REFERENCE_MOMENT, [0.0; 4]);
    for material in &mut request.materials {
        material.longitudinal_integration_points = 3;
    }
    request.stability.maximum_load_factor = None;
    request.stability.load_steps = None;
    request.load_factor_schedule = Some(vec![0.6, 0.0, -0.6]);
    let result = solve_frame_2d_material_p_delta(&request)
        .expect("cyclic axial-bending fiber-section path should converge");

    assert_eq!(result.material_history.len(), 3);
    let forward = &result.material_history[0].material_states[1];
    let unloaded = &result.material_history[1].material_states[1];
    let reversed = &result.material_history[2].material_states[1];
    let forward_moment = forward.section_end_moment_j.unwrap();
    let reversed_moment = reversed.section_end_moment_j.unwrap();

    assert!(forward.yielded_fiber_point_count > 0);
    assert_eq!(forward.fiber_point_count, 12);
    assert!(forward.yielded_fiber_point_count < forward.fiber_point_count);
    assert!(forward_moment.abs() > 1.0);
    assert!(reversed_moment.abs() > 1.0);
    assert!(forward_moment * reversed_moment < 0.0);
    assert!(unloaded.section_end_moment_j.unwrap().abs() < forward_moment.abs());
    assert!(
        reversed.max_fiber_equivalent_plastic_strain > forward.max_fiber_equivalent_plastic_strain
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

    let mut malformed = request(0.1, 0.0, [0.0; 4]);
    malformed.materials[0].section_library = Some(Frame2dSectionLibraryInput::Rectangle {
        width: 0.02,
        depth: 0.5,
        fiber_count: 8,
    });
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("library and explicit fibers must not be combined");
    assert!(error.contains("cannot combine section_library"));

    let mut malformed = request(0.1, 0.0, [0.0; 4]);
    malformed.materials[0].section_fibers.clear();
    malformed.materials[0].section_library = Some(Frame2dSectionLibraryInput::Rectangle {
        width: 0.02,
        depth: 0.5,
        fiber_count: 8,
    });
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("library inertia mismatch must be rejected");
    assert!(error.contains("fiber inertia must match"));

    let unbalanced_stress = 0.2 * YIELD_STRENGTH;
    let unbalanced = request(0.1, 0.0, [unbalanced_stress; 4]);
    let error = solve_frame_2d_material_p_delta(&unbalanced)
        .expect_err("unbalanced fiber residual stress must be rejected");
    assert!(error.contains("not self-equilibrated"));

    let mut malformed = request(0.1, 0.0, [0.0; 4]);
    malformed.materials[0].longitudinal_integration_points = 5;
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("unsupported longitudinal integration order must be rejected");
    assert!(error.contains("must be between 2 and 4"));

    let mut malformed = request(0.1, 0.0, [0.0; 4]);
    malformed.materials[0].section_fibers.clear();
    malformed.materials[0].longitudinal_integration_points = 3;
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("scalar material must not silently accept section integration controls");
    assert!(error.contains("require section_fibers"));

    let mut malformed = request(0.1, 0.0, [0.0; 4]);
    malformed.materials[0].longitudinal_integration_tolerance = 0.0;
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("adaptive integration tolerance must be bounded");
    assert!(error.contains("must be finite and in (0, 0.25]"));

    let mut malformed = request(0.1, 0.0, [0.0; 4]);
    malformed.materials[0].section_fibers.clear();
    malformed.materials[0].adaptive_longitudinal_integration = true;
    let error = solve_frame_2d_material_p_delta(&malformed)
        .expect_err("scalar material must reject adaptive section integration");
    assert!(error.contains("require section_fibers"));
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
            section_library: None,
            section_fibers: [-0.3, -0.1, 0.1, 0.3]
                .into_iter()
                .zip(initial_stresses)
                .map(|(y, initial_axial_stress)| Frame2dSectionFiberInput {
                    y,
                    area: AREA / 4.0,
                    initial_axial_stress,
                })
                .collect(),
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
