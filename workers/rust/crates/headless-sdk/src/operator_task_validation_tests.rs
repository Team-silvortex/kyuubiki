use crate::{validate_operator_task_for_agent, validate_operator_task_for_builtin_agent};
use kyuubiki_protocol::{SolverExecutionCapability, compute_operator_task_digest};
use serde_json::{Value, json};

#[test]
fn validates_solver_task_against_builtin_agent_capability() {
    let task = solver_task_fixture(false);

    let report = validate_operator_task_for_builtin_agent(&task);

    assert!(report.ok);
    assert!(report.digest_ok);
    assert_eq!(report.status, "accepted");
    assert_eq!(
        report
            .capability
            .as_ref()
            .map(|item| item.dispatch_route.as_str()),
        Some("solver_rpc")
    );
}

#[test]
fn rejects_tampered_task_before_agent_dispatch() {
    let task = solver_task_fixture(true);

    let report = validate_operator_task_for_builtin_agent(&task);

    assert!(!report.ok);
    assert!(!report.digest_ok);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("digest mismatch"))
    );
}

#[test]
fn rejects_task_not_supported_by_agent_capability() {
    let task = solver_task_fixture(false);
    let mut capability = SolverExecutionCapability::agent_builtin();
    capability.operator_ids = vec!["solve.electrostatic.plane_triangle_2d".to_string()];

    let report = validate_operator_task_for_agent(&task, &capability);

    assert!(!report.ok);
    assert!(report.digest_ok);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("operator_id"))
    );
}

fn solver_task_fixture(tampered: bool) -> Value {
    let heat = if tampered { 12.0 } else { 10.0 };
    let mut task = json!({
        "schema_version": "kyuubiki.operator-task-ir/v1",
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
                { "id": 1, "x": 1.0, "heat": heat }
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
    });
    let digest = compute_operator_task_digest(&task).expect("digest");
    task["integrity"] = json!({ "task_digest": digest });
    if tampered {
        task["input_artifact"]["nodes"][1]["heat"] = json!(14.0);
    }
    task
}
