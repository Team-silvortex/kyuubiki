use kyuubiki_headless_sdk::{
    CompositeCurrentConductionFeedbackSpec, CompositeCurrentConductionRegionSpec,
    CompositeDielectricLossSpec, CompositeJouleHeatingRegionSpec, CompositeJouleHeatingSpec,
    distribute_composite_dielectric_heat_load, project_composite_dielectric_loss_to_heat,
    project_composite_joule_heating_to_heat, project_composite_solved_current_to_heat,
    temperature_adjusted_composite_current_request,
};
use kyuubiki_protocol::{
    SolveElectricConductionPlaneQuad2dRequest, SolveElectricConductionPlaneQuad2dResult,
    SolveElectrostaticPlaneQuad2dRequest, SolveHeatPlaneQuad2dRequest,
};
use kyuubiki_solver::{
    solve_electric_conduction_plane_quad_2d, solve_electrostatic_plane_quad_2d,
    solve_heat_plane_quad_2d,
};
use serde_json::json;

fn heat_seed() -> SolveHeatPlaneQuad2dRequest {
    serde_json::from_value(json!({
        "nodes": [
            {"id": "a", "x": 0.0, "y": 0.0, "fix_temperature": true, "temperature": 20.0},
            {"id": "b", "x": 1.0, "y": 0.0, "fix_temperature": false},
            {"id": "c", "x": 1.0, "y": 1.0, "fix_temperature": false},
            {"id": "d", "x": 0.0, "y": 1.0, "fix_temperature": true, "temperature": 20.0}
        ],
        "elements": [{"id": "dielectric_core", "node_i": 0, "node_j": 1, "node_k": 2,
            "node_l": 3, "thickness": 1.0, "conductivity": 1.0}]
    }))
    .unwrap()
}

fn dielectric_spec() -> CompositeDielectricLossSpec {
    CompositeDielectricLossSpec {
        source_element_id: "dielectric_core".into(),
        frequency_hz: 1.0e6,
        relative_permittivity: 3.4,
        loss_tangent: 0.01,
        reference_temperature_c: 20.0,
    }
}

fn electrostatic_request(
    heat: &SolveHeatPlaneQuad2dRequest,
    potentials: [f64; 4],
) -> SolveElectrostaticPlaneQuad2dRequest {
    serde_json::from_value(json!({
        "nodes": heat.nodes.iter().zip(potentials).map(|(node, potential)| json!({
            "id": node.id, "x": node.x, "y": node.y, "fix_potential": true,
            "potential": potential, "charge_density": 0.0
        })).collect::<Vec<_>>(),
        "elements": [{"id": "dielectric_core", "node_i": 0, "node_j": 1, "node_k": 2,
            "node_l": 3, "thickness": 1.0, "permittivity": 4.0}]
    }))
    .unwrap()
}

fn expected_loss(mean_square: f64, volume: f64) -> f64 {
    let spec = dielectric_spec();
    2.0 * std::f64::consts::PI
        * spec.frequency_hz
        * 8.854_187_812_8e-12
        * spec.relative_permittivity
        * spec.loss_tangent
        * mean_square
        * volume
}

fn close(actual: f64, expected: f64, label: &str) {
    assert!(actual.is_finite());
    assert!(
        (actual - expected).abs() <= 1.0e-14 * expected.abs().max(1.0e-30),
        "{label}: actual={actual}, expected={expected}"
    );
}

#[test]
fn opposing_subcell_fields_produce_nonzero_dielectric_heat_and_temperature() {
    let heat = heat_seed();
    let solved =
        solve_electrostatic_plane_quad_2d(&electrostatic_request(&heat, [0.0, 1.0, 0.0, 1.0]))
            .unwrap();
    assert_eq!(solved.elements[0].electric_field_magnitude, 0.0);
    // Both triangles have |E|^2 = 2, although their mean vector is zero.
    let (projected, evidence) =
        project_composite_dielectric_loss_to_heat(&solved, &heat, &dielectric_spec()).unwrap();
    close(
        evidence.total_loss_w,
        expected_loss(2.0, 1.0),
        "dielectric power",
    );
    close(
        evidence.electric_field_rms_v_m,
        2.0_f64.sqrt(),
        "spatial RMS",
    );
    let thermal = solve_heat_plane_quad_2d(&projected).unwrap();
    for node in [1, 2] {
        assert!(
            (thermal.nodes[node].temperature - (20.0 + evidence.total_loss_w / 2.0)).abs()
                < 1.0e-12
        );
    }
}

#[test]
fn unequal_subcell_areas_preserve_integrated_loss_under_reference_shift() {
    let mut heat = heat_seed();
    for (node, [x, y]) in
        heat.nodes
            .iter_mut()
            .zip([[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 2.0]])
    {
        node.x = x;
        node.y = y;
    }
    // Areas 1 and 2, squared fields 1.25 and 0.3125: area mean = 0.625.
    for offset in [0.0, -2.0_f64.powi(40), 2.0_f64.powi(40)] {
        let solved = solve_electrostatic_plane_quad_2d(&electrostatic_request(
            &heat,
            [offset, offset + 1.0, offset, offset + 1.0],
        ))
        .unwrap();
        let (_, evidence) =
            project_composite_dielectric_loss_to_heat(&solved, &heat, &dielectric_spec()).unwrap();
        close(
            evidence.total_loss_w,
            expected_loss(0.625, 3.0),
            "unequal-area power",
        );
    }
}

fn current_request() -> SolveElectricConductionPlaneQuad2dRequest {
    let heat = heat_seed();
    serde_json::from_value(json!({
        "nodes": heat.nodes.iter().map(|node| json!({
            "id": node.id, "x": node.x, "y": node.y, "fix_electric_potential": true,
            "electric_potential_v": node.x, "current_source_a": 0.0
        })).collect::<Vec<_>>(),
        "elements": [{"id": "dielectric_core", "node_i": 0, "node_j": 1, "node_k": 2,
            "node_l": 3, "thickness": 1.0, "electrical_conductivity_s_m": 1.0}]
    }))
    .unwrap()
}

fn current_spec() -> CompositeCurrentConductionFeedbackSpec {
    CompositeCurrentConductionFeedbackSpec {
        regions: vec![CompositeCurrentConductionRegionSpec {
            element_id: "dielectric_core".into(),
            reference_resistivity_ohm_m: 1.0,
            reference_temperature_c: 20.0,
            resistivity_temperature_coefficient_1_k: 0.0,
        }],
        parameter_source: "analytic-unit-square".into(),
    }
}

fn prescribed_spec() -> CompositeJouleHeatingSpec {
    CompositeJouleHeatingSpec {
        regions: vec![CompositeJouleHeatingRegionSpec {
            element_id: "dielectric_core".into(),
            current_a: 1.0,
            path_length_m: 1.0,
            cross_section_area_m2: 1.0,
            reference_resistivity_ohm_m: 1.0,
            reference_temperature_c: 20.0,
            resistivity_temperature_coefficient_1_k: 0.0,
        }],
        parameter_source: "analytic-unit-square".into(),
    }
}

fn project_current(
    current: &SolveElectricConductionPlaneQuad2dResult,
    heat: &SolveHeatPlaneQuad2dRequest,
) -> Result<
    (
        SolveHeatPlaneQuad2dRequest,
        kyuubiki_headless_sdk::CompositeCurrentToHeatProjection,
    ),
    String,
> {
    project_composite_solved_current_to_heat(
        current,
        heat,
        &current_spec(),
        &[("dielectric_core".into(), 20.0)],
    )
}

#[test]
fn solved_current_rejects_nonfinite_seed_loads_and_recovers_on_clean_input() {
    let solved = solve_electric_conduction_plane_quad_2d(&current_request()).unwrap();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut heat = heat_seed();
        heat.nodes[0].heat_load = bad;
        assert!(
            project_current(&solved, &heat).is_err(),
            "accepted seed load {bad}"
        );
    }
    close(
        project_current(&solved, &heat_seed())
            .unwrap()
            .1
            .total_joule_loss_w,
        1.0,
        "clean replay",
    );
}

#[test]
fn solved_current_rejects_negative_or_nonfinite_element_power() {
    let mut solved = solve_electric_conduction_plane_quad_2d(&current_request()).unwrap();
    for bad in [-1.0, f64::NAN, f64::INFINITY] {
        solved.elements[0].joule_power_w = bad;
        assert!(
            project_current(&solved, &heat_seed()).is_err(),
            "accepted source power {bad}"
        );
    }
}

#[test]
fn unmapped_terminal_losses_are_not_silently_omitted() {
    let mut request = current_request();
    request.terminals = serde_json::from_value(json!([
        {"id": "supply", "node": 1, "external_potential_v": 2.0, "impedance_ohm": 1.0}
    ]))
    .unwrap();
    let solved = solve_electric_conduction_plane_quad_2d(&request).unwrap();
    close(
        solved.total_terminal_impedance_power_w,
        1.0,
        "terminal loss",
    );
    let error = project_current(&solved, &heat_seed()).unwrap_err();
    assert!(error.contains("terminal heat mappings"), "{error}");
}

fn heat_with_unrelated_large_load() -> SolveHeatPlaneQuad2dRequest {
    let mut heat = heat_seed();
    let mut unrelated = heat.nodes[0].clone();
    unrelated.id = "unrelated-fixed-sink".into();
    unrelated.x = 2.0;
    unrelated.heat_load = 2.0_f64.powi(60);
    heat.nodes.push(unrelated);
    heat
}

#[test]
fn unrelated_large_seed_load_does_not_hide_representable_current_heat_increments() {
    let solved = solve_electric_conduction_plane_quad_2d(&current_request()).unwrap();
    let heat = heat_with_unrelated_large_load();
    let (projected, evidence) = project_current(&solved, &heat).unwrap();
    for node in &projected.nodes[..4] {
        close(node.heat_load, 0.25, "nodal power");
    }
    assert_eq!(projected.nodes[4], heat.nodes[4]);
    close(
        evidence.distributed_total_heat_load_w,
        1.0,
        "actual added power",
    );
}

#[test]
fn unrelated_large_seed_load_does_not_hide_prescribed_joule_increments() {
    let heat = heat_with_unrelated_large_load();
    let (projected, evidence) = project_composite_joule_heating_to_heat(
        &heat,
        &prescribed_spec(),
        &[("dielectric_core".into(), 20.0)],
    )
    .unwrap();
    assert_eq!(projected.nodes[4], heat.nodes[4]);
    close(
        evidence.distributed_total_heat_load_w,
        1.0,
        "actual prescribed power",
    );
}

#[test]
fn swallowed_nodal_increments_are_rejected_instead_of_reporting_intended_power() {
    let mut heat = heat_seed();
    for node in &mut heat.nodes {
        node.heat_load = 2.0_f64.powi(60);
    }
    let solved = solve_electric_conduction_plane_quad_2d(&current_request()).unwrap();
    assert!(project_current(&solved, &heat).is_err());
    assert!(
        project_composite_joule_heating_to_heat(
            &heat,
            &prescribed_spec(),
            &[("dielectric_core".into(), 20.0)],
        )
        .is_err()
    );
}

#[test]
fn temperature_feedback_rejects_nonrepresentable_conductivity() {
    let mut spec = current_spec();
    spec.regions[0].reference_resistivity_ohm_m = 1.0e-320;
    let adjusted = temperature_adjusted_composite_current_request(
        &current_request(),
        &spec,
        &[("dielectric_core".into(), 20.0)],
    );
    assert!(adjusted.is_err());
}

#[test]
fn zero_dielectric_loss_keeps_the_explicit_seed_replacement_contract() {
    let mut heat = heat_seed();
    for node in &mut heat.nodes {
        node.heat_load = 7.0;
    }
    let solved =
        solve_electrostatic_plane_quad_2d(&electrostatic_request(&heat, [0.0, 1.0, 1.0, 0.0]))
            .unwrap();
    let mut spec = dielectric_spec();
    spec.loss_tangent = 0.0;
    let (projected, evidence) =
        project_composite_dielectric_loss_to_heat(&solved, &heat, &spec).unwrap();
    assert!(projected.nodes.iter().all(|node| node.heat_load == 0.0));
    assert!(heat.nodes.iter().all(|node| node.heat_load == 7.0));
    assert_eq!(evidence.total_loss_w, 0.0);
    assert_eq!(evidence.energy_balance_relative_error, 0.0);
}

#[test]
fn dielectric_projection_rejects_invalid_energy_and_loss_overflow() {
    let heat = heat_seed();
    let baseline =
        solve_electrostatic_plane_quad_2d(&electrostatic_request(&heat, [0.0, 1.0, 1.0, 0.0]))
            .unwrap();
    for invalid_energy in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
        let mut corrupt = baseline.clone();
        corrupt.elements[0].electric_energy_density = invalid_energy;
        assert!(
            project_composite_dielectric_loss_to_heat(&corrupt, &heat, &dielectric_spec()).is_err()
        );
    }
    for invalid_permittivity in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut corrupt = baseline.clone();
        corrupt.input.elements[0].permittivity = invalid_permittivity;
        assert!(
            project_composite_dielectric_loss_to_heat(&corrupt, &heat, &dielectric_spec()).is_err()
        );
    }
    let mut spec = dielectric_spec();
    spec.frequency_hz = f64::MAX;
    assert!(project_composite_dielectric_loss_to_heat(&baseline, &heat, &spec).is_err());
}

#[test]
fn dielectric_volume_weighting_is_coordinate_translation_invariant() {
    let mut heat = heat_seed();
    for node in &mut heat.nodes {
        node.x += 2.0_f64.powi(40);
        node.y -= 2.0_f64.powi(40);
    }
    let solved =
        solve_electrostatic_plane_quad_2d(&electrostatic_request(&heat, [0.0, 1.0, 0.0, 1.0]))
            .unwrap();
    let (_, evidence) =
        project_composite_dielectric_loss_to_heat(&solved, &heat, &dielectric_spec()).unwrap();
    close(
        evidence.total_loss_w,
        expected_loss(2.0, 1.0),
        "translated geometry",
    );
}

#[test]
fn tiny_positive_power_is_never_mislabeled_as_a_conserved_zero() {
    let mut heat = heat_seed();
    for node in &mut heat.nodes {
        node.heat_load = 1.0;
    }
    let mut spec = prescribed_spec();
    spec.regions[0].reference_resistivity_ohm_m = 1.0e-20;
    assert!(
        project_composite_joule_heating_to_heat(&heat, &spec, &[("dielectric_core".into(), 20.0)],)
            .is_err()
    );
    let smallest_positive = f64::from_bits(1);
    assert!(distribute_composite_dielectric_heat_load(&heat_seed(), smallest_positive).is_err());
}

#[test]
fn finite_cooling_loads_are_preserved_by_additive_joule_projection() {
    let mut heat = heat_seed();
    for node in &mut heat.nodes {
        node.heat_load = -1.0;
    }
    let solved = solve_electric_conduction_plane_quad_2d(&current_request()).unwrap();
    let (projected, evidence) = project_current(&solved, &heat).unwrap();
    assert!(projected.nodes.iter().all(|node| node.heat_load == -0.75));
    assert!(heat.nodes.iter().all(|node| node.heat_load == -1.0));
    close(
        evidence.distributed_total_heat_load_w,
        1.0,
        "power added to cooling loads",
    );
}

#[test]
fn malformed_summary_fields_and_repeated_target_nodes_are_rejected() {
    let baseline = solve_electric_conduction_plane_quad_2d(&current_request()).unwrap();
    let mut corrupt = baseline.clone();
    corrupt.current_balance_relative_error = f64::NAN;
    assert!(project_current(&corrupt, &heat_seed()).is_err());
    corrupt = baseline;
    corrupt.elements[0].peak_current_density_magnitude_a_m2 = f64::INFINITY;
    assert!(project_current(&corrupt, &heat_seed()).is_err());
    let mut heat = heat_seed();
    heat.elements[0].node_l = heat.elements[0].node_k;
    assert!(
        project_composite_joule_heating_to_heat(
            &heat,
            &prescribed_spec(),
            &[("dielectric_core".into(), 20.0)],
        )
        .is_err()
    );
}

#[test]
fn shared_nodes_accumulate_both_regions_without_double_counting_existing_loads() {
    let coordinates = [
        [0.0, 0.0],
        [1.0, 0.0],
        [2.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
        [2.0, 1.0],
    ];
    let mut current = current_request();
    let current_node = current.nodes[0].clone();
    let current_element = current.elements[0].clone();
    current.nodes = coordinates
        .iter()
        .enumerate()
        .map(|(index, &[x, y])| {
            let mut node = current_node.clone();
            node.id = format!("n{index}");
            node.x = x;
            node.y = y;
            node.electric_potential_v = x;
            node
        })
        .collect();
    let mut heat = heat_seed();
    let heat_node = heat.nodes[0].clone();
    let heat_element = heat.elements[0].clone();
    heat.nodes = coordinates
        .iter()
        .enumerate()
        .map(|(index, &[x, y])| {
            let mut node = heat_node.clone();
            node.id = format!("n{index}");
            node.x = x;
            node.y = y;
            node.heat_load = 0.125;
            node
        })
        .collect();
    current.elements.clear();
    heat.elements.clear();
    let mut spec = current_spec();
    let region = spec.regions[0].clone();
    spec.regions.clear();
    let mut temperatures = Vec::new();
    for (index, [i, j, k, l]) in [[0, 1, 4, 3], [1, 2, 5, 4]].into_iter().enumerate() {
        let id = format!("region-{index}");
        let mut electrical = current_element.clone();
        electrical.id = id.clone();
        electrical.node_i = i;
        electrical.node_j = j;
        electrical.node_k = k;
        electrical.node_l = l;
        current.elements.push(electrical);
        let mut thermal = heat_element.clone();
        thermal.id = id.clone();
        thermal.node_i = i;
        thermal.node_j = j;
        thermal.node_k = k;
        thermal.node_l = l;
        heat.elements.push(thermal);
        let mut item = region.clone();
        item.element_id = id.clone();
        spec.regions.push(item);
        temperatures.push((id, 20.0));
    }
    let solved = solve_electric_conduction_plane_quad_2d(&current).unwrap();
    let (projected, evidence) =
        project_composite_solved_current_to_heat(&solved, &heat, &spec, &temperatures).unwrap();
    for (node, added) in projected
        .nodes
        .iter()
        .zip([0.25, 0.5, 0.25, 0.25, 0.5, 0.25])
    {
        close(node.heat_load, 0.125 + added, "shared-node load");
    }
    close(evidence.total_joule_loss_w, 2.0, "two-region source power");
    close(
        evidence.distributed_total_heat_load_w,
        2.0,
        "two-region added power",
    );
    for region in evidence.regions {
        close(
            region.distributed_heat_load_w,
            1.0,
            "per-region actual power",
        );
        assert!(region.energy_balance_relative_error <= 1.0e-12);
    }
}
