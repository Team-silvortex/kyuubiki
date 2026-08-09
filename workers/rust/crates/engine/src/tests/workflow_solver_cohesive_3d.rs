use crate::{
    EngineSolveRequest, chunk_result, run_solve_operator, solve, supported_workflow_operator_ids,
};
use kyuubiki_protocol::{ResultChunkKind, ResultChunkRequest, SolveCohesiveInterfaceMesh3dRequest};

#[test]
fn runs_cohesive_interface_mesh_3d_through_the_workflow_executor() {
    let payload = serde_json::json!({
        "id": "workflow.mesh.3d",
        "nodes": [
            { "id": "lower-a", "x": 0.0, "y": 0.0, "z": 0.0, "fixed": [true, true, true], "load": [0.0, 0.0, 0.0] },
            { "id": "lower-b", "x": 1.0, "y": 0.0, "z": 0.0, "fixed": [true, true, true], "load": [0.0, 0.0, 0.0] },
            { "id": "lower-c", "x": 0.0, "y": 1.0, "z": 0.0, "fixed": [true, true, true], "load": [0.0, 0.0, 0.0] },
            { "id": "upper-a", "x": 0.0, "y": 0.0, "z": 0.0, "fixed": [true, true, false], "load": [0.0, 0.0, 0.8333333333333334] },
            { "id": "upper-b", "x": 1.0, "y": 0.0, "z": 0.0, "fixed": [true, true, false], "load": [0.0, 0.0, 0.8333333333333334] },
            { "id": "upper-c", "x": 0.0, "y": 1.0, "z": 0.0, "fixed": [true, true, false], "load": [0.0, 0.0, 0.8333333333333334] }
        ],
        "materials": [{
            "id": "adhesive",
            "properties": {
                "normal_initial_stiffness": 1000.0,
                "normal_compression_stiffness": 2000.0,
                "normal_peak_traction": 100.0,
                "normal_failure_separation": 1.0,
                "shear_initial_stiffness": 500.0,
                "shear_peak_traction": 50.0,
                "shear_failure_separation": 1.0
            }
        }],
        "elements": [{
            "id": "interface-0",
            "lower_a": 0, "lower_b": 1, "lower_c": 2,
            "upper_a": 3, "upper_b": 4, "upper_c": 5,
            "material_id": "adhesive"
        }],
        "load_steps": 1,
        "tolerance": 1.0e-11
    });
    let result = run_solve_operator("solve.cohesive_interface_mesh_3d", payload.clone())
        .expect("workflow solver should run");

    assert_eq!(result["converged"], serde_json::json!(true));
    assert!((result["elements"][0]["local_traction"][2].as_f64().unwrap() - 5.0).abs() < 1.0e-10);
    assert_eq!(
        result["_solver_provenance"]["operator_id"],
        "solve.cohesive_interface_mesh_3d"
    );
    assert!(supported_workflow_operator_ids().contains(&"solve.cohesive_interface_mesh_3d"));

    let request: SolveCohesiveInterfaceMesh3dRequest =
        serde_json::from_value(payload).expect("payload should decode");
    let analysis = solve(EngineSolveRequest::CohesiveInterfaceMesh3d(request))
        .expect("engine solve should run");
    let nodes = chunk_result(
        &analysis,
        &ResultChunkRequest {
            kind: ResultChunkKind::Nodes,
            offset: 1,
            limit: 2,
        },
    )
    .expect("node chunk should encode");
    let elements = chunk_result(
        &analysis,
        &ResultChunkRequest {
            kind: ResultChunkKind::Elements,
            offset: 0,
            limit: 1,
        },
    )
    .expect("element chunk should encode");
    assert_eq!(nodes.total, 6);
    assert_eq!(nodes.returned, 2);
    assert_eq!(elements.total, 1);
    assert_eq!(elements.returned, 1);
}
