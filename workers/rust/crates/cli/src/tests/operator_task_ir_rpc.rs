use super::*;
use crate::operator_task_runtime::{
    OPERATOR_PACKAGE_RUNTIME_NOT_ATTACHED, OPERATOR_TASK_BLOCKED_STAGE, OPERATOR_TASK_MODE_EXECUTE,
    OPERATOR_TASK_MODE_PREFLIGHT, OPERATOR_TASK_STATUS_VERIFIED_PENDING,
    OperatorPackageRuntimeAttachment, OperatorPackageRuntimeBinding, run_operator_task_ir,
    run_operator_task_ir_with_runtime,
};
use kyuubiki_protocol::compute_operator_task_digest;

mod agent_native_material;

fn golden_operator_task_ir() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.operator-task-ir.json"
    ))
    .expect("operator TaskIR example should decode")
}

pub(super) fn thermal_shock_operator_task_ir() -> serde_json::Value {
    let mut task = serde_json::json!({
        "schema_version": "kyuubiki.operator-task-ir/v1",
        "task_id": "agent-native-thermal-shock",
        "operator": {
            "id": "transform.evaluate_material_thermal_shock",
            "family": "material_thermal_shock",
            "kind": "transform"
        },
        "descriptor_authoring": {
            "schema_version": "kyuubiki.operator-descriptor-authoring/v1",
            "mode": "rust_native",
            "runtime": "rust",
            "source": "operator_task_ir_rpc_test",
            "hot_reloadable": false,
            "execution_language": "language_neutral"
        },
        "node": {},
        "input_artifact": {
            "candidates": {
                "alloy": {
                    "temperature_delta": 160.0,
                    "thermal_expansion": 1.2e-5,
                    "youngs_modulus": 70000000000.0,
                    "poisson_ratio": 0.33,
                    "yield_strength": 320000000.0
                },
                "ceramic": {
                    "temperature_delta": 160.0,
                    "thermal_expansion": 8.0e-6,
                    "youngs_modulus": 300000000000.0,
                    "poisson_ratio": 0.22,
                    "tensile_strength": 180000000.0,
                    "fracture_toughness": 3000000.0,
                    "flaw_size": 0.001
                }
            }
        },
        "config": { "constraint_factor": 0.7 },
        "execution_program": {
            "schema_version": "kyuubiki.operator-execution-program/v1",
            "program_id": "transform.evaluate_material_thermal_shock",
            "program_family": "material_thermal_shock",
            "program_kind": "transform",
            "operator_category_id": null,
            "package_ref": null,
            "package_version": "library-managed",
            "package_integrity": null,
            "runtime_protocol": "kyuubiki.operator-execution/v1",
            "abi": {
                "kind": "operator_task",
                "input_encoding": "json",
                "output_encoding": "json"
            },
            "entrypoint": {
                "kind": "operator_id",
                "name": "transform.evaluate_material_thermal_shock",
                "operator_kind": "transform"
            },
            "bindings": {
                "input_artifact": "task.input_artifact",
                "config": "task.config",
                "output_artifact": "task.output_artifact"
            },
            "node_binding": { "node_id": null, "input_ports": [], "output_ports": [] }
        },
        "dataset_contract": {},
        "orchestration_context": {},
        "runtime_hints": {
            "authority_mode": "central_operator_library",
            "execution_mode": "orchestra_fetch",
            "cache_scope": "job",
            "agent_fetchable": true,
            "operator_kind": "transform"
        }
    });
    let digest = compute_operator_task_digest(&task).expect("thermal shock task should digest");
    task["integrity"] = serde_json::json!({ "task_digest": digest });
    task
}

pub(super) fn transform_operator_task_ir(
    task_id: &str,
    operator_id: &str,
    family: &str,
    input_artifact: serde_json::Value,
    config: serde_json::Value,
) -> serde_json::Value {
    let mut task = serde_json::json!({
        "schema_version": "kyuubiki.operator-task-ir/v1",
        "task_id": task_id,
        "operator": {
            "id": operator_id,
            "family": family,
            "kind": "transform"
        },
        "descriptor_authoring": {
            "schema_version": "kyuubiki.operator-descriptor-authoring/v1",
            "mode": "rust_native",
            "runtime": "rust",
            "source": "operator_task_ir_rpc_test",
            "hot_reloadable": false,
            "execution_language": "language_neutral"
        },
        "node": {},
        "input_artifact": input_artifact,
        "config": config,
        "execution_program": {
            "schema_version": "kyuubiki.operator-execution-program/v1",
            "program_id": operator_id,
            "program_family": family,
            "program_kind": "transform",
            "operator_category_id": null,
            "package_ref": null,
            "package_version": "library-managed",
            "package_integrity": null,
            "runtime_protocol": "kyuubiki.operator-execution/v1",
            "abi": {
                "kind": "operator_task",
                "input_encoding": "json",
                "output_encoding": "json"
            },
            "entrypoint": {
                "kind": "operator_id",
                "name": operator_id,
                "operator_kind": "transform"
            },
            "bindings": {
                "input_artifact": "task.input_artifact",
                "config": "task.config",
                "output_artifact": "task.output_artifact"
            },
            "node_binding": { "node_id": null, "input_ports": [], "output_ports": [] }
        },
        "dataset_contract": {},
        "orchestration_context": {},
        "runtime_hints": {
            "authority_mode": "central_operator_library",
            "execution_mode": "orchestra_fetch",
            "cache_scope": "job",
            "agent_fetchable": true,
            "operator_kind": "transform"
        }
    });
    let digest = compute_operator_task_digest(&task).expect("transform task should digest");
    task["integrity"] = serde_json::json!({ "task_digest": digest });
    task
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
fn operator_task_runtime_rejects_digest_valid_inconsistent_package_mirrors() {
    let mut task_ir = golden_operator_task_ir();
    task_ir["runtime_hints"]["package_ref"] =
        serde_json::json!("orchestra://operator-package/wrong");
    let digest = compute_operator_task_digest(&task_ir).expect("changed task should digest");
    task_ir["integrity"] = serde_json::json!({ "task_digest": digest });

    let error = run_operator_task_ir(&serde_json::json!({
        "task_ir": task_ir
    }))
    .expect_err("runtime should reject package mirror mismatch after digest verification");

    assert_eq!(error.code, "operator_task_mirror_mismatch");
    assert!(
        error
            .message
            .contains("runtime_hints.package_ref must match execution_program.package_ref")
    );
}

#[test]
fn operator_task_runtime_reports_missing_digest() {
    let mut task_ir = golden_operator_task_ir();
    task_ir["integrity"] = serde_json::json!({});

    let error = run_operator_task_ir(&serde_json::json!({
        "task_ir": task_ir
    }))
    .expect_err("runtime should reject missing task digest");

    assert_eq!(error.code, "operator_task_digest_missing");
    assert_eq!(error.message, "missing operator task digest");
}

#[test]
fn operator_task_runtime_reports_execution_abi_mismatch() {
    let mut task_ir = golden_operator_task_ir();
    task_ir["execution_program"]["abi"]["kind"] = serde_json::json!("solver_rpc");
    let digest = compute_operator_task_digest(&task_ir).expect("changed task should digest");
    task_ir["integrity"] = serde_json::json!({ "task_digest": digest });

    let error = run_operator_task_ir(&serde_json::json!({
        "task_ir": task_ir
    }))
    .expect_err("runtime should reject inconsistent ABI after digest verification");

    assert_eq!(error.code, "operator_task_execution_abi_mismatch");
    assert!(
        error
            .message
            .contains("inconsistent runtime protocol, abi, or entrypoint")
    );
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
        "operator_task_digest_mismatch"
    );
}
