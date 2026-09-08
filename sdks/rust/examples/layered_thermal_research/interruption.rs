use super::{checks, model::Case};
use kyuubiki_headless_sdk::validate_workflow_result_against_graph;
use serde_json::{Value, json};

pub fn case() -> Case {
    Case {
        material: "high-contrast",
        conductivity: [2.0, 200.0],
        refinement: 8,
        reference: 293.15,
        rise: 20.0,
    }
}

pub fn request(policy: &str) -> Result<Value, String> {
    if !matches!(policy, "idempotent" | "checkpoint_required") {
        return Err("research replay policy must be idempotent or checkpoint_required".into());
    }
    let (mut graph, inputs) = case().workflow();
    graph["defaults"] = json!({"orchestrated":true});
    graph["recovery_policy"] = json!({"retry_safety":policy});
    for node in graph["nodes"].as_array_mut().unwrap() {
        if node["id"] == "structure" {
            node["retry_safety"] = json!(policy);
        }
    }
    Ok(json!({"graph":graph, "input_artifacts":inputs}))
}

fn payload(result: &Value) -> &Value {
    if result.get("artifacts").is_some() {
        result
    } else {
        &result["result"]
    }
}

pub fn held_after_heat(snapshot: &Value) -> bool {
    let job = &snapshot["job"]["job"];
    let result = payload(&snapshot["result"]);
    let progressed = result["progress_events"].as_array().is_some_and(|events| {
        events.iter().any(|e| e["node_id"] == "heat")
            && events.iter().any(|e| e["node_id"] == "bridge")
    });
    job["job_id"].is_string()
        && job["status"] == "solving"
        && progressed
        && snapshot["agent_observations"]
            .as_array()
            .is_some_and(|agents| {
                agents.iter().any(|agent| {
                    agent["descriptor"]["fault_injection"]["method_scope"]
                        == "solve_thermal_plane_quad_2d"
                        && agent["descriptor"]["watchdog"]["active_executions"]
                            .as_array()
                            .is_some_and(|active| {
                                active.iter().any(|execution| {
                                    execution["job_id"] == job["job_id"]
                                        && execution["method"] == "solve_thermal_plane_quad_2d"
                                })
                            })
                })
            })
}

pub fn validate_expectation(expected: &str) -> Result<(), String> {
    if matches!(
        expected,
        "completed" | "replayed" | "retry-blocked" | "recovery-blocked"
    ) {
        Ok(())
    } else {
        Err("unknown recovery expectation".into())
    }
}

pub fn validate_terminal(
    id: &str,
    terminal: &Value,
    result: &Value,
    expected: &str,
) -> Result<(), String> {
    validate_expectation(expected)?;
    let result = payload(result);
    let recovery = &result["recovery"];
    let success = matches!(expected, "completed" | "replayed");
    if terminal["job"]["job_id"] != id
        || terminal["job"]["status"] != if success { "completed" } else { "failed" }
        || result["workflow_id"] != format!("research.{}", case().id())
    {
        return Err("original job identity, terminal status or workflow identity mismatch".into());
    }
    let (state, generation) = match expected {
        "replayed" => ("completed", 2),
        "completed" => ("completed", 1),
        "retry-blocked" => ("failed", 1),
        _ => ("recovery_blocked", 1),
    };
    if recovery["state"] != state
        || recovery["generation"] != generation
        || recovery["attempt"] != generation
    {
        return Err("recovery state, generation or attempt mismatch".into());
    }
    let history = recovery["history"]
        .as_array()
        .ok_or("recovery history missing")?;
    let commits = history
        .iter()
        .filter(|item| item["event"] == "completed")
        .count();
    let claims = history
        .iter()
        .filter(|item| item["event"] == "claimed")
        .count();
    if commits != usize::from(success) || claims != generation as usize {
        return Err("unexpected terminal commits or execution claims".into());
    }
    if expected == "replayed"
        && !history.iter().any(|item| {
            item["event"] == "claimed"
                && item["generation"] == 2
                && item["reason"] == "process_restart"
        })
    {
        return Err("no process-restart replay evidence for generation two".into());
    }
    if !success {
        let message = terminal["job"]["message"].as_str().unwrap_or_default();
        let reason = if expected == "retry-blocked" {
            "agent_retry_blocked"
        } else {
            "workflow recovery blocked"
        };
        if !message.contains(reason)
            || !message.contains("checkpoint_required")
            || result["artifacts"].get("structure_out.result").is_some()
        {
            return Err(
                "blocked execution lacks its reason or exposed a successful structure output"
                    .into(),
            );
        }
    }
    Ok(())
}

pub fn validate_physics(request: &Value, result: &Value) -> Result<Value, String> {
    let graph =
        serde_json::from_value(request["graph"].clone()).map_err(|error| error.to_string())?;
    validate_workflow_result_against_graph(&graph, result).map_err(|error| error.to_string())?;
    let gates = checks::validate(case(), result)?;
    if gates["passed"] != true {
        return Err(format!("physical reference gates failed: {gates}"));
    }
    Ok(gates)
}

pub fn compare_physics(baseline: &Value, actual: &Value) -> Result<(), String> {
    for key in ["heat_out.result", "structure_out.result"] {
        for field in ["nodes", "elements"] {
            if payload(baseline)["artifacts"][key][field]
                != payload(actual)["artifacts"][key][field]
            {
                return Err(format!(
                    "recovered {key}.{field} differs from the uninterrupted reference"
                ));
            }
        }
    }
    if payload(baseline)["artifacts"]["bridge_out.model"]
        != payload(actual)["artifacts"]["bridge_out.model"]
    {
        return Err(
            "recovered temperature mapping differs from the uninterrupted reference".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_requirement_is_explicit_at_workflow_and_interrupted_node() {
        let request = request("checkpoint_required").unwrap();
        assert_eq!(
            request["graph"]["recovery_policy"]["retry_safety"],
            "checkpoint_required"
        );
        assert_eq!(
            request["graph"]["nodes"][3]["retry_safety"],
            "checkpoint_required"
        );
        assert_eq!(
            request["graph"]["nodes"][3]["operator_id"],
            "solve.thermal_plane_quad_2d"
        );
    }

    #[test]
    fn completed_status_alone_is_not_restart_acceptance() {
        let terminal = json!({"job":{"job_id":"original", "status":"completed"}});
        let result = json!({"workflow_id":format!("research.{}", case().id()), "artifacts":{},
            "recovery":{"state":"completed", "generation":1,"attempt":1,"history":[]}});
        assert!(validate_terminal("original", &terminal, &result, "replayed").is_err());
        assert!(validate_terminal("another", &terminal, &result, "completed").is_err());
        assert!(validate_physics(&request("idempotent").unwrap(), &result).is_err());
    }

    #[test]
    fn unknown_expectations_and_policies_fail_instead_of_skipping_gates() {
        assert!(request("checkpont_required").is_err());
        assert!(validate_expectation("skip").is_err());
    }

    #[test]
    fn fault_barrier_requires_same_job_method_and_completed_upstream_nodes() {
        let snapshot = json!({
            "job":{"job":{"job_id":"original", "status":"solving"}},
            "result":{"result":{"progress_events":[{"node_id":"heat"},{"node_id":"bridge"}]}},
            "agent_observations":[{"descriptor":{
                "fault_injection":{"method_scope":"solve_thermal_plane_quad_2d"},
                "watchdog":{"active_executions":[{
                    "job_id":"original", "method":"solve_thermal_plane_quad_2d"
                }]}
            }}]
        });
        assert!(held_after_heat(&snapshot));
        for (pointer, replacement) in [
            ("/job/job/status", json!("failed")),
            (
                "/result/result/progress_events",
                json!([{"node_id":"heat"}]),
            ),
            (
                "/agent_observations/0/descriptor/watchdog/active_executions/0/job_id",
                json!("another"),
            ),
            (
                "/agent_observations/0/descriptor/watchdog/active_executions/0/method",
                json!("solve_heat_plane_quad_2d"),
            ),
            (
                "/agent_observations/0/descriptor/fault_injection/method_scope",
                Value::Null,
            ),
            ("/agent_observations", json!([])),
        ] {
            let mut rejected = snapshot.clone();
            *rejected.pointer_mut(pointer).unwrap() = replacement;
            assert!(
                !held_after_heat(&rejected),
                "accepted invalid barrier: {pointer}"
            );
        }
    }
}
