use super::prelude::*;
use serde_json::{Value, json};

#[test]
fn solver_execution_capability_accepts_agent_builtin_solver_task() {
    let task = solver_task_fixture();
    let capability = SolverExecutionCapability::agent_builtin();

    let report =
        check_operator_task_execution_capability(&task, &capability).expect("report should build");

    assert!(report.accepted);
    assert_eq!(report.program_kind, "solver");
    assert_eq!(report.runtime_protocol, "kyuubiki.solver-rpc/v1");
    assert_eq!(report.dispatch_route, "solver_rpc");
    assert!(report.rejection_reasons.is_empty());
}

#[test]
fn solver_execution_capability_rejects_unsupported_solver_method() {
    let task = solver_task_fixture();
    let mut capability = SolverExecutionCapability::agent_builtin();
    capability.operator_ids = vec!["solve.thermal.plane_triangle_2d".to_string()];

    let report =
        check_operator_task_execution_capability(&task, &capability).expect("report should build");

    assert!(!report.accepted);
    assert!(
        report
            .rejection_reasons
            .iter()
            .any(|reason| reason.contains("operator_id"))
    );
}

#[test]
fn solver_execution_capability_rejects_fetch_when_agent_is_offline_only() {
    let mut task = solver_task_fixture();
    task["runtime_hints"]["execution_mode"] = json!("orchestra_fetch");
    task["runtime_hints"]["authority_mode"] = json!("central_operator_library");
    task["runtime_hints"]["agent_fetchable"] = json!(true);
    task["runtime_hints"]["cache_scope"] = json!("job");
    let capability = SolverExecutionCapability::agent_builtin();

    let report =
        check_operator_task_execution_capability(&task, &capability).expect("report should build");

    assert!(!report.accepted);
    assert!(
        report
            .rejection_reasons
            .iter()
            .any(|reason| reason.contains("package_fetch_required"))
    );
}

fn solver_task_fixture() -> Value {
    json!({
        "schema_version": OPERATOR_TASK_IR_SCHEMA,
        "task_id": "solver-fixture-task",
        "operator": {
            "id": "solve.thermal.bar_1d",
            "family": "thermal",
            "kind": "solver"
        },
        "descriptor_authoring": {
            "schema_version": "kyuubiki.operator-descriptor-authoring/v1",
            "mode": "rust_native",
            "runtime": "rust",
            "source": "fixture",
            "hot_reloadable": false,
            "execution_language": "language_neutral"
        },
        "node": {},
        "input_artifact": {
            "nodes": [
                { "id": 0, "x": 0.0, "temperature": 300.0, "fixed": true },
                { "id": 1, "x": 1.0, "heat": 10.0 }
            ],
            "elements": [
                { "id": 0, "n1": 0, "n2": 1, "conductivity": 5.0, "area": 1.0 }
            ]
        },
        "config": {},
        "execution_program": {
            "schema_version": "kyuubiki.operator-execution-program/v1",
            "program_id": "solve.thermal.bar_1d",
            "program_family": "thermal",
            "program_kind": "solver",
            "runtime_protocol": "kyuubiki.solver-rpc/v1",
            "abi": {
                "kind": "solver_rpc",
                "input_encoding": "json",
                "output_encoding": "json"
            },
            "entrypoint": {
                "kind": "solver_method",
                "name": "solve.thermal.bar_1d",
                "operator_kind": "solver"
            },
            "bindings": {
                "input_artifact": "task.input_artifact",
                "config": "task.config",
                "output_artifact": "task.output_artifact"
            }
        },
        "dataset_contract": {},
        "orchestration_context": {},
        "runtime_hints": {
            "authority_mode": "agent_local",
            "execution_mode": "agent_native",
            "placement_tags": ["thermal", "solver"],
            "required_capabilities": ["kyuubiki.solver-rpc/v1"],
            "cache_scope": "none",
            "agent_fetchable": false,
            "operator_kind": "solver"
        }
    })
}
