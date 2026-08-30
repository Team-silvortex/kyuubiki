use super::long_workflow_fixture::workflow_probe;
use super::report::CleanupEvidence as OrchestraCleanupEvidence;
use super::runtime::{ProcessRole, Session as OrchestraSession};
use super::runtime_http::{get_json, post_json};
use crate::distributed_task_recovery_operational_qualification::agent_process::ManagedAgents;
use crate::operational_agent_support::{
    connection_profile, query_agent_descriptor_value, remove_local_work_root, rpc_request,
    wait_endpoint_closed,
};
use crate::qualification_support::generated_at_unix_ms;
use kyuubiki_protocol::{RPC_VERSION, RpcMethod, RpcRequest};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

type RunnerResult<T> = Result<T, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct WorkflowObservation {
    pub(super) job_status: String,
    pub(super) recovery_state: String,
    pub(super) retry_safety: String,
    pub(super) generation: u64,
    pub(super) attempt: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) struct AgentCounters {
    pub(super) started: u64,
    pub(super) completed: u64,
    pub(super) failed: u64,
    pub(super) active: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct IdempotentTakeoverEvidence {
    pub(super) before_loss: WorkflowObservation,
    pub(super) after_takeover: WorkflowObservation,
    pub(super) terminal: WorkflowObservation,
    pub(super) takeover_elapsed_ms: u128,
    pub(super) takeover_fencing_token: u64,
    pub(super) agent_counters_before: AgentCounters,
    pub(super) agent_counters_after: AgentCounters,
    pub(super) active_execution_count_before_loss: u64,
    pub(super) active_execution_count_after_takeover: u64,
    pub(super) agent_started_delta: u64,
    pub(super) initial_claim_seen: bool,
    pub(super) restart_claim_seen: bool,
    pub(super) completed_history_count: u64,
    pub(super) result_verified: bool,
    pub(super) terminal_state_stable: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct CheckpointBlockedEvidence {
    pub(super) before_loss: WorkflowObservation,
    pub(super) after_takeover: WorkflowObservation,
    pub(super) takeover_elapsed_ms: u128,
    pub(super) takeover_fencing_token: u64,
    pub(super) agent_counters_before: AgentCounters,
    pub(super) agent_counters_at_block: AgentCounters,
    pub(super) agent_counters_after_release: AgentCounters,
    pub(super) active_execution_count_before_loss: u64,
    pub(super) active_execution_count_at_block: u64,
    pub(super) agent_started_delta_at_block: u64,
    pub(super) recovery_block_reason_retained: bool,
    pub(super) no_recovery_redispatch: bool,
    pub(super) remained_blocked_after_orphan_completion: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct JourneyEvidence {
    pub(super) orchestra_platform: String,
    pub(super) database_architecture: String,
    pub(super) agent_architecture: String,
    pub(super) initial_fencing_token: u64,
    pub(super) first_takeover_fencing_token: u64,
    pub(super) second_takeover_fencing_token: u64,
    pub(super) former_primary_rejoined_standby: bool,
    pub(super) former_standby_rejoined_standby: bool,
    pub(super) idempotent: IdempotentTakeoverEvidence,
    pub(super) checkpoint_required: CheckpointBlockedEvidence,
    pub(super) followup_solver_verified: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct CleanupEvidence {
    pub(super) orchestra: OrchestraCleanupEvidence,
    pub(super) remote_agent_stopped: bool,
    pub(super) remote_agent_port_closed: bool,
    pub(super) managed_remote_agent_root_removed: bool,
    pub(super) local_agent_work_root_removed: bool,
}

pub(super) fn capture(
    root: &Path,
    host: &str,
    postgres_image: &str,
    timeout: Duration,
) -> RunnerResult<(JourneyEvidence, CleanupEvidence)> {
    let mut session = Session::new(root, host, postgres_image, timeout)?;
    let journey = session.run();
    let diagnostics = journey
        .as_ref()
        .err()
        .map(|_| session.diagnostics())
        .unwrap_or_default();
    let cleanup = session.cleanup();
    match (journey, cleanup) {
        (Ok(journey), Ok(cleanup)) => Ok((journey, cleanup)),
        (Err(error), Ok(_)) => Err(with_diagnostics(error, diagnostics)),
        (Ok(_), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => Err(format!(
            "{}; cleanup: {cleanup}",
            with_diagnostics(error, diagnostics)
        )),
    }
}

struct Session {
    timeout: Duration,
    agent_work_root: PathBuf,
    agent_address: Option<SocketAddr>,
    agent_seed: u16,
    agent_architecture: String,
    agents: ManagedAgents,
    orchestra: OrchestraSession,
    cleaned: bool,
}

impl Session {
    fn new(root: &Path, host: &str, postgres_image: &str, timeout: Duration) -> RunnerResult<Self> {
        let connection = connection_profile(root, host)?;
        let nonce = generated_at_unix_ms()?;
        let suffix = format!("{nonce}-{}", std::process::id());
        let agent_work_root = root.join(format!("tmp/long-workflow-takeover-agent-{suffix}"));
        fs::create_dir_all(&agent_work_root)
            .map_err(|error| format!("failed to create Agent work root: {error}"))?;
        let run_root = format!("~/.kyuubiki/lab-runs/long-workflow-takeover-{suffix}");
        let agents = ManagedAgents::new(
            root,
            host,
            run_root,
            agent_work_root.clone(),
            connection.remote_ip,
        )?;
        let orchestra = OrchestraSession::new(root, host, postgres_image, timeout)?;
        Ok(Self {
            timeout,
            agent_work_root,
            agent_address: None,
            agent_seed: (nonce % u128::from(u16::MAX)) as u16,
            agent_architecture: connection.remote_architecture,
            agents,
            orchestra,
            cleaned: false,
        })
    }

    fn run(&mut self) -> RunnerResult<JourneyEvidence> {
        self.agents.prepare_remote_only(self.agent_seed)?;
        let agent_address = self.agents.remote_address()?;
        self.agent_address = Some(agent_address);
        wait_agent_ready(agent_address, self.timeout)?;
        self.orchestra.agent_endpoints = format!(
            "long-workflow-agent@{}:{}",
            agent_address.ip(),
            agent_address.port()
        );
        let primary_id = self.orchestra.primary_id.clone();
        let standby_id = self.orchestra.standby_id.clone();
        let (initial_owner, _initial_standby) = self.orchestra.initialize_cluster()?;
        let idempotent = self.run_idempotent(&standby_id)?;
        self.orchestra.start_orchestra(ProcessRole::Primary, true)?;
        let primary_rejoin =
            self.orchestra
                .wait_for_lease(ProcessRole::Primary, "standby", &standby_id)?;

        let checkpoint_required = self.run_checkpoint_required(&primary_id)?;
        self.orchestra.start_orchestra(ProcessRole::Standby, true)?;
        let standby_rejoin =
            self.orchestra
                .wait_for_lease(ProcessRole::Standby, "standby", &primary_id)?;
        let followup_solver_verified = verify_followup_solver(agent_address)?;

        Ok(JourneyEvidence {
            orchestra_platform: std::env::consts::OS.to_string(),
            database_architecture: self.orchestra.database_architecture.clone(),
            agent_architecture: self.agent_architecture.clone(),
            initial_fencing_token: initial_owner.fencing_token,
            first_takeover_fencing_token: idempotent.takeover_fencing_token,
            second_takeover_fencing_token: checkpoint_required.takeover_fencing_token,
            former_primary_rejoined_standby: primary_rejoin.status == "standby",
            former_standby_rejoined_standby: standby_rejoin.status == "standby",
            idempotent,
            checkpoint_required,
            followup_solver_verified,
        })
    }

    fn run_idempotent(&mut self, standby_id: &str) -> RunnerResult<IdempotentTakeoverEvidence> {
        let agent_address = self.required_agent_address()?;
        let counters_before = agent_counters(agent_address)?;
        self.agents.pause_remote()?;
        let job_id = submit_workflow(
            self.orchestra.primary_port,
            &self.orchestra.token,
            workflow_probe("idempotent", "idempotent"),
        )?;
        self.agents.hold_remote_execution(&job_id)?;
        self.agents.resume_remote()?;
        let active_before = wait_agent_job_count(agent_address, &job_id, 1, self.timeout)?;
        let before_loss = wait_workflow(
            self.orchestra.primary_port,
            &self.orchestra.token,
            &job_id,
            |value| workflow_matches(value, "solving", "running", 1, 1),
            self.timeout,
            "initial idempotent execution",
        )?;

        self.orchestra.crash(ProcessRole::Primary)?;
        let started = Instant::now();
        let takeover = self
            .orchestra
            .wait_for_lease(ProcessRole::Standby, "owner", standby_id)?;
        let after_takeover = wait_workflow(
            self.orchestra.standby_port,
            &self.orchestra.token,
            &job_id,
            |value| workflow_matches(value, "solving", "running", 2, 2),
            self.timeout,
            "idempotent replay",
        )?;
        let active_after = wait_agent_job_count(agent_address, &job_id, 2, self.timeout)?;
        let takeover_elapsed_ms = started.elapsed().as_millis();
        self.agents.release_remote_execution()?;
        let terminal_value = wait_workflow_value(
            self.orchestra.standby_port,
            &self.orchestra.token,
            &job_id,
            |value| workflow_matches(value, "completed", "completed", 2, 2),
            self.timeout,
            "idempotent terminal result",
        )?;
        wait_agent_job_count(agent_address, &job_id, 0, self.timeout)?;
        let counters_after = agent_counters(agent_address)?;
        let history = recovery_history(&terminal_value);
        let terminal = workflow_observation(&terminal_value)?;
        thread::sleep(Duration::from_millis(250));
        let stable = fetch_workflow(self.orchestra.standby_port, &self.orchestra.token, &job_id)?;

        if takeover.fencing_token <= 1 {
            return Err("idempotent takeover did not advance the lease token".to_string());
        }
        Ok(IdempotentTakeoverEvidence {
            before_loss,
            after_takeover,
            terminal,
            takeover_elapsed_ms,
            takeover_fencing_token: takeover.fencing_token,
            agent_counters_before: counters_before,
            agent_counters_after: counters_after,
            active_execution_count_before_loss: active_before,
            active_execution_count_after_takeover: active_after,
            agent_started_delta: counters_after
                .started
                .saturating_sub(counters_before.started),
            initial_claim_seen: history_event(&history, "claimed", Some("initial")),
            restart_claim_seen: history_event(&history, "claimed", Some("process_restart")),
            completed_history_count: history_event_count(&history, "completed"),
            result_verified: verify_bar_result(&terminal_value),
            terminal_state_stable: workflow_matches(&stable, "completed", "completed", 2, 2),
        })
    }

    fn run_checkpoint_required(
        &mut self,
        primary_id: &str,
    ) -> RunnerResult<CheckpointBlockedEvidence> {
        let agent_address = self.required_agent_address()?;
        let counters_before = agent_counters(agent_address)?;
        self.agents.pause_remote()?;
        let job_id = submit_workflow(
            self.orchestra.standby_port,
            &self.orchestra.token,
            workflow_probe("checkpoint-required", "checkpoint_required"),
        )?;
        self.agents.hold_remote_execution(&job_id)?;
        self.agents.resume_remote()?;
        let active_before = wait_agent_job_count(agent_address, &job_id, 1, self.timeout)?;
        let before_loss = wait_workflow(
            self.orchestra.standby_port,
            &self.orchestra.token,
            &job_id,
            |value| workflow_matches(value, "solving", "running", 1, 1),
            self.timeout,
            "initial checkpoint-required execution",
        )?;

        self.orchestra.crash(ProcessRole::Standby)?;
        let started = Instant::now();
        let takeover = self
            .orchestra
            .wait_for_lease(ProcessRole::Primary, "owner", primary_id)?;
        let blocked_value = wait_workflow_value(
            self.orchestra.primary_port,
            &self.orchestra.token,
            &job_id,
            |value| workflow_matches(value, "failed", "recovery_blocked", 1, 1),
            self.timeout,
            "checkpoint-required recovery block",
        )?;
        let counters_at_block = agent_counters(agent_address)?;
        let active_at_block = active_agent_job_count(agent_address, &job_id)?;
        let takeover_elapsed_ms = started.elapsed().as_millis();
        self.agents.release_remote_execution()?;
        wait_agent_job_count(agent_address, &job_id, 0, self.timeout)?;
        let counters_after_release = agent_counters(agent_address)?;
        thread::sleep(Duration::from_millis(250));
        let stable = fetch_workflow(self.orchestra.primary_port, &self.orchestra.token, &job_id)?;
        let history = recovery_history(&blocked_value);
        let started_delta = counters_at_block
            .started
            .saturating_sub(counters_before.started);

        Ok(CheckpointBlockedEvidence {
            before_loss,
            after_takeover: workflow_observation(&blocked_value)?,
            takeover_elapsed_ms,
            takeover_fencing_token: takeover.fencing_token,
            agent_counters_before: counters_before,
            agent_counters_at_block: counters_at_block,
            agent_counters_after_release: counters_after_release,
            active_execution_count_before_loss: active_before,
            active_execution_count_at_block: active_at_block,
            agent_started_delta_at_block: started_delta,
            recovery_block_reason_retained: history.iter().any(|event| {
                event.get("event").and_then(Value::as_str) == Some("recovery_blocked")
                    && event
                        .get("reason")
                        .and_then(Value::as_str)
                        .is_some_and(|reason| reason.contains("checkpoint_required"))
            }),
            no_recovery_redispatch: started_delta == 1,
            remained_blocked_after_orphan_completion: workflow_matches(
                &stable,
                "failed",
                "recovery_blocked",
                1,
                1,
            ),
        })
    }

    fn cleanup(&mut self) -> RunnerResult<CleanupEvidence> {
        if self.cleaned {
            return Err("long workflow takeover session was already cleaned".to_string());
        }
        let mut errors = Vec::new();
        let orchestra = self.orchestra.cleanup().map_err(|error| {
            errors.push(error);
        });
        let remote_agent_stopped =
            self.agents
                .stop_remote()
                .map(|_| true)
                .unwrap_or_else(|error| {
                    errors.push(error);
                    false
                });
        let remote_agent_port_closed = self
            .agent_address
            .is_none_or(|address| wait_endpoint_closed(address, Duration::from_secs(5)).is_ok());
        if !remote_agent_port_closed {
            errors.push("remote Agent qualification port remained open".to_string());
        }
        let managed_remote_agent_root_removed =
            self.agents.remove_remote_root().unwrap_or_else(|error| {
                errors.push(error);
                false
            });
        let local_agent_work_root_removed = remove_local_work_root(&self.agent_work_root)
            .unwrap_or_else(|error| {
                errors.push(error);
                false
            });
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
        self.cleaned = true;
        Ok(CleanupEvidence {
            orchestra: orchestra.map_err(|_| "Orchestra cleanup failed".to_string())?,
            remote_agent_stopped,
            remote_agent_port_closed,
            managed_remote_agent_root_removed,
            local_agent_work_root_removed,
        })
    }

    fn required_agent_address(&self) -> RunnerResult<SocketAddr> {
        self.agent_address
            .ok_or_else(|| "remote Agent address is unavailable".to_string())
    }

    fn diagnostics(&self) -> String {
        let mut sections = Vec::new();
        for name in [
            "orchestra-primary.log",
            "orchestra-standby.log",
            "orchestra-former-owner.log",
        ] {
            let path = self.orchestra.work_root.join(name);
            if let Ok(log) = fs::read_to_string(&path) {
                sections.push(format!("{name}: {}", tail_lines(&log, 40)));
            }
        }
        if let Ok(log) = self.agents.remote_log_tail() {
            if !log.is_empty() {
                sections.push(format!("remote-agent: {log}"));
            }
        }
        sections.join(" | ")
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.cleanup();
        }
    }
}

fn submit_workflow(port: u16, token: &str, body: Value) -> RunnerResult<String> {
    let response = post_json(port, "/api/v1/workflows/graph/jobs", token, &body)?;
    if response.status != 202 {
        return Err(format!(
            "workflow submission returned status {}: {}",
            response.status, response.body
        ));
    }
    response
        .body
        .pointer("/job/job_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "workflow submission omitted job_id".to_string())
}

fn fetch_workflow(port: u16, token: &str, job_id: &str) -> RunnerResult<Value> {
    get_json(port, &format!("/api/v1/jobs/{job_id}"), token)
}

fn wait_workflow(
    port: u16,
    token: &str,
    job_id: &str,
    predicate: impl Fn(&Value) -> bool,
    timeout: Duration,
    label: &str,
) -> RunnerResult<WorkflowObservation> {
    wait_workflow_value(port, token, job_id, predicate, timeout, label)
        .and_then(|value| workflow_observation(&value))
}

fn wait_workflow_value(
    port: u16,
    token: &str,
    job_id: &str,
    predicate: impl Fn(&Value) -> bool,
    timeout: Duration,
    label: &str,
) -> RunnerResult<Value> {
    let deadline = Instant::now() + timeout;
    let mut last_observation = "none".to_string();
    let mut last_error = "none".to_string();
    while Instant::now() < deadline {
        match fetch_workflow(port, token, job_id) {
            Ok(value) => {
                last_observation = format!("{:?}", workflow_observation(&value));
                if predicate(&value) {
                    return Ok(value);
                }
                if value
                    .pointer("/job/status")
                    .and_then(Value::as_str)
                    .is_some_and(|status| matches!(status, "completed" | "failed" | "cancelled"))
                {
                    let status = value
                        .pointer("/job/status")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown");
                    let message = value
                        .pointer("/job/message")
                        .and_then(Value::as_str)
                        .unwrap_or("no job message");
                    return Err(format!(
                        "{label} reached unexpected terminal state {status}: {message} ({last_observation})"
                    ));
                }
            }
            Err(error) => last_error = error,
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(format!(
        "{label} timed out (last observation: {last_observation}; last error: {last_error})"
    ))
}

fn tail_lines(value: &str, maximum: usize) -> String {
    let lines = value.lines().collect::<Vec<_>>();
    lines[lines.len().saturating_sub(maximum)..].join("\\n")
}

fn with_diagnostics(error: String, diagnostics: String) -> String {
    if diagnostics.is_empty() {
        error
    } else {
        format!("{error}; diagnostics: {diagnostics}")
    }
}

fn workflow_observation(value: &Value) -> RunnerResult<WorkflowObservation> {
    Ok(WorkflowObservation {
        job_status: required_str(value, "/job/status")?.to_string(),
        recovery_state: required_str(value, "/result/recovery/state")?.to_string(),
        retry_safety: required_str(value, "/result/recovery/retry_safety")?.to_string(),
        generation: required_u64(value, "/result/recovery/generation")?,
        attempt: required_u64(value, "/result/recovery/attempt")?,
    })
}

fn workflow_matches(
    value: &Value,
    job: &str,
    recovery: &str,
    generation: u64,
    attempt: u64,
) -> bool {
    value.pointer("/job/status").and_then(Value::as_str) == Some(job)
        && value
            .pointer("/result/recovery/state")
            .and_then(Value::as_str)
            == Some(recovery)
        && value
            .pointer("/result/recovery/generation")
            .and_then(Value::as_u64)
            == Some(generation)
        && value
            .pointer("/result/recovery/attempt")
            .and_then(Value::as_u64)
            == Some(attempt)
}

fn recovery_history(value: &Value) -> Vec<Value> {
    value
        .pointer("/result/recovery/history")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn history_event(history: &[Value], kind: &str, reason: Option<&str>) -> bool {
    history.iter().any(|event| {
        event.get("event").and_then(Value::as_str) == Some(kind)
            && reason.is_none_or(|expected| {
                event.get("reason").and_then(Value::as_str) == Some(expected)
            })
    })
}

fn history_event_count(history: &[Value], kind: &str) -> u64 {
    history
        .iter()
        .filter(|event| event.get("event").and_then(Value::as_str) == Some(kind))
        .count() as u64
}

fn wait_agent_ready(address: SocketAddr, timeout: Duration) -> RunnerResult<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if query_agent_descriptor_value(address)
            .ok()
            .and_then(|value| value.get("program").cloned())
            .and_then(|value| value.as_str().map(str::to_string))
            .as_deref()
            == Some("kyuubiki-rust-agent")
        {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err("remote Agent readiness timed out".to_string())
}

fn wait_agent_job_count(
    address: SocketAddr,
    job_id: &str,
    expected: u64,
    timeout: Duration,
) -> RunnerResult<u64> {
    let deadline = Instant::now() + timeout;
    let mut last = 0;
    while Instant::now() < deadline {
        if let Ok(count) = active_agent_job_count(address, job_id) {
            last = count;
            if count == expected {
                return Ok(count);
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err(format!(
        "Agent job {job_id} active count did not reach {expected} (last {last})"
    ))
}

fn active_agent_job_count(address: SocketAddr, job_id: &str) -> RunnerResult<u64> {
    let descriptor = query_agent_descriptor_value(address)?;
    Ok(descriptor
        .pointer("/watchdog/active_executions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|execution| execution.get("job_id").and_then(Value::as_str) == Some(job_id))
        .count() as u64)
}

fn agent_counters(address: SocketAddr) -> RunnerResult<AgentCounters> {
    let descriptor = query_agent_descriptor_value(address)?;
    Ok(AgentCounters {
        started: required_u64(&descriptor, "/watchdog/total_started_execution_count")?,
        completed: required_u64(&descriptor, "/watchdog/total_completed_execution_count")?,
        failed: required_u64(&descriptor, "/watchdog/total_failed_execution_count")?,
        active: required_u64(&descriptor, "/watchdog/active_execution_count")?,
    })
}

fn verify_bar_result(value: &Value) -> bool {
    find_number(value, "max_stress").is_some_and(|stress| (stress - 10.0).abs() <= 1.0e-9)
        && find_number(value, "tip_displacement")
            .is_some_and(|displacement| (displacement - 0.01).abs() <= 1.0e-12)
}

fn find_number(value: &Value, key: &str) -> Option<f64> {
    match value {
        Value::Object(entries) => entries
            .get(key)
            .and_then(Value::as_f64)
            .or_else(|| entries.values().find_map(|nested| find_number(nested, key))),
        Value::Array(entries) => entries.iter().find_map(|nested| find_number(nested, key)),
        _ => None,
    }
}

fn verify_followup_solver(address: SocketAddr) -> RunnerResult<bool> {
    let response = rpc_request(
        address,
        &RpcRequest {
            rpc_version: RPC_VERSION,
            id: "long-workflow-takeover-followup".to_string(),
            method: RpcMethod::SolveBar1d,
            params: json!({
                "length": 1.0,
                "area": 2.0,
                "youngs_modulus": 1000.0,
                "elements": 2,
                "tip_force": 20.0
            }),
        },
        Duration::from_secs(5),
    )?;
    let result = response
        .ok
        .then_some(response.result)
        .flatten()
        .ok_or("remote Agent rejected the follow-up solver")?;
    Ok(find_number(&result, "max_stress").is_some_and(|stress| (stress - 10.0).abs() <= 1.0e-9))
}

fn required_str<'a>(value: &'a Value, pointer: &str) -> RunnerResult<&'a str> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("workflow response misses {pointer}"))
}

fn required_u64(value: &Value, pointer: &str) -> RunnerResult<u64> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("runtime response misses {pointer}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recursive_result_probe_finds_nested_solver_metrics() {
        let value = json!({"result": {"artifacts": {"output": {
            "max_stress": 10.0,
            "tip_displacement": 0.01
        }}}});
        assert!(verify_bar_result(&value));
    }

    #[test]
    fn workflow_match_requires_generation_and_attempt() {
        let value = json!({
            "job": {"status": "completed"},
            "result": {"recovery": {
                "state": "completed", "generation": 2, "attempt": 2
            }}
        });
        assert!(workflow_matches(&value, "completed", "completed", 2, 2));
        assert!(!workflow_matches(&value, "completed", "completed", 1, 2));
    }
}
