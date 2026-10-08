use super::{OPERATOR_TASK_MODE_EXECUTE, run_operator_task_ir, transform_operator_task_ir};
use crate::operator_task_runtime::OperatorTaskRuntimeError;
use kyuubiki_protocol::compute_operator_task_digest;
use serde_json::{Value, json};

fn task(id: &str, kind: &str, input: Value, config: Value) -> Value {
    let mut task = transform_operator_task_ir("native-transport", id, "transport", input, config);
    task["operator"]["kind"] = json!(kind);
    task["execution_program"]["program_kind"] = json!(kind);
    task["execution_program"]["entrypoint"]["operator_kind"] = json!(kind);
    task["execution_program"]["package_ref"] = Value::Null;
    task["execution_program"]["package_version"] = json!("built-in");
    task["runtime_hints"] = json!({"authority_mode":"agent_local","execution_mode":"agent_native",
        "cache_scope":"none","agent_fetchable":false,"operator_kind":kind});
    task["integrity"]["task_digest"] = json!(compute_operator_task_digest(&task).unwrap());
    task
}

fn result() -> Value {
    kyuubiki_engine::run_solve_operator(
        "solve.advection_diffusion_bar_1d",
        json!({
        "scheme":"upwind","nodes":[{"x":0.0,"fix_concentration":true,"concentration":0.0},
        {"x":1.0,"fix_concentration":true,"concentration":1.0}],
        "elements":[{"node_i":0,"node_j":1,"area":1.0,"diffusivity":1.0,"velocity":20.0}]}),
    )
    .unwrap()
}

fn execute(task: Value) -> Result<Value, OperatorTaskRuntimeError> {
    run_operator_task_ir(&json!({"mode":OPERATOR_TASK_MODE_EXECUTE,"task_ir":task}))
}

#[test]
fn transport_task_ir_uses_engine_diagnostics_and_opt_in_quality() {
    let raw = result();
    let expected = kyuubiki_engine::run_extract_operator(
        "extract.transport_result_diagnostics",
        raw.clone(),
        json!({}),
    )
    .unwrap();
    let diagnostic = execute(task(
        "extract.transport_result_diagnostics",
        "extract",
        raw,
        json!({}),
    ))
    .unwrap();
    assert_eq!(diagnostic["operator_task_ir_status"], "executed");
    assert_eq!(diagnostic["result"], expected);
    assert_eq!(diagnostic["package_fetch_request"], Value::Null);
    let quality = execute(task(
        "transform.score_transport_quality",
        "transform",
        expected,
        json!({"enabled_terms":["transport_artificial_diffusivity_peak"]}),
    ))
    .unwrap();
    assert_eq!(quality["result"]["transport_quality_score"], 10.0);
    assert_eq!(quality["result"]["transport_quality_ready"], false);
}

#[test]
fn transport_task_ir_rejects_corruption_and_recovers_without_result_publication() {
    let mut raw = result();
    raw["elements"][0]["stabilization"]["numerical_flux"] = Value::Null;
    let error = execute(task(
        "extract.transport_result_diagnostics",
        "extract",
        raw,
        json!({}),
    ))
    .unwrap_err();
    assert_eq!(error.code, "operator_task_execution_failed");
    assert!(
        error
            .message
            .contains("payload.elements[0].stabilization.numerical_flux")
    );
    assert_eq!(
        execute(task(
            "extract.transport_result_diagnostics",
            "extract",
            result(),
            json!({})
        ))
        .unwrap()["operator_task_ir_status"],
        "executed"
    );
}

#[test]
fn transport_task_ir_cannot_change_operator_kind_to_choose_another_dispatch() {
    let error = execute(task(
        "extract.transport_result_diagnostics",
        "transform",
        result(),
        json!({}),
    ))
    .unwrap_err();
    assert_eq!(error.code, "operator_task_execution_failed");
    assert!(error.message.contains("requires operator kind extract"));
}

#[test]
fn transport_task_ir_preflight_and_unknown_operators_never_execute() {
    let task = task(
        "extract.transport_result_diagnostics",
        "extract",
        json!({}),
        json!({}),
    );
    let preflight = run_operator_task_ir(&json!({"mode":"preflight","task_ir":task})).unwrap();
    assert_ne!(preflight["operator_task_ir_status"], "executed");
    assert!(preflight.get("result").is_none() || preflight["result"].is_null());
    let unknown = execute(self::task(
        "extract.transport_unknown",
        "extract",
        result(),
        json!({}),
    ))
    .unwrap();
    assert_ne!(unknown["operator_task_ir_status"], "executed");
}
