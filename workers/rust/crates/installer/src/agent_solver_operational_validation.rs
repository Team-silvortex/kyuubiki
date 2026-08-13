use crate::agent_solver_operational::AGENT_SOLVER_OPERATIONAL_QUALIFICATION_SCHEMA_VERSION;
use kyuubiki_protocol::validate_agent_solver_qualification_report;
use serde_json::Value;
use std::collections::BTreeSet;

const REQUIRED_CHECKS: &[&str] = &[
    "package_sealed",
    "package_digest_verified",
    "installer_activation",
    "active_binary_verified",
    "first_solver_execution",
    "first_tamper_rejection",
    "first_recovery",
    "process_restart",
    "restarted_solver_execution",
    "restarted_tamper_rejection",
    "restarted_recovery",
    "watchdog_quiescent",
    "managed_store_isolated",
    "cleanup_complete",
];

#[derive(Clone, Debug, PartialEq)]
pub struct AgentSolverOperationalQualificationSummary {
    pub execution_host_role: String,
    pub platform: String,
    pub package_version: String,
    pub package_sha256: String,
    pub solver_run_count: usize,
    pub process_restart_confirmed: bool,
}

pub fn validate_agent_solver_operational_qualification_report(
    report: &Value,
) -> Result<AgentSolverOperationalQualificationSummary, Vec<String>> {
    let mut errors = Vec::new();
    expect_string(
        report,
        "/schema_version",
        AGENT_SOLVER_OPERATIONAL_QUALIFICATION_SCHEMA_VERSION,
        &mut errors,
    );
    expect_string(report, "/status", "pass", &mut errors);
    expect_string(
        report,
        "/journey",
        "installer-managed-packaged-agent-solver-recovery",
        &mut errors,
    );
    if u64_at(report, "/generated_at_unix_ms", &mut errors) == Some(0) {
        errors.push("/generated_at_unix_ms must be greater than zero".to_string());
    }

    let platform = string_at(report, "/platform", &mut errors)
        .unwrap_or_default()
        .to_string();
    if !matches!(platform.as_str(), "macos" | "linux" | "windows") {
        errors.push("/platform must be macos, linux, or windows".to_string());
    }
    let execution_host_role = string_at(report, "/execution_host_role", &mut errors)
        .unwrap_or_default()
        .to_string();
    let remote_role = format!("remote-{platform}-qualification-host");
    let local_role = format!("local-{platform}-qualification-host");
    if execution_host_role != remote_role && execution_host_role != local_role {
        errors.push("/execution_host_role does not match platform".to_string());
    }
    expect_non_empty_string(report, "/architecture", &mut errors);
    expect_string(
        report,
        "/control_boundary/deployment_owner",
        "kyuubiki-installer",
        &mut errors,
    );
    expect_string(
        report,
        "/control_boundary/execution_owner",
        "kyuubiki-agent-engine",
        &mut errors,
    );
    let expected_capture = if execution_host_role.starts_with("remote-") {
        "managed-remote-session"
    } else {
        "managed-local-session"
    };
    expect_string(
        report,
        "/control_boundary/capture_transport",
        expected_capture,
        &mut errors,
    );

    let package_version = string_at(report, "/package/version", &mut errors)
        .unwrap_or_default()
        .to_string();
    let package_sha256 = string_at(report, "/package/entrypoint_sha256", &mut errors)
        .unwrap_or_default()
        .to_string();
    validate_package(
        report,
        &platform,
        &package_version,
        &package_sha256,
        &mut errors,
    );
    validate_activation(
        report,
        &platform,
        &package_version,
        &package_sha256,
        &mut errors,
    );
    validate_installed_state(report, &package_version, &package_sha256, &mut errors);
    expect_string(
        report,
        "/transport/protocol",
        "tcp_framed_json",
        &mut errors,
    );
    expect_u64(report, "/transport/rpc_version", 1, &mut errors);
    expect_string(report, "/transport/bind_scope", "loopback", &mut errors);

    let (solver_run_count, process_restart_confirmed) = validate_solver_runs(report, &mut errors);
    expect_string(
        report,
        "/cleanup/scope",
        "qualification-work-root",
        &mut errors,
    );
    expect_bool(report, "/cleanup/work_root_removed", true, &mut errors);
    expect_u64(report, "/cleanup/residue_count", 0, &mut errors);
    validate_checks(report, &mut errors);
    reject_sensitive_or_host_paths(report, "$", &mut errors);

    if errors.is_empty() {
        Ok(AgentSolverOperationalQualificationSummary {
            execution_host_role,
            platform,
            package_version,
            package_sha256,
            solver_run_count,
            process_restart_confirmed,
        })
    } else {
        Err(errors)
    }
}

fn validate_package(
    report: &Value,
    platform: &str,
    version: &str,
    digest: &str,
    errors: &mut Vec<String>,
) {
    expect_string(
        report,
        "/package/schema_version",
        "kyuubiki.agent-update-package/v1",
        errors,
    );
    if !valid_version(version) {
        errors.push("/package/version is invalid".to_string());
    }
    expect_string(report, "/package/platform", platform, errors);
    expect_string(
        report,
        "/package/entrypoint",
        if platform == "windows" {
            "bin/kyuubiki-agent.exe"
        } else {
            "bin/kyuubiki-agent"
        },
        errors,
    );
    if !lower_hex_digest(digest) {
        errors.push("/package/entrypoint_sha256 must be a lowercase SHA-256 digest".to_string());
    }
    if u64_at(report, "/package/entrypoint_size_bytes", errors) == Some(0) {
        errors.push("/package/entrypoint_size_bytes must be greater than zero".to_string());
    }
}

fn validate_activation(
    report: &Value,
    platform: &str,
    version: &str,
    digest: &str,
    errors: &mut Vec<String>,
) {
    expect_string(
        report,
        "/activation/schema_version",
        "kyuubiki.agent-update-activation/v1",
        errors,
    );
    if u64_at(report, "/activation/generation", errors) == Some(0) {
        errors.push("/activation/generation must be greater than zero".to_string());
    }
    expect_string(report, "/activation/version", version, errors);
    expect_null(report, "/activation/previous_version", errors);
    expect_string(
        report,
        "/activation/relative_path",
        &format!("versions/{version}"),
        errors,
    );
    expect_string(report, "/activation/platform", platform, errors);
    expect_string(report, "/activation/entrypoint_sha256", digest, errors);
}

fn validate_installed_state(report: &Value, version: &str, digest: &str, errors: &mut Vec<String>) {
    expect_string(report, "/installed_state/active_version", version, errors);
    let installed = report
        .pointer("/installed_state/installed_versions")
        .and_then(Value::as_array);
    if !installed.is_some_and(|values| {
        values.len() == 1 && values.first().and_then(Value::as_str) == Some(version)
    }) {
        errors.push(
            "/installed_state/installed_versions must contain only the active version".to_string(),
        );
    }
    expect_string(
        report,
        "/installed_state/active_entrypoint_sha256",
        digest,
        errors,
    );
    expect_string(
        report,
        "/installed_state/package_relative_path",
        "packages/agent",
        errors,
    );
    expect_string(
        report,
        "/installed_state/store_relative_path",
        "managed-store",
        errors,
    );
}

fn validate_solver_runs(report: &Value, errors: &mut Vec<String>) -> (usize, bool) {
    let Some(runs) = report.pointer("/solver_runs").and_then(Value::as_array) else {
        errors.push("/solver_runs must be an array".to_string());
        return (0, false);
    };
    if runs.len() != 2 {
        errors.push("/solver_runs must contain initial and restarted process runs".to_string());
    }
    let expected_phases = ["initial-process", "restarted-process"];
    let mut process_ids = Vec::new();
    let mut task_digests = Vec::new();
    for (index, run) in runs.iter().enumerate() {
        let root = format!("/solver_runs/{index}");
        if let Some(expected) = expected_phases.get(index) {
            expect_string(report, &format!("{root}/phase"), expected, errors);
        }
        if let Some(pid) = u64_at(report, &format!("{root}/process_id"), errors) {
            if pid == 0 || pid > u32::MAX as u64 {
                errors.push(format!("{root}/process_id is invalid"));
            }
            process_ids.push(pid);
        }
        let Some(qualification) = run.get("qualification") else {
            errors.push(format!("{root}/qualification is missing"));
            continue;
        };
        match validate_agent_solver_qualification_report(qualification) {
            Ok(summary) => task_digests.push(summary.task_digest),
            Err(nested) => errors.extend(
                nested
                    .into_iter()
                    .map(|error| format!("{root}/qualification: {error}")),
            ),
        }
    }
    if task_digests.len() == 2 && task_digests[0] != task_digests[1] {
        errors.push("solver runs must execute the same TaskIR digest".to_string());
    }
    let restarted = process_ids.len() == 2 && process_ids[0] != process_ids[1];
    if !restarted {
        errors.push("solver process restart was not demonstrated".to_string());
    }
    (runs.len(), restarted)
}

fn validate_checks(report: &Value, errors: &mut Vec<String>) {
    let Some(checks) = report.pointer("/checks").and_then(Value::as_array) else {
        errors.push("/checks must be an array".to_string());
        return;
    };
    let observed = checks
        .iter()
        .filter_map(|entry| entry.get("id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let required = REQUIRED_CHECKS.iter().copied().collect::<BTreeSet<_>>();
    if observed != required || checks.len() != REQUIRED_CHECKS.len() {
        errors.push("/checks must contain the exact operational check set".to_string());
    }
    for (index, entry) in checks.iter().enumerate() {
        if entry.get("ok").and_then(Value::as_bool) != Some(true) {
            errors.push(format!("/checks/{index}/ok must be true"));
        }
    }
}

fn reject_sensitive_or_host_paths(value: &Value, path: &str, errors: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if matches!(
                    key.as_str(),
                    "hostname"
                        | "remote_host"
                        | "host_address"
                        | "address"
                        | "account"
                        | "user"
                        | "username"
                        | "password"
                        | "credential"
                ) {
                    errors.push(format!("{path}/{key} is forbidden in retained evidence"));
                }
                reject_sensitive_or_host_paths(child, &format!("{path}/{key}"), errors);
            }
        }
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                reject_sensitive_or_host_paths(child, &format!("{path}/{index}"), errors);
            }
        }
        Value::String(text) => {
            if absolute_host_path(text) || looks_like_ipv4(text) || text.contains("SSH_CONNECTION")
            {
                errors.push(format!("{path} contains a host-specific or sensitive path"));
            }
        }
        _ => {}
    }
}

fn absolute_host_path(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with("\\\\")
        || (value.len() >= 3
            && value.as_bytes()[0].is_ascii_alphabetic()
            && value.as_bytes()[1] == b':'
            && matches!(value.as_bytes()[2], b'\\' | b'/'))
}

fn looks_like_ipv4(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() == 4
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.parse::<u8>().is_ok())
}

fn string_at<'a>(value: &'a Value, pointer: &str, errors: &mut Vec<String>) -> Option<&'a str> {
    match value.pointer(pointer).and_then(Value::as_str) {
        Some(text) => Some(text),
        None => {
            errors.push(format!("{pointer} must be a string"));
            None
        }
    }
}

fn u64_at(value: &Value, pointer: &str, errors: &mut Vec<String>) -> Option<u64> {
    match value.pointer(pointer).and_then(Value::as_u64) {
        Some(number) => Some(number),
        None => {
            errors.push(format!("{pointer} must be an unsigned integer"));
            None
        }
    }
}

fn expect_string(value: &Value, pointer: &str, expected: &str, errors: &mut Vec<String>) {
    if value.pointer(pointer).and_then(Value::as_str) != Some(expected) {
        errors.push(format!("{pointer} must be {expected}"));
    }
}

fn expect_non_empty_string(value: &Value, pointer: &str, errors: &mut Vec<String>) {
    if !value
        .pointer(pointer)
        .and_then(Value::as_str)
        .is_some_and(|text| !text.trim().is_empty())
    {
        errors.push(format!("{pointer} must be a non-empty string"));
    }
}

fn expect_u64(value: &Value, pointer: &str, expected: u64, errors: &mut Vec<String>) {
    if value.pointer(pointer).and_then(Value::as_u64) != Some(expected) {
        errors.push(format!("{pointer} must be {expected}"));
    }
}

fn expect_bool(value: &Value, pointer: &str, expected: bool, errors: &mut Vec<String>) {
    if value.pointer(pointer).and_then(Value::as_bool) != Some(expected) {
        errors.push(format!("{pointer} must be {expected}"));
    }
}

fn expect_null(value: &Value, pointer: &str, errors: &mut Vec<String>) {
    if value.pointer(pointer) != Some(&Value::Null) {
        errors.push(format!("{pointer} must be null"));
    }
}

fn lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_a_host_path_before_evidence_retention() {
        let mut errors = Vec::new();
        reject_sensitive_or_host_paths(
            &json!({ "log": "/home/example/run.log" }),
            "$",
            &mut errors,
        );
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn rejects_cross_platform_paths_and_network_identity() {
        for value in ["/private/tmp/run", r"C:\Users\example", "192.0.2.10"] {
            let mut errors = Vec::new();
            reject_sensitive_or_host_paths(&json!({ "log": value }), "$", &mut errors);
            assert_eq!(errors.len(), 1, "expected rejection for {value}");
        }
    }

    #[test]
    fn required_checks_are_unique() {
        assert_eq!(
            REQUIRED_CHECKS
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len(),
            REQUIRED_CHECKS.len()
        );
    }
}
