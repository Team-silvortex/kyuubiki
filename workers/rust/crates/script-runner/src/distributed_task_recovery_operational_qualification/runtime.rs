use super::agent_process::ManagedAgents;
use super::probe::ProbeRun;
use crate::operational_agent_support::{
    connection_profile, query_agent_descriptor_value, remove_local_work_root, rpc_request,
    wait_endpoint_closed,
};
use crate::qualification_support::generated_at_unix_ms;
use kyuubiki_protocol::{RPC_VERSION, RpcMethod, RpcRequest};
use serde_json::{Value, json};
use std::fs;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

type RunnerResult<T> = Result<T, String>;

const SCENARIOS: [ScenarioSpec; 3] = [
    ScenarioSpec {
        id: "idempotent",
        job_id: "distributed-recovery-idempotent",
        method: "solve_bar_1d",
    },
    ScenarioSpec {
        id: "side_effect_blocked",
        job_id: "distributed-recovery-side-effect",
        method: "run_operator_task_ir",
    },
    ScenarioSpec {
        id: "checkpointed",
        job_id: "distributed-recovery-checkpointed",
        method: "run_operator_task_ir",
    },
];

#[derive(Debug, Clone, Copy)]
struct ScenarioSpec {
    id: &'static str,
    job_id: &'static str,
    method: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ExecutionCounters {
    pub(crate) started: u64,
    pub(crate) completed: u64,
    pub(crate) failed: u64,
    pub(crate) active: u64,
}

#[derive(Debug)]
pub(crate) struct ScenarioCapture {
    pub(crate) id: String,
    pub(crate) job_id: String,
    pub(crate) method: String,
    pub(crate) primary_before: ExecutionCounters,
    pub(crate) primary_inflight: ExecutionCounters,
    pub(crate) fallback_before: ExecutionCounters,
    pub(crate) fallback_after: ExecutionCounters,
    pub(crate) probe: Value,
    pub(crate) remote_rejoined: bool,
    pub(crate) followup_solver_verified: bool,
}

#[derive(Debug)]
pub(crate) struct CleanupCapture {
    pub(crate) local_agent_stopped: bool,
    pub(crate) remote_agent_stopped: bool,
    pub(crate) local_port_closed: bool,
    pub(crate) remote_port_closed: bool,
    pub(crate) managed_remote_root_removed: bool,
    pub(crate) local_work_root_removed: bool,
}

#[derive(Debug)]
pub(crate) struct Captured {
    pub(crate) remote_architecture: String,
    pub(crate) remote_restart_count: u64,
    pub(crate) scenarios: Vec<ScenarioCapture>,
    pub(crate) cleanup: CleanupCapture,
}

pub(crate) fn capture(root: &Path, host: &str, timeout: Duration) -> RunnerResult<Captured> {
    let mut session = Session::new(root, host, timeout)?;
    let journey = session.run();
    let cleanup = session.cleanup();
    match (journey, cleanup) {
        (Ok((remote_architecture, scenarios)), Ok(cleanup)) => Ok(Captured {
            remote_architecture,
            remote_restart_count: scenarios.len() as u64,
            scenarios,
            cleanup,
        }),
        (Err(error), Ok(_)) => Err(error),
        (Ok(_), Err(cleanup_error)) => Err(cleanup_error),
        (Err(error), Err(cleanup_error)) => Err(format!("{error}; cleanup: {cleanup_error}")),
    }
}

struct Session {
    root: PathBuf,
    timeout: Duration,
    work_root: PathBuf,
    remote_ip: Ipv4Addr,
    remote_architecture: String,
    agents: ManagedAgents,
    cleaned: bool,
}

impl Session {
    fn new(root: &Path, host: &str, timeout: Duration) -> RunnerResult<Self> {
        if std::env::consts::OS != "macos" {
            return Err(
                "distributed recovery capture currently requires macOS locally".to_string(),
            );
        }
        let connection = connection_profile(root, host)?;
        let nonce = generated_at_unix_ms()?;
        let run_root = format!(
            "~/.kyuubiki/lab-runs/distributed-task-recovery-{nonce}-{}",
            std::process::id()
        );
        let work_root = root.join(format!(
            "tmp/distributed-task-recovery-{nonce}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&work_root)
            .map_err(|error| format!("failed to create recovery work root: {error}"))?;
        let agents = ManagedAgents::new(
            root,
            host,
            run_root,
            work_root.clone(),
            connection.remote_ip,
        )?;
        Ok(Self {
            root: root.to_path_buf(),
            timeout,
            work_root,
            remote_ip: connection.remote_ip,
            remote_architecture: connection.remote_architecture,
            agents,
            cleaned: false,
        })
    }

    fn run(&mut self) -> RunnerResult<(String, Vec<ScenarioCapture>)> {
        let seed = (generated_at_unix_ms()? % u128::from(u16::MAX)) as u16;
        self.agents.prepare(seed)?;
        wait_agent_ready(self.agents.local_address(), self.timeout, "local fallback")?;
        wait_agent_ready(
            self.agents.remote_address()?,
            self.timeout,
            "remote primary",
        )?;

        let mut scenarios = Vec::with_capacity(SCENARIOS.len());
        for spec in SCENARIOS {
            scenarios.push(self.run_scenario(spec)?);
        }
        Ok((self.remote_architecture.clone(), scenarios))
    }

    fn run_scenario(&mut self, spec: ScenarioSpec) -> RunnerResult<ScenarioCapture> {
        if !self.agents.local_alive()? || !self.agents.remote_alive()? {
            return Err(format!("{} scenario requires both Agents alive", spec.id));
        }
        let primary_before = descriptor_counters(self.agents.remote_address()?)?;
        let fallback_before = descriptor_counters(self.agents.local_address())?;
        self.agents.hold_remote_execution(spec.job_id)?;
        let mut probe = ProbeRun::spawn(
            &self.root,
            &self.work_root,
            spec.id,
            self.remote_ip,
            self.agents.remote_port()?,
            self.agents.local_port(),
        )?;
        let primary_inflight = wait_for_inflight(
            &mut probe,
            self.agents.remote_address()?,
            spec,
            primary_before.started,
            self.timeout,
        )?;
        self.agents.terminate_remote_inflight()?;
        self.agents.release_remote_execution()?;
        wait_endpoint_closed(self.agents.remote_address()?, Duration::from_secs(5))?;
        let probe_report = probe.finish(self.timeout)?;
        let fallback_after = descriptor_counters(self.agents.local_address())?;

        self.agents.restart_remote()?;
        wait_agent_ready(
            self.agents.remote_address()?,
            self.timeout,
            "rejoined remote primary",
        )?;
        let remote_rejoined = self.agents.remote_alive()?;
        let followup_solver_verified = verify_followup_solver(self.agents.remote_address()?)?;

        Ok(ScenarioCapture {
            id: spec.id.to_string(),
            job_id: spec.job_id.to_string(),
            method: spec.method.to_string(),
            primary_before,
            primary_inflight,
            fallback_before,
            fallback_after,
            probe: probe_report,
            remote_rejoined,
            followup_solver_verified,
        })
    }

    fn cleanup(&mut self) -> RunnerResult<CleanupCapture> {
        if self.cleaned {
            return Err("distributed recovery session was already cleaned".to_string());
        }
        let mut errors = Vec::new();
        let remote_address = self.agents.remote_address().ok();
        let local_address = self.agents.local_address();
        let remote_agent_stopped =
            self.agents
                .stop_remote()
                .map(|_| true)
                .unwrap_or_else(|error| {
                    errors.push(error);
                    false
                });
        let local_agent_stopped = self
            .agents
            .stop_local()
            .map(|_| true)
            .unwrap_or_else(|error| {
                errors.push(error);
                false
            });
        let remote_port_closed = remote_address
            .map(|address| wait_endpoint_closed(address, Duration::from_secs(5)).is_ok())
            .unwrap_or(true);
        let local_port_closed = wait_endpoint_closed(local_address, Duration::from_secs(5)).is_ok();
        if !remote_port_closed {
            errors.push("remote qualification port remained open".to_string());
        }
        if !local_port_closed {
            errors.push("local qualification port remained open".to_string());
        }
        let managed_remote_root_removed =
            self.agents.remove_remote_root().unwrap_or_else(|error| {
                errors.push(error);
                false
            });
        let local_work_root_removed =
            remove_local_work_root(&self.work_root).unwrap_or_else(|error| {
                errors.push(error);
                false
            });
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
        self.cleaned = true;
        Ok(CleanupCapture {
            local_agent_stopped,
            remote_agent_stopped,
            local_port_closed,
            remote_port_closed,
            managed_remote_root_removed,
            local_work_root_removed,
        })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.cleanup();
        }
    }
}

fn wait_agent_ready(address: SocketAddr, timeout: Duration, label: &str) -> RunnerResult<()> {
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
    Err(format!("{label} Agent readiness timed out"))
}

fn wait_for_inflight(
    probe: &mut ProbeRun,
    address: SocketAddr,
    spec: ScenarioSpec,
    baseline_started: u64,
    timeout: Duration,
) -> RunnerResult<ExecutionCounters> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        probe.ensure_running()?;
        if let Ok(descriptor) = query_agent_descriptor_value(address) {
            let counters = counters_from_descriptor(&descriptor)?;
            let active = descriptor
                .pointer("/watchdog/active_executions")
                .and_then(Value::as_array)
                .is_some_and(|executions| {
                    executions.iter().any(|execution| {
                        execution.get("job_id").and_then(Value::as_str) == Some(spec.job_id)
                            && execution.get("method").and_then(Value::as_str) == Some(spec.method)
                    })
                });
            if counters.started > baseline_started && counters.active > 0 && active {
                return Ok(counters);
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err(format!(
        "{} scenario never exposed its in-flight execution",
        spec.id
    ))
}

fn descriptor_counters(address: SocketAddr) -> RunnerResult<ExecutionCounters> {
    let descriptor = query_agent_descriptor_value(address)?;
    counters_from_descriptor(&descriptor)
}

fn counters_from_descriptor(descriptor: &Value) -> RunnerResult<ExecutionCounters> {
    Ok(ExecutionCounters {
        started: required_u64(descriptor, "/watchdog/total_started_execution_count")?,
        completed: required_u64(descriptor, "/watchdog/total_completed_execution_count")?,
        failed: required_u64(descriptor, "/watchdog/total_failed_execution_count")?,
        active: required_u64(descriptor, "/watchdog/active_execution_count")?,
    })
}

fn required_u64(value: &Value, pointer: &str) -> RunnerResult<u64> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("Agent descriptor misses {pointer}"))
}

fn verify_followup_solver(address: SocketAddr) -> RunnerResult<bool> {
    let response = rpc_request(
        address,
        &RpcRequest {
            rpc_version: RPC_VERSION,
            id: "distributed-recovery-followup".to_string(),
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
        .ok_or("rejoined Agent rejected the follow-up solver request")?;
    let stress = result
        .get("max_stress")
        .and_then(Value::as_f64)
        .ok_or("follow-up solver omitted max_stress")?;
    let displacement = result
        .get("tip_displacement")
        .and_then(Value::as_f64)
        .ok_or("follow-up solver omitted tip_displacement")?;
    Ok((stress - 10.0).abs() <= 1.0e-9 && (displacement - 0.01).abs() <= 1.0e-12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_counter_parser_requires_all_monotonic_fields() {
        let descriptor = json!({
            "watchdog": {
                "total_started_execution_count": 3,
                "total_completed_execution_count": 2,
                "total_failed_execution_count": 1,
                "active_execution_count": 0
            }
        });
        let counters = counters_from_descriptor(&descriptor).expect("counter descriptor");
        assert_eq!(counters.started, 3);
        assert_eq!(counters.completed, 2);
        assert_eq!(counters.failed, 1);
    }
}
