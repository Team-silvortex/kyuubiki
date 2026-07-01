use super::*;
use crate::operator_task_runtime::{
    OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED, OPERATOR_TASK_BLOCKED_STAGE, OPERATOR_TASK_MODE_EXECUTE,
    OPERATOR_TASK_MODE_PREFLIGHT, OPERATOR_TASK_STATUS_VERIFIED_PENDING,
    OperatorPackageRuntimeAttachment, OperatorPackageRuntimeBinding, run_operator_task_ir,
    run_operator_task_ir_with_runtime,
};

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
        OPERATOR_TASK_STATUS_VERIFIED_PENDING
    );
    assert_eq!(
        result["execution_runtime_status"],
        OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED
    );
    assert_eq!(result["requested_mode"], OPERATOR_TASK_MODE_PREFLIGHT);
    assert_eq!(result["blocked_stage"], OPERATOR_TASK_BLOCKED_STAGE);
    assert_eq!(result["operator_package_runtime_ready"], false);
    assert_eq!(
        result["package_fetch_request"]["schema_version"],
        "kyuubiki.operator-package-fetch-request/v1"
    );
    assert_eq!(
        result["package_fetch_request"]["request_status"],
        "blocked_runtime_not_attached"
    );
    assert_eq!(
        result["package_fetch_request"]["target"]["runtime_attached"],
        false
    );
    assert_eq!(
        result["operator_package_runtime"]["expected_host"],
        "kyuubiki-engine.operator-sdk-host/v1"
    );
    assert_eq!(
        result["operator_package_runtime"]["required_interfaces"][0],
        "resolve_package"
    );
    assert_eq!(
        result["operator_package_runtime"]["trust_policy"]["allowed_runtimes"][0],
        "rust_crate"
    );
    assert_eq!(
        result["operator_package_runtime"]["trust_policy"]["allow_absolute_entrypoints"],
        false
    );
    assert_eq!(result["execution_plan"][0]["stage"], "verify_digest");
    assert_eq!(result["execution_plan"][0]["status"], "complete");
    assert_eq!(
        result["execution_plan"][2]["stage"],
        OPERATOR_TASK_BLOCKED_STAGE
    );
    assert_eq!(result["execution_plan"][2]["status"], "blocked");
    assert_eq!(
        result["execution_plan"][2]["reason"],
        OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED
    );
    assert_eq!(
        result["execution_plan"][2]["requested_mode"],
        OPERATOR_TASK_MODE_PREFLIGHT
    );
    assert_eq!(result["operator_id"], "transform.fixture");
    assert_eq!(result["program_id"], "transform.fixture");
    assert_eq!(result["runtime_protocol"], "kyuubiki.operator-execution/v1");
}

#[test]
fn operator_task_runtime_accepts_explicit_execute_mode_as_blocked_dispatch() {
    let result = run_operator_task_ir(&serde_json::json!({
        "mode": OPERATOR_TASK_MODE_EXECUTE,
        "task_ir": golden_operator_task_ir()
    }))
    .expect("runtime should validate explicit execute mode before package dispatch");

    assert_eq!(result["requested_mode"], OPERATOR_TASK_MODE_EXECUTE);
    assert_eq!(result["blocked_stage"], OPERATOR_TASK_BLOCKED_STAGE);
    assert_eq!(result["operator_package_runtime_ready"], false);
    assert_eq!(result["operator_package_runtime"]["status"], "not_attached");
    assert_eq!(
        result["operator_package_runtime"]["fetch_policy"]["agent_fetchable"],
        true
    );
    assert_eq!(
        result["execution_plan"][2]["requested_mode"],
        OPERATOR_TASK_MODE_EXECUTE
    );
    assert_eq!(
        result["execution_runtime_status"],
        OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED
    );
}

#[test]
fn operator_task_runtime_can_report_attached_package_host_readiness() {
    let result = run_operator_task_ir_with_runtime(
        &serde_json::json!({
            "mode": OPERATOR_TASK_MODE_EXECUTE,
            "task_ir": golden_operator_task_ir()
        }),
        OperatorPackageRuntimeBinding::Attached(OperatorPackageRuntimeAttachment {
            host_id: "agent-local/operator-host".to_string(),
            packages_root: "/tmp/kyuubiki/operator-packages".to_string(),
            activated_package_count: 2,
        }),
    )
    .expect("attached package runtime should still validate TaskIR first");

    assert_eq!(result["operator_package_runtime_ready"], true);
    assert!(result["blocked_stage"].is_null());
    assert_eq!(result["next_stage"], OPERATOR_TASK_BLOCKED_STAGE);
    assert_eq!(result["operator_package_runtime"]["status"], "attached");
    assert_eq!(
        result["execution_runtime_status"],
        "operator_package_runtime_attached_pending_package_fetch"
    );
    assert_eq!(result["execution_plan"][2]["status"], "pending");
    assert_eq!(
        result["execution_plan"][2]["reason"],
        "operator_package_runtime_ready_for_fetch"
    );
    assert_eq!(
        result["package_fetch_request"]["request_status"],
        "ready_to_resolve"
    );
    assert_eq!(
        result["package_fetch_request"]["target"]["host_id"],
        "agent-local/operator-host"
    );
    assert_eq!(
        result["package_fetch_request"]["target"]["packages_root"],
        "/tmp/kyuubiki/operator-packages"
    );
    assert_eq!(
        result["operator_package_runtime"]["host_id"],
        "agent-local/operator-host"
    );
    assert_eq!(
        result["operator_package_runtime"]["packages_root"],
        "/tmp/kyuubiki/operator-packages"
    );
    assert_eq!(
        result["operator_package_runtime"]["activated_package_count"],
        2
    );
}

#[test]
fn operator_task_runtime_rejects_missing_task_ir_before_rpc_wrapping() {
    let error = run_operator_task_ir(&serde_json::json!({}))
        .expect_err("runtime should reject missing task_ir");

    assert_eq!(error.code, "invalid_params");
    assert_eq!(error.message, "missing task_ir");
}

#[test]
fn operator_task_runtime_rejects_unknown_mode() {
    let error = run_operator_task_ir(&serde_json::json!({
        "mode": "background",
        "task_ir": golden_operator_task_ir()
    }))
    .expect_err("runtime should reject unsupported modes");

    assert_eq!(error.code, "invalid_params");
    assert_eq!(error.message, "unsupported operator task mode: background");
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
