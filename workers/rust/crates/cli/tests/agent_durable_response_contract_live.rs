use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{error::Error, fs};

#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;
use support::*;

fn replace_with_invalid_response(receipt: &mut Value, case: &str) {
    let response = &mut receipt["response"];
    match case {
        "version" => response["rpc_version"] = json!(0),
        "null_result" => response["result"] = Value::Null,
        "missing_result" => {
            response.as_object_mut().unwrap().remove("result");
        }
        "success_and_error" => {
            response["error"] = json!({"code":"solve_failed", "message":"solver failed"})
        }
        "failed_result" => {
            response["ok"] = json!(false);
            response["error"] = json!({"code":"solve_failed", "message":"solver failed"});
        }
        "missing_error" => {
            response["ok"] = json!(false);
            response["result"] = Value::Null;
        }
        "empty_code" | "empty_message" => {
            response["ok"] = json!(false);
            response["result"] = Value::Null;
            response["error"] = json!({"code":"solve_failed", "message":"solver failed"});
            response["error"][if case == "empty_code" {
                "code"
            } else {
                "message"
            }] = json!(" ");
        }
        _ => panic!("unknown fixture case"),
    }
}

#[test]
fn invalid_retained_responses_reject_fetch_replay_and_cancel_but_preserve_healthy_work()
-> Result<(), Box<dyn Error>> {
    for case in [
        "version",
        "null_result",
        "missing_result",
        "success_and_error",
        "failed_result",
        "missing_error",
        "empty_code",
        "empty_message",
    ] {
        let mut agent = LiveAgent::start_with_durable_store()?;
        let work = task('a', "shared-job");
        let completed = agent.request("original", "run_durable_agent_task", work.clone())?;
        let original = successful_result(&completed, "original");
        assert_eq!(original["status"], "completed");
        agent.stop_process()?;

        // Synthetic old-writer record: retain the real identity and a valid checksum.
        // Only the response contract is broken, while the Agent is stopped.
        let path = agent
            .root
            .join("executions")
            .join(format!("{}.receipt", "a".repeat(32)));
        let bytes = fs::read(&path)?;
        let mut receipt: Value = serde_json::from_slice(&bytes[65..])?;
        replace_with_invalid_response(&mut receipt, case);
        let payload = serde_json::to_vec(&receipt)?;
        let mut retained = format!("{:x}\n", Sha256::digest(&payload)).into_bytes();
        retained.extend_from_slice(&payload);
        fs::write(&path, &retained)?;
        agent.start_process()?;

        let query = json!({"execution_id":original["execution_id"],
            "request_digest":original["request_digest"], "process_instance_id":original["process_instance_id"]});
        for (method, params) in [
            (
                "fetch_durable_agent_execution",
                json!({"execution_id":"a".repeat(32)}),
            ),
            ("run_durable_agent_task", work.clone()),
            ("submit_durable_agent_task", work.clone()),
            ("cancel_durable_agent_execution", query.clone()),
        ] {
            let response = agent.request("inspect-invalid", method, params)?;
            assert_eq!(response["ok"], false, "{case}: {method}: {response}");
            assert_eq!(response["error"]["code"], "durable_execution_unavailable");
            assert_eq!(
                response["error"]["details"]["automatic_replay_authorized"],
                false
            );
        }
        assert_eq!(
            fs::read(&path)?,
            retained,
            "{case}: inspection rewrote the receipt"
        );
        let index = agent.request("index", "list_durable_agent_executions", json!({"limit":1}))?;
        assert_eq!(
            index["result"]["executions"][0]["execution_id"],
            "a".repeat(32)
        );
        let state = agent.request("descriptor", "describe_agent", json!({}))?;
        assert_eq!(
            state["result"]["watchdog"]["total_started_execution_count"],
            0
        );

        fs::write(&agent.hold_path, "shared-job")?;
        let next = agent.request(
            "original",
            "submit_durable_agent_task",
            task('b', "shared-job"),
        )?;
        assert_eq!(next["result"]["status"], "running");
        assert_eq!(
            agent.request("cancel-invalid", "cancel_durable_agent_execution", query)?["ok"],
            false
        );
        assert_eq!(fetch(&agent, 'b')?["status"], "running");
        fs::remove_file(&agent.hold_path)?;
        let healthy = wait_status(&agent, 'b', "completed")?;
        assert_eq!(healthy["response"]["result"]["max_stress"], 10.0);
        wait_for_lifecycle(&agent, "accepting", 0)?;
        agent.stop_process()?;
        agent.start_process()?;
        let invalid = agent.request(
            "after-restart",
            "fetch_durable_agent_execution",
            json!({"execution_id":"a".repeat(32)}),
        )?;
        assert_eq!(invalid["ok"], false);
        assert_eq!(invalid["error"]["code"], "durable_execution_unavailable");
        assert_eq!(fetch(&agent, 'b')?, healthy);
        assert_eq!(fs::read(&path)?, retained);
    }
    Ok(())
}
