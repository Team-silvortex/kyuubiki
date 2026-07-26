use kyuubiki_protocol::{
    CohesiveInterface2dMaterialInput, CohesiveInterfaceMesh2dElementInput,
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
        load_steps: Some(4),
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
        load_steps: Some(5),
        max_iterations: Some(12),
        tolerance: Some(1.0e-11),
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

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "expected {expected}, got {actual}"
    );
}
