use crate::{
    BucklingBeam1dElementInput, BucklingBeam1dNodeInput, Frame2dElementInput, Frame2dNodeInput,
    RPC_VERSION, RpcMethod, RpcRequest, SolveBucklingBeam1dRequest, SolveBucklingFrame2dRequest,
    SolveFrame2dRequest,
};

#[test]
fn buckling_beam_rpc_round_trip_preserves_reference_load_pattern() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "buckling-column".to_string(),
        method: RpcMethod::SolveBucklingBeam1d,
        params: serde_json::to_value(SolveBucklingBeam1dRequest {
            nodes: vec![node("a", 0.0, true), node("b", 2.0, true)],
            elements: vec![BucklingBeam1dElementInput {
                id: "column".to_string(),
                node_i: 0,
                node_j: 1,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 8.0e-6,
                reference_compressive_force: 100_000.0,
            }],
            mode_count: Some(1),
        })
        .expect("buckling request should serialize"),
    };
    let encoded = serde_json::to_string(&request).expect("rpc should serialize");
    let decoded: RpcRequest = serde_json::from_str(&encoded).expect("rpc should decode");
    let params: SolveBucklingBeam1dRequest =
        serde_json::from_value(decoded.params).expect("buckling params should decode");

    assert_eq!(decoded.method, RpcMethod::SolveBucklingBeam1d);
    assert_eq!(params.elements[0].reference_compressive_force, 100_000.0);
    assert_eq!(params.mode_count, Some(1));
}

#[test]
fn buckling_frame_rpc_round_trip_preserves_static_preload_model() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "buckling-frame".to_string(),
        method: RpcMethod::SolveBucklingFrame2d,
        params: serde_json::to_value(SolveBucklingFrame2dRequest {
            frame: SolveFrame2dRequest {
                nodes: vec![frame_node("base", 0.0, true), frame_node("top", 2.0, false)],
                elements: vec![Frame2dElementInput {
                    id: "column".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 0.01,
                    youngs_modulus: 210.0e9,
                    moment_of_inertia: 8.0e-6,
                    section_modulus: 1.0e-4,
                }],
            },
            mode_count: Some(2),
        })
        .expect("buckling frame request should serialize"),
    };
    let encoded = serde_json::to_string(&request).expect("rpc should serialize");
    let decoded: RpcRequest = serde_json::from_str(&encoded).expect("rpc should decode");
    let params: SolveBucklingFrame2dRequest =
        serde_json::from_value(decoded.params).expect("buckling frame params should decode");

    assert_eq!(decoded.method, RpcMethod::SolveBucklingFrame2d);
    assert_eq!(params.frame.nodes[1].load_y, -100_000.0);
    assert_eq!(params.mode_count, Some(2));
}

fn node(id: &str, x: f64, fix_y: bool) -> BucklingBeam1dNodeInput {
    BucklingBeam1dNodeInput {
        id: id.to_string(),
        x,
        fix_y,
        fix_rz: false,
    }
}

fn frame_node(id: &str, y: f64, base: bool) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.to_string(),
        x: 0.0,
        y,
        fix_x: true,
        fix_y: base,
        fix_rz: false,
        load_x: 0.0,
        load_y: if base { 0.0 } else { -100_000.0 },
        moment_z: 0.0,
    }
}
