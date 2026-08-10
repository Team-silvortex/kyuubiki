use crate::{
    AGENT_SOLVER_QUALIFICATION_EXPECTED_TIP_DISPLACEMENT, AGENT_SOLVER_QUALIFICATION_SCHEMA,
    validate_agent_solver_qualification_report,
};
use serde_json::{Value, json};

const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn agent_solver_qualification_accepts_consistent_execution_and_recovery() {
    let report = qualification_report();
    let summary = validate_agent_solver_qualification_report(&report)
        .expect("consistent solver qualification should pass");

    assert_eq!(summary.operator_id, "solve.bar_1d");
    assert_eq!(summary.task_digest, DIGEST);
    assert_eq!(summary.initial_absolute_error, 0.0);
    assert_eq!(summary.recovery_absolute_error, 0.0);
    assert_eq!(summary.recent_failure_count, 2);
}

#[test]
fn agent_solver_qualification_rejects_claim_without_solver_admission() {
    let mut report = qualification_report();
    report["stages"]["initial_execution"]["solver_execution_capability"]["accepted"] = json!(false);
    report["stages"]["initial_execution"]["solver_execution_capability"]["rejection_reasons"] =
        json!(["unsupported"]);

    let errors = validate_agent_solver_qualification_report(&report)
        .expect_err("rejected capability must fail qualification");
    assert!(errors.iter().any(|error| error.contains("/accepted")));
    assert!(
        errors
            .iter()
            .any(|error| error.contains("/rejection_reasons"))
    );
}

#[test]
fn agent_solver_qualification_rejects_fabricated_numerical_result() {
    let mut report = qualification_report();
    report["stages"]["recovery_execution"]["result_assertion"]["actual"] = json!(0.5);
    report["stages"]["recovery_execution"]["result_assertion"]["absolute_error"] = json!(0.0);

    let errors = validate_agent_solver_qualification_report(&report)
        .expect_err("fabricated result must fail qualification");
    assert!(errors.iter().any(|error| error.contains("absolute_error")));
    assert!(
        errors
            .iter()
            .any(|error| error.contains("numerical tolerance"))
    );
}

#[test]
fn agent_solver_qualification_rejects_missing_recovery_evidence() {
    let mut report = qualification_report();
    report["stages"]["recovery_execution"]["provenance_receipt"]["lineage"]["preview_digest"] =
        json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    report["watchdog"]["recent_failures"] = json!([]);

    let errors = validate_agent_solver_qualification_report(&report)
        .expect_err("inconsistent recovery evidence must fail qualification");
    assert!(errors.iter().any(|error| error.contains("preview_digest")));
    assert!(
        errors
            .iter()
            .any(|error| error.contains("tamper rejection"))
    );
}

fn qualification_report() -> Value {
    json!({
        "schema_version": AGENT_SOLVER_QUALIFICATION_SCHEMA,
        "generated_at_unix_ms": 1,
        "status": "passed",
        "transport": "tcp_framed_json",
        "rpc_version": 1,
        "operator_id": "solve.bar_1d",
        "program_kind": "solver",
        "runtime_protocol": "kyuubiki.solver-rpc/v1",
        "task_digest": DIGEST,
        "stages": {
            "initial_execution": successful_execution(),
            "unsupported_solver_rejection": {
                "reason_code": "operator_task_solver_capability_rejected",
                "failure_receipt": {
                    "schema_version": "kyuubiki.agent-operator-task-failure/v1",
                    "failure_stage": "check_solver_capability",
                    "reason_code": "operator_task_solver_capability_rejected",
                    "operator_id": "solve.thermal_bar_1d",
                    "task_digest": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "recovery": {
                        "required_action": "select_advertised_solver_operator",
                        "safe_to_continue_other_tasks": true
                    }
                }
            },
            "tamper_rejection": {
                "reason_code": "operator_task_digest_mismatch",
                "failure_receipt": {
                    "schema_version": "kyuubiki.agent-operator-task-failure/v1",
                    "failure_stage": "verify_digest",
                    "reason_code": "operator_task_digest_mismatch",
                    "operator_id": "solve.bar_1d",
                    "task_digest": DIGEST,
                    "recovery": {
                        "required_action": "rebuild_task_ir_and_recompute_digest",
                        "safe_to_continue_other_tasks": true
                    }
                }
            },
            "recovery_execution": successful_execution()
        },
        "watchdog": {
            "state": "watch",
            "active_execution_count": 0,
            "recent_failure_count": 2,
            "recent_failures": [
                {
                    "request_id": "qualification-unsupported-solver",
                    "reason_code": "operator_task_solver_capability_rejected"
                },
                {
                    "request_id": "qualification-tampered",
                    "reason_code": "operator_task_digest_mismatch"
                }
            ]
        }
    })
}

fn successful_execution() -> Value {
    json!({
        "status": "executed",
        "solver_execution_capability": {
            "accepted": true,
            "capability_id": "agent-builtin-solver-execution",
            "operator_id": "solve.bar_1d",
            "task_id": "agent-qualification-bar-1d",
            "operator_kind": "solver",
            "program_kind": "solver",
            "runtime_protocol": "kyuubiki.solver-rpc/v1",
            "dispatch_route": "solver_rpc",
            "rejection_reasons": []
        },
        "validation_receipt": {
            "schema_version": "kyuubiki.agent-operator-task-validation/v1",
            "validation_status": "accepted",
            "digest_verified": true,
            "execution_program_verified": true,
            "runtime_protocol": "kyuubiki.solver-rpc/v1",
            "abi_kind": "solver_rpc",
            "dispatch_route": "solver_rpc",
            "package_fetch_required": false,
            "blocked_reason": null
        },
        "provenance_receipt": {
            "schema_version": "kyuubiki.agent-operator-task-provenance/v1",
            "operator_id": "solve.bar_1d",
            "task_digest": DIGEST,
            "requested_mode": "execute",
            "runtime_protocol": "kyuubiki.solver-rpc/v1",
            "abi_kind": "solver_rpc",
            "dispatch_route": "solver_rpc",
            "offline_runnable": true,
            "lineage": {
                "digest_verified": true,
                "execution_program_verified": true,
                "preview_digest": DIGEST
            }
        },
        "result_assertion": {
            "metric": "tip_displacement",
            "expected": AGENT_SOLVER_QUALIFICATION_EXPECTED_TIP_DISPLACEMENT,
            "actual": AGENT_SOLVER_QUALIFICATION_EXPECTED_TIP_DISPLACEMENT,
            "absolute_error": 0.0,
            "tolerance": 1.0e-12,
            "passed": true
        }
    })
}
