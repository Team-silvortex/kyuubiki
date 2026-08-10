use kyuubiki_headless_sdk::{HeadlessExecutionBatch, build_preflight_failure_report};
use serde_json::Value;

use super::{Flags, classify_cli_error, cli_error_stage, print_json, write_json_file};

pub(super) fn emit_run_failure(
    flags: &Flags,
    batch: Option<&HeadlessExecutionBatch>,
    workflow_id: &str,
    message: String,
    issues: &[String],
) -> Result<(), String> {
    let code = classify_cli_error(&message);
    let report = build_preflight_failure_report(
        batch,
        workflow_id,
        &run_mode(flags),
        code,
        cli_error_stage(code),
        &message,
        issues,
    );
    if let Some(report_out) = &flags.report_out {
        write_json_file(report_out, &report)?;
    }
    if flags.json {
        print_json(&report)?;
    }
    Err(message)
}

pub(super) fn workflow_id(value: &Value) -> &str {
    value
        .get("workflow_id")
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("workflow")
                .and_then(|workflow| workflow.get("id"))
                .and_then(Value::as_str)
        })
        .unwrap_or("unresolved")
}

fn run_mode(flags: &Flags) -> String {
    if flags.execute {
        format!(
            "execute:{}",
            flags.executor.as_deref().unwrap_or("unspecified")
        )
    } else {
        "dry_run".to_string()
    }
}
