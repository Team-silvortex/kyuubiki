use super::prelude::*;
use serde_json::{Value, json};

#[test]
fn canonical_json_sorts_nested_object_keys() {
    let value = json!({
        "z": 1,
        "a": {
            "b": true,
            "a": null
        },
        "list": [
            {
                "y": 2,
                "x": 1
            }
        ]
    });

    assert_eq!(
        canonical_json(&value),
        r#"{"a":{"a":null,"b":true},"list":[{"x":1,"y":2}],"z":1}"#
    );
}

#[test]
fn canonical_json_encodes_floats_without_exponent_notation() {
    let value = json!({
        "a": 160.0,
        "b": 1.2e-5,
        "c": 7.0e10,
        "d": 0.33
    });

    assert_eq!(
        canonical_json(&value),
        r#"{"a":160.0,"b":0.000012,"c":70000000000.0,"d":0.33}"#
    );
}

#[test]
fn operator_task_digest_matches_elixir_golden_fixture() {
    let task = golden_task_fixture(None);

    assert_eq!(
        compute_operator_task_digest(&task).expect("digest should compute"),
        "86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f"
    );
}

#[test]
fn operator_task_digest_matches_elixir_float_golden_fixture() {
    let task = golden_float_task_fixture();

    assert_eq!(
        compute_operator_task_digest(&task).expect("digest should compute"),
        "d87818ffb27cc8f01e6a360f973ebf1d40025362b28cda0909078b99cd6139b7"
    );
}

#[test]
fn operator_task_digest_matches_fractional_schema_fixture() {
    let task: Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.operator-task-ir-float.json"
    ))
    .expect("fractional fixture should parse");

    assert_eq!(
        compute_operator_task_digest(&task).expect("digest should compute"),
        task["integrity"]["task_digest"]
            .as_str()
            .expect("fixture should carry task digest")
    );
    verify_operator_task_digest(&task).expect("fixture digest should verify");
}

#[test]
fn operator_task_digest_verifier_rejects_tampering() {
    let mut task = golden_task_fixture(Some(
        "86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f",
    ));
    task["config"]["alpha"] = json!(false);

    assert_eq!(
        verify_operator_task_digest(&task),
        Err(OperatorTaskDigestError::Mismatch {
            expected: "86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f"
                .to_string(),
            actual: "18a4f75f84d8a7c846934ae77eba61d40b7d1a8d45f7b29d7f31fe951e4008fd".to_string()
        })
    );
}

#[test]
fn operator_task_execution_summary_exposes_agent_ready_fields() {
    let task = golden_task_fixture(Some(
        "86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f",
    ));

    let summary = summarize_operator_task_execution(&task).expect("summary should build");

    assert_eq!(summary.task_id, "fixture-task");
    assert_eq!(summary.operator_id, "transform.fixture");
    assert_eq!(summary.operator_kind, "transform");
    assert_eq!(summary.program_id, "transform.fixture");
    assert_eq!(summary.program_kind, "transform");
    assert_eq!(summary.runtime_protocol, "kyuubiki.operator-execution/v1");
    assert_eq!(summary.abi_kind, "operator_task");
    assert_eq!(summary.entrypoint_kind, "operator_id");
    assert_eq!(
        summary.package_ref.as_deref(),
        Some("orchestra://operator-package/transform.fixture")
    );
    assert_eq!(
        summary.authority_mode.as_deref(),
        Some("central_operator_library")
    );
    assert_eq!(summary.execution_mode.as_deref(), Some("orchestra_fetch"));
    assert_eq!(summary.cache_scope.as_deref(), Some("job"));
    assert_eq!(summary.agent_fetchable, Some(true));
}

#[test]
fn operator_task_execution_summary_rejects_inconsistent_abi() {
    let mut task = golden_task_fixture(Some(
        "86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f",
    ));
    task["execution_program"]["abi"]["kind"] = json!("solver_rpc");

    let error = summarize_operator_task_execution(&task).expect_err("abi should be rejected");

    assert!(error.contains("inconsistent runtime protocol, abi, or entrypoint"));
}

#[test]
fn checked_operator_task_summary_reports_structured_error_codes() {
    let mut task = golden_task_fixture(Some(
        "86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f",
    ));
    task["execution_program"]["abi"]["kind"] = json!("solver_rpc");

    let error =
        summarize_operator_task_execution_checked(&task).expect_err("abi should be rejected");

    assert_eq!(
        error.code,
        OperatorTaskSummaryErrorCode::ExecutionAbiMismatch
    );
    assert!(
        error
            .message
            .contains("inconsistent runtime protocol, abi, or entrypoint")
    );
}

#[test]
fn operator_task_execution_summary_rejects_inconsistent_operator_kind_mirrors() {
    let mut task = golden_task_fixture(Some(
        "86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f",
    ));
    task["runtime_hints"]["operator_kind"] = json!("solver");

    let error = summarize_operator_task_execution(&task).expect_err("kind mirror should fail");

    assert!(error.contains("runtime_hints.operator_kind must match operator.kind"));
}

#[test]
fn operator_task_execution_summary_rejects_inconsistent_entrypoint_kind_mirror() {
    let mut task = golden_task_fixture(Some(
        "86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f",
    ));
    task["execution_program"]["entrypoint"]["operator_kind"] = json!("export");

    let error =
        summarize_operator_task_execution(&task).expect_err("entrypoint kind mirror should fail");

    assert!(error.contains("execution_program.entrypoint.operator_kind must match operator.kind"));
}

#[test]
fn operator_task_execution_summary_rejects_inconsistent_package_ref_mirrors() {
    let mut task = golden_task_fixture(Some(
        "86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f",
    ));
    task["runtime_hints"]["package_ref"] = json!("orchestra://operator-package/wrong");

    let error =
        summarize_operator_task_execution(&task).expect_err("package ref mirror should fail");

    assert!(error.contains("runtime_hints.package_ref must match execution_program.package_ref"));
}

#[test]
fn operator_task_execution_summary_rejects_inconsistent_package_version_mirror() {
    let mut task = golden_task_fixture(Some(
        "86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f",
    ));
    task["runtime_hints"]["package_version"] = json!("1.0.0");

    let error =
        summarize_operator_task_execution(&task).expect_err("package version mirror should fail");

    assert!(
        error
            .contains("runtime_hints.package_version must match execution_program.package_version")
    );
}

fn golden_task_fixture(task_digest: Option<&str>) -> Value {
    let mut task = json!({
        "schema_version": OPERATOR_TASK_IR_SCHEMA,
        "task_id": "fixture-task",
        "operator": {
            "id": "transform.fixture",
            "family": "fixture",
            "kind": "transform",
            "execution": {
                "package_ref": "orchestra://operator-package/transform.fixture"
            }
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
            "x": 1
        },
        "config": {
            "alpha": true
        },
        "execution_program": {
            "schema_version": "kyuubiki.operator-execution-program/v1",
            "program_id": "transform.fixture",
            "program_family": "fixture",
            "program_kind": "transform",
            "operator_category_id": null,
            "package_ref": "orchestra://operator-package/transform.fixture",
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
                "name": "transform.fixture",
                "operator_kind": "transform"
            },
            "bindings": {
                "input_artifact": "task.input_artifact",
                "config": "task.config",
                "output_artifact": "task.output_artifact"
            },
            "node_binding": {
                "node_id": null,
                "input_ports": [],
                "output_ports": []
            }
        },
        "dataset_contract": {},
        "orchestration_context": {},
        "runtime_hints": {
            "authority_mode": "central_operator_library",
            "execution_mode": "orchestra_fetch",
            "source_ref": null,
            "package_ref": "orchestra://operator-package/transform.fixture",
            "package_version": "library-managed",
            "placement_tags": [],
            "required_capabilities": [],
            "cache_scope": "job",
            "agent_fetchable": true,
            "operator_kind": "transform"
        }
    });

    if let Some(digest) = task_digest {
        task["integrity"] = json!({
            "task_digest": digest
        });
    }

    task
}

fn golden_float_task_fixture() -> Value {
    json!({
        "schema_version": OPERATOR_TASK_IR_SCHEMA,
        "task_id": "float-fixture-task",
        "operator": {
            "id": "transform.float_fixture",
            "family": "fixture",
            "kind": "transform",
            "execution": {
                "package_ref": "orchestra://operator-package/transform.float_fixture"
            }
        },
        "descriptor_authoring": {
            "schema_version": "kyuubiki.operator-descriptor-authoring/v1",
            "mode": "rust_native",
            "runtime": "rust",
            "source": "float_fixture",
            "hot_reloadable": false,
            "execution_language": "language_neutral"
        },
        "node": {},
        "input_artifact": {
            "temperature_delta": 160.0,
            "thermal_expansion": 1.2e-5,
            "youngs_modulus": 70.0e9,
            "poisson_ratio": 0.33
        },
        "config": {
            "constraint_factor": 0.7
        },
        "execution_program": {
            "schema_version": "kyuubiki.operator-execution-program/v1",
            "program_id": "transform.float_fixture",
            "program_family": "fixture",
            "program_kind": "transform",
            "operator_category_id": null,
            "package_ref": "orchestra://operator-package/transform.float_fixture",
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
                "name": "transform.float_fixture",
                "operator_kind": "transform"
            },
            "bindings": {
                "input_artifact": "task.input_artifact",
                "config": "task.config",
                "output_artifact": "task.output_artifact"
            },
            "node_binding": {
                "node_id": null,
                "input_ports": [],
                "output_ports": []
            }
        },
        "dataset_contract": {},
        "orchestration_context": {},
        "runtime_hints": {
            "authority_mode": "central_operator_library",
            "execution_mode": "orchestra_fetch",
            "source_ref": null,
            "package_ref": "orchestra://operator-package/transform.float_fixture",
            "package_version": "library-managed",
            "placement_tags": [],
            "required_capabilities": [],
            "cache_scope": "job",
            "agent_fetchable": true,
            "operator_kind": "transform"
        }
    })
}
