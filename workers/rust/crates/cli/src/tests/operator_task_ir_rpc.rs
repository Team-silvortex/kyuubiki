use super::*;

fn golden_operator_task_ir() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.operator-task-ir.json"
    ))
    .expect("operator TaskIR example should decode")
}

#[test]
fn describes_operator_task_ir_rpc_method() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-describe-task-ir".to_string(),
        method: RpcMethod::DescribeAgent,
        params: serde_json::json!({}),
    };

    let AgentReply::Stream(_, final_response) =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    let descriptor: AgentDescriptor =
        serde_json::from_value(final_response.result.expect("descriptor result"))
            .expect("agent descriptor should decode");

    assert!(
        descriptor
            .protocol
            .methods
            .contains(&RpcMethod::RunOperatorTaskIr)
    );
}

#[test]
fn handles_operator_task_ir_rpc_requests_as_agent_native_preflight() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-task-ir".to_string(),
        method: RpcMethod::RunOperatorTaskIr,
        params: serde_json::json!({ "task_ir": golden_operator_task_ir() }),
    };

    let AgentReply::Stream(progress_frames, final_response) =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    assert!(progress_frames.is_empty());
    assert!(final_response.ok);

    let result = final_response.result.expect("operator task result");
    assert_eq!(
        result["operator_task_ir_status"],
        "verified_pending_engine_execution"
    );
    assert_eq!(
        result["execution_runtime_status"],
        "operator_package_runtime_not_yet_attached"
    );
    assert_eq!(result["operator_id"], "transform.fixture");
    assert_eq!(result["program_id"], "transform.fixture");
    assert_eq!(result["runtime_protocol"], "kyuubiki.operator-execution/v1");
}

#[test]
fn rejects_operator_task_ir_rpc_requests_with_tampered_digest() {
    let mut task_ir = golden_operator_task_ir();
    task_ir["config"]["alpha"] = serde_json::json!(false);

    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-task-ir-tampered".to_string(),
        method: RpcMethod::RunOperatorTaskIr,
        params: serde_json::json!({ "task_ir": task_ir }),
    };

    let AgentReply::Stream(progress_frames, final_response) =
        handle_request_bytes(&serde_json::to_vec(&request).expect("request should serialize"));

    assert!(progress_frames.is_empty());
    assert!(!final_response.ok);
    assert_eq!(
        final_response.error.expect("operator task error").code,
        "operator_task_digest_invalid"
    );
}
