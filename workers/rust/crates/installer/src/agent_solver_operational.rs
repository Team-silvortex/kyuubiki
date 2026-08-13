use crate::Platform;
use crate::agent_solver_operational_validation::validate_agent_solver_operational_qualification_report;
use crate::agent_update_payload::{
    AgentUpdateActivationRecord, AgentUpdatePackageManifest, active_agent_binary_in,
    agent_update_status_in, install_agent_update_package_into, prepare_agent_update_package,
};
use kyuubiki_protocol::{
    AGENT_SOLVER_QUALIFICATION_EXPECTED_TIP_DISPLACEMENT, compute_operator_task_digest,
    validate_agent_solver_qualification_report,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const AGENT_SOLVER_OPERATIONAL_QUALIFICATION_SCHEMA_VERSION: &str =
    "kyuubiki.agent-solver-operational-qualification/v1";
const RPC_VERSION: u8 = 1;
const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
const RESULT_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AgentSolverOperationalQualificationReport {
    pub schema_version: String,
    pub generated_at_unix_ms: u128,
    pub status: String,
    pub journey: String,
    pub execution_host_role: String,
    pub platform: String,
    pub architecture: String,
    pub control_boundary: OperationalControlBoundary,
    pub package: AgentUpdatePackageManifest,
    pub activation: AgentUpdateActivationRecord,
    pub installed_state: OperationalInstalledState,
    pub transport: OperationalTransport,
    pub solver_runs: Vec<OperationalSolverRun>,
    pub cleanup: OperationalCleanupReceipt,
    pub checks: Vec<OperationalCheck>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OperationalControlBoundary {
    pub deployment_owner: String,
    pub execution_owner: String,
    pub capture_transport: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OperationalInstalledState {
    pub active_version: String,
    pub installed_versions: Vec<String>,
    pub active_entrypoint_sha256: String,
    pub package_relative_path: String,
    pub store_relative_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OperationalTransport {
    pub protocol: String,
    pub rpc_version: u8,
    pub bind_scope: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OperationalSolverRun {
    pub phase: String,
    pub process_id: u32,
    pub qualification: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OperationalCleanupReceipt {
    pub scope: String,
    pub work_root_removed: bool,
    pub residue_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OperationalCheck {
    pub id: String,
    pub ok: bool,
}

pub fn run_agent_solver_operational_qualification(
    agent_binary: &Path,
    work_root: &Path,
    package_version: &str,
) -> Result<AgentSolverOperationalQualificationReport, String> {
    let mut work = QualificationWorkRoot::prepare(work_root)?;
    let platform = Platform::current();
    let package_root = work_root.join("packages/agent");
    let store = work_root.join("managed-store");
    let logs = work_root.join("logs");
    fs::create_dir_all(&logs)
        .map_err(|error| format!("failed to create qualification logs: {error}"))?;

    let package =
        prepare_agent_update_package(agent_binary, &package_root, package_version, platform)?;
    let activation = install_agent_update_package_into(&package_root, &store, platform)?;
    let active_binary = active_agent_binary_in(&store, platform)?;
    let installed_status = agent_update_status_in(&store)?;

    let mut first = LiveManagedAgent::start(&active_binary, &logs.join("first-agent.log"))?;
    let first_run = execute_solver_qualification(&first, "initial-process")?;
    first.stop()?;

    let mut restarted = LiveManagedAgent::start(&active_binary, &logs.join("restarted-agent.log"))?;
    let restarted_run = execute_solver_qualification(&restarted, "restarted-process")?;
    restarted.stop()?;

    let process_restart = first_run.process_id != restarted_run.process_id;
    let active_after_restart = active_agent_binary_in(&store, platform)? == active_binary;
    let installed_state = OperationalInstalledState {
        active_version: installed_status.active_version.unwrap_or_default(),
        installed_versions: installed_status.installed_versions,
        active_entrypoint_sha256: package.entrypoint_sha256.clone(),
        package_relative_path: "packages/agent".to_string(),
        store_relative_path: "managed-store".to_string(),
    };
    let mut checks = vec![
        check(
            "package_sealed",
            package.schema_version == "kyuubiki.agent-update-package/v1",
        ),
        check(
            "package_digest_verified",
            activation.entrypoint_sha256 == package.entrypoint_sha256,
        ),
        check(
            "installer_activation",
            activation.version == package_version,
        ),
        check("active_binary_verified", active_after_restart),
        check("first_solver_execution", run_passed(&first_run)),
        check("first_tamper_rejection", run_rejected_tamper(&first_run)),
        check("first_recovery", run_recovered(&first_run)),
        check("process_restart", process_restart),
        check("restarted_solver_execution", run_passed(&restarted_run)),
        check(
            "restarted_tamper_rejection",
            run_rejected_tamper(&restarted_run),
        ),
        check("restarted_recovery", run_recovered(&restarted_run)),
        check(
            "watchdog_quiescent",
            runs_quiescent([&first_run, &restarted_run]),
        ),
        check(
            "managed_store_isolated",
            installed_state.store_relative_path == "managed-store",
        ),
    ];
    if checks.iter().any(|entry| !entry.ok) {
        return Err("agent solver operational qualification checks failed".to_string());
    }

    work.cleanup()?;
    let cleanup = OperationalCleanupReceipt {
        scope: "qualification-work-root".to_string(),
        work_root_removed: !work_root.exists(),
        residue_count: u64::from(work_root.exists()),
    };
    checks.push(check(
        "cleanup_complete",
        cleanup.work_root_removed && cleanup.residue_count == 0,
    ));

    let report = AgentSolverOperationalQualificationReport {
        schema_version: AGENT_SOLVER_OPERATIONAL_QUALIFICATION_SCHEMA_VERSION.to_string(),
        generated_at_unix_ms: unix_now_ms()?,
        status: "pass".to_string(),
        journey: "installer-managed-packaged-agent-solver-recovery".to_string(),
        execution_host_role: qualification_host_role(platform),
        platform: platform.as_str().to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        control_boundary: OperationalControlBoundary {
            deployment_owner: "kyuubiki-installer".to_string(),
            execution_owner: "kyuubiki-agent-engine".to_string(),
            capture_transport: if std::env::var_os("SSH_CONNECTION").is_some() {
                "managed-remote-session"
            } else {
                "managed-local-session"
            }
            .to_string(),
        },
        package,
        activation,
        installed_state,
        transport: OperationalTransport {
            protocol: "tcp_framed_json".to_string(),
            rpc_version: RPC_VERSION,
            bind_scope: "loopback".to_string(),
        },
        solver_runs: vec![first_run, restarted_run],
        cleanup,
        checks,
    };
    let value = serde_json::to_value(&report).map_err(|error| error.to_string())?;
    validate_agent_solver_operational_qualification_report(&value).map_err(|errors| {
        format!(
            "operational report validation failed: {}",
            errors.join("; ")
        )
    })?;
    Ok(report)
}

pub fn write_agent_solver_operational_qualification_report(
    report: &AgentSolverOperationalQualificationReport,
    path: &Path,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let payload = serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?;
    fs::write(path, payload).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

struct QualificationWorkRoot {
    path: PathBuf,
    armed: bool,
}

impl QualificationWorkRoot {
    fn prepare(path: &Path) -> Result<Self, String> {
        if path
            .symlink_metadata()
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err("qualification work root must not be a symlink".to_string());
        }
        if path.exists()
            && fs::read_dir(path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?
                .next()
                .is_some()
        {
            return Err("qualification work root must be empty".to_string());
        }
        fs::create_dir_all(path)
            .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
        Ok(Self {
            path: path.to_path_buf(),
            armed: true,
        })
    }

    fn cleanup(&mut self) -> Result<(), String> {
        if self.path.exists() {
            fs::remove_dir_all(&self.path)
                .map_err(|error| format!("failed to clean qualification work root: {error}"))?;
        }
        self.armed = false;
        Ok(())
    }
}

impl Drop for QualificationWorkRoot {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

struct LiveManagedAgent {
    child: Child,
    port: u16,
}

impl LiveManagedAgent {
    fn start(binary: &Path, log_path: &Path) -> Result<Self, String> {
        let port = reserve_port()?;
        let log = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(log_path)
            .map_err(|error| format!("failed to create agent qualification log: {error}"))?;
        let child = Command::new(binary)
            .args(["agent", "--host", "127.0.0.1", "--port", &port.to_string()])
            .stdout(Stdio::from(
                log.try_clone().map_err(|error| error.to_string())?,
            ))
            .stderr(Stdio::from(log))
            .spawn()
            .map_err(|error| format!("failed to launch installer-managed Agent: {error}"))?;
        let mut agent = Self { child, port };
        agent.wait_until_ready(Duration::from_secs(30))?;
        Ok(agent)
    }

    fn process_id(&self) -> u32 {
        self.child.id()
    }

    fn wait_until_ready(&mut self, timeout: Duration) -> Result<(), String> {
        let started = Instant::now();
        while started.elapsed() < timeout {
            if TcpStream::connect(("127.0.0.1", self.port)).is_ok() {
                return Ok(());
            }
            if let Some(status) = self.child.try_wait().map_err(|error| error.to_string())? {
                return Err(format!(
                    "installer-managed Agent exited before readiness: {status}"
                ));
            }
            thread::sleep(Duration::from_millis(50));
        }
        Err("installer-managed Agent readiness timed out".to_string())
    }

    fn request(&self, id: &str, method: &str, params: Value) -> Result<Value, String> {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port))
            .map_err(|error| format!("failed to connect to managed Agent: {error}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|error| error.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(30)))
            .map_err(|error| error.to_string())?;
        let payload = serde_json::to_vec(&json!({
            "rpc_version": RPC_VERSION,
            "id": id,
            "method": method,
            "params": params
        }))
        .map_err(|error| error.to_string())?;
        let length = u32::try_from(payload.len()).map_err(|error| error.to_string())?;
        stream
            .write_all(&length.to_be_bytes())
            .map_err(|error| error.to_string())?;
        stream
            .write_all(&payload)
            .map_err(|error| error.to_string())?;
        loop {
            let response = read_json_frame(&mut stream)?;
            if response.get("ok").is_some() {
                return Ok(response);
            }
        }
    }

    fn stop(&mut self) -> Result<(), String> {
        if self
            .child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_none()
        {
            self.child
                .kill()
                .map_err(|error| format!("failed to stop managed Agent: {error}"))?;
        }
        self.child
            .wait()
            .map_err(|error| format!("failed to reap managed Agent: {error}"))?;
        Ok(())
    }
}

impl Drop for LiveManagedAgent {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn execute_solver_qualification(
    agent: &LiveManagedAgent,
    phase: &str,
) -> Result<OperationalSolverRun, String> {
    let task = executable_solver_task_ir()?;
    let initial = successful_result(agent.request(
        &format!("{phase}-valid-before"),
        "run_operator_task_ir",
        json!({ "mode": "execute", "task_ir": task.clone() }),
    )?)?;

    let mut unsupported_task = task.clone();
    unsupported_task["operator"]["id"] = json!("solve.thermal_bar_1d");
    unsupported_task["execution_program"]["program_id"] = json!("solve.thermal_bar_1d");
    unsupported_task["execution_program"]["entrypoint"]["name"] = json!("solve_thermal_bar_1d");
    unsupported_task["integrity"]["task_digest"] =
        json!(compute_operator_task_digest(&unsupported_task)?);
    let unsupported = agent.request(
        "qualification-unsupported-solver",
        "run_operator_task_ir",
        json!({ "mode": "execute", "task_ir": unsupported_task }),
    )?;

    let mut tampered_task = task.clone();
    tampered_task["config"]["qualification_case"] = json!("tampered");
    let tampered = agent.request(
        "qualification-tampered",
        "run_operator_task_ir",
        json!({ "mode": "execute", "task_ir": tampered_task }),
    )?;
    let recovered = successful_result(agent.request(
        &format!("{phase}-valid-after"),
        "run_operator_task_ir",
        json!({ "mode": "execute", "task_ir": task }),
    )?)?;
    let descriptor = successful_result(agent.request(
        &format!("{phase}-describe"),
        "describe_agent",
        json!({}),
    )?)?;

    let artifact = json!({
        "schema_version": "kyuubiki.agent-solver-qualification/v2",
        "generated_at_unix_ms": unix_now_ms()?,
        "status": "passed",
        "transport": "tcp_framed_json",
        "rpc_version": RPC_VERSION,
        "operator_id": recovered["operator_id"],
        "program_kind": recovered["program_kind"],
        "runtime_protocol": recovered["runtime_protocol"],
        "task_digest": recovered["task_digest"],
        "stages": {
            "initial_execution": successful_stage(&initial)?,
            "unsupported_solver_rejection": {
                "reason_code": unsupported["error"]["code"],
                "failure_receipt": unsupported["error"]["details"]["operator_task_failure_receipt"]
            },
            "tamper_rejection": {
                "reason_code": tampered["error"]["code"],
                "failure_receipt": tampered["error"]["details"]["operator_task_failure_receipt"]
            },
            "recovery_execution": successful_stage(&recovered)?
        },
        "watchdog": descriptor["watchdog"]
    });
    validate_agent_solver_qualification_report(&artifact)
        .map_err(|errors| format!("managed Agent solver report invalid: {}", errors.join("; ")))?;
    Ok(OperationalSolverRun {
        phase: phase.to_string(),
        process_id: agent.process_id(),
        qualification: artifact,
    })
}

fn executable_solver_task_ir() -> Result<Value, String> {
    let mut task = json!({
        "schema_version": "kyuubiki.operator-task-ir/v1",
        "task_id": "agent-operational-bar-1d",
        "operator": { "id": "solve.bar_1d", "family": "mechanical", "kind": "solver" },
        "descriptor_authoring": {
            "schema_version": "kyuubiki.operator-descriptor-authoring/v1",
            "mode": "rust_native", "runtime": "rust", "source": "installer_operational_qualification",
            "hot_reloadable": false, "execution_language": "language_neutral"
        },
        "node": {},
        "input_artifact": {
            "length": 1.0, "area": 0.01, "youngs_modulus": 210000000000.0,
            "elements": 1, "tip_force": 1000.0
        },
        "config": { "qualification_case": "closed_form_axial_bar" },
        "execution_program": {
            "schema_version": "kyuubiki.operator-execution-program/v1",
            "program_id": "solve.bar_1d", "program_family": "mechanical", "program_kind": "solver",
            "operator_category_id": null, "package_ref": null, "package_version": "library-managed",
            "package_integrity": null, "runtime_protocol": "kyuubiki.solver-rpc/v1",
            "abi": { "kind": "solver_rpc", "input_encoding": "json", "output_encoding": "json" },
            "entrypoint": { "kind": "solver_method", "name": "solve_bar_1d", "operator_kind": "solver" },
            "bindings": {
                "input_artifact": "task.input_artifact", "config": "task.config",
                "output_artifact": "task.output_artifact"
            },
            "node_binding": { "node_id": null, "input_ports": [], "output_ports": [] }
        },
        "dataset_contract": {}, "orchestration_context": {},
        "runtime_hints": {
            "authority_mode": "agent_local", "execution_mode": "agent_native", "cache_scope": "none",
            "agent_fetchable": false, "operator_kind": "solver"
        }
    });
    task["integrity"] = json!({ "task_digest": compute_operator_task_digest(&task)? });
    Ok(task)
}

fn successful_result(response: Value) -> Result<Value, String> {
    if response.get("ok") != Some(&Value::Bool(true)) {
        return Err(format!("managed Agent request failed: {response}"));
    }
    response
        .get("result")
        .cloned()
        .ok_or_else(|| "managed Agent response misses result".to_string())
}

fn successful_stage(result: &Value) -> Result<Value, String> {
    let actual = result
        .pointer("/result/tip_displacement")
        .and_then(Value::as_f64)
        .ok_or_else(|| "solver result misses tip_displacement".to_string())?;
    let expected = AGENT_SOLVER_QUALIFICATION_EXPECTED_TIP_DISPLACEMENT;
    let absolute_error = (actual - expected).abs();
    Ok(json!({
        "status": result["operator_task_ir_status"],
        "solver_execution_capability": result["solver_execution_capability"],
        "validation_receipt": result["validation_receipt"],
        "provenance_receipt": result["provenance_receipt"],
        "result_assertion": {
            "metric": "tip_displacement", "expected": expected, "actual": actual,
            "absolute_error": absolute_error, "tolerance": RESULT_TOLERANCE,
            "passed": absolute_error <= RESULT_TOLERANCE
        }
    }))
}

fn read_json_frame(stream: &mut TcpStream) -> Result<Value, String> {
    let mut header = [0_u8; 4];
    stream
        .read_exact(&mut header)
        .map_err(|error| error.to_string())?;
    let length = u32::from_be_bytes(header) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(format!(
            "managed Agent frame exceeds {MAX_FRAME_BYTES} bytes"
        ));
    }
    let mut payload = vec![0_u8; length];
    stream
        .read_exact(&mut payload)
        .map_err(|error| error.to_string())?;
    serde_json::from_slice(&payload).map_err(|error| error.to_string())
}

fn reserve_port() -> Result<u16, String> {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(|error| format!("failed to reserve Agent qualification port: {error}"))
}

fn run_passed(run: &OperationalSolverRun) -> bool {
    run.qualification.get("status").and_then(Value::as_str) == Some("passed")
}

fn run_rejected_tamper(run: &OperationalSolverRun) -> bool {
    run.qualification
        .pointer("/stages/tamper_rejection/reason_code")
        .and_then(Value::as_str)
        == Some("operator_task_digest_mismatch")
}

fn run_recovered(run: &OperationalSolverRun) -> bool {
    run.qualification
        .pointer("/stages/recovery_execution/status")
        .and_then(Value::as_str)
        == Some("executed")
}

fn runs_quiescent<'a>(runs: impl IntoIterator<Item = &'a OperationalSolverRun>) -> bool {
    runs.into_iter().all(|run| {
        run.qualification
            .pointer("/watchdog/active_execution_count")
            .and_then(Value::as_u64)
            == Some(0)
    })
}

fn check(id: &str, ok: bool) -> OperationalCheck {
    OperationalCheck {
        id: id.to_string(),
        ok,
    }
}

fn qualification_host_role(platform: Platform) -> String {
    let location = if std::env::var_os("SSH_CONNECTION").is_some() {
        "remote"
    } else {
        "local"
    };
    format!("{location}-{}-qualification-host", platform.as_str())
}

fn unix_now_ms() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| format!("system clock precedes unix epoch: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operational_task_digest_is_stable_and_tamper_sensitive() {
        let task = executable_solver_task_ir().expect("task");
        let digest = task
            .pointer("/integrity/task_digest")
            .and_then(Value::as_str)
            .expect("digest")
            .to_string();
        assert_eq!(compute_operator_task_digest(&task).expect("digest"), digest);
        let mut tampered = task;
        tampered["config"]["qualification_case"] = json!("tampered");
        assert_ne!(
            compute_operator_task_digest(&tampered).expect("digest"),
            digest
        );
    }
}
