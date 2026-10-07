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
        "frontend_proxy_artifact_limit" | "model_artifact_limit_exceeded" => "artifact_upload",
        "job_wait_timeout" => "job_wait",
        "headless_execution_failed" | "headless_execution_blocked" => "execution",
        "document_validation" => "document_decode",
        "executor_compatibility" | "executor_selection" => "executor_preflight",
        "endpoint_configuration" => "endpoint_configuration",
        "report_output_failure" | "report_generation_failure" => "artifact_output",
        "parameter_patch_validation" => "parameter_patch",
        "research_round_validation" => "research_round",
        "material_report_template_mismatch"
        | "material_report_template_provenance_missing"
        | "material_report_study_unsupported"
        | "material_report_input_contract_mismatch"
        | "material_report_output_required" => "material_report_validation",
        _ => "command_validation",
    }
}

pub(super) fn classify_cli_error(error: &str) -> &'static str {
    if error.starts_with(super::kyuubiki_headless_output_paths::PATH_CONFLICT) {
        "output_path_conflict"
    } else if error.starts_with(super::kyuubiki_headless_post_run::GENERATION_FAILURE) {
        "report_generation_failure"
    } else if error.starts_with(super::kyuubiki_headless_report_io::OUTPUT_FAILURE) {
        "report_output_failure"
    } else if error.starts_with(super::kyuubiki_headless_research_round::PREFLIGHT_FAILURE) {
        "research_round_validation"
    } else if error.starts_with("material-report input contract mismatch") {
        "material_report_input_contract_mismatch"
    } else if error.contains("model_artifact_limit_exceeded")
        || (error.contains("direct FEM model exceeds artifact transport limit")
            && error.contains("limit_bytes="))
    {
        "model_artifact_limit_exceeded"
    } else if error.contains("frontend_proxy_artifact_limit") {
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
    } else if error.starts_with("headless execution blocked") {
        "headless_execution_blocked"
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
    } else if error.contains("research round") || error.contains("--research-round") {
        "research_round_validation"
    } else if error.contains("parameter patch") {
        "parameter_patch_validation"
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

pub(super) fn cli_error_recovery(code: &str) -> &'static str {
    match code {
        "frontend_proxy_artifact_limit" => {
            "Use the runtime control-plane endpoint for Headless execution instead of the GUI frontend."
        }
        "model_artifact_limit_exceeded" => {
            "Align KYUUBIKI_MODEL_ARTIFACT_MAX_BYTES across Headless, Orchestra, and Agent, or reduce the serialized model before retrying."
        }
        "job_wait_timeout" => {
            "Inspect the job timing receipt, then resume the same job_id while its server deadline remains active."
        }
        "headless_execution_failed" => {
            "Inspect execution_summary.failure in the run report before retrying."
        }
        "headless_execution_blocked" => {
            "Inspect the blocking step's execution_readiness.required_action, satisfy its runtime or confirmation gate, then explicitly retry."
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
        "report_output_failure" => {
            "Inspect the run report on stdout and existing job/results before repairing the output path. A failed file publication does not mean computation failed; do not replay the step or batch automatically."
        }
        "report_generation_failure" => {
            "Inspect the completed run receipt on stdout or its saved run file and original job/results. Repair result extraction or research metric/lineage mapping only against those retained results, then regenerate derived artifacts; do not replay the step or batch automatically. Missing or invalid results leave the study incomplete; computation success does not qualify missing research evidence."
        }
        "output_path_conflict" => {
            "Select distinct artifact output files separate from every input and previous-round evidence. If execution already completed, inspect its run receipt and original job/result; do not replay the step or batch automatically."
        }
        "parameter_patch_validation" => {
            "Regenerate the patch from the current rendered batch and keep expected baseline values synchronized."
        }
        "research_round_validation" => {
            "Use a service research run, retain numeric result metrics, and link iteration 2+ to the previous qualified evidence with a guarded parameter patch."
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
        "material_report_input_contract_mismatch" => {
            "Restore the complete built-in candidate chain, its fixed physical input profile and synchronized SI properties. SI consistency alone does not prove the original candidate constants. For changed physical inputs, use a custom research specification or a supported materialized-candidate report; do not relabel old results or automatically replay completed work."
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
    fn material_preflight_keywords_cannot_authorize_job_wait_recovery() {
        for text in [
            "timed out waiting for job",
            "model_artifact_limit_exceeded",
            "parameter patch",
        ] {
            let code =
                classify_cli_error(&format!("material-report input contract mismatch: {text}"));
            assert_eq!(code, "material_report_input_contract_mismatch");
            assert_eq!(cli_error_stage(code), "material_report_validation");
            assert!(!cli_error_retryable(code));
        }
    }

    #[test]
    fn research_preflight_text_never_authorizes_job_timeout_recovery() {
        for label in [
            "timed out waiting for job",
            "model_artifact_limit_exceeded",
            "missing field",
        ] {
            let error = format!(
                "{} {label}",
                super::super::kyuubiki_headless_research_round::PREFLIGHT_FAILURE
            );
            let code = classify_cli_error(&error);
            assert_eq!(code, "research_round_validation");
            assert_eq!(cli_error_stage(code), "research_round");
            assert!(!cli_error_retryable(code));
        }
    }

    #[test]
    fn generation_failure_precedes_metric_and_artifact_labels_without_authorizing_replay() {
        for label in [
            "timed out waiting for job",
            "--research-round",
            "parameter patch",
            "missing field",
            "headless execution failed",
            "model_artifact_limit_exceeded",
        ] {
            let error = super::super::kyuubiki_headless_post_run::generation_error(
                "derived artifact",
                label.into(),
            );
            let code = classify_cli_error(&error);
            assert_eq!(code, "report_generation_failure");
            assert_eq!(cli_error_stage(code), "artifact_output");
            assert!(!cli_error_retryable(code));
            assert!(cli_error_recovery(code).contains("retained results"));
            assert!(cli_error_recovery(code).contains("do not replay"));
        }
    }

    #[test]
    fn path_conflict_precedes_path_labels_and_never_authorizes_replay() {
        for path in [
            "--research-round",
            "parameter patch",
            "timed out waiting for job",
        ] {
            let code = classify_cli_error(&format!("headless output path conflict: {path}"));
            assert_eq!(code, "output_path_conflict");
            assert_eq!(cli_error_stage(code), "command_validation");
            assert!(!cli_error_retryable(code));
            assert!(cli_error_recovery(code).contains("do not replay"));
        }
    }

    #[test]
    fn output_failure_takes_precedence_over_labels_and_paths_without_authorizing_replay() {
        for label in [
            "research round",
            "parameter patch",
            "timed out waiting for job",
        ] {
            let code = classify_cli_error(&format!("headless output failed: write {label}"));
            assert_eq!(code, "report_output_failure");
            assert_eq!(cli_error_stage(code), "artifact_output");
            assert!(!cli_error_retryable(code));
            assert!(cli_error_recovery(code).contains("do not replay"));
        }
    }

    #[test]
    fn blocked_execution_is_not_a_retryable_transport_failure() {
        let code = classify_cli_error(
            "headless execution blocked at step 1 (operator_task_execute): attach_operator_package_runtime",
        );
        assert_eq!(code, "headless_execution_blocked");
        assert_eq!(cli_error_stage(code), "execution");
        assert!(!cli_error_retryable(code));
        assert!(cli_error_recovery(code).contains("required_action"));
    }

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
    fn classifies_model_artifact_limit_for_automation() {
        let code = classify_cli_error(
            "headless execution failed: model_artifact_limit_exceeded: direct FEM model exceeds artifact transport limit: size_bytes=600000389 limit_bytes=536870912",
        );
        assert_eq!(code, "model_artifact_limit_exceeded");
        assert_eq!(cli_error_stage(code), "artifact_upload");
        assert!(!cli_error_retryable(code));
        assert!(cli_error_recovery(code).contains("Headless, Orchestra, and Agent"));
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

    #[test]
    fn classifies_material_input_contract_mismatch_as_non_retryable() {
        let code = classify_cli_error(
            "material-report input contract mismatch: update both fields together",
        );
        assert_eq!(code, "material_report_input_contract_mismatch");
        assert_eq!(cli_error_stage(code), "material_report_validation");
        assert!(!cli_error_retryable(code));
        assert!(cli_error_recovery(code).contains("SI properties"));
    }

    #[test]
    fn classifies_parameter_patch_drift_as_non_retryable() {
        let code = classify_cli_error(
            "parameter patch round-2 change 1 baseline mismatch at /steps/0/payload/load",
        );
        assert_eq!(code, "parameter_patch_validation");
        assert_eq!(cli_error_stage(code), "parameter_patch");
        assert!(!cli_error_retryable(code));
        assert!(cli_error_recovery(code).contains("rendered batch"));
    }

    #[test]
    fn classifies_research_round_failure_as_non_retryable() {
        let code = classify_cli_error(
            "headless research round iteration 2 or later requires previous-round evidence",
        );
        assert_eq!(code, "research_round_validation");
        assert_eq!(cli_error_stage(code), "research_round");
        assert!(!cli_error_retryable(code));
        assert!(cli_error_recovery(code).contains("numeric result metrics"));
    }
}
