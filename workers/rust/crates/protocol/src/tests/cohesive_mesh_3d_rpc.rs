use crate::{RPC_VERSION, RpcMethod, RpcProtocolDescriptor, RpcRequest};

#[test]
fn cohesive_mesh_3d_rpc_method_round_trips_and_is_advertised() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "cohesive-mesh-3d".to_string(),
        method: RpcMethod::SolveCohesiveInterfaceMesh3d,
        params: serde_json::json!({ "id": "model" }),
    };
    let encoded = serde_json::to_string(&request).expect("request should encode");
    let decoded: RpcRequest = serde_json::from_str(&encoded).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveCohesiveInterfaceMesh3d);
    assert!(encoded.contains("solve_cohesive_interface_mesh_3d"));
    assert!(
        RpcProtocolDescriptor::solver_agent_default()
            .methods
            .contains(&RpcMethod::SolveCohesiveInterfaceMesh3d)
    );
}
