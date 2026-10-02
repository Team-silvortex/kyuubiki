use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{prepare_operator_task_payload, validate_operator_task_for_agent};
use kyuubiki_protocol::SolverExecutionCapability;
use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[path = "support/agent_lifecycle.rs"]
mod support;
use support::*;

#[path = "support/modal_agent.rs"]
mod modal;

fn execute(agent: &LiveAgent, id: &str, task: &Value) -> Result<Value, Box<dyn Error>> {
    agent.request(
        id,
        "run_operator_task_ir",
        json!({"mode":"execute", "task_ir":task}),
    )
}

fn task_result<'a>(response: &'a Value, id: &str, task: &Value) -> &'a Value {
    let result = successful_result(response, id);
    assert_eq!(result["operator_task_ir_status"], "executed");
    assert_eq!(
        result["execution_runtime_status"],
        "agent_engine_solver_executed"
    );
    assert_eq!(result["solver_execution_capability"]["accepted"], true);
    assert_eq!(result["validation_receipt"]["digest_verified"], true);
    assert_eq!(
        result["provenance_receipt"]["task_digest"],
        task["integrity"]["task_digest"]
    );
    assert_eq!(result["task_digest"], task["integrity"]["task_digest"]);
    &result["result"]
}

fn raw_reference(space: bool, model: Value) -> Value {
    let mut result = run_solve_operator(modal::operator(space), model).unwrap();
    result.as_object_mut().unwrap().remove("_solver_provenance");
    result
}

fn assert_failure(response: &Value, id: &str, code: &str, stage: &str) {
    assert_eq!(response["id"], id);
    assert_eq!(response["ok"], false);
    assert_eq!(response["error"]["code"], code, "{response}");
    assert!(response.get("result").is_none_or(Value::is_null));
    let receipt = &response["error"]["details"]["operator_task_failure_receipt"];
    assert_eq!(receipt["failure_stage"], stage);
    assert_eq!(receipt["recovery"]["safe_to_continue_other_tasks"], true);
}

#[test]
fn agent_advertises_only_attached_task_ir_solver_routes() -> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    let response = agent.request("solver-description", "describe_agent", json!({}))?;
    let descriptor = successful_result(&response, "solver-description");
    let capability: SolverExecutionCapability = serde_json::from_value(
        descriptor["headless_bridge"]["headless_entrypoints"][0]["solver_execution_capability"]
            .clone(),
    )?;
    assert_eq!(capability, SolverExecutionCapability::agent_builtin());
    assert_eq!(
        capability.operator_ids,
        [
            "solve.bar_1d",
            "solve.modal_frame_2d",
            "solve.modal_frame_3d"
        ]
    );
    assert!(!capability.supports_package_fetch);
    let manifest = kyuubiki_engine::solve_operator_runtime_manifest();
    for id in capability.operator_ids {
        assert!(
            manifest["operators"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["operator_id"] == id)
        );
    }
    Ok(())
}

#[test]
fn direct_modal_rpc_preserves_numeric_bits_and_independent_residuals_over_tcp()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    for (segments, space) in [(96, false), (100, false), (100, true)] {
        let model = modal::model(segments, space);
        let expected = raw_reference(space, model.clone());
        let response = agent.request("direct-modal", modal::method(space), model)?;
        let result = successful_result(&response, "direct-modal");
        modal::assert_bits(&expected, result);
        modal::check_result(result, segments, space);
        wait_for_lifecycle(&agent, "accepting", 0)?;
    }
    Ok(())
}

#[test]
fn modal_task_ir_executes_through_agent_engine_and_preserves_numeric_digest_over_tcp()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    for (segments, space) in [(96, false), (100, false), (100, true)] {
        let model = modal::model(segments, space);
        let expected = raw_reference(space, model.clone());
        let task = modal::task("modal-task-ir", space, model);
        let prepared = prepare_operator_task_payload(&json!({"task":task}))?;
        assert_eq!(prepared["task_digest"], task["integrity"]["task_digest"]);
        let response = execute(&agent, "modal-task-ir", &task)?;
        let result = task_result(&response, "modal-task-ir", &task);
        modal::assert_bits(&expected, result);
        modal::check_result(result, segments, space);
        assert!(
            validate_operator_task_for_agent(&task, &SolverExecutionCapability::agent_builtin()).ok
        );
        wait_for_lifecycle(&agent, "accepting", 0)?;
    }
    Ok(())
}

#[test]
fn modal_task_ir_invalid_input_is_not_execution_success_and_slot_recovers()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    for space in [false, true] {
        let mut invalid = modal::task("bad-modal-input", space, modal::model(100, space));
        invalid["input_artifact"]["nodes"][1]["x"] = json!("not-a-coordinate");
        modal::seal(&mut invalid);
        let failure = execute(&agent, "bad-modal-input", &invalid)?;
        assert_failure(
            &failure,
            "bad-modal-input",
            "operator_task_solver_input_invalid",
            "decode_solver_input",
        );
        wait_for_lifecycle(&agent, "accepting", 0)?;
        let valid = modal::task("after-input-error", space, modal::model(100, space));
        let replay = execute(&agent, "after-input-error", &valid)?;
        modal::check_result(
            task_result(&replay, "after-input-error", &valid),
            100,
            space,
        );
    }
    Ok(())
}

#[test]
fn modal_task_ir_digest_mirror_and_authority_gates_survive_agent_transport()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    for space in [false, true] {
        let valid = modal::task("guarded-modal", space, modal::model(100, space));
        let mut tampered = valid.clone();
        tampered["input_artifact"]["elements"][0]["density"] = json!(2.0);
        let rejected = execute(&agent, "digest-tamper", &tampered)?;
        assert_failure(
            &rejected,
            "digest-tamper",
            "operator_task_digest_mismatch",
            "verify_digest",
        );
        let mut mirror = valid.clone();
        mirror["execution_program"]["entrypoint"]["name"] = json!("solve_bar_1d");
        modal::seal(&mut mirror);
        let rejected = execute(&agent, "mirror-tamper", &mirror)?;
        assert_failure(
            &rejected,
            "mirror-tamper",
            "operator_task_entrypoint_mismatch",
            "summarize_execution_program",
        );
        let mut central = valid.clone();
        central["runtime_hints"]["authority_mode"] = json!("central_operator_library");
        modal::seal(&mut central);
        let rejected = execute(&agent, "authority-tamper", &central)?;
        assert_failure(
            &rejected,
            "authority-tamper",
            "operator_task_admission_rejected",
            "validate_admission_policy",
        );
        central["runtime_hints"]["execution_mode"] = json!("orchestra_fetch");
        central["runtime_hints"]["agent_fetchable"] = json!(true);
        central["runtime_hints"]["cache_scope"] = json!("job");
        central["execution_program"]["package_ref"] = json!(format!(
            "orchestra://operator-package/{}",
            modal::operator(space)
        ));
        modal::seal(&mut central);
        let rejected = execute(&agent, "unattached-central-fetch", &central)?;
        assert_failure(
            &rejected,
            "unattached-central-fetch",
            "operator_task_solver_capability_rejected",
            "check_solver_capability",
        );
        let mut unknown = valid.clone();
        unknown["operator"]["id"] = json!("solve.thermal_bar_1d");
        unknown["execution_program"]["program_id"] = json!("solve.thermal_bar_1d");
        unknown["execution_program"]["entrypoint"]["name"] = json!("solve_thermal_bar_1d");
        modal::seal(&mut unknown);
        let rejected = execute(&agent, "unsupported-solver", &unknown)?;
        assert_failure(
            &rejected,
            "unsupported-solver",
            "operator_task_solver_capability_rejected",
            "check_solver_capability",
        );
        wait_for_lifecycle(&agent, "accepting", 0)?;
        let replay = execute(&agent, "guard-replay", &valid)?;
        modal::check_result(task_result(&replay, "guard-replay", &valid), 100, space);
    }
    Ok(())
}

fn send(stream: &mut TcpStream, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    stream.write_all(&u32::try_from(bytes.len())?.to_be_bytes())?;
    // Deliberately split frames; TCP is a byte stream, not message delivery.
    for part in bytes.chunks(37) {
        stream.write_all(part)?;
    }
    Ok(())
}

fn terminal(stream: &mut TcpStream) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        let frame = read_json_frame(stream)?;
        if frame.get("ok").is_some() {
            return Ok(frame);
        }
    }
    Err("Agent did not return a terminal modal response".into())
}

#[test]
fn modal_agent_same_connection_recovers_after_invalid_json_and_numerical_failure()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    let mut stream = TcpStream::connect(("127.0.0.1", agent.port))?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
    send(&mut stream, br#"{"rpc_version":1,"id":"invalid-json","method":"solve_modal_frame_2d","params":{"probe":1e309}}"#)?;
    let invalid = terminal(&mut stream)?;
    assert_eq!(invalid["ok"], false);
    assert_eq!(invalid["error"]["code"], "invalid_json");
    let mut unresolved_model = modal::model(128, false);
    unresolved_model["mode_count"] = json!(6);
    let unresolved = modal::task("unresolved-modal", false, unresolved_model);
    let request = |id: &str, task: &Value| {
        serde_json::to_vec(&json!({"rpc_version":1,"id":id,
        "method":"run_operator_task_ir","params":{"mode":"execute","task_ir":task}}))
        .unwrap()
    };
    send(&mut stream, &request("unresolved-modal", &unresolved))?;
    let failure = terminal(&mut stream)?;
    assert_failure(
        &failure,
        "unresolved-modal",
        "operator_task_solver_execution_failed",
        "dispatch_engine_solver",
    );
    assert!(
        failure["error"]["message"]
            .as_str()
            .unwrap()
            .contains("refinement did not converge")
    );
    wait_for_lifecycle(&agent, "accepting", 0)?;
    let valid = modal::task("same-connection-replay", false, modal::model(100, false));
    send(&mut stream, &request("same-connection-replay", &valid))?;
    let replay = terminal(&mut stream)?;
    modal::check_result(
        task_result(&replay, "same-connection-replay", &valid),
        100,
        false,
    );
    wait_for_lifecycle(&agent, "accepting", 0)?;
    Ok(())
}

#[test]
fn modal_task_ir_cancellation_discards_modes_and_recovers_the_agent_slot()
-> Result<(), Box<dyn Error>> {
    let stage = "sparse_matvec";
    let agent = LiveAgent::start_with_solver_hold(stage, "run_operator_task_ir", "1")?;
    let job = "cancelled-modal-job";
    fs::write(&agent.hold_path, job)?;
    let task = modal::task("held-modal", false, modal::model(100, false));
    let mut stream = TcpStream::connect(("127.0.0.1", agent.port))?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
    send(
        &mut stream,
        &serde_json::to_vec(&json!({"rpc_version":1,"id":"held-modal",
        "method":"run_operator_task_ir","params":{"mode":"execute","task_ir":task,"job_id":job}}))?,
    )?;
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let state = agent.request("modal-progress", "describe_agent", json!({}))?;
        let active = &state["result"]["solver_control"]["active"];
        if active.as_array().unwrap().iter().any(|entry| {
            entry["request_id"] == "held-modal"
                && entry["checkpoint"]["stage"] == stage
                && entry["checkpoint"]["completed_steps"]
                    .as_u64()
                    .is_some_and(|n| n >= 3)
        }) {
            break;
        }
        if Instant::now() >= deadline {
            return Err("modal execution did not reach the held numerical stage".into());
        }
        thread::sleep(Duration::from_millis(20));
    }
    let receipt = agent.request("cancel-modal", "cancel_job", json!({"job_id":job}))?;
    assert!(
        receipt["result"]["cancelled"] == true
            || receipt["error"]["details"]["cancel_registered"] == true
    );
    let failure = terminal(&mut stream)?;
    assert_failure(&failure, "held-modal", "cancelled", "execute_solver");
    let point = &failure["error"]["details"]["solver_checkpoint"];
    assert_eq!(point["stage"], stage);
    assert_eq!(point["resumable"], false);
    assert!(point["completed_steps"].as_u64().unwrap() >= 3);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    assert!(agent.hold_path.exists());
    let healthy = modal::task("cancel-replay", false, modal::model(100, false));
    let replay = execute(&agent, "cancel-replay", &healthy)?;
    modal::check_result(task_result(&replay, "cancel-replay", &healthy), 100, false);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    Ok(())
}
