use serde_json::{Value, json};
use std::error::Error;
use std::thread;
use std::time::{Duration, Instant};

#[path = "agent_lifecycle.rs"]
mod lifecycle;
pub(crate) use lifecycle::*;

pub(crate) fn task(id: char, job: &str) -> Value {
    json!({"execution_id":id.to_string().repeat(32), "method":"solve_bar_1d", "params":solver_params(job)})
}

pub(crate) fn native_task_ir(job: &str) -> Result<Value, Box<dyn Error>> {
    let mut ir: Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.operator-task-ir.json"
    ))?;
    ir["operator"] = json!({"id":"solve.bar_1d", "family":"mechanical", "kind":"solver"});
    ir["input_artifact"] = solver_params(job);
    let program = &mut ir["execution_program"];
    program["program_id"] = json!("solve.bar_1d");
    program["program_family"] = json!("mechanical");
    program["program_kind"] = json!("solver");
    program["package_ref"] = Value::Null;
    program["runtime_protocol"] = json!("kyuubiki.solver-rpc/v1");
    program["abi"]["kind"] = json!("solver_rpc");
    program["entrypoint"] =
        json!({"kind":"solver_method", "name":"solve_bar_1d", "operator_kind":"solver"});
    ir["runtime_hints"] = json!({"authority_mode":"agent_local", "execution_mode":"agent_native",
        "cache_scope":"none", "agent_fetchable":false, "operator_kind":"solver"});
    ir["integrity"] = json!({"task_digest":kyuubiki_protocol::compute_operator_task_digest(&ir)?});
    Ok(ir)
}

pub(crate) fn fetch(agent: &LiveAgent, id: char) -> Result<Value, Box<dyn Error>> {
    let response = agent.request(
        "inspect",
        "fetch_durable_agent_execution",
        json!({"execution_id":id.to_string().repeat(32)}),
    )?;
    Ok(successful_result(&response, "inspect").clone())
}

pub(crate) fn wait_status(
    agent: &LiveAgent,
    id: char,
    status: &str,
) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let response = agent.request(
            "inspect",
            "fetch_durable_agent_execution",
            json!({"execution_id":id.to_string().repeat(32)}),
        )?;
        // Poll only this read operation. Busy is neither a task state nor permission to replay.
        if response["ok"] == false && response["error"]["code"] == "durable_execution_busy" {
            assert_eq!(response["id"], "inspect");
            assert_eq!(response["error"]["details"]["operation_started"], false);
            assert_eq!(response["error"]["details"]["execution_state_known"], false);
            assert_eq!(
                response["error"]["details"]["automatic_replay_authorized"],
                false
            );
        } else {
            let report = successful_result(&response, "inspect");
            if report["status"] == status {
                return Ok(report.clone());
            }
        }
        if Instant::now() >= deadline {
            return Err(format!("expected {status}, got {response}").into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}
