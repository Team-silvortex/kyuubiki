use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type RunnerResult<T> = Result<T, String>;

const REPORT_SCHEMA: &str = "kyuubiki.orchestra-process-loss-fault-injection/v1";

#[derive(Debug, Deserialize, Serialize)]
struct Report {
    schema_version: String,
    status: String,
    scenario_count: usize,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Scenario {
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

pub(crate) fn run_check_orchestra_recovery_fault_injection(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(root, args)?;
    if let Some(path) = options.verify_report {
        validate_report(&read_report(&path)?)?;
        println!(
            "Orchestra process-loss fault injection report passed: {}",
            path.display()
        );
        return Ok(0);
    }

    let temporary = root.join("tmp/orchestra-process-loss-fault-injection.self-test.json");
    let path = options.out.as_deref().unwrap_or(&temporary);
    run_probe(root, path)?;
    let mut report = read_report(path)?;
    validate_report(&report)?;

    if options.self_test {
        report.scenarios[0].observations["result_retained"] = Value::Bool(false);
        if validate_report(&report).is_ok() {
            return Err("Orchestra recovery self-test accepted tampered evidence".to_string());
        }
        let _ = fs::remove_file(path);
        println!("Orchestra process-loss fault injection self-test passed");
    } else {
        println!(
            "Orchestra process-loss fault injection passed: {}",
            path.display()
        );
    }
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
            other => return Err(format!("unknown Orchestra recovery argument: {other}")),
        }
    }
    if options.out.is_some() && options.verify_report.is_some() {
        return Err("--out and --verify-report cannot be combined".to_string());
    }
    Ok(options)
}

fn required_path(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    iter.next()
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{flag} requires a repository-relative path"))
}

fn repo_path(root: &Path, relative: String) -> RunnerResult<PathBuf> {
    let path = Path::new(&relative);
    if path.is_absolute() || path.components().any(|part| part.as_os_str() == "..") {
        return Err(format!(
            "Orchestra recovery report path escapes repository: {relative}"
        ));
    }
    Ok(root.join(path))
}

fn run_probe(root: &Path, path: &Path) -> RunnerResult<()> {
    let status = Command::new("mix")
        .current_dir(root.join("apps/web"))
        .env("MIX_ENV", "test")
        .args([
            "kyuubiki.orchestra_recovery_probe",
            "--out",
            path.to_string_lossy().as_ref(),
        ])
        .status()
        .map_err(|error| format!("failed to launch Orchestra recovery probe: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("Orchestra recovery probe exited with {status}"))
    }
}

fn validate_report(report: &Report) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.status != "pass"
        || report.scenario_count != 3
        || report.scenarios.len() != 3
    {
        return Err("Orchestra recovery report summary is invalid".to_string());
    }
    let ids = report
        .scenarios
        .iter()
        .map(|scenario| scenario.id.as_str())
        .collect::<BTreeSet<_>>();
    if ids
        != BTreeSet::from([
            "checkpointed_side_effect_process_loss_failover",
            "idempotent_task_process_loss_failover",
            "side_effect_replay_blocked_without_checkpoint",
        ])
    {
        return Err("Orchestra recovery report scenario set is invalid".to_string());
    }
    if report
        .scenarios
        .iter()
        .any(|scenario| scenario.status != "pass")
    {
        return Err("Orchestra recovery report contains a failed scenario".to_string());
    }

    validate_recovery(
        scenario(report, "idempotent_task_process_loss_failover")?,
        "idempotent",
        true,
        "retry_next_agent",
    )?;
    let idempotent = scenario(report, "idempotent_task_process_loss_failover")?;
    require_bool(idempotent, "/failed_agent_received_request", true)?;
    require_bool(idempotent, "/fallback_agent_received_request", true)?;
    require_bool(idempotent, "/result_retained", true)?;

    validate_recovery(
        scenario(report, "side_effect_replay_blocked_without_checkpoint")?,
        "checkpoint_required",
        false,
        "checkpoint_before_retry",
    )?;
    let blocked = scenario(report, "side_effect_replay_blocked_without_checkpoint")?;
    require_bool(blocked, "/failed_agent_received_request", true)?;
    require_bool(blocked, "/fallback_agent_received_request", false)?;
    require_bool(blocked, "/duplicate_side_effect_prevented", true)?;

    validate_recovery(
        scenario(report, "checkpointed_side_effect_process_loss_failover")?,
        "checkpointed",
        true,
        "retry_next_agent",
    )?;
    let checkpointed = scenario(report, "checkpointed_side_effect_process_loss_failover")?;
    require_bool(checkpointed, "/failed_agent_received_request", true)?;
    require_bool(checkpointed, "/fallback_agent_received_request", true)?;
    require_bool(checkpointed, "/checkpointed_result_retained", true)?;
    require_nonempty(checkpointed, "/recovery/checkpoint_digest")?;
    Ok(())
}

fn validate_recovery(
    scenario: &Scenario,
    retry_safety: &str,
    retryable: bool,
    next_action: &str,
) -> RunnerResult<()> {
    require_value(scenario, "/recovery/reason_code", "agent_process_lost")?;
    require_value(scenario, "/recovery/failure_stage", "receive")?;
    require_value(scenario, "/recovery/retry_safety", retry_safety)?;
    require_value(scenario, "/recovery/next_action", next_action)?;
    require_bool(scenario, "/recovery/process_loss", true)?;
    require_bool(scenario, "/recovery/retryable", retryable)?;
    require_bool(scenario, "/recovery/safe_to_continue_other_tasks", true)
}

fn scenario<'a>(report: &'a Report, id: &str) -> RunnerResult<&'a Scenario> {
    report
        .scenarios
        .iter()
        .find(|scenario| scenario.id == id)
        .ok_or_else(|| format!("Orchestra recovery report misses scenario {id}"))
}

fn require_bool(scenario: &Scenario, pointer: &str, expected: bool) -> RunnerResult<()> {
    if scenario
        .observations
        .pointer(pointer)
        .and_then(Value::as_bool)
        == Some(expected)
    {
        Ok(())
    } else {
        Err(format!(
            "Orchestra recovery scenario {} {pointer} must be {expected}",
            scenario.id
        ))
    }
}

fn require_value(scenario: &Scenario, pointer: &str, expected: &str) -> RunnerResult<()> {
    if scenario
        .observations
        .pointer(pointer)
        .and_then(Value::as_str)
        == Some(expected)
    {
        Ok(())
    } else {
        Err(format!(
            "Orchestra recovery scenario {} {pointer} must be {expected}",
            scenario.id
        ))
    }
}

fn require_nonempty(scenario: &Scenario, pointer: &str) -> RunnerResult<()> {
    if scenario
        .observations
        .pointer(pointer)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.is_empty())
    {
        Ok(())
    } else {
        Err(format!(
            "Orchestra recovery scenario {} {pointer} must be non-empty",
            scenario.id
        ))
    }
}

fn read_report(path: &Path) -> RunnerResult<Report> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "failed to read Orchestra recovery report {}: {error}",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "invalid Orchestra recovery report {}: {error}",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_path_escape() {
        let error = repo_path(Path::new("/repo"), "../report.json".to_string()).unwrap_err();
        assert!(error.contains("escapes repository"));
    }

    #[test]
    fn rejects_missing_process_loss_evidence() {
        let report: Report = serde_json::from_value(fixture()).unwrap();
        let error = validate_report(&report).unwrap_err();
        assert!(error.contains("process_loss"));
    }

    fn fixture() -> Value {
        let ids = [
            "idempotent_task_process_loss_failover",
            "side_effect_replay_blocked_without_checkpoint",
            "checkpointed_side_effect_process_loss_failover",
        ];
        json!({
            "schema_version": REPORT_SCHEMA,
            "status": "pass",
            "scenario_count": 3,
            "scenarios": ids.map(|id| json!({
                "id": id,
                "status": "pass",
                "injected_fault": "fixture",
                "recovery_policy": "fixture",
                "observations": {"recovery": {}}
            }))
        })
    }
}
