use crate::qualification_support::{
    generated_at_unix_ms, parse_options, read_json, repo_path, write_json,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

mod host;
pub(crate) mod remote;
pub(crate) mod support;

const CONTRACT_PATH: &str = "config/architecture/installed-runtime-operational-qualification.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.installed-runtime-operational-qualification-contract/v1";
const REPORT_SCHEMA: &str = "kyuubiki.installed-runtime-operational-qualification/v1";
const QUALIFICATION_ID: &str = "remote-linux-installer-managed-runtime-operational";
const JOURNEY: &str = "installed-headless-orchestra-agent-restart";
const DEFAULT_OUT: &str = "tmp/installed-runtime-operational-qualification.json";
const REQUIRED_CHECKS: &[&str] = &[
    "installed_payload_activated",
    "source_tree_detached",
    "native_runtime_managed",
    "frontend_omitted",
    "headless_service_execution",
    "agent_dispatch_observed",
    "numerical_result_verified",
    "orchestra_restart_completed",
    "result_retained_after_restart",
    "detached_restart_completed",
    "cleanup_complete",
    "retention_sanitized",
];

#[derive(Deserialize)]
struct Contract {
    schema_version: String,
    qualification_id: String,
    capture: CaptureContract,
    source_guard: SourceGuard,
    retention: Retention,
    required_checks: Vec<String>,
}

#[derive(Deserialize)]
struct CaptureContract {
    solve_report: String,
    restart_fetch_report: String,
    detached_fetch_report: String,
    detached_status: String,
    cleanup_report: String,
    installed_digests: String,
    execution_host_role: String,
    platform: String,
    architecture: String,
    package_version: String,
    workflow_id: String,
    minimum_agent_count: u64,
    restart_count: u64,
}

#[derive(Deserialize)]
struct SourceGuard {
    files: Vec<String>,
    required_text: Vec<String>,
}

#[derive(Deserialize)]
struct Retention {
    report_schema: String,
    report_schema_path: String,
    report_path: String,
    forbidden_content: Vec<String>,
}

struct Captures {
    solve: Value,
    restart: Value,
    detached: Value,
    cleanup: Value,
    status: String,
    installed_digests: BTreeMap<String, String>,
    capture_digests: BTreeMap<String, String>,
}

pub(crate) fn run_check_installed_runtime_operational_qualification(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_options(args, "installed Runtime operational qualification")?;
    let contract: Contract = read_json(root, CONTRACT_PATH)?;
    validate_contract(root, &contract)?;
    if options.self_test {
        validator_self_test(&contract)?;
        println!("Installed Runtime operational qualification self-test passed");
        return Ok(0);
    }
    if let Some(path) = options.verify_report {
        let report: Value = read_json(root, &path)?;
        validate_report(&contract, &report)?;
        println!("Installed Runtime operational qualification report passed: {path}");
        return Ok(0);
    }

    let captures = load_captures(root, &contract.capture)?;
    validate_captures(&contract.capture, &captures)?;
    let report = build_report(&contract, &captures)?;
    validate_report(&contract, &report)?;
    let out = options.out.as_deref().unwrap_or(DEFAULT_OUT);
    write_json(root, out, &report)?;
    println!("Installed Runtime operational qualification passed: {out}");
    Ok(0)
}

pub(crate) fn run_qualify_remote(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    remote::run(root, args)
}

pub(crate) fn run_capture_host(args: Vec<OsString>) -> RunnerResult<u8> {
    host::run(args)
}

fn validate_contract(root: &Path, contract: &Contract) -> RunnerResult<()> {
    if contract.schema_version != CONTRACT_SCHEMA || contract.qualification_id != QUALIFICATION_ID {
        return Err("installed Runtime operational contract identity is invalid".into());
    }
    let capture = &contract.capture;
    if capture.execution_host_role != "remote-linux-qualification-host"
        || capture.platform != "linux"
        || capture.architecture != "x86_64"
        || !valid_version(&capture.package_version)
        || capture.workflow_id != "qualification.installed-runtime.bar"
        || capture.minimum_agent_count < 2
        || capture.restart_count < 2
    {
        return Err("installed Runtime capture thresholds are invalid".into());
    }
    require_exact_set(
        contract.required_checks.iter().map(String::as_str),
        REQUIRED_CHECKS.iter().copied(),
        "required checks",
    )?;
    if contract.retention.report_schema != REPORT_SCHEMA
        || contract.retention.report_schema_path
            != "schemas/installed-runtime-operational-qualification-report.schema.json"
        || !contract
            .retention
            .report_path
            .starts_with("releases/usability-evidence/")
    {
        return Err("installed Runtime retention contract is invalid".into());
    }
    validate_schema_const(
        root,
        "schemas/installed-runtime-operational-qualification-contract.schema.json",
        CONTRACT_SCHEMA,
    )?;
    validate_schema_const(root, &contract.retention.report_schema_path, REPORT_SCHEMA)?;
    validate_source_guard(root, &contract.source_guard)
}

fn validate_schema_const(root: &Path, path: &str, expected: &str) -> RunnerResult<()> {
    let schema: Value = read_json(root, path)?;
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(expected)
    {
        return Err(format!("schema {path} does not enforce {expected}"));
    }
    Ok(())
}

fn validate_source_guard(root: &Path, guard: &SourceGuard) -> RunnerResult<()> {
    let mut source = String::new();
    for path in &guard.files {
        source.push_str(
            &fs::read_to_string(repo_path(root, path)?)
                .map_err(|error| format!("failed to read source guard {path}: {error}"))?,
        );
    }
    for required in &guard.required_text {
        if !source.contains(required) {
            return Err(format!("installed Runtime source guard misses {required}"));
        }
    }
    Ok(())
}

fn load_captures(root: &Path, capture: &CaptureContract) -> RunnerResult<Captures> {
    let (solve, solve_digest) = read_capture_json(root, &capture.solve_report)?;
    let (restart, restart_digest) = read_capture_json(root, &capture.restart_fetch_report)?;
    let (detached, detached_digest) = read_capture_json(root, &capture.detached_fetch_report)?;
    let (cleanup, cleanup_digest) = read_capture_json(root, &capture.cleanup_report)?;
    let (status, status_digest) = read_capture_text(root, &capture.detached_status)?;
    let (digests, digest_capture) = read_capture_text(root, &capture.installed_digests)?;
    Ok(Captures {
        solve,
        restart,
        detached,
        cleanup,
        status,
        installed_digests: parse_installed_digests(&digests)?,
        capture_digests: BTreeMap::from([
            ("solve_report_sha256".into(), solve_digest),
            ("restart_fetch_report_sha256".into(), restart_digest),
            ("detached_fetch_report_sha256".into(), detached_digest),
            ("detached_status_sha256".into(), status_digest),
            ("cleanup_report_sha256".into(), cleanup_digest),
            ("installed_digests_sha256".into(), digest_capture),
        ]),
    })
}

fn read_capture_json(root: &Path, path: &str) -> RunnerResult<(Value, String)> {
    let (text, digest) = read_capture_text(root, path)?;
    let value = serde_json::from_str(&text)
        .map_err(|error| format!("invalid operational capture {path}: {error}"))?;
    Ok((value, digest))
}

fn read_capture_text(root: &Path, path: &str) -> RunnerResult<(String, String)> {
    let bytes = fs::read(repo_path(root, path)?)
        .map_err(|error| format!("failed to read operational capture {path}: {error}"))?;
    let text = String::from_utf8(bytes.clone())
        .map_err(|error| format!("operational capture {path} is not UTF-8: {error}"))?;
    Ok((text, digest(&bytes)))
}

fn parse_installed_digests(text: &str) -> RunnerResult<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let mut fields = line.split_whitespace();
        let hash = fields.next().ok_or("installed digest misses hash")?;
        let path = fields.next().ok_or("installed digest misses path")?;
        if !valid_digest(hash) {
            return Err("installed digest is malformed".into());
        }
        let name = Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("installed digest path has no file name")?;
        values.insert(name.to_string(), hash.to_string());
    }
    for required in [
        "runtime-payload.json",
        "service-launch.json",
        "kyuubiki-runtime",
        "kyuubiki-headless",
        "kyuubiki-cli",
    ] {
        if !values.contains_key(required) {
            return Err(format!("installed digest capture misses {required}"));
        }
    }
    Ok(values)
}

fn validate_captures(contract: &CaptureContract, captures: &Captures) -> RunnerResult<()> {
    require_str(
        &captures.solve,
        "/schema_version",
        "kyuubiki.headless-execution-run/v1",
    )?;
    require_str(&captures.solve, "/workflow_id", &contract.workflow_id)?;
    require_str(&captures.solve, "/mode", "execute:service")?;
    require_str(&captures.solve, "/status", "ok")?;
    require_u64(&captures.solve, "/executed_step_count", 3)?;
    require_u64(&captures.solve, "/execution_summary/job_count", 1)?;
    require_str(
        &captures.solve,
        "/execution_summary/jobs/0/status",
        "completed",
    )?;
    let worker = string_at(&captures.solve, "/execution_summary/jobs/0/worker_id")?;
    if !worker.starts_with("rust-agent-rpc@") {
        return Err("installed workflow was not dispatched to a Rust Agent".into());
    }
    let job_id = string_at(&captures.solve, "/execution_summary/job_ids/0")?;
    let tip = number_at(
        &captures.solve,
        "/steps/2/result_preview/result/tip_displacement",
    )?;
    let stress = number_at(&captures.solve, "/steps/2/result_preview/result/max_stress")?;
    if !tip.is_finite() || tip <= 0.0 || !stress.is_finite() || stress <= 0.0 {
        return Err("installed workflow numerical result is invalid".into());
    }
    for report in [&captures.restart, &captures.detached] {
        require_str(
            report,
            "/schema_version",
            "kyuubiki.headless-execution-run/v1",
        )?;
        require_str(report, "/status", "ok")?;
        require_str(report, "/steps/0/result_preview/job_id", job_id)?;
        require_str(report, "/steps/0/result_preview/status", "completed")?;
        if number_at(report, "/steps/0/result_preview/result/tip_displacement")? != tip {
            return Err("retained result changed after Runtime restart".into());
        }
    }
    let running_agents = captures
        .status
        .lines()
        .filter(|line| line.starts_with("agent[") && line.contains(": running on tcp://"))
        .count() as u64;
    if running_agents < contract.minimum_agent_count
        || !captures
            .status
            .contains("runtime-policy: installer-managed")
        || !captures
            .status
            .contains("frontend: disabled by runtime configuration")
        || !captures.status.contains("orchestrator: running on http://")
        || captures.status.contains("development-source")
    {
        return Err("installed Runtime status does not prove the managed topology".into());
    }
    for pointer in [
        "/runtime_ports_closed",
        "/managed_pid_files_removed",
        "/source_tree_removed",
        "/managed_remote_root_removed",
    ] {
        require_bool(&captures.cleanup, pointer, true)?;
    }
    Ok(())
}

fn build_report(contract: &Contract, captures: &Captures) -> RunnerResult<Value> {
    let job_id = string_at(&captures.solve, "/execution_summary/job_ids/0")?;
    let tip = number_at(
        &captures.solve,
        "/steps/2/result_preview/result/tip_displacement",
    )?;
    let stress = number_at(&captures.solve, "/steps/2/result_preview/result/max_stress")?;
    let checks = REQUIRED_CHECKS
        .iter()
        .map(|id| json!({"id": id, "status": "pass"}))
        .collect::<Vec<_>>();
    Ok(json!({
        "schema_version": REPORT_SCHEMA,
        "generated_at_unix_ms": generated_at_unix_ms()?,
        "status": "pass",
        "qualification_id": QUALIFICATION_ID,
        "journey": JOURNEY,
        "execution_host_role": contract.capture.execution_host_role,
        "platform": {"os": "linux", "architecture": contract.capture.architecture},
        "installation": {
            "package_version": contract.capture.package_version,
            "activation_generation": 1,
            "runtime_policy": "installer-managed",
            "source_tree_detached": true,
            "source_fallback": false,
            "payload_manifest_sha256": captures.installed_digests["runtime-payload.json"],
            "service_manifest_sha256": captures.installed_digests["service-launch.json"],
            "runtime_binary_sha256": captures.installed_digests["kyuubiki-runtime"],
            "headless_binary_sha256": captures.installed_digests["kyuubiki-headless"],
            "agent_binary_sha256": captures.installed_digests["kyuubiki-cli"]
        },
        "runtime": {
            "control_mode": "standalone",
            "agent_count": contract.capture.minimum_agent_count,
            "orchestrator_managed": true,
            "frontend_loaded": false
        },
        "execution": {
            "workflow_id": contract.capture.workflow_id,
            "mode": "execute:service",
            "status": "completed",
            "job_id_sha256": digest(job_id.as_bytes()),
            "worker_transport": "rust-agent-rpc",
            "agent_dispatched": true,
            "tip_displacement": tip,
            "max_stress": stress
        },
        "restart_recovery": {
            "restart_count": contract.capture.restart_count,
            "result_retained_after_restart": true,
            "result_retained_after_source_detach": true,
            "numerical_result_unchanged": true
        },
        "cleanup": {
            "runtime_ports_closed": true,
            "managed_pid_files_removed": true,
            "managed_remote_root_removed": true,
            "residue_count": 0
        },
        "capture_digests": captures.capture_digests,
        "checks": checks
    }))
}

fn validate_report(contract: &Contract, report: &Value) -> RunnerResult<()> {
    for (pointer, expected) in [
        ("/schema_version", REPORT_SCHEMA),
        ("/status", "pass"),
        ("/qualification_id", QUALIFICATION_ID),
        ("/journey", JOURNEY),
        (
            "/installation/package_version",
            contract.capture.package_version.as_str(),
        ),
        ("/installation/runtime_policy", "installer-managed"),
        ("/execution/mode", "execute:service"),
        ("/execution/worker_transport", "rust-agent-rpc"),
    ] {
        require_str(report, pointer, expected)?;
    }
    for pointer in [
        "/installation/source_tree_detached",
        "/runtime/orchestrator_managed",
        "/execution/agent_dispatched",
        "/restart_recovery/result_retained_after_restart",
        "/restart_recovery/result_retained_after_source_detach",
        "/cleanup/runtime_ports_closed",
        "/cleanup/managed_pid_files_removed",
        "/cleanup/managed_remote_root_removed",
    ] {
        require_bool(report, pointer, true)?;
    }
    require_bool(report, "/installation/source_fallback", false)?;
    require_bool(report, "/runtime/frontend_loaded", false)?;
    require_u64(report, "/runtime/agent_count", 2)?;
    require_u64(report, "/restart_recovery/restart_count", 2)?;
    require_u64(report, "/cleanup/residue_count", 0)?;
    for pointer in [
        "/installation/payload_manifest_sha256",
        "/installation/service_manifest_sha256",
        "/installation/runtime_binary_sha256",
        "/installation/headless_binary_sha256",
        "/installation/agent_binary_sha256",
        "/execution/job_id_sha256",
    ] {
        if !valid_digest(string_at(report, pointer)?) {
            return Err(format!("report digest is invalid at {pointer}"));
        }
    }
    if number_at(report, "/execution/tip_displacement")? <= 0.0
        || number_at(report, "/execution/max_stress")? <= 0.0
    {
        return Err("report numerical result is invalid".into());
    }
    let checks = report
        .get("checks")
        .and_then(Value::as_array)
        .ok_or("report checks are missing")?;
    require_exact_set(
        checks
            .iter()
            .filter_map(|check| check.get("id").and_then(Value::as_str)),
        REQUIRED_CHECKS.iter().copied(),
        "report checks",
    )?;
    if checks
        .iter()
        .any(|check| check.get("status").and_then(Value::as_str) != Some("pass"))
    {
        return Err("report contains a failed check".into());
    }
    let rendered = serde_json::to_string(report).map_err(|error| error.to_string())?;
    for forbidden in &contract.retention.forbidden_content {
        if rendered
            .to_ascii_lowercase()
            .contains(&forbidden.to_ascii_lowercase())
        {
            return Err(format!("report retains forbidden content: {forbidden}"));
        }
    }
    Ok(())
}

fn validator_self_test(contract: &Contract) -> RunnerResult<()> {
    let mut report: Value = read_json(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../..")
            .as_path(),
        &contract.retention.report_path,
    )?;
    validate_report(contract, &report)?;
    report["runtime"]["frontend_loaded"] = Value::Bool(true);
    if validate_report(contract, &report).is_ok() {
        return Err("validator accepted a loaded frontend".into());
    }
    Ok(())
}

fn require_exact_set<'a>(
    actual: impl Iterator<Item = &'a str>,
    expected: impl Iterator<Item = &'a str>,
    label: &str,
) -> RunnerResult<()> {
    let actual = actual.collect::<BTreeSet<_>>();
    let expected = expected.collect::<BTreeSet<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{label} do not match the contract"))
    }
}

fn string_at<'a>(value: &'a Value, pointer: &str) -> RunnerResult<&'a str> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string at {pointer}"))
}

fn number_at(value: &Value, pointer: &str) -> RunnerResult<f64> {
    value
        .pointer(pointer)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("missing number at {pointer}"))
}

fn require_str(value: &Value, pointer: &str, expected: &str) -> RunnerResult<()> {
    if string_at(value, pointer)? == expected {
        Ok(())
    } else {
        Err(format!("unexpected value at {pointer}"))
    }
}

fn require_bool(value: &Value, pointer: &str, expected: bool) -> RunnerResult<()> {
    if value.pointer(pointer).and_then(Value::as_bool) == Some(expected) {
        Ok(())
    } else {
        Err(format!("unexpected boolean at {pointer}"))
    }
}

fn require_u64(value: &Value, pointer: &str, expected: u64) -> RunnerResult<()> {
    if value.pointer(pointer).and_then(Value::as_u64) == Some(expected) {
        Ok(())
    } else {
        Err(format!("unexpected integer at {pointer}"))
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}
