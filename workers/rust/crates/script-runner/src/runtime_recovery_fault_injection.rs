use kyuubiki_engine::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowNodeRunStatus, WorkflowPort,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const REPORT_SCHEMA: &str = "kyuubiki.runtime-recovery-fault-injection/v1";

#[derive(Debug, Deserialize, Serialize)]
struct RecoveryReport {
    schema_version: String,
    status: String,
    scenario_count: usize,
    scenarios: Vec<ScenarioReport>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ScenarioReport {
    id: String,
    status: String,
    injected_fault: String,
    recovery_policy: String,
    observations: Value,
}

#[derive(Default)]
struct Options {
    out: Option<PathBuf>,
    verify_report: Option<PathBuf>,
    self_test: bool,
}

pub(crate) fn run_check_runtime_recovery_fault_injection(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(root, args)?;
    if options.self_test {
        let mut report = execute_fault_injection()?;
        validate_report(&report)?;
        report.scenarios[0].status = "fail".to_string();
        if validate_report(&report).is_ok() {
            return Err("fault injection self-test accepted a failed scenario".to_string());
        }
        println!("runtime recovery fault injection self-test passed");
        return Ok(0);
    }
    if let Some(path) = options.verify_report {
        let report = read_report(&path)?;
        validate_report(&report)?;
        println!(
            "runtime recovery fault injection report passed: {}",
            path.display()
        );
        return Ok(0);
    }

    let report = execute_fault_injection()?;
    validate_report(&report)?;
    if let Some(path) = options.out {
        write_report(&path, &report)?;
        println!(
            "runtime recovery fault injection report written: {}",
            path.display()
        );
    }
    println!(
        "runtime recovery fault injection passed: {}/{} scenario(s)",
        report.scenario_count, report.scenario_count
    );
    Ok(0)
}

fn parse_options(root: &Path, args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options::default();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--out" => options.out = Some(repo_path(root, required_path(&mut iter, "--out")?)?),
            "--verify-report" => {
                options.verify_report = Some(repo_path(
                    root,
                    required_path(&mut iter, "--verify-report")?,
                )?)
            }
            "--self-test" => options.self_test = true,
            other => {
                return Err(format!(
                    "unknown recovery fault injection argument: {other}"
                ));
            }
        }
    }
    if options.out.is_some() && options.verify_report.is_some() {
        return Err("--out and --verify-report cannot be combined".to_string());
    }
    Ok(options)
}

fn required_path(iter: &mut impl Iterator<Item = OsString>, name: &str) -> RunnerResult<String> {
    iter.next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{name} requires a repository-relative path"))
}

fn repo_path(root: &Path, relative: String) -> RunnerResult<PathBuf> {
    let path = Path::new(&relative);
    if path.is_absolute() || path.components().any(|part| part.as_os_str() == "..") {
        return Err(format!(
            "recovery report path escapes repository: {relative}"
        ));
    }
    Ok(root.join(path))
}

fn execute_fault_injection() -> RunnerResult<RecoveryReport> {
    let recoverable = run_workflow_graph(request(true))
        .map_err(|error| format!("recoverable injection unexpectedly failed: {error}"))?;
    let failed_trace = recoverable
        .node_runs
        .iter()
        .find(|trace| trace.node_id == "injected_failure")
        .ok_or_else(|| "recoverable injection lost failed-node trace".to_string())?;
    let failure_message = failed_trace.error_message.clone().unwrap_or_default();
    let recovery_scenario = ScenarioReport {
        id: "recoverable_branch_isolation".to_string(),
        status: "pass".to_string(),
        injected_fault: "unsupported_condition_operator".to_string(),
        recovery_policy: "skip".to_string(),
        observations: json!({
            "failed_nodes": recoverable.failed_nodes,
            "skipped_nodes": recoverable.skipped_nodes,
            "independent_branch_completed": recoverable.completed_nodes.contains(&"independent_output".to_string()),
            "independent_result": recoverable.artifacts.get("independent_output.result"),
            "failed_trace_status": failed_trace.status,
            "failure_message": failure_message,
            "cascading_failure": false
        }),
    };

    let fail_fast_error = run_workflow_graph(request(false))
        .expect_err("fault without recovery policy must fail fast");
    let fail_fast_scenario = ScenarioReport {
        id: "fail_fast_without_policy".to_string(),
        status: "pass".to_string(),
        injected_fault: "unsupported_condition_operator".to_string(),
        recovery_policy: "none".to_string(),
        observations: json!({
            "run_rejected": true,
            "error": fail_fast_error,
            "silent_recovery": false
        }),
    };

    let watchdog_observations = kyuubiki_cli::agent_watchdog::run_fault_injection_probe()?;
    let watchdog_scenario = ScenarioReport {
        id: "agent_watchdog_failure_then_success".to_string(),
        status: "pass".to_string(),
        injected_fault: "invalid_params".to_string(),
        recovery_policy: "release_slot_and_retain_reason".to_string(),
        observations: watchdog_observations,
    };
    let timeout_observations = kyuubiki_cli::agent_watchdog::run_timeout_fault_injection_probe()?;
    let timeout_scenario = ScenarioReport {
        id: "agent_watchdog_stale_timeout".to_string(),
        status: "pass".to_string(),
        injected_fault: "stale_execution_heartbeat".to_string(),
        recovery_policy: "cancel_reject_and_retain_reason".to_string(),
        observations: timeout_observations,
    };

    Ok(RecoveryReport {
        schema_version: REPORT_SCHEMA.to_string(),
        status: "pass".to_string(),
        scenario_count: 4,
        scenarios: vec![
            recovery_scenario,
            fail_fast_scenario,
            watchdog_scenario,
            timeout_scenario,
        ],
    })
}

fn validate_report(report: &RecoveryReport) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.status != "pass"
        || report.scenario_count != 4
        || report.scenarios.len() != 4
    {
        return Err("runtime recovery report summary is invalid".to_string());
    }
    let ids = report
        .scenarios
        .iter()
        .map(|scenario| scenario.id.as_str())
        .collect::<BTreeSet<_>>();
    if ids
        != BTreeSet::from([
            "agent_watchdog_failure_then_success",
            "agent_watchdog_stale_timeout",
            "fail_fast_without_policy",
            "recoverable_branch_isolation",
        ])
    {
        return Err("runtime recovery report scenario set is invalid".to_string());
    }
    if report
        .scenarios
        .iter()
        .any(|scenario| scenario.status != "pass")
    {
        return Err("runtime recovery report contains a failed scenario".to_string());
    }
    let recovery = scenario(report, "recoverable_branch_isolation")?;
    require_json(recovery, "/failed_nodes", json!(["injected_failure"]))?;
    require_json(recovery, "/skipped_nodes", json!(["dependent_output"]))?;
    require_json(recovery, "/independent_branch_completed", json!(true))?;
    require_json(recovery, "/independent_result", json!({ "value": 7 }))?;
    require_json(
        recovery,
        "/failed_trace_status",
        json!(WorkflowNodeRunStatus::Failed),
    )?;
    require_json(recovery, "/cascading_failure", json!(false))?;
    let failure_message = recovery
        .observations
        .get("failure_message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !failure_message.contains("unsupported condition operator") {
        return Err("recoverable scenario lost the injected failure reason".to_string());
    }

    let fail_fast = scenario(report, "fail_fast_without_policy")?;
    require_json(fail_fast, "/run_rejected", json!(true))?;
    require_json(fail_fast, "/silent_recovery", json!(false))?;
    let error = fail_fast
        .observations
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !error.contains("workflow node injected_failure failed")
        || !error.contains("unsupported condition operator")
    {
        return Err("fail-fast scenario lost the injected failure reason".to_string());
    }

    let watchdog = scenario(report, "agent_watchdog_failure_then_success")?;
    for (pointer, expected) in [
        ("/failure_recorded", json!(true)),
        ("/failure_reason_code", json!("invalid_params")),
        ("/watchdog_state_after_failure", json!("watch")),
        ("/slot_released_after_failure", json!(true)),
        ("/recent_failure_count_after_failure", json!(1)),
        ("/healthy_execution_completed", json!(true)),
        ("/slot_released_after_healthy", json!(true)),
        ("/recent_failure_retained", json!(true)),
        ("/new_failure_after_healthy", json!(false)),
        ("/probe_cleanup_completed", json!(true)),
    ] {
        require_json(watchdog, pointer, expected)?;
    }

    let timeout = scenario(report, "agent_watchdog_stale_timeout")?;
    for (pointer, expected) in [
        ("/policy_enabled", json!(true)),
        ("/stale_execution_ms", json!(100)),
        ("/progress_refreshed", json!(true)),
        ("/expired_before_budget", json!(false)),
        ("/timeout_count", json!(1)),
        ("/timeout_reason_code", json!("watchdog_timeout")),
        ("/timeout_job_id", json!("watchdog-stale-job")),
        ("/timeout_elapsed_ms", json!(150)),
        ("/timeout_message_has_budget", json!(true)),
        ("/slot_released_after_timeout", json!(true)),
        ("/timeout_failure_recorded", json!(true)),
        ("/late_failure_reused_timeout", json!(true)),
        ("/duplicate_failure_created", json!(false)),
        ("/healthy_follow_up_completed", json!(true)),
        ("/timeout_reason_retained", json!(true)),
        ("/probe_cleanup_completed", json!(true)),
    ] {
        require_json(timeout, pointer, expected)?;
    }
    Ok(())
}

fn scenario<'a>(report: &'a RecoveryReport, id: &str) -> RunnerResult<&'a ScenarioReport> {
    report
        .scenarios
        .iter()
        .find(|scenario| scenario.id == id)
        .ok_or_else(|| format!("runtime recovery report misses scenario {id}"))
}

fn require_json(scenario: &ScenarioReport, pointer: &str, expected: Value) -> RunnerResult<()> {
    let actual = scenario.observations.pointer(pointer);
    if actual != Some(&expected) {
        return Err(format!(
            "runtime recovery scenario {} {pointer} must be {expected}, got {}",
            scenario.id,
            actual.unwrap_or(&Value::Null)
        ));
    }
    Ok(())
}

fn read_report(path: &Path) -> RunnerResult<RecoveryReport> {
    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read recovery report {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid recovery report {}: {error}", path.display()))
}

fn write_report(path: &Path, report: &RecoveryReport) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("failed to encode recovery report: {error}"))?;
    fs::write(path, bytes).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn request(recover: bool) -> WorkflowGraphRunRequest {
    WorkflowGraphRunRequest {
        graph: graph(recover),
        input_artifacts: BTreeMap::from([
            ("independent_input".to_string(), json!({ "value": 7 })),
            ("fault_input".to_string(), json!({ "value": 3 })),
        ]),
    }
}

fn graph(recover: bool) -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.recovery.fault-injection".to_string(),
        name: "Runtime recovery fault injection".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["independent_input".to_string(), "fault_input".to_string()],
        output_nodes: vec!["independent_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            input_node("independent_input"),
            input_node("fault_input"),
            fault_node(recover),
            output_node("dependent_output", "value"),
            output_node("independent_output", "result"),
        ],
        edges: vec![
            edge(
                "independent",
                "independent_input",
                "value",
                "independent_output",
                "result",
            ),
            edge(
                "inject",
                "fault_input",
                "value",
                "injected_failure",
                "value",
            ),
            edge(
                "dependent",
                "injected_failure",
                "if_true",
                "dependent_output",
                "value",
            ),
        ],
    }
}

fn input_node(id: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port("value")],
    }
}

fn fault_node(recover: bool) -> WorkflowNode {
    let mut config = json!({ "predicate": { "operator": "injected_unsupported" } });
    if recover {
        config["recovery"] = json!({ "on_error": "skip" });
    }
    WorkflowNode {
        id: "injected_failure".to_string(),
        kind: WorkflowNodeKind::Condition,
        operator_id: None,
        name: None,
        description: None,
        config: Some(config),
        cache_policy: None,
        inputs: vec![port("value")],
        outputs: vec![port("if_true"), port("if_false")],
    }
}

fn output_node(id: &str, input: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Output,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port(input)],
        outputs: vec![],
    }
}

fn port(id: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: "generic/json".to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: None,
    }
}

fn edge(id: &str, from_node: &str, from_port: &str, to_node: &str, to_port: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        from: WorkflowNodePortRef {
            node: from_node.to_string(),
            port: from_port.to_string(),
        },
        to: WorkflowNodePortRef {
            node: to_node.to_string(),
            port: to_port.to_string(),
        },
        artifact_type: "generic/json".to_string(),
        dataset_value: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executes_recoverable_and_fail_fast_faults() {
        let report = execute_fault_injection().expect("fault injection should execute");
        validate_report(&report).expect("fault injection report should pass");
    }

    #[test]
    fn rejects_tampered_recovery_observation() {
        let mut report = execute_fault_injection().expect("fault injection should execute");
        scenario_mut(&mut report, "recoverable_branch_isolation").observations["cascading_failure"] =
            json!(true);
        let error = validate_report(&report).expect_err("cascading failure must be rejected");
        assert!(error.contains("/cascading_failure"));
    }

    #[test]
    fn rejects_tampered_watchdog_timeout_deduplication() {
        let mut report = execute_fault_injection().expect("fault injection should execute");
        scenario_mut(&mut report, "agent_watchdog_stale_timeout").observations["duplicate_failure_created"] =
            json!(true);
        let error = validate_report(&report).expect_err("duplicate failure must be rejected");
        assert!(error.contains("/duplicate_failure_created"));
    }

    #[test]
    fn rejects_report_path_escape() {
        let error = repo_path(Path::new("/repo"), "../report.json".to_string())
            .expect_err("parent traversal must fail");
        assert!(error.contains("escapes repository"));
    }

    fn scenario_mut<'a>(report: &'a mut RecoveryReport, id: &str) -> &'a mut ScenarioReport {
        report
            .scenarios
            .iter_mut()
            .find(|scenario| scenario.id == id)
            .expect("fixture scenario should exist")
    }
}
