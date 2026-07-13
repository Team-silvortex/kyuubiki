pub(crate) const AGENT_HEADLESS_BRIDGE_SCHEMA_VERSION: &str = "kyuubiki.agent-headless-bridge/v1";

pub(crate) fn agent_headless_bridge_manifest() -> serde_json::Value {
    serde_json::json!({
        "schema_version": AGENT_HEADLESS_BRIDGE_SCHEMA_VERSION,
        "bridge_owner": "runtime_agent_cli",
        "headless_entrypoints": [
            {
                "rpc_method": "RunOperatorTaskIr",
                "task_format": "kyuubiki.operator-task-ir/v1",
                "modes": ["preflight", "execute"],
                "result_receipts": [
                    "kyuubiki.agent-operator-task-validation/v1",
                    "kyuubiki.agent-operator-task-provenance/v1"
                ],
                "package_runtime_gate": "operator_package_runtime",
                "blocked_reason": "operator_package_runtime_not_yet_attached"
            }
        ],
        "workflow_composition": {
            "accepts_language_neutral_task_ir": true,
            "dispatch_contract": "kyuubiki.operator-execution/v1",
            "supports_agent_native_builtins": true,
            "supports_package_fetch_request": true
        },
        "sdk_contract": {
            "control_plane_independent": true,
            "requires_rpc_transport": true,
            "stable_descriptor_field": "headless_bridge"
        }
    })
}
