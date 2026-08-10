use serde::Serialize;
use std::env;

pub(super) fn print_cli_error(error: &str) {
    if !env::args().any(|argument| argument == "--json") {
        eprintln!("{error}");
        return;
    }
    let code = classify_cli_error(error);
    let output = CliErrorOutput {
        schema_version: "kyuubiki.headless-cli-error/v1",
        ok: false,
        error: CliErrorView {
            code,
            message: error,
            stage: cli_error_stage(code),
            retryable: cli_error_retryable(code),
            recommended_action: cli_error_recovery(code),
        },
    };
    match serde_json::to_string(&output) {
        Ok(payload) => eprintln!("{payload}"),
        Err(_) => eprintln!("{error}"),
    }
}

pub(super) fn cli_error_stage(code: &str) -> &'static str {
    match code {
        "frontend_proxy_artifact_limit" => "artifact_upload",
        "job_wait_timeout" => "job_wait",
        "headless_execution_failed" => "execution",
        "document_validation" => "document_decode",
        "executor_compatibility" | "executor_selection" => "executor_preflight",
        "endpoint_configuration" => "endpoint_configuration",
        "material_report_template_mismatch"
        | "material_report_template_provenance_missing"
        | "material_report_study_unsupported"
        | "material_report_output_required" => "material_report_validation",
        _ => "command_validation",
    }
}

pub(super) fn classify_cli_error(error: &str) -> &'static str {
    if error.contains("frontend_proxy_artifact_limit") {
        "frontend_proxy_artifact_limit"
    } else if error.contains("timed out waiting for job") {
        "job_wait_timeout"
    } else if error.contains("not supported by template") {
        "material_report_template_mismatch"
    } else if error.contains("requires template provenance") {
        "material_report_template_provenance_missing"
    } else if error.starts_with("unsupported material report study") {
        "material_report_study_unsupported"
    } else if error.contains("--material-report with --json requires --material-report-out") {
        "material_report_output_required"
    } else if error.starts_with("headless execution failed") {
        "headless_execution_failed"
    } else if error.starts_with("executor compatibility check failed") {
        "executor_compatibility"
    } else if error.contains("explicit --executor")
        || error.starts_with("unsupported executor")
        || error.starts_with("research execution requires")
    {
        "executor_selection"
    } else if error.starts_with("invalid --api-base-url") {
        "endpoint_configuration"
    } else if error.contains("missing field")
        || error.starts_with("invalid headless")
        || error.starts_with("unsupported headless document schema")
        || error.starts_with("failed to parse")
    {
        "document_validation"
    } else {
        "headless_command_failed"
    }
}

fn cli_error_recovery(code: &str) -> &'static str {
    match code {
        "frontend_proxy_artifact_limit" => {
            "Use the runtime control-plane endpoint for Headless execution instead of the GUI frontend."
        }
        "job_wait_timeout" => {
            "Inspect the job timing receipt, then resume the same job_id while its server deadline remains active."
        }
        "headless_execution_failed" => {
            "Inspect execution_summary.failure in the run report before retrying."
        }
        "document_validation" => {
            "Repair the execution document against the supported Headless schema before retrying."
        }
        "executor_compatibility" => {
            "Choose one of the compatible executors listed in the preflight run report."
        }
        "executor_selection" => {
            "Select mock, service, or hybrid explicitly and use service for research posture."
        }
        "endpoint_configuration" => {
            "Use a supported control-plane HTTP authority without paths, queries, or credentials."
        }
        "material_report_template_mismatch" => {
            "Choose a study listed by the selected template's material_report_studies field."
        }
        "material_report_template_provenance_missing" => {
            "Regenerate the batch through headless init so template provenance is retained."
        }
        "material_report_study_unsupported" => {
            "Use headless templates --json to select a supported material report study."
        }
        "material_report_output_required" => {
            "Provide --material-report-out when requesting a JSON material report."
        }
        _ => "Repair the command arguments using kyuubiki headless help before retrying.",
    }
}

fn cli_error_retryable(code: &str) -> bool {
    code == "job_wait_timeout"
}

#[derive(Debug, Serialize)]
struct CliErrorOutput<'a> {
    schema_version: &'static str,
    ok: bool,
    error: CliErrorView<'a>,
}

#[derive(Debug, Serialize)]
struct CliErrorView<'a> {
    code: &'static str,
    message: &'a str,
    stage: &'static str,
    retryable: bool,
    recommended_action: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_frontend_proxy_artifact_limit_for_automation() {
        let code = classify_cli_error(
            "headless execution failed: frontend_proxy_artifact_limit: use control plane",
        );
        assert_eq!(code, "frontend_proxy_artifact_limit");
        assert_eq!(cli_error_stage(code), "artifact_upload");
        assert!(cli_error_recovery(code).contains("control-plane endpoint"));
    }

    #[test]
    fn classifies_job_wait_timeout_as_retryable() {
        let code = classify_cli_error(
            "headless execution failed at step 2 (job_wait): timed out waiting for job job-long",
        );
        assert_eq!(code, "job_wait_timeout");
        assert_eq!(cli_error_stage(code), "job_wait");
        assert!(cli_error_retryable(code));
        assert!(cli_error_recovery(code).contains("same job_id"));
    }
}
