use kyuubiki_protocol::{
    ElectricConductionContactInput, ElectricConductionPlaneNodeInput,
    ElectricConductionPlaneQuadElementInput, ElectricConductionTerminalInput,
    SolveElectricConductionPlaneQuad2dRequest,
};
use kyuubiki_solver::solve_electric_conduction_plane_quad_2d;

#[test]
fn rejects_duplicate_identity_connectivity_and_non_finite_material_inputs() {
    let mut duplicate_node = valid_request();
    duplicate_node.nodes[1].id = duplicate_node.nodes[0].id.clone();
    assert_error(
        duplicate_node,
        "electric conduction node parameters are invalid",
    );

    let mut duplicate_connectivity = valid_request();
    duplicate_connectivity.elements[0].node_l = duplicate_connectivity.elements[0].node_i;
    assert_error(
        duplicate_connectivity,
        "electric conduction element parameters are invalid",
    );

    let mut duplicate_element = valid_request();
    duplicate_element
        .elements
        .push(duplicate_element.elements[0].clone());
    assert_error(
        duplicate_element,
        "electric conduction element parameters are invalid",
    );

    let mut non_finite = valid_request();
    non_finite.elements[0].electrical_conductivity_s_m = f64::NAN;
    assert_error(
        non_finite,
        "electric conduction element parameters are invalid",
    );
}

#[test]
fn rejects_degenerate_and_self_intersecting_quads_before_assembly() {
    let mut degenerate = valid_request();
    degenerate.nodes[2].x = 2.0;
    degenerate.nodes[2].y = 0.0;
    assert_error(degenerate, "electric conduction quad geometry is invalid");

    let mut bow_tie = valid_request();
    bow_tie.elements[0].node_j = 2;
    bow_tie.elements[0].node_k = 1;
    assert_error(bow_tie, "electric conduction quad geometry is invalid");
}

#[test]
fn rejects_unanchored_or_partially_unanchored_models_without_panicking() {
    let mut unanchored = valid_request();
    for node in &mut unanchored.nodes {
        node.fix_electric_potential = false;
    }
    assert_error_contains(
        unanchored,
        "requires a fixed potential or impedance terminal",
    );

    assert_error_contains(
        request_with_free_island(),
        "topology component containing node 4 (island-0) is not anchored",
    );
}

#[test]
fn accepts_terminal_or_contact_anchoring_for_disconnected_components() {
    let mut terminal_anchored = request_with_free_island();
    terminal_anchored
        .terminals
        .push(ElectricConductionTerminalInput {
            id: "island-terminal".to_string(),
            node: 4,
            external_potential_v: 0.5,
            impedance_ohm: 2.0,
        });
    solve_electric_conduction_plane_quad_2d(&terminal_anchored)
        .expect("a finite-impedance terminal must anchor its component");

    let mut contact_anchored = request_with_free_island();
    contact_anchored
        .contact_interfaces
        .push(ElectricConductionContactInput {
            id: "bridge-contact".to_string(),
            node_i: 1,
            node_j: 4,
            contact_resistance_ohm: 2.0,
        });
    solve_electric_conduction_plane_quad_2d(&contact_anchored)
        .expect("a finite-resistance contact must connect the island to an anchored component");
}

fn valid_request() -> SolveElectricConductionPlaneQuad2dRequest {
    SolveElectricConductionPlaneQuad2dRequest {
        nodes: vec![
            fixed_node("n0", 0.0, 0.0, 0.0),
            fixed_node("n1", 1.0, 0.0, 1.0),
            fixed_node("n2", 1.0, 1.0, 1.0),
            fixed_node("n3", 0.0, 1.0, 0.0),
        ],
        elements: vec![ElectricConductionPlaneQuadElementInput {
            id: "conductor".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 1.0,
            electrical_conductivity_s_m: 1.0,
        }],
        contact_interfaces: vec![],
        terminals: vec![],
    }
}

fn request_with_free_island() -> SolveElectricConductionPlaneQuad2dRequest {
    let mut request = valid_request();
    let offset = request.nodes.len();
    request.nodes.extend([
        free_node("island-0", 2.0, 0.0),
        free_node("island-1", 3.0, 0.0),
        free_node("island-2", 3.0, 1.0),
        free_node("island-3", 2.0, 1.0),
    ]);
    request
        .elements
        .push(ElectricConductionPlaneQuadElementInput {
            id: "free-island".to_string(),
            node_i: offset,
            node_j: offset + 1,
            node_k: offset + 2,
            node_l: offset + 3,
            thickness: 1.0,
            electrical_conductivity_s_m: 1.0,
        });
    request
}

fn fixed_node(
    id: &str,
    x: f64,
    y: f64,
    electric_potential_v: f64,
) -> ElectricConductionPlaneNodeInput {
    ElectricConductionPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_electric_potential: true,
        electric_potential_v,
        current_source_a: 0.0,
    }
}

fn free_node(id: &str, x: f64, y: f64) -> ElectricConductionPlaneNodeInput {
    ElectricConductionPlaneNodeInput {
        fix_electric_potential: false,
        ..fixed_node(id, x, y, 0.0)
    }
}

fn assert_error(request: SolveElectricConductionPlaneQuad2dRequest, expected: &str) {
    let error = solve_electric_conduction_plane_quad_2d(&request).expect_err("request must fail");
    assert_eq!(error, expected);
}

fn assert_error_contains(request: SolveElectricConductionPlaneQuad2dRequest, expected: &str) {
    let error = solve_electric_conduction_plane_quad_2d(&request).expect_err("request must fail");
    assert!(error.contains(expected), "unexpected error: {error}");
}
