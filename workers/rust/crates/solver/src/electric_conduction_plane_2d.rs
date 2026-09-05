use crate::electric_conduction_interfaces::{
    assemble_interfaces, recover_contacts, recover_terminals, terminal_currents_by_node,
    validate_interfaces,
};
use crate::heat_plane_2d_element::{
    HeatPlaneQuadComputed, plane_triangle_scalar_gradient,
    precompute_heat_plane_quad_from_coordinates,
};
use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system_with_prescribed,
    solve_spd_system_profile_with_options,
};
use crate::linear_solver_profile::SpdSolveOptions;
use kyuubiki_protocol::{
    ElectricConductionPlaneNodeResult, ElectricConductionPlaneQuadElementInput,
    ElectricConductionPlaneQuadElementResult, SolveElectricConductionPlaneQuad2dRequest,
    SolveElectricConductionPlaneQuad2dResult,
};
use std::{borrow::Cow, collections::HashSet};

pub fn solve_electric_conduction_plane_quad_2d(
    request: &SolveElectricConductionPlaneQuad2dRequest,
) -> Result<SolveElectricConductionPlaneQuad2dResult, String> {
    solve_electric_conduction_plane_quad_2d_internal(
        Cow::Borrowed(request),
        SpdSolveOptions::default(),
    )
}

pub fn solve_electric_conduction_plane_quad_2d_owned(
    request: SolveElectricConductionPlaneQuad2dRequest,
) -> Result<SolveElectricConductionPlaneQuad2dResult, String> {
    solve_electric_conduction_plane_quad_2d_internal(
        Cow::Owned(request),
        SpdSolveOptions::default(),
    )
}

pub fn solve_electric_conduction_plane_quad_2d_with_options(
    request: &SolveElectricConductionPlaneQuad2dRequest,
    options: SpdSolveOptions,
) -> Result<SolveElectricConductionPlaneQuad2dResult, String> {
    solve_electric_conduction_plane_quad_2d_internal(Cow::Borrowed(request), options)
}

fn solve_electric_conduction_plane_quad_2d_internal(
    request: Cow<'_, SolveElectricConductionPlaneQuad2dRequest>,
    options: SpdSolveOptions,
) -> Result<SolveElectricConductionPlaneQuad2dResult, String> {
    validate_request(request.as_ref())?;
    let computed_elements = request
        .elements
        .iter()
        .map(|element| {
            precompute_heat_plane_quad_from_coordinates(
                [
                    conduction_point(request.as_ref(), element.node_i),
                    conduction_point(request.as_ref(), element.node_j),
                    conduction_point(request.as_ref(), element.node_k),
                    conduction_point(request.as_ref(), element.node_l),
                ],
                element.thickness,
                element.electrical_conductivity_s_m,
            )
        })
        .collect::<Result<Vec<_>, String>>()?;
    let (global_conductance, applied_currents) =
        assemble_system(request.as_ref(), &computed_elements);
    let potentials = solve_potentials(
        request.as_ref(),
        &global_conductance,
        &applied_currents,
        options,
    )?;
    let terminals = recover_terminals(request.as_ref(), &potentials);
    let terminal_currents = terminal_currents_by_node(request.nodes.len(), &terminals);
    let nodes = recover_node_currents(
        request.as_ref(),
        &global_conductance,
        &applied_currents,
        &potentials,
        &terminal_currents,
    );
    let elements = recover_element_fields(request.as_ref(), &computed_elements, &potentials);
    let contact_interfaces = recover_contacts(request.as_ref(), &potentials);
    let total_injected_current_a = nodes
        .iter()
        .map(|node| node.net_injected_current_a.max(0.0))
        .sum::<f64>();
    let total_extracted_current_a = nodes
        .iter()
        .map(|node| (-node.net_injected_current_a).max(0.0))
        .sum::<f64>();
    let current_balance_relative_error =
        relative_error(total_injected_current_a, total_extracted_current_a);
    let max_free_current_residual_a = nodes
        .iter()
        .filter(|node| !node.fix_electric_potential)
        .map(|node| node.reaction_current_a.abs())
        .fold(0.0_f64, f64::max);
    let free_current_residual_relative_error = max_free_current_residual_a
        / total_injected_current_a
            .max(total_extracted_current_a)
            .max(1.0e-30);
    let total_electrical_input_power_w = nodes
        .iter()
        .map(|node| node.electric_potential_v * node.net_injected_current_a)
        .sum::<f64>();
    let total_bulk_joule_power_w = elements.iter().map(|element| element.joule_power_w).sum();
    let total_contact_joule_power_w = contact_interfaces
        .iter()
        .map(|contact| contact.joule_power_w)
        .sum::<f64>();
    let total_joule_power_w = total_bulk_joule_power_w + total_contact_joule_power_w;
    let power_balance_relative_error =
        relative_error(total_electrical_input_power_w, total_joule_power_w);
    let total_terminal_impedance_power_w = terminals
        .iter()
        .map(|terminal| terminal.impedance_joule_power_w)
        .sum::<f64>();
    let total_source_power_w = nodes
        .iter()
        .map(|node| {
            let constraint_current = if node.fix_electric_potential {
                node.reaction_current_a
            } else {
                0.0
            };
            node.electric_potential_v * (node.current_source_a + constraint_current)
        })
        .sum::<f64>()
        + terminals
            .iter()
            .map(|terminal| terminal.source_power_w)
            .sum::<f64>();
    let total_dissipated_power_w = total_joule_power_w + total_terminal_impedance_power_w;
    let source_power_balance_relative_error =
        relative_error(total_source_power_w, total_dissipated_power_w);

    Ok(SolveElectricConductionPlaneQuad2dResult {
        input: request.into_owned(),
        max_electric_potential_v: nodes
            .iter()
            .map(|node| node.electric_potential_v.abs())
            .fold(0.0_f64, f64::max),
        max_electric_field_v_m: elements
            .iter()
            .map(|element| element.peak_electric_field_magnitude_v_m)
            .fold(0.0_f64, f64::max),
        max_current_density_a_m2: elements
            .iter()
            .map(|element| element.peak_current_density_magnitude_a_m2)
            .fold(0.0_f64, f64::max),
        nodes,
        elements,
        contact_interfaces,
        terminals,
        total_injected_current_a,
        total_extracted_current_a,
        current_balance_relative_error,
        max_free_current_residual_a,
        free_current_residual_relative_error,
        total_electrical_input_power_w,
        total_bulk_joule_power_w,
        total_contact_joule_power_w,
        total_joule_power_w,
        power_balance_relative_error,
        total_terminal_impedance_power_w,
        total_source_power_w,
        total_dissipated_power_w,
        source_power_balance_relative_error,
    })
}

fn conduction_point(request: &SolveElectricConductionPlaneQuad2dRequest, node: usize) -> [f64; 2] {
    [request.nodes[node].x, request.nodes[node].y]
}

fn assemble_system(
    request: &SolveElectricConductionPlaneQuad2dRequest,
    computed_elements: &[HeatPlaneQuadComputed],
) -> (SparseMatrix, Vec<f64>) {
    let mut conductance = SparseMatrix::with_uniform_row_capacity(request.nodes.len(), 12);
    for (element, computed) in request.elements.iter().zip(computed_elements) {
        let triangles = [
            (
                [element.node_i, element.node_j, element.node_k],
                &computed.first,
            ),
            (
                [element.node_i, element.node_k, element.node_l],
                &computed.second,
            ),
        ];
        for (nodes, triangle) in triangles {
            for row in 0..3 {
                for column in 0..3 {
                    add_at(
                        &mut conductance,
                        nodes[row],
                        nodes[column],
                        triangle.stiffness[row][column],
                    );
                }
            }
        }
    }
    let mut currents = request
        .nodes
        .iter()
        .map(|node| node.current_source_a)
        .collect::<Vec<_>>();
    assemble_interfaces(request, &mut conductance, &mut currents);
    (conductance, currents)
}

fn solve_potentials(
    request: &SolveElectricConductionPlaneQuad2dRequest,
    conductance: &SparseMatrix,
    currents: &[f64],
    options: SpdSolveOptions,
) -> Result<Vec<f64>, String> {
    let prescribed = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            node.fix_electric_potential
                .then_some((index, node.electric_potential_v))
        })
        .collect::<Vec<_>>();
    let (reduced, reduced_currents, free) =
        reduce_sparse_system_with_prescribed(conductance, currents, &prescribed);
    let solved = solve_spd_system_profile_with_options(&reduced, &reduced_currents, options)
        .map_err(|error| format!("electric conduction solve failed: {error}"))?
        .solution;
    let mut potentials = vec![0.0; request.nodes.len()];
    for &(index, value) in &prescribed {
        potentials[index] = value;
    }
    for (index, &dof) in free.iter().enumerate() {
        potentials[dof] = solved[index];
    }
    Ok(potentials)
}

fn recover_node_currents(
    request: &SolveElectricConductionPlaneQuad2dRequest,
    conductance: &SparseMatrix,
    applied_currents: &[f64],
    potentials: &[f64],
    terminal_currents: &[f64],
) -> Vec<ElectricConductionPlaneNodeResult> {
    request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let conductive_current_a = conductance
                .row_entries(index)
                .iter()
                .map(|(column, value)| value * potentials[*column])
                .sum::<f64>();
            let reaction_current_a = conductive_current_a - applied_currents[index];
            ElectricConductionPlaneNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                fix_electric_potential: node.fix_electric_potential,
                electric_potential_v: potentials[index],
                current_source_a: node.current_source_a,
                reaction_current_a,
                net_injected_current_a: if node.fix_electric_potential {
                    node.current_source_a + terminal_currents[index] + reaction_current_a
                } else {
                    node.current_source_a + terminal_currents[index]
                },
            }
        })
        .collect()
}

fn recover_element_fields(
    request: &SolveElectricConductionPlaneQuad2dRequest,
    computed_elements: &[HeatPlaneQuadComputed],
    potentials: &[f64],
) -> Vec<ElectricConductionPlaneQuadElementResult> {
    request
        .elements
        .iter()
        .zip(computed_elements)
        .enumerate()
        .map(|(index, (element, computed))| {
            recover_element_field(index, element, computed, potentials)
        })
        .collect()
}

fn recover_element_field(
    index: usize,
    element: &ElectricConductionPlaneQuadElementInput,
    computed: &HeatPlaneQuadComputed,
    potentials: &[f64],
) -> ElectricConductionPlaneQuadElementResult {
    let first = triangle_field(
        &computed.first,
        [
            potentials[element.node_i],
            potentials[element.node_j],
            potentials[element.node_k],
        ],
    );
    let second = triangle_field(
        &computed.second,
        [
            potentials[element.node_i],
            potentials[element.node_k],
            potentials[element.node_l],
        ],
    );
    let area_m2 = computed.first.area + computed.second.area;
    let weighted = |left: f64, right: f64| {
        (left * computed.first.area + right * computed.second.area) / area_m2
    };
    let electric_field_x_v_m = weighted(first[0], second[0]);
    let electric_field_y_v_m = weighted(first[1], second[1]);
    let rms_electric_field_magnitude_v_m = weighted(first[2].powi(2), second[2].powi(2)).sqrt();
    let peak_electric_field_magnitude_v_m = first[2].max(second[2]);
    let sigma = element.electrical_conductivity_s_m;
    let volumetric_joule_heating_w_m3 = sigma * rms_electric_field_magnitude_v_m.powi(2);

    ElectricConductionPlaneQuadElementResult {
        index,
        id: element.id.clone(),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        node_l: element.node_l,
        area_m2,
        average_electric_potential_v: (potentials[element.node_i]
            + potentials[element.node_j]
            + potentials[element.node_k]
            + potentials[element.node_l])
            / 4.0,
        electric_potential_gradient_x_v_m: -electric_field_x_v_m,
        electric_potential_gradient_y_v_m: -electric_field_y_v_m,
        electric_field_x_v_m,
        electric_field_y_v_m,
        electric_field_magnitude_v_m: electric_field_x_v_m.hypot(electric_field_y_v_m),
        rms_electric_field_magnitude_v_m,
        peak_electric_field_magnitude_v_m,
        current_density_x_a_m2: sigma * electric_field_x_v_m,
        current_density_y_a_m2: sigma * electric_field_y_v_m,
        current_density_magnitude_a_m2: sigma * electric_field_x_v_m.hypot(electric_field_y_v_m),
        rms_current_density_magnitude_a_m2: sigma * rms_electric_field_magnitude_v_m,
        peak_current_density_magnitude_a_m2: sigma * peak_electric_field_magnitude_v_m,
        volumetric_joule_heating_w_m3,
        joule_power_w: volumetric_joule_heating_w_m3 * area_m2 * element.thickness,
    }
}

fn triangle_field(
    computed: &crate::heat_plane_2d_element::HeatPlaneTriangleComputed,
    potentials: [f64; 3],
) -> [f64; 3] {
    let gradient =
        plane_triangle_scalar_gradient(&computed.gradient_x, &computed.gradient_y, &potentials);
    let electric_x = -gradient[0];
    let electric_y = -gradient[1];
    [electric_x, electric_y, electric_x.hypot(electric_y)]
}

fn relative_error(left: f64, right: f64) -> f64 {
    (left - right).abs() / left.abs().max(right.abs()).max(1.0e-30)
}

fn validate_request(request: &SolveElectricConductionPlaneQuad2dRequest) -> Result<(), String> {
    if request.nodes.is_empty() || request.elements.is_empty() {
        return Err("electric conduction model requires nodes and elements".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_electric_potential) && request.terminals.is_empty()
    {
        return Err(
            "electric conduction model requires a fixed potential or impedance terminal"
                .to_string(),
        );
    }
    let mut node_ids = HashSet::new();
    if request.nodes.iter().any(|node| {
        node.id.trim().is_empty()
            || !node_ids.insert(node.id.as_str())
            || !node.x.is_finite()
            || !node.y.is_finite()
            || !node.electric_potential_v.is_finite()
            || !node.current_source_a.is_finite()
    }) {
        return Err("electric conduction node parameters are invalid".to_string());
    }
    let node_count = request.nodes.len();
    let mut element_ids = HashSet::new();
    if request.elements.iter().any(|element| {
        let node_indices = [
            element.node_i,
            element.node_j,
            element.node_k,
            element.node_l,
        ];
        element.id.trim().is_empty()
            || !element_ids.insert(element.id.as_str())
            || node_indices.iter().any(|index| *index >= node_count)
            || !has_unique_quad_nodes(node_indices)
            || !element.thickness.is_finite()
            || element.thickness <= 0.0
            || !element.electrical_conductivity_s_m.is_finite()
            || element.electrical_conductivity_s_m <= 0.0
    }) {
        return Err("electric conduction element parameters are invalid".to_string());
    }
    if request
        .elements
        .iter()
        .any(|element| !has_consistent_quad_orientation(request, element))
    {
        return Err("electric conduction quad geometry is invalid".to_string());
    }
    validate_interfaces(request)?;
    Ok(())
}

fn has_unique_quad_nodes(nodes: [usize; 4]) -> bool {
    nodes
        .iter()
        .enumerate()
        .all(|(index, node)| !nodes[..index].contains(node))
}

fn has_consistent_quad_orientation(
    request: &SolveElectricConductionPlaneQuad2dRequest,
    element: &ElectricConductionPlaneQuadElementInput,
) -> bool {
    let points = [
        conduction_point(request, element.node_i),
        conduction_point(request, element.node_j),
        conduction_point(request, element.node_k),
        conduction_point(request, element.node_l),
    ];
    let first = signed_twice_area(points[0], points[1], points[2]);
    let second = signed_twice_area(points[0], points[2], points[3]);
    first.is_finite()
        && second.is_finite()
        && first.abs() > 2.0e-12
        && second.abs() > 2.0e-12
        && first.signum() == second.signum()
}

fn signed_twice_area(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
}

#[cfg(test)]
mod tests {
    use super::solve_electric_conduction_plane_quad_2d;
    use kyuubiki_protocol::{
        ElectricConductionContactInput, ElectricConductionPlaneNodeInput,
        ElectricConductionPlaneQuadElementInput, ElectricConductionTerminalInput,
        SolveElectricConductionPlaneQuad2dRequest,
    };

    #[test]
    fn voltage_driven_conductor_matches_uniform_current_and_power_balance() {
        let resistivity_ohm_m = 1.68e-8;
        let voltage_v = 2.0 * resistivity_ohm_m * 0.03 / 3.0e-5;
        let result = solve_electric_conduction_plane_quad_2d(&uniform_request(
            1.0 / resistivity_ohm_m,
            voltage_v,
        ))
        .expect("electric conduction solve");

        assert!((result.total_injected_current_a - 2.0).abs() < 1.0e-12);
        assert!((result.total_extracted_current_a - 2.0).abs() < 1.0e-12);
        assert!(result.current_balance_relative_error < 1.0e-12);
        assert_eq!(result.max_free_current_residual_a, 0.0);
        assert!((result.total_joule_power_w - 2.0_f64.powi(2) * 1.68e-5).abs() < 1.0e-15);
        assert!(result.power_balance_relative_error < 1.0e-12);
    }

    #[test]
    fn series_conductors_match_nonuniform_analytic_fields() {
        let result = solve_electric_conduction_plane_quad_2d(&series_request())
            .expect("series electric conduction solve");

        assert!((result.nodes[1].electric_potential_v - 2.0).abs() < 1.0e-12);
        assert!((result.nodes[4].electric_potential_v - 2.0).abs() < 1.0e-12);
        assert!((result.max_electric_field_v_m - 2.0).abs() < 1.0e-12);
        assert!((result.max_current_density_a_m2 - 2.0).abs() < 1.0e-12);
        assert!((result.total_injected_current_a - 2.0).abs() < 1.0e-12);
        assert!(result.free_current_residual_relative_error < 1.0e-12);
        assert!((result.total_joule_power_w - 6.0).abs() < 1.0e-12);
        assert!(result.power_balance_relative_error < 1.0e-12);
    }

    #[test]
    fn current_driven_conductor_recovers_voltage_and_free_node_residual() {
        let resistivity_ohm_m = 1.68e-8;
        let mut request = uniform_request(1.0 / resistivity_ohm_m, 0.0);
        for index in [1, 2] {
            request.nodes[index].fix_electric_potential = false;
            request.nodes[index].current_source_a = 1.0;
        }

        let result = solve_electric_conduction_plane_quad_2d(&request)
            .expect("current-driven electric conduction solve");

        let expected_voltage_v = 2.0 * resistivity_ohm_m * 0.03 / 3.0e-5;
        assert!((result.nodes[1].electric_potential_v - expected_voltage_v).abs() < 1.0e-15);
        assert!((result.nodes[2].electric_potential_v - expected_voltage_v).abs() < 1.0e-15);
        assert!((result.total_injected_current_a - 2.0).abs() < 1.0e-12);
        assert!(result.free_current_residual_relative_error < 1.0e-12);
        assert!((result.total_joule_power_w - 6.72e-5).abs() < 1.0e-15);
        assert!(result.power_balance_relative_error < 1.0e-12);
    }

    #[test]
    fn explicit_contact_resistance_partitions_bulk_and_interface_power() {
        let result = solve_electric_conduction_plane_quad_2d(&contact_request())
            .expect("contact-resistance electric conduction solve");

        assert!((result.nodes[1].electric_potential_v - 1.0).abs() < 1.0e-12);
        assert!((result.nodes[4].electric_potential_v - 2.0).abs() < 1.0e-12);
        assert!((result.total_injected_current_a - 1.0).abs() < 1.0e-12);
        assert!((result.total_bulk_joule_power_w - 2.0).abs() < 1.0e-12);
        assert!((result.total_contact_joule_power_w - 1.0).abs() < 1.0e-12);
        assert!((result.total_joule_power_w - 3.0).abs() < 1.0e-12);
        assert!(result.power_balance_relative_error < 1.0e-12);
        assert!(result.source_power_balance_relative_error < 1.0e-12);
    }

    #[test]
    fn impedance_terminals_anchor_floating_model_and_close_source_power() {
        let result = solve_electric_conduction_plane_quad_2d(&terminal_request())
            .expect("terminal-impedance electric conduction solve");

        assert!((result.nodes[0].electric_potential_v - 1.0).abs() < 1.0e-12);
        assert!((result.nodes[1].electric_potential_v - 2.0).abs() < 1.0e-12);
        assert!((result.total_injected_current_a - 1.0).abs() < 1.0e-12);
        assert!((result.total_bulk_joule_power_w - 1.0).abs() < 1.0e-12);
        assert!((result.total_terminal_impedance_power_w - 2.0).abs() < 1.0e-12);
        assert!((result.total_source_power_w - 3.0).abs() < 1.0e-12);
        assert!((result.total_dissipated_power_w - 3.0).abs() < 1.0e-12);
        assert!(result.source_power_balance_relative_error < 1.0e-12);
    }

    #[test]
    fn rejects_out_of_range_element_nodes_without_panicking() {
        let mut request = uniform_request(1.0, 1.0);
        request.elements[0].node_l = request.nodes.len();

        let error = solve_electric_conduction_plane_quad_2d(&request).expect_err("invalid node");

        assert_eq!(error, "electric conduction element parameters are invalid");
    }

    #[test]
    fn rejects_nonpositive_contact_and_terminal_impedance() {
        let mut contact_model = contact_request();
        contact_model.contact_interfaces[0].contact_resistance_ohm = 0.0;
        assert_eq!(
            solve_electric_conduction_plane_quad_2d(&contact_model).expect_err("zero contact"),
            "electric conduction contact interface parameters are invalid"
        );

        let mut terminal_model = terminal_request();
        terminal_model.terminals[0].impedance_ohm = -1.0;
        assert_eq!(
            solve_electric_conduction_plane_quad_2d(&terminal_model)
                .expect_err("negative terminal impedance"),
            "electric conduction terminal parameters are invalid"
        );
    }

    fn uniform_request(
        conductivity_s_m: f64,
        voltage_v: f64,
    ) -> SolveElectricConductionPlaneQuad2dRequest {
        let mut conductor = element("conductor", 0, 1, 2, 3, conductivity_s_m);
        conductor.thickness = 0.001;
        SolveElectricConductionPlaneQuad2dRequest {
            nodes: vec![
                node("n0", 0.0, 0.0, true, 0.0),
                node("n1", 0.03, 0.0, true, voltage_v),
                node("n2", 0.03, 0.03, true, voltage_v),
                node("n3", 0.0, 0.03, true, 0.0),
            ],
            elements: vec![conductor],
            contact_interfaces: vec![],
            terminals: vec![],
        }
    }

    fn series_request() -> SolveElectricConductionPlaneQuad2dRequest {
        SolveElectricConductionPlaneQuad2dRequest {
            nodes: vec![
                node("left_bottom", 0.0, 0.0, true, 0.0),
                node("interface_bottom", 1.0, 0.0, false, 0.0),
                node("right_bottom", 2.0, 0.0, true, 3.0),
                node("left_top", 0.0, 1.0, true, 0.0),
                node("interface_top", 1.0, 1.0, false, 0.0),
                node("right_top", 2.0, 1.0, true, 3.0),
            ],
            elements: vec![
                element("sigma_one", 0, 1, 4, 3, 1.0),
                element("sigma_two", 1, 2, 5, 4, 2.0),
            ],
            contact_interfaces: vec![],
            terminals: vec![],
        }
    }

    fn contact_request() -> SolveElectricConductionPlaneQuad2dRequest {
        SolveElectricConductionPlaneQuad2dRequest {
            nodes: vec![
                node("left_bottom", 0.0, 0.0, true, 0.0),
                node("left_interface_bottom", 1.0, 0.0, false, 0.0),
                node("left_interface_top", 1.0, 1.0, false, 0.0),
                node("left_top", 0.0, 1.0, true, 0.0),
                node("right_interface_bottom", 1.0, 0.0, false, 0.0),
                node("right_bottom", 2.0, 0.0, true, 3.0),
                node("right_top", 2.0, 1.0, true, 3.0),
                node("right_interface_top", 1.0, 1.0, false, 0.0),
            ],
            elements: vec![
                element("left_bulk", 0, 1, 2, 3, 1.0),
                element("right_bulk", 4, 5, 6, 7, 1.0),
            ],
            contact_interfaces: vec![
                contact("contact_bottom", 1, 4, 2.0),
                contact("contact_top", 2, 7, 2.0),
            ],
            terminals: vec![],
        }
    }

    fn terminal_request() -> SolveElectricConductionPlaneQuad2dRequest {
        SolveElectricConductionPlaneQuad2dRequest {
            nodes: vec![
                node("left_bottom", 0.0, 0.0, false, 0.0),
                node("right_bottom", 1.0, 0.0, false, 0.0),
                node("right_top", 1.0, 1.0, false, 0.0),
                node("left_top", 0.0, 1.0, false, 0.0),
            ],
            elements: vec![element("bulk", 0, 1, 2, 3, 1.0)],
            contact_interfaces: vec![],
            terminals: vec![
                terminal("left_bottom_terminal", 0, 0.0, 2.0),
                terminal("right_bottom_terminal", 1, 3.0, 2.0),
                terminal("right_top_terminal", 2, 3.0, 2.0),
                terminal("left_top_terminal", 3, 0.0, 2.0),
            ],
        }
    }

    fn node(
        id: &str,
        x: f64,
        y: f64,
        fix_electric_potential: bool,
        electric_potential_v: f64,
    ) -> ElectricConductionPlaneNodeInput {
        ElectricConductionPlaneNodeInput {
            id: id.to_string(),
            x,
            y,
            fix_electric_potential,
            electric_potential_v,
            current_source_a: 0.0,
        }
    }

    fn element(
        id: &str,
        node_i: usize,
        node_j: usize,
        node_k: usize,
        node_l: usize,
        electrical_conductivity_s_m: f64,
    ) -> ElectricConductionPlaneQuadElementInput {
        ElectricConductionPlaneQuadElementInput {
            id: id.to_string(),
            node_i,
            node_j,
            node_k,
            node_l,
            thickness: 1.0,
            electrical_conductivity_s_m,
        }
    }

    fn contact(
        id: &str,
        node_i: usize,
        node_j: usize,
        contact_resistance_ohm: f64,
    ) -> ElectricConductionContactInput {
        ElectricConductionContactInput {
            id: id.to_string(),
            node_i,
            node_j,
            contact_resistance_ohm,
        }
    }

    fn terminal(
        id: &str,
        node: usize,
        external_potential_v: f64,
        impedance_ohm: f64,
    ) -> ElectricConductionTerminalInput {
        ElectricConductionTerminalInput {
            id: id.to_string(),
            node,
            external_potential_v,
            impedance_ohm,
        }
    }
}
