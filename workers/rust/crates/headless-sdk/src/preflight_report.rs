use crate::{
    HEADLESS_FAILURE_RECEIPT_SCHEMA_VERSION, HeadlessExecutionBatch, HeadlessExecutionSummary,
    HeadlessFailureReceipt, HeadlessRunReport, HeadlessValidationReport, validate_batch,
};

pub fn build_preflight_failure_report(
    batch: Option<&HeadlessExecutionBatch>,
    workflow_id: &str,
    mode: &str,
    error_code: &str,
    stage: &str,
    message: &str,
    issues: &[String],
) -> HeadlessRunReport {
    let mut validation = batch
        .map(validate_batch)
        .unwrap_or_else(empty_validation_report);
    if issues.is_empty() {
        let issue = message.to_string();
        if !validation.issues.contains(&issue) {
            validation.issues.push(issue);
        }
    } else {
        for issue in issues {
            if !validation.issues.contains(issue) {
                validation.issues.push(issue.clone());
            }
        }
    }
    validation.ok = false;
    validation.issue_count = validation.issues.len();

    build_failure_report(workflow_id, mode, error_code, stage, message, validation)
}

pub(crate) fn build_batch_validation_failure_report(
    batch: &HeadlessExecutionBatch,
    mode: &str,
    validation: HeadlessValidationReport,
) -> HeadlessRunReport {
    let message = format!(
        "headless execution batch validation failed with {} issue(s)",
        validation.issue_count
    );
    build_failure_report(
        &batch.workflow_id,
        mode,
        "document_validation",
        "batch_validation",
        &message,
        validation,
    )
}

fn build_failure_report(
    workflow_id: &str,
    mode: &str,
    error_code: &str,
    stage: &str,
    message: &str,
    validation: HeadlessValidationReport,
) -> HeadlessRunReport {
    let canonical_error_code = if error_code.starts_with("kyuubiki.headless.") {
        error_code.to_string()
    } else {
        format!("kyuubiki.headless.{error_code}")
    };
    let mut execution_summary = HeadlessExecutionSummary::default();
    execution_summary.failure = Some(HeadlessFailureReceipt {
        schema_version: HEADLESS_FAILURE_RECEIPT_SCHEMA_VERSION.to_string(),
        error_code: canonical_error_code,
        category: "contract_failure".to_string(),
        stage: stage.to_string(),
        step_index: 0,
        action: "run_preflight".to_string(),
        message: message.to_string(),
        retryable: false,
        retry_strategy: "none".to_string(),
        recommended_action:
            "Repair the execution document, command options, or executor selection before retrying."
                .to_string(),
    });

    HeadlessRunReport {
        schema_version: "kyuubiki.headless-execution-run/v1".to_string(),
        workflow_id: if workflow_id.trim().is_empty() {
            "unresolved".to_string()
        } else {
            workflow_id.to_string()
        },
        mode: mode.to_string(),
        status: "invalid".to_string(),
        executed_step_count: 0,
        warning_count: validation.warning_count,
        blocked_by_confirmation: None,
        validation,
        execution_summary,
        steps: Vec::new(),
    }
}

fn empty_validation_report() -> HeadlessValidationReport {
    HeadlessValidationReport {
        ok: false,
        issue_count: 0,
        issues: Vec::new(),
        warning_count: 0,
        warnings: Vec::new(),
        summary: None,
        policy: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_failure_is_a_standard_non_retryable_run_report() {
        let report = build_preflight_failure_report(
            None,
            "workflow.decode",
            "execute:service",
            "document_validation",
            "document_decode",
            "missing field `action`",
            &[],
        );

        assert_eq!(report.schema_version, "kyuubiki.headless-execution-run/v1");
        assert_eq!(report.status, "invalid");
        assert_eq!(report.validation.issues, ["missing field `action`"]);
        let failure = report
            .execution_summary
            .failure
            .expect("preflight failure receipt");
        assert_eq!(failure.error_code, "kyuubiki.headless.document_validation");
        assert_eq!(failure.stage, "document_decode");
        assert!(!failure.retryable);
    }
}
