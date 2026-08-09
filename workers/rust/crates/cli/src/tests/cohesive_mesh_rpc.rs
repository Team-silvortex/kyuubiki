use super::*;
use kyuubiki_protocol::{
    CohesiveInterface2dMaterialInput, CohesiveInterfaceMesh2dElementInput,
    CohesiveInterfaceMesh2dMaterialInput, CohesiveInterfaceMesh2dNodeInput, PlaneQuadElementInput,
    PlaneTriangleElementInput, SolveCohesiveInterfaceMesh2dRequest,
};

#[test]
fn handles_cohesive_interface_mesh_2d_host_plane_coassembly_rpc() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-cohesive-host-plane".to_string(),
        method: RpcMethod::SolveCohesiveInterfaceMesh2d,
        params: serde_json::to_value(host_plane_request()).expect("params should serialize"),
    };
    let AgentReply::Stream(progress_frames, final_response) =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveCohesiveInterfaceMesh2dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("host plane coassembly result");
    assert!(result.converged);
    assert_eq!(result.host_plane_triangles.len(), 1);
    assert!((result.nodes[2].displacement[1] - 0.005).abs() < 1.0e-10);
    assert!((result.max_host_plane_stress - 5.0).abs() < 1.0e-10);
}

#[test]
fn handles_cohesive_interface_mesh_2d_host_plane_quad_coassembly_rpc() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-cohesive-host-plane-quad".to_string(),
        method: RpcMethod::SolveCohesiveInterfaceMesh2d,
        params: serde_json::to_value(host_plane_quad_request()).expect("params should serialize"),
    };
    let AgentReply::Stream(progress_frames, final_response) =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    assert_eq!(progress_frames.len(), 4);
    assert!(final_response.ok);
    let result: kyuubiki_protocol::SolveCohesiveInterfaceMesh2dResult =
        serde_json::from_value(final_response.result.expect("solver result"))
            .expect("host plane quad coassembly result");
    assert!(result.converged);
    assert_eq!(result.host_plane_quads.len(), 1);
    assert!((result.nodes[2].displacement[1] - 0.005).abs() < 1.0e-10);
    assert!((result.max_host_plane_stress - 5.0).abs() < 1.0e-10);
}

fn host_plane_request() -> SolveCohesiveInterfaceMesh2dRequest {
    SolveCohesiveInterfaceMesh2dRequest {
        id: "mesh.host-plane.rpc".to_string(),
        nodes: vec![
            node("lower-0", 0.0, 0.0, [true, true], None),
            node("lower-1", 1.0, 0.0, [true, true], None),
            node("upper-0", 0.0, 0.0, [true, false], None),
            node("upper-1", 1.0, 0.0, [true, false], None),
            node("driver", 0.5, 1.0, [true, true], Some([0.0, 0.015])),
        ],
        materials: vec![CohesiveInterfaceMesh2dMaterialInput {
            id: "adhesive".to_string(),
            properties: CohesiveInterface2dMaterialInput {
                normal_initial_stiffness: 1000.0,
                normal_compression_stiffness: 2000.0,
                normal_peak_traction: 10.0,
                normal_failure_separation: 0.05,
                shear_initial_stiffness: 500.0,
                shear_peak_traction: 5.0,
                shear_failure_separation: 0.05,
            },
        }],
        elements: vec![CohesiveInterfaceMesh2dElementInput {
            id: "interface-0".to_string(),
            lower_i: 0,
            lower_j: 1,
            upper_i: 2,
            upper_j: 3,
            thickness: 1.0,
            material_id: "adhesive".to_string(),
        }],
        connector_springs: vec![],
        host_trusses: vec![],
        host_plane_triangles: vec![PlaneTriangleElementInput {
            id: "host-plane-0".to_string(),
            node_i: 2,
            node_j: 3,
            node_k: 4,
            thickness: 2.0,
            youngs_modulus: 500.0,
            poisson_ratio: 0.0,
        }],
        host_plane_quads: vec![],
        load_steps: Some(3),
        control_history: None,
        max_iterations: Some(12),
        tolerance: Some(1.0e-11),
    }
}

fn host_plane_quad_request() -> SolveCohesiveInterfaceMesh2dRequest {
    SolveCohesiveInterfaceMesh2dRequest {
        id: "mesh.host-plane-quad.rpc".to_string(),
        nodes: vec![
            node("lower-0", 0.0, 0.0, [true, true], None),
            node("lower-1", 1.0, 0.0, [true, true], None),
            node("upper-0", 0.0, 0.0, [true, false], None),
            node("upper-1", 1.0, 0.0, [true, false], None),
            node("driver-right", 1.0, 1.0, [true, true], Some([0.0, 0.015])),
            node("driver-left", 0.0, 1.0, [true, true], Some([0.0, 0.015])),
        ],
        materials: vec![CohesiveInterfaceMesh2dMaterialInput {
            id: "adhesive".to_string(),
            properties: CohesiveInterface2dMaterialInput {
                normal_initial_stiffness: 1000.0,
                normal_compression_stiffness: 2000.0,
                normal_peak_traction: 10.0,
                normal_failure_separation: 0.05,
                shear_initial_stiffness: 500.0,
                shear_peak_traction: 5.0,
                shear_failure_separation: 0.05,
            },
        }],
        elements: vec![CohesiveInterfaceMesh2dElementInput {
            id: "interface-0".to_string(),
            lower_i: 0,
            lower_j: 1,
            upper_i: 2,
            upper_j: 3,
            thickness: 1.0,
            material_id: "adhesive".to_string(),
        }],
        connector_springs: vec![],
        host_trusses: vec![],
        host_plane_triangles: vec![],
        host_plane_quads: vec![PlaneQuadElementInput {
            id: "host-plane-quad-0".to_string(),
            node_i: 2,
            node_j: 3,
            node_k: 4,
            node_l: 5,
            thickness: 1.0,
            youngs_modulus: 500.0,
            poisson_ratio: 0.0,
        }],
        load_steps: Some(3),
        control_history: None,
        max_iterations: Some(12),
        tolerance: Some(1.0e-11),
    }
}

fn node(
    id: &str,
    x: f64,
    y: f64,
    fixed: [bool; 2],
    prescribed_displacement: Option<[f64; 2]>,
) -> CohesiveInterfaceMesh2dNodeInput {
    CohesiveInterfaceMesh2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fixed,
        prescribed_displacement,
        load: [0.0, 0.0],
    }
}
