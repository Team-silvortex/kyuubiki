use super::*;
use kyuubiki_protocol::{
    CohesiveInterface3dMaterialInput, CohesiveInterfaceMesh3dElementInput,
    CohesiveInterfaceMesh3dMaterialInput, CohesiveInterfaceMesh3dNodeInput,
    SolveCohesiveInterfaceMesh3dRequest, SolveCohesiveInterfaceMesh3dResult,
};

#[test]
fn handles_cohesive_interface_mesh_3d_rpc_requests() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-cohesive-mesh-3d".to_string(),
        method: RpcMethod::SolveCohesiveInterfaceMesh3d,
        params: serde_json::to_value(request()).expect("params should serialize"),
    };
    let AgentReply::Stream(progress_frames, final_response) =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    assert!(!progress_frames.is_empty());
    assert!(final_response.ok);
    let result: SolveCohesiveInterfaceMesh3dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("cohesive mesh 3d result");
    assert!(result.converged);
    assert!((result.elements[0].local_traction[2] - 5.0).abs() < 1.0e-10);
    assert_eq!(result.linear_solver_methods, ["symmetric_band_cholesky"]);
}

fn request() -> SolveCohesiveInterfaceMesh3dRequest {
    let mut nodes = vec![
        node("lower-a", 0.0, 0.0, [true; 3]),
        node("lower-b", 1.0, 0.0, [true; 3]),
        node("lower-c", 0.0, 1.0, [true; 3]),
        node("upper-a", 0.0, 0.0, [true, true, false]),
        node("upper-b", 1.0, 0.0, [true, true, false]),
        node("upper-c", 0.0, 1.0, [true, true, false]),
    ];
    for node in &mut nodes[3..] {
        node.load[2] = 5.0 * 0.5 / 3.0;
    }
    SolveCohesiveInterfaceMesh3dRequest {
        id: "mesh.rpc.3d".to_string(),
        nodes,
        materials: vec![CohesiveInterfaceMesh3dMaterialInput {
            id: "adhesive".to_string(),
            properties: CohesiveInterface3dMaterialInput {
                normal_initial_stiffness: 1000.0,
                normal_compression_stiffness: 2000.0,
                normal_peak_traction: 100.0,
                normal_failure_separation: 1.0,
                shear_initial_stiffness: 500.0,
                shear_peak_traction: 50.0,
                shear_failure_separation: 1.0,
            },
        }],
        elements: vec![CohesiveInterfaceMesh3dElementInput {
            id: "interface-0".to_string(),
            lower_a: 0,
            lower_b: 1,
            lower_c: 2,
            upper_a: 3,
            upper_b: 4,
            upper_c: 5,
            material_id: "adhesive".to_string(),
        }],
        host_tetrahedra: Vec::new(),
        load_steps: Some(1),
        control_history: None,
        max_iterations: Some(12),
        tolerance: Some(1.0e-11),
    }
}

fn node(id: &str, x: f64, y: f64, fixed: [bool; 3]) -> CohesiveInterfaceMesh3dNodeInput {
    CohesiveInterfaceMesh3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z: 0.0,
        fixed,
        prescribed_displacement: None,
        load: [0.0; 3],
    }
}
