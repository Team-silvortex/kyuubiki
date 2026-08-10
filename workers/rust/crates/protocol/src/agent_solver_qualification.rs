use serde::Serialize;
use serde_json::Value;

pub const AGENT_SOLVER_QUALIFICATION_SCHEMA: &str = "kyuubiki.agent-solver-qualification/v2";
pub const AGENT_SOLVER_QUALIFICATION_OPERATOR_ID: &str = "solve.bar_1d";
pub const AGENT_SOLVER_QUALIFICATION_EXPECTED_TIP_DISPLACEMENT: f64 = 4.761_904_761_904_762e-7;

const SOLVER_RUNTIME_PROTOCOL: &str = "kyuubiki.solver-rpc/v1";
const SOLVER_DISPATCH_ROUTE: &str = "solver_rpc";
const MAX_RESULT_TOLERANCE: f64 = 1.0e-12;
const NUMBER_CONSISTENCY_TOLERANCE: f64 = 1.0e-18;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AgentSolverQualificationSummary {
    pub operator_id: String,
    pub task_digest: String,
    pub initial_absolute_error: f64,
    pub recovery_absolute_error: f64,
    pub recent_failure_count: u64,
}

pub fn validate_agent_solver_qualification_report(
    report: &Value,
) -> Result<AgentSolverQualificationSummary, Vec<String>> {
    let mut errors = Vec::new();
    expect_string(
        report,
        "/schema_version",
        AGENT_SOLVER_QUALIFICATION_SCHEMA,
        &mut errors,
    );
    expect_string(report, "/status", "passed", &mut errors);
    expect_string(report, "/transport", "tcp_framed_json", &mut errors);
    expect_u64(report, "/rpc_version", 1, &mut errors);
    if u64_at(report, "/generated_at_unix_ms", &mut errors) == Some(0) {
        errors.push("/generated_at_unix_ms must be greater than zero".to_string());
    }
    expect_string(
        report,
        "/operator_id",
        AGENT_SOLVER_QUALIFICATION_OPERATOR_ID,
        &mut errors,
    );
    expect_string(report, "/program_kind", "solver", &mut errors);
    expect_string(
        report,
        "/runtime_protocol",
        SOLVER_RUNTIME_PROTOCOL,
        &mut errors,
    );

    let operator_id = string_at(report, "/operator_id", &mut errors)
        .unwrap_or_default()
        .to_string();
    let task_digest = string_at(report, "/task_digest", &mut errors)
        .unwrap_or_default()
        .to_string();
    if !is_lower_hex_digest(&task_digest) {
        errors.push("/task_digest must be a 64-character lowercase SHA-256 digest".to_string());
    }

    let initial_absolute_error = validate_success_stage(
        report,
        "initial_execution",
        &operator_id,
        &task_digest,
        &mut errors,
    )
    .unwrap_or(f64::INFINITY);
    validate_unsupported_solver_rejection(report, &mut errors);
    validate_tamper_rejection(report, &operator_id, &task_digest, &mut errors);
    let recovery_absolute_error = validate_success_stage(
        report,
        "recovery_execution",
        &operator_id,
        &task_digest,
        &mut errors,
    )
    .unwrap_or(f64::INFINITY);
    let recent_failure_count = validate_watchdog(report, &mut errors).unwrap_or(0);

    if errors.is_empty() {
        Ok(AgentSolverQualificationSummary {
            operator_id,
            task_digest,
            initial_absolute_error,
            recovery_absolute_error,
            recent_failure_count,
        })
    } else {
        Err(errors)
    }
}

fn validate_unsupported_solver_rejection(report: &Value, errors: &mut Vec<String>) {
    let root = "/stages/unsupported_solver_rejection";
    expect_string(
        report,
        &format!("{root}/reason_code"),
        "operator_task_solver_capability_rejected",
        errors,
    );
    let receipt = format!("{root}/failure_receipt");
    expect_string(
        report,
        &format!("{receipt}/schema_version"),
        "kyuubiki.agent-operator-task-failure/v1",
        errors,
    );
    expect_string(
        report,
        &format!("{receipt}/failure_stage"),
        "check_solver_capability",
        errors,
    );
    expect_string(
        report,
        &format!("{receipt}/reason_code"),
        "operator_task_solver_capability_rejected",
        errors,
    );
    expect_string(
        report,
        &format!("{receipt}/operator_id"),
        "solve.thermal_bar_1d",
        errors,
    );
    let digest_path = format!("{receipt}/task_digest");
    if let Some(digest) = string_at(report, &digest_path, errors)
        && !is_lower_hex_digest(digest)
    {
        errors.push(format!("{digest_path} must be a lowercase SHA-256 digest"));
    }
    expect_string(
        report,
        &format!("{receipt}/recovery/required_action"),
        "select_advertised_solver_operator",
        errors,
    );
    expect_bool(
        report,
        &format!("{receipt}/recovery/safe_to_continue_other_tasks"),
        true,
        errors,
    );
}

fn validate_success_stage(
    report: &Value,
    stage: &str,
    operator_id: &str,
    task_digest: &str,
    errors: &mut Vec<String>,
) -> Option<f64> {
    let root = format!("/stages/{stage}");
    expect_string(report, &format!("{root}/status"), "executed", errors);

    let capability = format!("{root}/solver_execution_capability");
    expect_bool(report, &format!("{capability}/accepted"), true, errors);
    expect_string(
        report,
        &format!("{capability}/capability_id"),
        "agent-builtin-solver-execution",
        errors,
    );
    expect_string(
        report,
        &format!("{capability}/operator_id"),
        operator_id,
        errors,
    );
    expect_non_empty_string(report, &format!("{capability}/task_id"), errors);
    expect_string(
        report,
        &format!("{capability}/operator_kind"),
        "solver",
        errors,
    );
    expect_string(
        report,
        &format!("{capability}/program_kind"),
        "solver",
        errors,
    );
    expect_string(
        report,
        &format!("{capability}/runtime_protocol"),
        SOLVER_RUNTIME_PROTOCOL,
        errors,
    );
    expect_string(
        report,
        &format!("{capability}/dispatch_route"),
        SOLVER_DISPATCH_ROUTE,
        errors,
    );
    expect_empty_array(report, &format!("{capability}/rejection_reasons"), errors);

    let validation = format!("{root}/validation_receipt");
    expect_string(
        report,
        &format!("{validation}/schema_version"),
        "kyuubiki.agent-operator-task-validation/v1",
        errors,
    );
    expect_string(
        report,
        &format!("{validation}/validation_status"),
        "accepted",
        errors,
    );
    expect_bool(
        report,
        &format!("{validation}/digest_verified"),
        true,
        errors,
    );
    expect_bool(
        report,
        &format!("{validation}/execution_program_verified"),
        true,
        errors,
    );
    validate_solver_contract(report, &validation, errors);
    expect_bool(
        report,
        &format!("{validation}/package_fetch_required"),
        false,
        errors,
    );
    expect_null(report, &format!("{validation}/blocked_reason"), errors);

    let provenance = format!("{root}/provenance_receipt");
    expect_string(
        report,
        &format!("{provenance}/schema_version"),
        "kyuubiki.agent-operator-task-provenance/v1",
        errors,
    );
    expect_string(
        report,
        &format!("{provenance}/operator_id"),
        operator_id,
        errors,
    );
    expect_string(
        report,
        &format!("{provenance}/task_digest"),
        task_digest,
        errors,
    );
    expect_string(
        report,
        &format!("{provenance}/requested_mode"),
        "execute",
        errors,
    );
    validate_solver_contract(report, &provenance, errors);
    expect_bool(
        report,
        &format!("{provenance}/offline_runnable"),
        true,
        errors,
    );
    expect_bool(
        report,
        &format!("{provenance}/lineage/digest_verified"),
        true,
        errors,
    );
    expect_bool(
        report,
        &format!("{provenance}/lineage/execution_program_verified"),
        true,
        errors,
    );
    expect_string(
        report,
        &format!("{provenance}/lineage/preview_digest"),
        task_digest,
        errors,
    );

    validate_result_assertion(report, &format!("{root}/result_assertion"), errors)
}

fn validate_solver_contract(report: &Value, root: &str, errors: &mut Vec<String>) {
    expect_string(
        report,
        &format!("{root}/runtime_protocol"),
        SOLVER_RUNTIME_PROTOCOL,
        errors,
    );
    expect_string(report, &format!("{root}/abi_kind"), "solver_rpc", errors);
    expect_string(
        report,
        &format!("{root}/dispatch_route"),
        SOLVER_DISPATCH_ROUTE,
        errors,
    );
}

fn validate_result_assertion(report: &Value, root: &str, errors: &mut Vec<String>) -> Option<f64> {
    expect_string(
        report,
        &format!("{root}/metric"),
        "tip_displacement",
        errors,
    );
    expect_bool(report, &format!("{root}/passed"), true, errors);
    let expected = number_at(report, &format!("{root}/expected"), errors)?;
    let actual = number_at(report, &format!("{root}/actual"), errors)?;
    let reported_error = number_at(report, &format!("{root}/absolute_error"), errors)?;
    let tolerance = number_at(report, &format!("{root}/tolerance"), errors)?;

    if (expected - AGENT_SOLVER_QUALIFICATION_EXPECTED_TIP_DISPLACEMENT).abs()
        > NUMBER_CONSISTENCY_TOLERANCE
    {
        errors.push(format!(
            "{root}/expected must match the closed-form qualification fixture"
        ));
    }
    if !(0.0..=MAX_RESULT_TOLERANCE).contains(&tolerance) {
        errors.push(format!(
            "{root}/tolerance must be between 0 and {MAX_RESULT_TOLERANCE}"
        ));
    }
    let computed_error = (actual - expected).abs();
    if (reported_error - computed_error).abs() > NUMBER_CONSISTENCY_TOLERANCE {
        errors.push(format!(
            "{root}/absolute_error does not match expected and actual"
        ));
    }
    if computed_error > tolerance {
        errors.push(format!("{root} exceeds its numerical tolerance"));
    }
    Some(computed_error)
}

fn validate_tamper_rejection(
    report: &Value,
    operator_id: &str,
    task_digest: &str,
    errors: &mut Vec<String>,
) {
    let root = "/stages/tamper_rejection";
    expect_string(
        report,
        &format!("{root}/reason_code"),
        "operator_task_digest_mismatch",
        errors,
    );
    let receipt = format!("{root}/failure_receipt");
    expect_string(
        report,
        &format!("{receipt}/schema_version"),
        "kyuubiki.agent-operator-task-failure/v1",
        errors,
    );
    expect_string(
        report,
        &format!("{receipt}/failure_stage"),
        "verify_digest",
        errors,
    );
    expect_string(
        report,
        &format!("{receipt}/reason_code"),
        "operator_task_digest_mismatch",
        errors,
    );
    expect_string(
        report,
        &format!("{receipt}/operator_id"),
        operator_id,
        errors,
    );
    expect_string(
        report,
        &format!("{receipt}/task_digest"),
        task_digest,
        errors,
    );
    expect_string(
        report,
        &format!("{receipt}/recovery/required_action"),
        "rebuild_task_ir_and_recompute_digest",
        errors,
    );
    expect_bool(
        report,
        &format!("{receipt}/recovery/safe_to_continue_other_tasks"),
        true,
        errors,
    );
}

fn validate_watchdog(report: &Value, errors: &mut Vec<String>) -> Option<u64> {
    expect_string(report, "/watchdog/state", "watch", errors);
    expect_u64(report, "/watchdog/active_execution_count", 0, errors);
    let count = u64_at(report, "/watchdog/recent_failure_count", errors)?;
    if count < 2 {
        errors.push("/watchdog/recent_failure_count must be at least 2".to_string());
    }
    let failures = match report.pointer("/watchdog/recent_failures") {
        Some(Value::Array(failures)) => failures,
        Some(_) => {
            errors.push("/watchdog/recent_failures must be an array".to_string());
            return Some(count);
        }
        None => {
            errors.push("missing /watchdog/recent_failures".to_string());
            return Some(count);
        }
    };
    if failures.len() < 2 {
        errors.push("/watchdog/recent_failures must retain at least two failures".to_string());
    }
    let has_tamper_failure = failures.iter().any(|failure| {
        failure.get("request_id").and_then(Value::as_str) == Some("qualification-tampered")
            && failure.get("reason_code").and_then(Value::as_str)
                == Some("operator_task_digest_mismatch")
    });
    if !has_tamper_failure {
        errors.push("/watchdog/recent_failures must retain the tamper rejection".to_string());
    }
    let has_capability_failure = failures.iter().any(|failure| {
        failure.get("request_id").and_then(Value::as_str)
            == Some("qualification-unsupported-solver")
            && failure.get("reason_code").and_then(Value::as_str)
                == Some("operator_task_solver_capability_rejected")
    });
    if !has_capability_failure {
        errors.push(
            "/watchdog/recent_failures must retain the unsupported solver rejection".to_string(),
        );
    }
    Some(count)
}

fn expect_string(report: &Value, pointer: &str, expected: &str, errors: &mut Vec<String>) {
    if let Some(actual) = string_at(report, pointer, errors)
        && actual != expected
    {
        errors.push(format!("{pointer} must equal {expected:?}, got {actual:?}"));
    }
}

fn expect_non_empty_string(report: &Value, pointer: &str, errors: &mut Vec<String>) {
    if let Some(actual) = string_at(report, pointer, errors)
        && actual.is_empty()
    {
        errors.push(format!("{pointer} must not be empty"));
    }
}

fn string_at<'a>(report: &'a Value, pointer: &str, errors: &mut Vec<String>) -> Option<&'a str> {
    match report.pointer(pointer) {
        Some(Value::String(value)) => Some(value),
        Some(_) => {
            errors.push(format!("{pointer} must be a string"));
            None
        }
        None => {
            errors.push(format!("missing {pointer}"));
            None
        }
    }
}

fn expect_bool(report: &Value, pointer: &str, expected: bool, errors: &mut Vec<String>) {
    match report.pointer(pointer).and_then(Value::as_bool) {
        Some(actual) if actual == expected => {}
        Some(actual) => errors.push(format!("{pointer} must equal {expected}, got {actual}")),
        None => errors.push(format!("{pointer} must be a boolean")),
    }
}

fn expect_u64(report: &Value, pointer: &str, expected: u64, errors: &mut Vec<String>) {
    if let Some(actual) = u64_at(report, pointer, errors)
        && actual != expected
    {
        errors.push(format!("{pointer} must equal {expected}, got {actual}"));
    }
}

fn u64_at(report: &Value, pointer: &str, errors: &mut Vec<String>) -> Option<u64> {
    match report.pointer(pointer).and_then(Value::as_u64) {
        Some(value) => Some(value),
        None => {
            errors.push(format!("{pointer} must be an unsigned integer"));
            None
        }
    }
}

fn number_at(report: &Value, pointer: &str, errors: &mut Vec<String>) -> Option<f64> {
    match report.pointer(pointer).and_then(Value::as_f64) {
        Some(value) if value.is_finite() => Some(value),
        _ => {
            errors.push(format!("{pointer} must be a finite number"));
            None
        }
    }
}

fn expect_null(report: &Value, pointer: &str, errors: &mut Vec<String>) {
    match report.pointer(pointer) {
        Some(Value::Null) => {}
        Some(_) => errors.push(format!("{pointer} must be null")),
        None => errors.push(format!("missing {pointer}")),
    }
}

fn expect_empty_array(report: &Value, pointer: &str, errors: &mut Vec<String>) {
    match report.pointer(pointer) {
        Some(Value::Array(values)) if values.is_empty() => {}
        Some(Value::Array(_)) => errors.push(format!("{pointer} must be empty")),
        Some(_) => errors.push(format!("{pointer} must be an array")),
        None => errors.push(format!("missing {pointer}")),
    }
}

fn is_lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
