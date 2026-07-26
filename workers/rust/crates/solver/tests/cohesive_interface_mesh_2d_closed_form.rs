use kyuubiki_protocol::{
    CohesiveInterface2dMaterialInput, CohesiveInterfaceMesh2dConnectorSpringInput,
    CohesiveInterfaceMesh2dControlStepInput, CohesiveInterfaceMesh2dElementInput,
    CohesiveInterfaceMesh2dMaterialInput, CohesiveInterfaceMesh2dNodeInput,
    SolveCohesiveInterfaceMesh2dRequest,
};
use kyuubiki_solver::solve_cohesive_interface_mesh_2d;

const TOLERANCE: f64 = 1.0e-10;

#[test]
fn single_element_matches_uniform_elastic_opening() {
    let result = solve_cohesive_interface_mesh_2d(&single_element_request())
        .expect("single cohesive mesh element should solve");

    assert!(result.converged);
    assert_eq!(result.steps.len(), 4);
    assert!(result.steps.iter().all(|step| step.converged));
    assert_close(result.nodes[2].displacement[1], 0.005);
    assert_close(result.nodes[3].displacement[1], 0.005);
    assert_close(result.nodes[0].reaction[1], -2.5);
    assert_close(result.nodes[1].reaction[1], -2.5);
    assert_close(result.elements[0].local_traction[1], 5.0);
    assert_close(result.max_normal_damage, 0.0);
    assert_close(result.completed_load_factor, 1.0);
}

#[test]
fn two_elements_assemble_shared_node_forces() {
    let result = solve_cohesive_interface_mesh_2d(&two_element_request())
        .expect("two cohesive mesh elements should solve");

    assert!(result.converged);
    assert_eq!(result.elements.len(), 2);
    for node in &result.nodes[3..] {
        assert_close(node.displacement[1], 0.005);
    }
    assert_close(result.nodes[0].reaction[1], -2.5);
    assert_close(result.nodes[1].reaction[1], -5.0);
    assert_close(result.nodes[2].reaction[1], -2.5);
    for element in &result.elements {
        assert_close(element.local_traction[1], 5.0);
        assert_close(element.max_normal_damage, 0.0);
    }
}

#[test]
fn singular_step_rolls_back_without_committing_trial_state() {
    let mut request = single_element_request();
    for node in &mut request.nodes {
        node.fixed[0] = false;
    }

    let result = solve_cohesive_interface_mesh_2d(&request)
        .expect("singular models return rollback diagnostics");

    assert!(!result.converged);
    assert_close(result.completed_load_factor, 0.0);
    assert_eq!(result.steps.len(), 1);
    assert!(!result.steps[0].converged);
    assert!(
        result.failure_reason.as_deref().is_some_and(|reason| {
            reason.contains("singular") && reason.contains("constraints")
        })
    );
    assert!(
        result
            .nodes
            .iter()
            .all(|node| node.displacement == [0.0, 0.0])
    );
    assert_close(result.max_normal_damage, 0.0);
}

#[test]
fn invalid_material_reference_is_rejected() {
    let mut request = single_element_request();
    request.elements[0].material_id = "missing".to_string();

    let error = solve_cohesive_interface_mesh_2d(&request)
        .expect_err("unknown material reference must fail");
    assert!(error.contains("unknown material"));
}

#[test]
fn prescribed_opening_crosses_peak_and_retains_softening_damage() {
    let mut request = prescribed_opening_request(0.025);
    request.load_steps = Some(5);

    let result = solve_cohesive_interface_mesh_2d(&request)
        .expect("prescribed opening should traverse stable softening");

    assert!(result.converged);
    assert_eq!(result.steps.len(), 5);
    assert!(result.steps.iter().all(|step| step.iterations == 1));
    assert_close(result.nodes[2].displacement[1], 0.025);
    assert_close(result.nodes[3].displacement[1], 0.025);
    assert_close(result.elements[0].local_traction[1], 5.0);
    assert_close(result.max_normal_damage, 0.8);
    assert_close(result.nodes[0].reaction[1], -2.5);
    assert_close(result.nodes[2].reaction[1], 2.5);
}

#[test]
fn prescribed_displacement_on_free_axis_is_rejected() {
    let mut request = single_element_request();
    request.nodes[2].prescribed_displacement = Some([0.0, 0.01]);

    let error = solve_cohesive_interface_mesh_2d(&request)
        .expect_err("free-axis prescribed displacement is ambiguous");
    assert!(error.contains("prescribed displacement on free axis"));
}

#[test]
fn prescribed_opening_reaches_complete_failure_without_residual_reaction() {
    let mut request = prescribed_opening_request(0.04);
    request.load_steps = Some(8);

    let result = solve_cohesive_interface_mesh_2d(&request)
        .expect("failure separation should remain a converged displacement path");

    assert!(result.converged);
    assert_close(result.max_normal_damage, 1.0);
    assert_close(result.elements[0].local_traction[1], 0.0);
    for node in &result.nodes {
        assert_close(node.reaction[1], 0.0);
    }
}

#[test]
fn explicit_control_history_retains_damage_through_unload_and_reload() {
    let mut request = single_element_request();
    for node in &mut request.nodes {
        node.fixed = [true, true];
        node.load = [0.0, 0.0];
        node.prescribed_displacement = None;
    }
    request.load_steps = None;
    request.control_history = Some(
        [0.02, 0.01, 0.025, 0.0125]
            .into_iter()
            .map(controlled_opening)
            .collect(),
    );

    let result = solve_cohesive_interface_mesh_2d(&request)
        .expect("explicit unload and reload history should solve");

    assert!(result.converged);
    assert_eq!(result.steps.len(), 4);
    assert_close(result.steps[0].max_normal_damage, 2.0 / 3.0);
    assert_close(result.steps[1].max_normal_damage, 2.0 / 3.0);
    assert_close(result.steps[2].max_normal_damage, 0.8);
    assert_close(result.steps[3].max_normal_damage, 0.8);
    assert_close(result.steps[0].max_resultant_traction, 20.0 / 3.0);
    assert_close(result.steps[1].max_resultant_traction, 10.0 / 3.0);
    assert_close(result.steps[2].max_resultant_traction, 5.0);
    assert_close(result.steps[3].max_resultant_traction, 2.5);
    assert_close(result.steps[3].reaction_norm, 2.5);
    assert_close(result.nodes[2].displacement[1], 0.0125);
    assert_close(result.max_displacement, 0.025);
    assert_close(result.max_normal_damage, 0.8);
}

#[test]
fn explicit_control_history_separates_normal_and_shear_paths() {
    let mut request = single_element_request();
    for node in &mut request.nodes {
        node.fixed = [true, true];
        node.load = [0.0, 0.0];
        node.prescribed_displacement = None;
    }
    request.load_steps = None;
    request.control_history = Some(vec![
        controlled_jump([0.02, 0.005]),
        controlled_jump([0.01, 0.02]),
    ]);

    let result = solve_cohesive_interface_mesh_2d(&request)
        .expect("independent normal and shear control should solve");

    assert!(result.converged);
    assert_close(result.steps[0].max_shear_damage, 0.625);
    assert_close(result.steps[0].max_normal_damage, 0.0);
    assert_close(result.steps[1].max_shear_damage, 0.625);
    assert_close(result.steps[1].max_normal_damage, 2.0 / 3.0);
    assert_close(result.max_shear_damage, 0.625);
    assert_close(result.max_normal_damage, 2.0 / 3.0);
}

#[test]
fn explicit_control_history_rejects_ambiguous_or_free_dof_values() {
    let mut request = single_element_request();
    request.control_history = Some(vec![controlled_opening(0.01)]);
    let conflict = solve_cohesive_interface_mesh_2d(&request)
        .expect_err("generated and explicit controls must not coexist");
    assert!(conflict.contains("mutually exclusive"));

    request.load_steps = None;
    let free_dof = solve_cohesive_interface_mesh_2d(&request)
        .expect_err("explicit control must not prescribe a free dof");
    assert!(free_dof.contains("prescribes a free dof"));
}

#[test]
fn connector_springs_share_equilibrium_with_the_cohesive_interface() {
    let mut request = single_element_request();
    request.nodes[2].load = [0.0, 0.0];
    request.nodes[3].load = [0.0, 0.0];
    request.nodes.push(node("driver-0", 0.0, false, 2.5));
    request.nodes.push(node("driver-1", 1.0, false, 2.5));
    request.connector_springs = vec![
        connector("host-0", 2, 4, [0.0, 500.0]),
        connector("host-1", 3, 5, [0.0, 500.0]),
    ];

    let result = solve_cohesive_interface_mesh_2d(&request)
        .expect("connector and cohesive elements should co-assemble");

    assert!(result.converged);
    assert_close(result.nodes[2].displacement[1], 0.005);
    assert_close(result.nodes[3].displacement[1], 0.005);
    assert_close(result.nodes[4].displacement[1], 0.01);
    assert_close(result.nodes[5].displacement[1], 0.01);
    assert_close(result.elements[0].local_traction[1], 5.0);
    assert_close(result.max_connector_force, 2.5);
    for spring in &result.connector_springs {
        assert_close(spring.relative_displacement[1], 0.005);
        assert_close(spring.force[1], 2.5);
        assert_close(spring.strain_energy, 0.00625);
    }
}

#[test]
fn invalid_connector_contracts_are_rejected() {
    let mut duplicate = single_element_request();
    duplicate.connector_springs = vec![
        connector("host", 0, 2, [0.0, 500.0]),
        connector("host", 1, 3, [0.0, 500.0]),
    ];
    let error = solve_cohesive_interface_mesh_2d(&duplicate)
        .expect_err("duplicate connector ids must fail");
    assert!(error.contains("duplicate connector spring id"));

    let mut bounds = single_element_request();
    bounds.connector_springs = vec![connector("host", 0, 99, [0.0, 500.0])];
    let error = solve_cohesive_interface_mesh_2d(&bounds)
        .expect_err("out-of-bounds connector nodes must fail");
    assert!(error.contains("node index is out of bounds"));

    let mut stiffness = single_element_request();
    stiffness.connector_springs = vec![connector("host", 0, 2, [0.0, -1.0])];
    let error = solve_cohesive_interface_mesh_2d(&stiffness)
        .expect_err("negative connector stiffness must fail");
    assert!(error.contains("stiffness must be finite, non-negative, and non-zero"));
}

fn single_element_request() -> SolveCohesiveInterfaceMesh2dRequest {
    SolveCohesiveInterfaceMesh2dRequest {
        id: "mesh.single".to_string(),
        nodes: vec![
            node("lower-0", 0.0, true, 0.0),
            node("lower-1", 1.0, true, 0.0),
            node("upper-0", 0.0, false, 2.5),
            node("upper-1", 1.0, false, 2.5),
        ],
        materials: vec![material()],
        elements: vec![element("interface-0", 0, 1, 2, 3)],
        connector_springs: vec![],
        load_steps: Some(4),
        control_history: None,
        max_iterations: Some(12),
        tolerance: Some(1.0e-11),
    }
}

fn two_element_request() -> SolveCohesiveInterfaceMesh2dRequest {
    SolveCohesiveInterfaceMesh2dRequest {
        id: "mesh.double".to_string(),
        nodes: vec![
            node("lower-0", 0.0, true, 0.0),
            node("lower-1", 1.0, true, 0.0),
            node("lower-2", 2.0, true, 0.0),
            node("upper-0", 0.0, false, 2.5),
            node("upper-1", 1.0, false, 5.0),
            node("upper-2", 2.0, false, 2.5),
        ],
        materials: vec![material()],
        elements: vec![
            element("interface-0", 0, 1, 3, 4),
            element("interface-1", 1, 2, 4, 5),
        ],
        connector_springs: vec![],
        load_steps: Some(5),
        control_history: None,
        max_iterations: Some(12),
        tolerance: Some(1.0e-11),
    }
}

fn prescribed_opening_request(opening: f64) -> SolveCohesiveInterfaceMesh2dRequest {
    let mut request = single_element_request();
    for node in &mut request.nodes {
        node.load = [0.0, 0.0];
        node.fixed = [true, true];
    }
    request.nodes[2].prescribed_displacement = Some([0.0, opening]);
    request.nodes[3].prescribed_displacement = Some([0.0, opening]);
    request
}

fn controlled_opening(opening: f64) -> CohesiveInterfaceMesh2dControlStepInput {
    controlled_jump([0.0, opening])
}

fn controlled_jump(jump: [f64; 2]) -> CohesiveInterfaceMesh2dControlStepInput {
    CohesiveInterfaceMesh2dControlStepInput {
        load_factor: 0.0,
        prescribed_displacements: vec![[0.0, 0.0], [0.0, 0.0], jump, jump],
    }
}

fn node(
    id: &str,
    x: f64,
    lower_surface: bool,
    vertical_load: f64,
) -> CohesiveInterfaceMesh2dNodeInput {
    CohesiveInterfaceMesh2dNodeInput {
        id: id.to_string(),
        x,
        y: 0.0,
        fixed: [true, lower_surface],
        prescribed_displacement: None,
        load: [0.0, vertical_load],
    }
}

fn material() -> CohesiveInterfaceMesh2dMaterialInput {
    CohesiveInterfaceMesh2dMaterialInput {
        id: "adhesive".to_string(),
        properties: CohesiveInterface2dMaterialInput {
            normal_initial_stiffness: 1000.0,
            normal_compression_stiffness: 1200.0,
            normal_peak_traction: 10.0,
            normal_failure_separation: 0.04,
            shear_initial_stiffness: 800.0,
            shear_peak_traction: 8.0,
            shear_failure_separation: 0.05,
        },
    }
}

fn element(
    id: &str,
    lower_i: usize,
    lower_j: usize,
    upper_i: usize,
    upper_j: usize,
) -> CohesiveInterfaceMesh2dElementInput {
    CohesiveInterfaceMesh2dElementInput {
        id: id.to_string(),
        lower_i,
        lower_j,
        upper_i,
        upper_j,
        thickness: 1.0,
        material_id: "adhesive".to_string(),
    }
}

fn connector(
    id: &str,
    node_i: usize,
    node_j: usize,
    stiffness: [f64; 2],
) -> CohesiveInterfaceMesh2dConnectorSpringInput {
    CohesiveInterfaceMesh2dConnectorSpringInput {
        id: id.to_string(),
        node_i,
        node_j,
        stiffness,
    }
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "expected {expected}, got {actual}"
    );
}
