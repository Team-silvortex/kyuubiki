use crate::qualification_support::read_json;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(crate) const CONTRACT_PATH: &str =
    "config/architecture/headless-sdk-operational-qualification.json";
pub(crate) const CONTRACT_SCHEMA: &str =
    "kyuubiki.headless-sdk-operational-qualification-contract/v1";
pub(crate) const REPORT_SCHEMA: &str = "kyuubiki.headless-sdk-operational-qualification/v1";
pub(crate) const QUALIFICATION_ID: &str = "rust-headless-sdk-installed-linux-operational";
pub(crate) const DEFAULT_REPORT: &str =
    "releases/usability-evidence/2.14.1/headless-sdk-operational-qualification.json";
pub(crate) const REQUIRED_CHECKS: [&str; 14] = [
    "remote_linux_capture",
    "cargo_install_completed",
    "installed_binary_digests",
    "source_removed_before_execution",
    "minimal_runtime_path",
    "template_discovery",
    "workflow_initialized",
    "workflow_validated",
    "workflow_rendered",
    "headless_execution_report",
    "real_solver_execution",
    "expected_failure_closed",
    "recovery_after_failure",
    "cleanup_complete",
];

pub(crate) fn validate_contract(root: &Path, require_report: bool) -> RunnerResult<()> {
    let contract: Value = read_json(root, CONTRACT_PATH)?;
    for (pointer, expected) in [
        ("/schema_version", CONTRACT_SCHEMA),
        ("/qualification_id", QUALIFICATION_ID),
        ("/target_coordinate/module_id", "sdk-headless"),
        ("/target_coordinate/paradigm", "sdk_headless"),
        ("/target_coordinate/target_grade", "operational"),
        (
            "/capture/execution_host_role",
            "remote-linux-qualification-host",
        ),
        ("/capture/installation_method", "cargo-install-path"),
        ("/capture/build_profile", "release"),
        ("/retention/report_schema", REPORT_SCHEMA),
        ("/retention/report_path", DEFAULT_REPORT),
    ] {
        if contract.pointer(pointer).and_then(Value::as_str) != Some(expected) {
            return Err(format!("{CONTRACT_PATH}: {pointer} must be {expected}"));
        }
    }
    for pointer in [
        "/capture/source_removed_before_execution",
        "/capture/minimal_runtime_path",
        "/capture/real_solver_required",
        "/capture/expected_failure_required",
        "/capture/recovery_required",
        "/capture/cleanup_required",
    ] {
        if contract.pointer(pointer).and_then(Value::as_bool) != Some(true) {
            return Err(format!("{CONTRACT_PATH}: {pointer} must be true"));
        }
    }
    let checks = contract
        .pointer("/required_checks")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{CONTRACT_PATH}: required_checks must be an array"))?;
    let ids = checks
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if ids != REQUIRED_CHECKS.into_iter().collect() || checks.len() != REQUIRED_CHECKS.len() {
        return Err(format!("{CONTRACT_PATH}: required checks drifted"));
    }
    for pointer in [
        "/retention/contract_schema_path",
        "/retention/report_schema_path",
    ] {
        let path = string_at(&contract, pointer)?;
        if !root.join(&path).is_file() {
            return Err(format!("{CONTRACT_PATH}: missing {path}"));
        }
    }
    if require_report && !root.join(DEFAULT_REPORT).is_file() {
        return Err(format!(
            "{CONTRACT_PATH}: missing retained report {DEFAULT_REPORT}"
        ));
    }
    Ok(())
}

pub(crate) fn validate_report(report: &Value) -> RunnerResult<()> {
    for (pointer, expected) in [
        ("/schema_version", REPORT_SCHEMA),
        ("/status", "pass"),
        ("/qualification_id", QUALIFICATION_ID),
        ("/execution_host_role", "remote-linux-qualification-host"),
        ("/platform", "linux"),
        ("/installation/package_id", "kyuubiki-cli"),
        ("/installation/method", "cargo-install-path"),
        ("/installation/build_profile", "release"),
        ("/installation/runtime_path_mode", "isolated-empty"),
        ("/workflow/workflow_schema", "kyuubiki.headless-workflow/v1"),
        (
            "/workflow/workflow_id",
            "qualification.headless.operational",
        ),
        ("/workflow/template_id", "direct_bar_1d"),
        (
            "/workflow/rendered_schema",
            "kyuubiki.headless-execution-batch/v1",
        ),
        (
            "/workflow/execution_report_schema",
            "kyuubiki.headless-execution-run/v1",
        ),
        ("/workflow/execution_mode", "execute:mock"),
        ("/workflow/execution_status", "ok"),
        (
            "/real_solver/schema_version",
            "kyuubiki.material-exploration-run/v1",
        ),
        ("/real_solver/study", "material_heat_spreader_screening"),
        ("/real_solver/execution_class", "real_solver"),
        ("/real_solver/executor_id", "kyuubiki.rust.local-solver"),
        ("/real_solver/runtime", "rust_native"),
        ("/real_solver/result_origin", "computed_in_process"),
        (
            "/failure_recovery/failure_report_schema",
            "kyuubiki.headless-execution-run/v1",
        ),
        ("/failure_recovery/failure_status", "invalid"),
        ("/failure_recovery/failure_category", "contract_failure"),
        ("/failure_recovery/failure_stage", "command_validation"),
        ("/cleanup/scope", "managed-remote-run-root"),
    ] {
        if report.pointer(pointer).and_then(Value::as_str) != Some(expected) {
            return Err(format!("operational report {pointer} must be {expected}"));
        }
    }
    if u64_at(report, "/generated_at_unix_ms")? == 0
        || u64_at(report, "/workflow/template_count")? < 30
        || u64_at(report, "/workflow/step_count")? != 3
        || u64_at(report, "/workflow/executed_step_count")? != 3
        || u64_at(report, "/real_solver/candidate_count")? < 3
        || u64_at(report, "/failure_recovery/expected_failure_exit_code")? != 1
        || u64_at(report, "/failure_recovery/executed_step_count")? != 0
        || u64_at(report, "/cleanup/residue_count")? != 0
    {
        return Err("operational report thresholds are not met".to_string());
    }
    for pointer in [
        "/installation/isolated_prefix",
        "/installation/source_removed_before_execution",
        "/workflow/validation_ok",
        "/real_solver/production_eligible",
        "/failure_recovery/recovery_validation_ok",
        "/cleanup/work_root_removed",
    ] {
        if !bool_at(report, pointer)? {
            return Err(format!("operational report {pointer} must be true"));
        }
    }
    for pointer in [
        "/real_solver/mock_execution",
        "/real_solver/fallback_used",
        "/failure_recovery/retryable",
    ] {
        if bool_at(report, pointer)? {
            return Err(format!("operational report {pointer} must be false"));
        }
    }
    validate_binaries(report)?;
    validate_checks(report)?;
    reject_sensitive_content(report, "$")
}

pub(crate) fn validate_binaries(report: &Value) -> RunnerResult<()> {
    let binaries = report
        .pointer("/installation/binaries")
        .and_then(Value::as_array)
        .ok_or_else(|| "operational report misses installed binaries".to_string())?;
    let mut ids = BTreeSet::new();
    for binary in binaries {
        let id = string_at(binary, "/id")?;
        let digest = string_at(binary, "/sha256")?;
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("installed binary {id} has an invalid digest"));
        }
        ids.insert(id);
    }
    let expected = BTreeSet::from([
        "kyuubiki-headless".to_string(),
        "kyuubiki-material-explore".to_string(),
    ]);
    if ids != expected || binaries.len() != expected.len() {
        return Err("installed binary set drifted".to_string());
    }
    Ok(())
}

fn validate_checks(report: &Value) -> RunnerResult<()> {
    let checks = report
        .pointer("/checks")
        .and_then(Value::as_array)
        .ok_or_else(|| "operational report misses checks".to_string())?;
    let mut ids = BTreeSet::new();
    for check in checks {
        if !bool_at(check, "/ok")? {
            return Err(format!(
                "operational check {} failed",
                string_at(check, "/id")?
            ));
        }
        ids.insert(string_at(check, "/id")?);
    }
    let expected = REQUIRED_CHECKS
        .iter()
        .map(|value| (*value).to_string())
        .collect::<BTreeSet<_>>();
    if ids != expected || checks.len() != expected.len() {
        return Err("operational report check set drifted".to_string());
    }
    Ok(())
}

fn reject_sensitive_content(value: &Value, location: &str) -> RunnerResult<()> {
    match value {
        Value::Object(values) => {
            for (key, child) in values {
                let lower = key.to_ascii_lowercase();
                if [
                    "hostname",
                    "host_address",
                    "username",
                    "credential",
                    "absolute_host_path",
                ]
                .contains(&lower.as_str())
                {
                    return Err(format!(
                        "operational report retains forbidden key {location}.{key}"
                    ));
                }
                reject_sensitive_content(child, &format!("{location}.{key}"))?;
            }
        }
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                reject_sensitive_content(child, &format!("{location}[{index}]"))?;
            }
        }
        Value::String(value)
            if value.starts_with('/')
                || value.starts_with("~/")
                || value.contains("/Users/")
                || value.contains("/home/")
                || value.contains("192.168.") =>
        {
            return Err(format!(
                "operational report retains host path or address at {location}"
            ));
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn validator_self_test() -> RunnerResult<()> {
    let mut report = sample_report();
    validate_report(&report)?;
    report["hostname"] = Value::String("must-not-be-retained".to_string());
    if validate_report(&report).is_ok() {
        return Err("validator accepted host identity".to_string());
    }
    let mut report = sample_report();
    report["real_solver"]["mock_execution"] = Value::Bool(true);
    if validate_report(&report).is_ok() {
        return Err("validator accepted mock execution as a real solver".to_string());
    }
    Ok(())
}

fn sample_report() -> Value {
    let checks = REQUIRED_CHECKS
        .iter()
        .map(|id| json!({"id": id, "ok": true}))
        .collect::<Vec<_>>();
    json!({
        "schema_version": REPORT_SCHEMA,
        "generated_at_unix_ms": 1,
        "status": "pass",
        "qualification_id": QUALIFICATION_ID,
        "execution_host_role": "remote-linux-qualification-host",
        "platform": "linux",
        "architecture": "x86_64",
        "installation": {"package_id":"kyuubiki-cli","package_version":"2.7.0","method":"cargo-install-path","build_profile":"release","isolated_prefix":true,"source_removed_before_execution":true,"runtime_path_mode":"isolated-empty","binaries":[{"id":"kyuubiki-headless","sha256":"a".repeat(64)},{"id":"kyuubiki-material-explore","sha256":"b".repeat(64)}]},
        "workflow": {"template_count":35,"workflow_schema":"kyuubiki.headless-workflow/v1","workflow_id":"qualification.headless.operational","template_id":"direct_bar_1d","step_count":3,"validation_ok":true,"rendered_schema":"kyuubiki.headless-execution-batch/v1","execution_report_schema":"kyuubiki.headless-execution-run/v1","execution_mode":"execute:mock","execution_status":"ok","executed_step_count":3},
        "real_solver": {"schema_version":"kyuubiki.material-exploration-run/v1","study":"material_heat_spreader_screening","candidate_count":3,"winner_candidate_id":"copper_c110","execution_class":"real_solver","executor_id":"kyuubiki.rust.local-solver","runtime":"rust_native","result_origin":"computed_in_process","mock_execution":false,"fallback_used":false,"production_eligible":true},
        "failure_recovery": {"expected_failure_exit_code":1,"failure_report_schema":"kyuubiki.headless-execution-run/v1","failure_status":"invalid","executed_step_count":0,"failure_category":"contract_failure","failure_stage":"command_validation","retryable":false,"recovery_validation_ok":true},
        "cleanup": {"scope":"managed-remote-run-root","work_root_removed":true,"residue_count":0},
        "checks": checks
    })
}

pub(crate) fn string_at(value: &Value, pointer: &str) -> RunnerResult<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("JSON value {pointer} must be a non-empty string"))
}

pub(crate) fn u64_at(value: &Value, pointer: &str) -> RunnerResult<u64> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("JSON value {pointer} must be an unsigned integer"))
}

pub(crate) fn bool_at(value: &Value, pointer: &str) -> RunnerResult<bool> {
    value
        .pointer(pointer)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("JSON value {pointer} must be a boolean"))
}

pub(crate) fn array_len(value: &Value, pointer: &str) -> RunnerResult<usize> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or_else(|| format!("JSON value {pointer} must be an array"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn validator_rejects_host_identity_and_mock_solver_claims() {
        super::validator_self_test().expect("validator self-test");
    }
}
