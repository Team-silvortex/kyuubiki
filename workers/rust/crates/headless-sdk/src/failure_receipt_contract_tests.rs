use crate::execution_observability::failure_preview;
use crate::{HeadlessExecutionBatchStep, HeadlessRisk, build_preflight_failure_report};
use serde_json::{Value, json};

fn schema() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../schemas/headless-failure-receipt.schema.json"
    ))
    .unwrap()
}

fn assert_contract(receipt: &Value) {
    let schema = schema();
    let fields = receipt.as_object().unwrap();
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(fields.len(), schema["required"].as_array().unwrap().len());
    for key in schema["required"].as_array().unwrap() {
        assert!(fields.contains_key(key.as_str().unwrap()));
    }
    assert_eq!(
        receipt["schema_version"],
        schema["properties"]["schema_version"]["const"]
    );
    for key in ["category", "stage", "retry_strategy"] {
        assert!(
            schema["properties"][key]["enum"]
                .as_array()
                .unwrap()
                .contains(&receipt[key]),
            "undeclared {key}: {receipt}"
        );
    }
    assert!(
        receipt["step_index"].as_u64().unwrap()
            >= schema["properties"]["step_index"]["minimum"]
                .as_u64()
                .unwrap()
    );
    if receipt["retryable"] == false {
        assert_eq!(receipt["retry_strategy"], "none");
    }
}

#[test]
fn official_runtime_failure_categories_and_stages_are_declared_in_the_public_schema() {
    for (action, message) in [
        ("job_wait", "timed out waiting for job owned"),
        ("job_wait", "agent_queue_timeout"),
        (
            "job_wait",
            "invalid_params: failed to decode model artifact",
        ),
        ("job_wait", "cancelled"),
        (
            "project_create",
            "service request failed 403: /api/v1/projects: denied",
        ),
        ("project_create", "failed to connect"),
        (
            "project_create",
            "service_request_outcome_unknown: failed to read",
        ),
        (
            "result_fetch",
            "job_receipt_invalid: identity does not match",
        ),
        ("result_fetch", "job_result_unavailable: not completed"),
        ("result_fetch", "result unavailable"),
        (
            "result_fetch",
            "service_response_limit_exceeded: oversized response",
        ),
        ("solve_bar_1d", "model_artifact_limit_exceeded"),
        ("solve_bar_1d", "frontend_proxy_artifact_limit"),
        ("solve_bar_1d", "endpoint not deployed (404)"),
        ("solve_bar_1d", "unsupported action"),
        ("solve_bar_1d", "computation failed"),
        (
            "operator_task_execute",
            "operator task transport failed (execution outcome unknown; inspect the task before explicit rerun): response lost",
        ),
    ] {
        assert_contract(&failure_preview(1, action, message.into())["failure_receipt"]);
    }
    let step = HeadlessExecutionBatchStep {
        index: 2,
        action: "project_create".into(),
        risk: HeadlessRisk::Normal,
        payload: json!({"name":"{{steps.1.result.job_id}}"}),
    };
    let failure =
        crate::workflow_bindings::binding_failure_step(&step, "missing source output".into());
    assert_contract(&failure.result_preview["failure_receipt"]);
}

#[test]
fn official_preflight_receipts_use_declared_zero_index_stages() {
    for stage in [
        "batch_validation",
        "document_decode",
        "executor_preflight",
        "endpoint_configuration",
        "parameter_patch",
        "research_round",
        "material_report_validation",
        "command_validation",
    ] {
        let report = build_preflight_failure_report(
            None,
            "owned",
            "execute:service",
            "document_validation",
            stage,
            "invalid input",
            &[],
        );
        let failure = serde_json::to_value(report.execution_summary.failure.unwrap()).unwrap();
        assert_contract(&failure);
        assert_eq!(failure["action"], "run_preflight");
        assert_eq!(failure["step_index"], 0);
    }
    let schema = schema();
    let zero_gate = schema["allOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["if"]["properties"]["step_index"]["const"] == 0)
        .expect("step zero must be reserved for preflight");
    assert_eq!(
        zero_gate["then"]["properties"]["action"]["const"],
        "run_preflight"
    );
    assert_eq!(
        zero_gate["then"]["properties"]["category"]["const"],
        "contract_failure"
    );
    assert_eq!(zero_gate["then"]["properties"]["retryable"]["const"], false);
}

#[test]
fn unknown_outcomes_and_invalid_bindings_cannot_acquire_retry_permission_in_schema() {
    let schema = schema();
    let policy = schema["allOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| rule["if"]["properties"]["category"]["enum"].is_array())
        .expect("non-replayable categories need a schema gate");
    for category in [
        "service_response_limit_exceeded",
        "service_request_outcome_unknown",
        "operator_task_outcome_unknown",
        "job_receipt_invalid",
        "job_result_unavailable",
        "binding_resolution",
    ] {
        assert!(
            policy["if"]["properties"]["category"]["enum"]
                .as_array()
                .unwrap()
                .contains(&json!(category))
        );
    }
    assert_eq!(policy["then"]["properties"]["retryable"]["const"], false);
    assert_eq!(
        policy["then"]["properties"]["retry_strategy"]["const"],
        "none"
    );
}

#[test]
fn registered_direct_solver_previews_supply_job_bindings_but_not_computation_results() {
    let contracts = crate::all_action_contracts()
        .iter()
        .filter(|contract| {
            matches!(contract.category, "solve" | "material_solve")
                && contract.output_keys.contains(&"job_id")
                && !contract.output_keys.contains(&"result")
        })
        .collect::<Vec<_>>();
    assert!(
        contracts
            .iter()
            .any(|contract| contract.id == "solve_composite_thermo_electric_panel")
    );
    for contract in contracts {
        let preview = crate::run::build_result_preview(contract.id, 7, &json!({}));
        assert_eq!(preview["job_id"], "job_007", "{}", contract.id);
        assert_eq!(preview["preview_only"], true);
        assert!(preview.get("result").is_none());
    }
}
