use kyuubiki_headless_sdk::{HeadlessExecutionBatch, build_preflight_failure_report};
use serde_json::Value;

use super::{
    Flags, classify_cli_error, cli_error_stage, kyuubiki_headless_flags::MAX_JOB_WAIT_TIMEOUT_MS,
    print_json, write_json_file,
};

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

pub(super) fn apply_job_wait_timeout_override(
    batch: &mut HeadlessExecutionBatch,
    timeout_ms: Option<u64>,
) -> Result<usize, String> {
    let Some(timeout_ms) = timeout_ms else {
        return Ok(0);
    };
    let mut changed = 0;
    for step in &mut batch.steps {
        if step.action != "job_wait" {
            continue;
        }
        let payload = step.payload.as_object_mut().ok_or_else(|| {
            format!(
                "step {} job_wait payload must be an object before applying timeout override",
                step.index
            )
        })?;
        let existing_max = payload
            .get("max_total_timeout_ms")
            .or_else(|| payload.get("maxTotalTimeoutMs"))
            .map(|value| {
                value.as_u64().ok_or_else(|| {
                    format!(
                        "step {} job_wait max_total_timeout_ms must be a positive integer",
                        step.index
                    )
                })
            })
            .transpose()?
            .unwrap_or(timeout_ms);
        let effective_max = existing_max.max(timeout_ms);
        if effective_max > MAX_JOB_WAIT_TIMEOUT_MS {
            return Err(format!(
                "step {} job_wait max_total_timeout_ms must not exceed {MAX_JOB_WAIT_TIMEOUT_MS}",
                step.index
            ));
        }
        payload.remove("timeoutMs");
        payload.remove("maxTotalTimeoutMs");
        payload.insert("timeout_ms".to_string(), Value::from(timeout_ms));
        payload.insert(
            "max_total_timeout_ms".to_string(),
            Value::from(effective_max),
        );
        changed += 1;
    }
    if changed == 0 {
        return Err("--job-wait-timeout-ms requires at least one job_wait step".to_string());
    }
    batch.warnings.push(format!(
        "CLI overrode {changed} job_wait timeout budget(s) to {timeout_ms} ms for this run"
    ));
    Ok(changed)
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

#[cfg(test)]
mod tests {
    use super::apply_job_wait_timeout_override;
    use kyuubiki_headless_sdk::{HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessRisk};
    use serde_json::json;

    fn batch(action: &str, payload: serde_json::Value) -> HeadlessExecutionBatch {
        HeadlessExecutionBatch {
            schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
            exported_at: "1970-01-01T00:00:00.000Z".to_string(),
            language: "en".to_string(),
            workflow_id: "wait-override".to_string(),
            template_id: None,
            steps: vec![HeadlessExecutionBatchStep {
                index: 1,
                action: action.to_string(),
                risk: HeadlessRisk::Normal,
                payload,
            }],
            warnings: vec![],
        }
    }

    #[test]
    fn timeout_override_normalizes_legacy_wait_without_shrinking_total_budget() {
        let mut workflow = batch(
            "job_wait",
            json!({
                "job_id": "{{steps.1.result.job_id}}",
                "timeoutMs": 60_000,
                "maxTotalTimeoutMs": 3_600_000
            }),
        );

        assert_eq!(
            apply_job_wait_timeout_override(&mut workflow, Some(1_200_000)).unwrap(),
            1
        );
        let payload = &workflow.steps[0].payload;
        assert_eq!(payload["timeout_ms"], 1_200_000);
        assert_eq!(payload["max_total_timeout_ms"], 3_600_000);
        assert!(payload.get("timeoutMs").is_none());
        assert_eq!(workflow.warnings.len(), 1);
    }

    #[test]
    fn timeout_override_rejects_workflow_without_wait_step() {
        let mut workflow = batch("service_health", json!({}));
        let error = apply_job_wait_timeout_override(&mut workflow, Some(1_200_000))
            .expect_err("missing job_wait should fail");
        assert!(error.contains("at least one job_wait"));
    }
}
