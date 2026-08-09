use kyuubiki_protocol::{
    CohesiveInterface2dMaterialInput, CohesiveInterfaceMesh2dElementInput,
    CohesiveInterfaceMesh2dMaterialInput, CohesiveInterfaceMesh2dNodeInput,
    SolveCohesiveInterfaceMesh2dRequest, SolveCohesiveInterfaceMesh2dResult,
};
use kyuubiki_solver::solve_cohesive_interface_mesh_2d;

const SEGMENT_COUNT: usize = 96;

#[test]
fn large_interface_mesh_uses_sparse_banded_newton_assembly() {
    let result = solve_cohesive_interface_mesh_2d(&segmented_request(SEGMENT_COUNT))
        .expect("large block interface mesh should solve");

    assert!(result.converged);
    assert_eq!(result.nodes.len(), SEGMENT_COUNT * 4);
    assert_eq!(result.elements.len(), SEGMENT_COUNT);
    assert_eq!(result.steps.len(), 1);
    assert_eq!(result.linear_solver_methods, ["symmetric_band_cholesky"]);
    let step = &result.steps[0];
    assert_eq!(step.linear_solver, "symmetric_band_cholesky");
    assert!(step.tangent_non_zero_count > SEGMENT_COUNT * 16);
    assert_eq!(
        result.max_tangent_non_zero_count,
        step.tangent_non_zero_count
    );
    assert!(step.tangent_fill_ratio > 0.0);
    assert!(step.tangent_fill_ratio < 0.02);
    assert_eq!(result.max_tangent_fill_ratio, step.tangent_fill_ratio);
    assert_eq!(result.max_normal_damage, 0.0);
    println!(
        "cohesive sparse assembly: nodes={}, dofs={}, non_zero={}, fill_ratio={:.6}, solver={}",
        result.nodes.len(),
        result.nodes.len() * 2,
        step.tangent_non_zero_count,
        step.tangent_fill_ratio,
        step.linear_solver
    );
}

#[test]
fn persisted_result_without_sparse_diagnostics_defaults_cleanly() {
    let result = solve_cohesive_interface_mesh_2d(&segmented_request(1))
        .expect("single interface segment should solve");
    let mut payload = serde_json::to_value(result).expect("result should serialize");
    let object = payload.as_object_mut().expect("result should be an object");
    object.remove("max_tangent_non_zero_count");
    object.remove("max_tangent_fill_ratio");
    object.remove("linear_solver_methods");
    for step in object["steps"]
        .as_array_mut()
        .expect("steps should be an array")
    {
        let step = step.as_object_mut().expect("step should be an object");
        step.remove("tangent_non_zero_count");
        step.remove("tangent_fill_ratio");
        step.remove("linear_solver");
    }

    let decoded: SolveCohesiveInterfaceMesh2dResult =
        serde_json::from_value(payload).expect("legacy result should decode");
    assert_eq!(decoded.max_tangent_non_zero_count, 0);
    assert_eq!(decoded.max_tangent_fill_ratio, 0.0);
    assert!(decoded.linear_solver_methods.is_empty());
    assert!(decoded.steps.iter().all(|step| {
        step.tangent_non_zero_count == 0
            && step.tangent_fill_ratio == 0.0
            && step.linear_solver.is_empty()
    }));
}

fn segmented_request(segment_count: usize) -> SolveCohesiveInterfaceMesh2dRequest {
    let mut nodes = Vec::with_capacity(segment_count * 4);
    let mut elements = Vec::with_capacity(segment_count);
    for segment in 0..segment_count {
        let x = segment as f64 * 1.25;
        let first = nodes.len();
        nodes.extend([
            node(format!("lower-{segment}-i"), x, true, [0.0, 0.0]),
            node(format!("lower-{segment}-j"), x + 1.0, true, [0.0, 0.0]),
            node(format!("upper-{segment}-i"), x, false, [0.25, 0.5]),
            node(format!("upper-{segment}-j"), x + 1.0, false, [0.25, 0.5]),
        ]);
        elements.push(CohesiveInterfaceMesh2dElementInput {
            id: format!("interface-{segment}"),
            lower_i: first,
            lower_j: first + 1,
            upper_i: first + 2,
            upper_j: first + 3,
            thickness: 1.0,
            material_id: "adhesive".to_string(),
        });
    }

    SolveCohesiveInterfaceMesh2dRequest {
        id: format!("sparse-interface-{segment_count}"),
        nodes,
        materials: vec![CohesiveInterfaceMesh2dMaterialInput {
            id: "adhesive".to_string(),
            properties: CohesiveInterface2dMaterialInput {
                normal_initial_stiffness: 1_000.0,
                normal_compression_stiffness: 2_000.0,
                normal_peak_traction: 100.0,
                normal_failure_separation: 0.5,
                shear_initial_stiffness: 800.0,
                shear_peak_traction: 80.0,
                shear_failure_separation: 0.5,
            },
        }],
        elements,
        connector_springs: vec![],
        host_trusses: vec![],
        host_plane_triangles: vec![],
        host_plane_quads: vec![],
        host_frames: vec![],
        load_steps: Some(1),
        control_history: None,
        max_iterations: Some(8),
        tolerance: Some(1.0e-11),
    }
}

fn node(id: String, x: f64, fixed: bool, load: [f64; 2]) -> CohesiveInterfaceMesh2dNodeInput {
    CohesiveInterfaceMesh2dNodeInput {
        id,
        x,
        y: 0.0,
        fixed: [fixed, fixed],
        prescribed_displacement: None,
        load,
        fixed_rotation: false,
        prescribed_rotation: None,
        moment_z: 0.0,
    }
}
