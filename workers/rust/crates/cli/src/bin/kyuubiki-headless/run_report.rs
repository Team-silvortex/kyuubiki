use kyuubiki_headless_sdk::{HeadlessEngine, HeadlessRunReport};
use serde_json::Value;

pub(super) fn print_run_report(report: &HeadlessRunReport) {
    println!("Headless run: {}", report.workflow_id);
    println!("Mode: {}", report.mode);
    println!("Status: {}", report.status);
    println!(
        "Executed steps: {}/{}",
        report.executed_step_count,
        report.steps.len()
    );
    if report.warning_count > 0 {
        println!("Warnings: {}", report.warning_count);
    }
    if let Some(blocked) = &report.blocked_by_confirmation {
        println!(
            "Blocked: step {} requires {:?} confirmation",
            blocked.index, blocked.risk
        );
    }
    for job in &report.execution_summary.jobs {
        println!(
            "Job {}: status={} phase={} queue={}ms execution={}ms worker={}",
            job.job_id,
            job.status.as_deref().unwrap_or("unknown"),
            job.phase.as_deref().unwrap_or("unknown"),
            optional_millis(job.queue_wait_ms),
            optional_millis(job.execution_elapsed_ms),
            job.worker_id.as_deref().unwrap_or("unassigned")
        );
    }
    if let Some(failure) = &report.execution_summary.failure {
        println!(
            "Failure: {} at step {} ({}) retryable={} strategy={}",
            failure.error_code,
            failure.step_index,
            failure.action,
            failure.retryable,
            failure.retry_strategy
        );
        println!("Recovery: {}", failure.recommended_action);
    }
    for step in &report.steps {
        println!("{}. {} -> {}", step.index, step.action, step.status);
        println!("   payload: {}", step.payload);
        println!("   preview: {}", step.result_preview);
    }
}

fn optional_millis(value: Option<u64>) -> String {
    value.map_or_else(|| "--".to_string(), |value| value.to_string())
}

pub(super) fn print_material_report_summary(report: &Value) {
    println!(
        "Material report: {}",
        report.get("study").and_then(Value::as_str).unwrap_or("--")
    );
    println!(
        "Material winner: {}",
        report
            .get("winner_candidate_id")
            .and_then(Value::as_str)
            .unwrap_or("--")
    );
}

pub(super) fn engine_label(engine: &HeadlessEngine) -> &'static str {
    match engine {
        HeadlessEngine::Browser => "browser",
        HeadlessEngine::Service => "service",
    }
}
