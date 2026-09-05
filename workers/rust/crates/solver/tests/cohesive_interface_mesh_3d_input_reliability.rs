use kyuubiki_protocol::{
    CohesiveInterface3dMaterialInput, CohesiveInterfaceMesh3dElementInput,
    CohesiveInterfaceMesh3dMaterialInput, CohesiveInterfaceMesh3dNodeInput,
    SolveCohesiveInterfaceMesh3dRequest,
};
use kyuubiki_solver::solve_cohesive_interface_mesh_3d;

#[test]
fn rejects_orphan_nodes_and_under_restrained_components_before_iteration() {
    let mut unconstrained = loaded_request();
    for node in &mut unconstrained.nodes {
        node.fixed = [false; 3];
    }
    assert_error_contains(unconstrained, "requires constrained dofs");

    let mut orphan = loaded_request();
    orphan.nodes.push(node("orphan", 4.0, 0.0, 0.0, [true; 3]));
    assert_error_contains(orphan, "node 6 (orphan) is not referenced");

    let mut under_restrained = loaded_request();
    for node in &mut under_restrained.nodes {
        node.fixed = [false; 3];
    }
    under_restrained.nodes[0].fixed = [true; 3];
    assert_error_contains(under_restrained, "restrains rigid-body rank 3/6");

    let mut disconnected = loaded_request();
    append_component(&mut disconnected, "floating", 2.0, false);
    assert_error_contains(
        disconnected,
        "component 1 (first node 6:floating-lower-a) restrains rigid-body rank 3/6",
    );
}

#[test]
fn independently_restrained_components_solve_in_one_global_system() {
    let mut request = loaded_request();
    append_component(&mut request, "second", 2.0, true);

    let result = solve_cohesive_interface_mesh_3d(&request)
        .expect("independently restrained interface components must solve");

    assert!(result.converged);
    assert_eq!(result.nodes.len(), 12);
    assert_eq!(result.elements.len(), 2);
    for node in result.nodes.iter().skip(3).take(3) {
        assert_close(node.displacement[2], 0.005);
    }
    for node in result.nodes.iter().skip(9).take(3) {
        assert_close(node.displacement[2], 0.005);
    }
}

fn loaded_request() -> SolveCohesiveInterfaceMesh3dRequest {
    let mut request = SolveCohesiveInterfaceMesh3dRequest {
        id: "topology-review".to_string(),
        nodes: Vec::new(),
        materials: vec![CohesiveInterfaceMesh3dMaterialInput {
            id: "adhesive".to_string(),
            properties: CohesiveInterface3dMaterialInput {
                normal_initial_stiffness: 1000.0,
                normal_compression_stiffness: 1200.0,
                normal_peak_traction: 100.0,
                normal_failure_separation: 1.0,
                shear_initial_stiffness: 800.0,
                shear_peak_traction: 80.0,
                shear_failure_separation: 1.0,
            },
        }],
        elements: Vec::new(),
        host_tetrahedra: Vec::new(),
        load_steps: Some(1),
        control_history: None,
        max_iterations: Some(8),
        tolerance: Some(1.0e-11),
    };
    append_component(&mut request, "first", 0.0, true);
    request
}

fn append_component(
    request: &mut SolveCohesiveInterfaceMesh3dRequest,
    prefix: &str,
    origin_x: f64,
    fully_restrained: bool,
) {
    let base = request.nodes.len();
    for (suffix, x, y, upper) in [
        ("lower-a", origin_x, 0.0, false),
        ("lower-b", origin_x + 1.0, 0.0, false),
        ("lower-c", origin_x, 1.0, false),
        ("upper-a", origin_x, 0.0, true),
        ("upper-b", origin_x + 1.0, 0.0, true),
        ("upper-c", origin_x, 1.0, true),
    ] {
        let fixed = if fully_restrained {
            if upper {
                [true, true, false]
            } else {
                [true; 3]
            }
        } else if suffix == "lower-a" {
            [true; 3]
        } else {
            [false; 3]
        };
        let mut entry = node(&format!("{prefix}-{suffix}"), x, y, 0.0, fixed);
        if upper {
            entry.load[2] = 5.0 * 0.5 / 3.0;
        }
        request.nodes.push(entry);
    }
    request.elements.push(CohesiveInterfaceMesh3dElementInput {
        id: format!("{prefix}-interface"),
        lower_a: base,
        lower_b: base + 1,
        lower_c: base + 2,
        upper_a: base + 3,
        upper_b: base + 4,
        upper_c: base + 5,
        material_id: "adhesive".to_string(),
    });
}

fn node(id: &str, x: f64, y: f64, z: f64, fixed: [bool; 3]) -> CohesiveInterfaceMesh3dNodeInput {
    CohesiveInterfaceMesh3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fixed,
        prescribed_displacement: None,
        load: [0.0; 3],
    }
}

fn assert_error_contains(request: SolveCohesiveInterfaceMesh3dRequest, expected: &str) {
    let error = solve_cohesive_interface_mesh_3d(&request).expect_err("request must fail");
    assert!(error.contains(expected), "unexpected error: {error}");
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1.0e-9 * expected.abs().max(1.0),
        "expected {expected:.12e}, got {actual:.12e}"
    );
}
