use kyuubiki_headless_sdk::{
    CompositeThermalExpansionFeedbackSpec, CompositeThermalExpansionRegionSpec,
    project_composite_heat_to_thermal, project_composite_temperature_dependent_expansion,
};
use kyuubiki_protocol::{
    HeatPlaneNodeInput, HeatPlaneNodeResult, SolveHeatPlaneQuad2dRequest,
    SolveHeatPlaneQuad2dResult, SolveThermalPlaneQuad2dRequest, ThermalPlaneNodeInput,
};
use kyuubiki_solver::{solve_heat_plane_quad_2d, solve_thermal_plane_quad_2d};
use serde_json::json;

fn models(temperature: f64) -> (SolveHeatPlaneQuad2dResult, SolveThermalPlaneQuad2dRequest) {
    let coordinates = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let nodes = coordinates
        .iter()
        .enumerate()
        .map(|(index, [x, y])| {
            json!({
                "id": format!("n{index}"), "x": x, "y": y,
                "fix_temperature": true, "temperature": temperature, "heat_load": 0.0,
                "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0,
                "temperature_delta": 0.0
            })
        })
        .collect::<Vec<_>>();
    let element = json!({"id": "core", "node_i": 0, "node_j": 1, "node_k": 2,
        "node_l": 3, "thickness": 0.02, "conductivity": 1.0,
        "youngs_modulus": 70.0e9, "poisson_ratio": 0.3, "thermal_expansion": 1.0e-5});
    let heat: SolveHeatPlaneQuad2dRequest = serde_json::from_value(json!({
        "nodes": nodes, "elements": [element]
    }))
    .unwrap();
    let thermal = serde_json::from_value(json!({"nodes": nodes, "elements": [element]})).unwrap();
    (solve_heat_plane_quad_2d(&heat).unwrap(), thermal)
}

fn spec(reference: f64) -> CompositeThermalExpansionFeedbackSpec {
    CompositeThermalExpansionFeedbackSpec {
        regions: vec![CompositeThermalExpansionRegionSpec {
            element_id: "core".into(),
            reference_temperature_c: reference,
            temperature_coefficient_1_k: 0.001,
        }],
        parameter_source: "analytic-thermal-patch".into(),
    }
}

fn close(actual: f64, expected: f64) {
    assert!(actual.is_finite());
    assert!(
        (actual - expected).abs() <= 1.0e-10 * expected.abs().max(1.0e-30),
        "actual={actual:e}, expected={expected:e}"
    );
}

#[test]
fn real_heat_to_restrained_structure_matches_analytic_stress_and_reference_shift() {
    for offset in [0.0, 2.0_f64.powi(40), -2.0_f64.powi(40)] {
        let (heat, thermal) = models(offset + 45.0);
        let (projected, transfer) =
            project_composite_heat_to_thermal(&heat, &thermal, offset + 35.0).unwrap();
        assert_eq!(transfer.minimum_temperature_delta_c, 10.0);
        assert_eq!(transfer.maximum_temperature_delta_c, 10.0);
        let (adjusted, expansion) = project_composite_temperature_dependent_expansion(
            &heat,
            &projected,
            &spec(offset + 35.0),
        )
        .unwrap();
        close(expansion.max_relative_change, 0.01);
        let solved = solve_thermal_plane_quad_2d(&adjusted).unwrap();
        let stress = -70.0e9 * 1.01e-5 * 10.0 / (1.0 - 0.3);
        close(solved.elements[0].stress_x, stress);
        close(solved.elements[0].stress_y, stress);
        assert_eq!(solved.max_displacement, 0.0);
    }
}

#[test]
fn reordered_nodes_and_rotated_connectivity_preserve_free_expansion() {
    let (mut heat, mut thermal) = models(45.0);
    heat.nodes.reverse();
    for node in &mut thermal.nodes {
        node.fix_x = node.id == "n0";
        node.fix_y = node.id == "n0" || node.id == "n1";
    }
    thermal.nodes = [2, 0, 3, 1]
        .map(|index| thermal.nodes[index].clone())
        .to_vec();
    // Same physical boundary n1,n2,n3,n0, with different array positions.
    let element = &mut thermal.elements[0];
    element.node_i = 3;
    element.node_j = 0;
    element.node_k = 2;
    element.node_l = 1;
    let (projected, _) = project_composite_heat_to_thermal(&heat, &thermal, 35.0).unwrap();
    let (adjusted, _) =
        project_composite_temperature_dependent_expansion(&heat, &projected, &spec(35.0)).unwrap();
    let solved = solve_thermal_plane_quad_2d(&adjusted).unwrap();
    for node in &solved.nodes {
        assert!((node.ux - 1.01e-4 * node.x).abs() < 1.0e-14);
        assert!((node.uy - 1.01e-4 * node.y).abs() < 1.0e-14);
    }
    assert!(
        solved.max_stress < 1.0e-7,
        "spurious stress={}",
        solved.max_stress
    );
}

#[test]
fn nonuniform_temperatures_follow_node_ids_through_both_projections() {
    let (mut heat, mut thermal) = models(45.0);
    for (node, temperature) in heat.input.nodes.iter_mut().zip([35.0, 40.0, 45.0, 50.0]) {
        node.temperature = temperature;
    }
    heat = solve_heat_plane_quad_2d(&heat.input).unwrap();
    heat.nodes.reverse();
    thermal.nodes = [2, 0, 3, 1]
        .map(|index| thermal.nodes[index].clone())
        .to_vec();
    let element = &mut thermal.elements[0];
    element.node_i = 1;
    element.node_j = 3;
    element.node_k = 0;
    element.node_l = 2;
    let (projected, evidence) = project_composite_heat_to_thermal(&heat, &thermal, 35.0).unwrap();
    assert_eq!(evidence.minimum_temperature_delta_c, 0.0);
    assert_eq!(evidence.maximum_temperature_delta_c, 15.0);
    for (node, expected) in projected.nodes.iter().zip([10.0, 0.0, 15.0, 5.0]) {
        assert_eq!(node.temperature_delta, expected);
    }
    let (adjusted, expansion) =
        project_composite_temperature_dependent_expansion(&heat, &projected, &spec(35.0)).unwrap();
    assert_eq!(expansion.updates[0].mean_temperature_c, 42.5);
    close(adjusted.elements[0].thermal_expansion, 1.0075e-5);
    let result = solve_thermal_plane_quad_2d(&adjusted).unwrap();
    close(result.elements[0].average_temperature_delta, 7.5);
    close(result.elements[0].stress_x, -70.0e9 * 1.0075e-5 * 7.5 / 0.7);
}

#[test]
fn crossed_repeated_and_unknown_connectivity_cannot_match_a_quad_cycle() {
    for mutation in 0..3 {
        let (mut heat, mut thermal) = models(45.0);
        match mutation {
            0 => {
                thermal.elements[0].node_j = 2;
                thermal.elements[0].node_k = 1;
            }
            1 => {
                thermal.elements[0].node_j = 0;
                heat.input.elements[0].node_j = 0;
            }
            _ => thermal.elements[0].node_k = usize::MAX,
        }
        assert!(
            project_composite_temperature_dependent_expansion(&heat, &thermal, &spec(35.0))
                .is_err(),
            "mutation={mutation}"
        );
    }
}

#[test]
fn nonfinite_coordinates_and_incomplete_fields_are_rejected_by_both_apis() {
    for mutation in 0..5 {
        let (mut heat, mut thermal) = models(45.0);
        match mutation {
            0 => heat.nodes[0].x = f64::NAN,
            1 => heat.input.nodes[0].y = f64::INFINITY,
            2 => thermal.nodes[0].x = f64::NEG_INFINITY,
            3 => {
                heat.input.nodes.pop();
            }
            _ => {
                thermal.nodes.pop();
            }
        }
        assert!(project_composite_heat_to_thermal(&heat, &thermal, 35.0).is_err());
        assert!(
            project_composite_temperature_dependent_expansion(&heat, &thermal, &spec(35.0))
                .is_err()
        );
    }
}

#[test]
fn shared_nodes_and_reordered_regions_preserve_local_material_coefficients() {
    let (mut heat, mut thermal) = models(45.0);
    for (id, index) in [("n4", 1), ("n5", 2)] {
        let mut source = heat.input.nodes[index].clone();
        source.x = 2.0;
        source.id = id.into();
        heat.input.nodes.push(source);
        let mut target = thermal.nodes[index].clone();
        target.x = 2.0;
        target.id = id.into();
        thermal.nodes.push(target);
    }
    let mut shell = heat.input.elements[0].clone();
    shell.id = "shell".into();
    shell.node_i = 1;
    shell.node_j = 4;
    shell.node_k = 5;
    shell.node_l = 2;
    heat.input.elements.push(shell);
    heat = solve_heat_plane_quad_2d(&heat.input).unwrap();
    let mut shell = thermal.elements[0].clone();
    shell.id = "shell".into();
    shell.node_i = 1;
    shell.node_j = 4;
    shell.node_k = 5;
    shell.node_l = 2;
    shell.thermal_expansion = 2.0e-5;
    thermal.elements.insert(0, shell);
    let mut feedback = spec(35.0);
    feedback.regions.push(CompositeThermalExpansionRegionSpec {
        element_id: "shell".into(),
        reference_temperature_c: 35.0,
        temperature_coefficient_1_k: -0.002,
    });
    let (projected, _) = project_composite_heat_to_thermal(&heat, &thermal, 35.0).unwrap();
    let (adjusted, evidence) =
        project_composite_temperature_dependent_expansion(&heat, &projected, &feedback).unwrap();
    assert_eq!(evidence.updated_region_count, 2);
    close(adjusted.elements[0].thermal_expansion, 1.96e-5);
    close(adjusted.elements[1].thermal_expansion, 1.01e-5);
    let result = solve_thermal_plane_quad_2d(&adjusted).unwrap();
    close(result.elements[0].stress_x, -70.0e9 * 1.96e-5 * 10.0 / 0.7);
    close(result.elements[1].stress_x, -70.0e9 * 1.01e-5 * 10.0 / 0.7);
}

#[test]
fn empty_nodal_fields_cannot_emit_infinite_projection_evidence() {
    let (mut heat, mut thermal) = models(45.0);
    heat.nodes.clear();
    heat.input.nodes.clear();
    thermal.nodes.clear();
    assert!(project_composite_heat_to_thermal(&heat, &thermal, 35.0).is_err());
}

#[test]
fn finite_temperatures_with_unrepresentable_difference_are_rejected() {
    let (mut heat, thermal) = models(45.0);
    heat.nodes[0].temperature = f64::MAX;
    assert!(project_composite_heat_to_thermal(&heat, &thermal, -f64::MAX).is_err());
}

#[test]
fn repeated_target_identity_cannot_reuse_one_source_and_omit_another() {
    let (heat, mut thermal) = models(45.0);
    thermal.nodes[1] = thermal.nodes[0].clone();
    assert!(project_composite_heat_to_thermal(&heat, &thermal, 35.0).is_err());
}

#[test]
fn result_indices_must_match_the_original_heat_mesh() {
    for mutation in 0..4 {
        let (mut heat, thermal) = models(45.0);
        match mutation {
            0 => heat.nodes[1].index = 0,
            1 => heat.nodes[1].index = usize::MAX,
            2 => heat.input.nodes[1].id = "different-node".into(),
            _ => heat.input.nodes[1].x += 0.1,
        }
        assert!(
            project_composite_heat_to_thermal(&heat, &thermal, 35.0).is_err(),
            "mutation={mutation}"
        );
    }
}

#[test]
fn expansion_requires_matching_id_and_geometry_not_just_equal_indices() {
    for geometry in [false, true] {
        let (heat, mut thermal) = models(45.0);
        if geometry {
            thermal.nodes[0].x += 0.1;
        } else {
            thermal.nodes[0].id = "unrelated".into();
        }
        assert!(
            project_composite_temperature_dependent_expansion(&heat, &thermal, &spec(35.0))
                .is_err()
        );
    }
}

#[test]
fn ambiguous_element_identity_is_rejected_on_either_side() {
    let (mut heat, thermal) = models(45.0);
    heat.input.elements.push(heat.input.elements[0].clone());
    assert!(
        project_composite_temperature_dependent_expansion(&heat, &thermal, &spec(35.0)).is_err()
    );
    let (heat, mut thermal) = models(45.0);
    thermal.elements.push(thermal.elements[0].clone());
    assert!(
        project_composite_temperature_dependent_expansion(&heat, &thermal, &spec(35.0)).is_err()
    );
}

#[test]
fn blank_node_identities_are_not_accepted_as_a_mapping_contract() {
    let (mut heat, mut thermal) = models(45.0);
    heat.nodes[0].id = " ".into();
    heat.input.nodes[0].id = " ".into();
    thermal.nodes[0].id = " ".into();
    assert!(project_composite_heat_to_thermal(&heat, &thermal, 35.0).is_err());
}

#[test]
fn expansion_relative_change_stays_relative_for_small_coefficients() {
    let (heat, mut thermal) = models(45.0);
    let mut feedback = spec(35.0);
    feedback.regions[0].temperature_coefficient_1_k = 0.1;
    for coefficient in [f64::from_bits(1), 1.0e-20, 1.0e-5] {
        thermal.elements[0].thermal_expansion = coefficient;
        let (adjusted, evidence) =
            project_composite_temperature_dependent_expansion(&heat, &thermal, &feedback).unwrap();
        assert_eq!(adjusted.elements[0].thermal_expansion, coefficient * 2.0);
        close(evidence.max_relative_change, 1.0);
    }
}

#[test]
fn unsupported_negative_coefficients_are_rejected_before_structural_dispatch() {
    let (heat, mut thermal) = models(45.0);
    thermal.elements[0].thermal_expansion = f64::MAX;
    let mut feedback = spec(35.0);
    feedback.regions[0].temperature_coefficient_1_k = -0.2;
    let error =
        project_composite_temperature_dependent_expansion(&heat, &thermal, &feedback).unwrap_err();
    assert!(
        error.contains("negative coefficient unsupported by the thermal plane solver"),
        "{error}"
    );
    thermal.elements[0].thermal_expansion = -1.0e-5;
    let error = project_composite_temperature_dependent_expansion(&heat, &thermal, &spec(35.0))
        .unwrap_err();
    assert!(
        error.contains("non-negative reference coefficient"),
        "{error}"
    );
}

#[test]
fn nonzero_expansion_cannot_silently_underflow_to_zero() {
    let (heat, mut thermal) = models(45.0);
    thermal.elements[0].thermal_expansion = f64::from_bits(1);
    let mut feedback = spec(35.0);
    feedback.regions[0].temperature_coefficient_1_k = -0.075;
    assert!(project_composite_temperature_dependent_expansion(&heat, &thermal, &feedback).is_err());
}

#[test]
fn rejected_mapping_leaves_seed_intact_and_clean_replay_still_solves() {
    let (heat, thermal) = models(45.0);
    let before = serde_json::to_value(&thermal).unwrap();
    let mut corrupt = heat.clone();
    corrupt.nodes[2].temperature = f64::NAN;
    assert!(project_composite_heat_to_thermal(&corrupt, &thermal, 35.0).is_err());
    assert_eq!(serde_json::to_value(&thermal).unwrap(), before);
    let (projected, _) = project_composite_heat_to_thermal(&heat, &thermal, 35.0).unwrap();
    assert!(solve_thermal_plane_quad_2d(&projected).unwrap().max_stress > 0.0);
}

#[test]
fn explicit_zero_coefficients_and_zero_temperature_scales_remain_valid() {
    let (heat, mut thermal) = models(45.0);
    thermal.elements[0].thermal_expansion = 0.0;
    let (_, evidence) =
        project_composite_temperature_dependent_expansion(&heat, &thermal, &spec(35.0)).unwrap();
    assert_eq!(evidence.max_relative_change, 0.0);
    thermal.elements[0].thermal_expansion = 1.0e-5;
    let mut feedback = spec(35.0);
    feedback.regions[0].temperature_coefficient_1_k = -0.1;
    let (adjusted, evidence) =
        project_composite_temperature_dependent_expansion(&heat, &thermal, &feedback).unwrap();
    assert_eq!(adjusted.elements[0].thermal_expansion, 0.0);
    assert_eq!(evidence.max_relative_change, 1.0);
}

fn nodal_field(count: usize) -> (SolveHeatPlaneQuad2dResult, SolveThermalPlaneQuad2dRequest) {
    let (mut heat, mut thermal) = models(45.0);
    heat.input.elements.clear();
    heat.elements.clear();
    thermal.elements.clear();
    heat.input.nodes = (0..count)
        .map(|index| HeatPlaneNodeInput {
            id: format!("node-{index}"),
            x: index as f64,
            y: 0.0,
            fix_temperature: true,
            temperature: 45.0,
            heat_load: 0.0,
        })
        .collect();
    heat.nodes = heat
        .input
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| HeatPlaneNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            temperature: node.temperature,
            heat_load: 0.0,
        })
        .collect();
    thermal.nodes = heat
        .input
        .nodes
        .iter()
        .rev()
        .map(|node| ThermalPlaneNodeInput {
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            fix_x: true,
            fix_y: true,
            load_x: 0.0,
            load_y: 0.0,
            temperature_delta: 0.0,
        })
        .collect();
    (heat, thermal)
}

#[test]
#[ignore = "bounded local release microbenchmark, not a full solver benchmark"]
fn heat_to_thermal_mapping_microbenchmark() {
    for count in [1_000, 4_000, 16_000] {
        let (heat, thermal) = nodal_field(count);
        std::hint::black_box(project_composite_heat_to_thermal(&heat, &thermal, 35.0).unwrap());
        let mut samples = Vec::new();
        for _ in 0..5 {
            let started = std::time::Instant::now();
            let mapped = project_composite_heat_to_thermal(&heat, &thermal, 35.0).unwrap();
            samples.push(started.elapsed().as_secs_f64());
            assert_eq!(mapped.1.mapped_node_count, count);
            assert!(
                mapped
                    .0
                    .nodes
                    .iter()
                    .all(|node| node.temperature_delta == 10.0)
            );
            std::hint::black_box(mapped);
        }
        samples.sort_by(f64::total_cmp);
        eprintln!(
            "nodes={count} median_ms={:.6} ns_per_node={:.3}",
            samples[2] * 1e3,
            samples[2] * 1e9 / count as f64
        );
    }
}
