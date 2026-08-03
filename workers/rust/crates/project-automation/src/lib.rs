mod macro_file;
mod model;
mod templates;

use chrono::{SecondsFormat, Utc};
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessRisk, ServiceHeadlessExecutor,
    collect_executor_compatibility_issues, execute_batch_with_executor, find_action_contract,
    run_batch_dry, validate_batch,
};
use kyuubiki_project_bundle::read_project_bundle;
pub use macro_file::{
    inspect_macro_file, list_automation_action_capabilities, normalize_macro_file,
    render_macro_file, run_macro_file, validate_macro_file,
};
pub use model::{
    AutomationEnvelope, AutomationPresetSummary, AutomationRunOptions, AutomationRunReport,
    AutomationStepReport, MacroDraft, MacroStep, MacroSummary, MacroValidationReport,
};
use model::{
    AutomationMetadata, AutomationPlan, AutomationPlanStep, AutomationRiskSummary,
    AutomationSource, RequestedCapabilities, RequiredConfirmation,
};
use serde_json::{Value, json};

type AutomationResult<T> = Result<T, String>;

pub fn list_project_automation_presets(
    path: &str,
) -> AutomationResult<Vec<AutomationPresetSummary>> {
    let bundle = read_project_bundle(path)?;
    presets(&bundle)
        .iter()
        .map(preset_summary)
        .collect::<AutomationResult<Vec<_>>>()
}

pub fn render_project_automation_preset(
    path: &str,
    selector: &str,
    payload: Value,
    state: Value,
) -> AutomationResult<AutomationEnvelope> {
    let bundle = read_project_bundle(path)?;
    let preset = find_preset(&bundle, selector)?;
    let macro_value = preset
        .get("macro")
        .filter(|value| value.is_object())
        .ok_or_else(|| "automation preset is missing macro".to_string())?;
    let draft = macro_file::parse_macro_value(macro_value.clone())?;
    build_envelope_from_macro(
        AutomationSource {
            kind: "project_automation_preset".to_string(),
            preset_id: Some(required_string(preset, "presetId", "automation preset")?.to_string()),
            preset_name: Some(required_string(preset, "name", "automation preset")?.to_string()),
            project_id: Some(
                required_string(preset, "projectId", "automation preset")?.to_string(),
            ),
            updated_at: Some(
                required_string(preset, "updatedAt", "automation preset")?.to_string(),
            ),
            input_path: None,
        },
        &draft,
        payload,
        state,
    )
}

pub fn run_project_automation_preset(
    path: &str,
    selector: &str,
    payload: Value,
    state: Value,
    options: &AutomationRunOptions,
) -> AutomationResult<AutomationRunReport> {
    let envelope = render_project_automation_preset(path, selector, payload, state)?;
    run_envelope(&envelope, options)
}

fn build_envelope_from_macro(
    source: AutomationSource,
    draft: &MacroDraft,
    payload: Value,
    state: Value,
) -> AutomationResult<AutomationEnvelope> {
    let payload = object_or_empty(payload);
    let state = object_or_empty(state);
    let steps = draft
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| build_plan_step(index, step, &payload, &state))
        .collect::<AutomationResult<Vec<_>>>()?;
    let batch = batch_from_steps(&draft.id, &steps);
    let validation = validate_batch(&batch);
    if !validation.ok {
        return Err(validation.issues.join("; "));
    }
    let sensitive_step_count = steps
        .iter()
        .filter(|step| step.risk == HeadlessRisk::Sensitive)
        .count();
    let destructive_step_count = steps
        .iter()
        .filter(|step| step.risk == HeadlessRisk::Destructive)
        .count();
    let highest_risk = if destructive_step_count > 0 {
        HeadlessRisk::Destructive
    } else if sensitive_step_count > 0 {
        HeadlessRisk::Sensitive
    } else {
        HeadlessRisk::Normal
    };
    let required_confirmations = steps
        .iter()
        .filter(|step| step.requires_confirmation)
        .map(|step| RequiredConfirmation {
            step_index: step.index,
            action: step.action.clone(),
            risk: step.risk,
        })
        .collect::<Vec<_>>();
    let generated_at = now();
    Ok(AutomationEnvelope {
        schema_version: "kyuubiki.headless-automation-plan/v1".to_string(),
        source,
        metadata: AutomationMetadata {
            macro_id: draft.id.clone(),
            generated_at,
            step_count: steps.len(),
            action_count: steps.len(),
        },
        risk_summary: AutomationRiskSummary {
            highest_risk,
            sensitive_step_count,
            destructive_step_count,
        },
        required_confirmations,
        plan: AutomationPlan {
            id: draft.id.clone(),
            step_count: steps.len(),
            actions: steps.iter().map(|step| step.action.clone()).collect(),
            payload,
            state,
            steps,
        },
    })
}

fn run_envelope(
    envelope: &AutomationEnvelope,
    options: &AutomationRunOptions,
) -> AutomationResult<AutomationRunReport> {
    let batch = batch_from_steps(&envelope.metadata.macro_id, &envelope.plan.steps);
    let started_at = now();
    let sdk_report = if options.execute {
        let compatibility = collect_executor_compatibility_issues(&batch, "service");
        if !compatibility.is_empty() {
            return Err(format!(
                "native automation live execution is service-only: {}",
                compatibility.join("; ")
            ));
        }
        let mut executor = ServiceHeadlessExecutor::with_token(
            options
                .api_base_url
                .as_deref()
                .unwrap_or("http://127.0.0.1:3000"),
            options.api_token.as_deref(),
        );
        execute_batch_with_executor(
            &batch,
            &mut executor,
            options.allow_sensitive,
            options.allow_destructive,
        )
    } else {
        // A dry run never performs a risky action, so confirmation flags are informational only.
        run_batch_dry(&batch, true, true)
    };
    let steps = sdk_report
        .steps
        .iter()
        .zip(&envelope.plan.steps)
        .map(|(step, original)| AutomationStepReport::from_sdk(step, original, options.execute))
        .collect::<Vec<_>>();
    let blocked_by_confirmation = steps.iter().find(|step| step.status == "blocked").cloned();
    let failed_step = steps.iter().find(|step| step.status == "failed").cloned();
    let status = if failed_step.is_some() {
        "failed"
    } else if blocked_by_confirmation.is_some() {
        "blocked"
    } else if options.execute {
        "completed"
    } else {
        "simulated"
    };
    Ok(AutomationRunReport {
        schema_version: "kyuubiki.headless-automation-run/v1".to_string(),
        source: envelope.source.clone(),
        metadata: envelope.metadata.clone(),
        started_at,
        completed_at: now(),
        status: status.to_string(),
        dry_run: !options.execute,
        requested_capabilities: RequestedCapabilities {
            allow_sensitive: options.allow_sensitive,
            allow_destructive: options.allow_destructive,
        },
        risk_summary: envelope.risk_summary.clone(),
        blocked_by_confirmation,
        failed_step,
        executed_step_count: sdk_report.executed_step_count,
        steps,
    })
}

fn build_plan_step(
    index: usize,
    step: &MacroStep,
    payload: &Value,
    state: &Value,
) -> AutomationResult<AutomationPlanStep> {
    let resolved = templates::resolve(step.payload.clone(), payload, state);
    build_plan_step_with_payload(index, step, resolved)
}

fn build_unresolved_plan_step(
    index: usize,
    step: &MacroStep,
) -> AutomationResult<AutomationPlanStep> {
    build_plan_step_with_payload(index, step, step.payload.clone())
}

fn build_plan_step_with_payload(
    index: usize,
    step: &MacroStep,
    payload: Value,
) -> AutomationResult<AutomationPlanStep> {
    let action = step.action.trim();
    if action.is_empty() {
        return Err(format!("automation step {index} is missing action"));
    }
    let canonical = canonical_action(action);
    let contract = find_action_contract(canonical)
        .ok_or_else(|| format!("step {index} uses unsupported action \"{action}\""))?;
    if !payload.is_object() {
        return Err(format!("step {index} has an invalid payload"));
    }
    Ok(AutomationPlanStep {
        index,
        action: action.to_string(),
        canonical_action: canonical.to_string(),
        engine: contract.engine,
        risk: contract.risk,
        requires_confirmation: contract.risk != HeadlessRisk::Normal,
        payload,
    })
}

fn batch_from_steps(id: &str, steps: &[AutomationPlanStep]) -> HeadlessExecutionBatch {
    HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
        exported_at: now(),
        language: "en".to_string(),
        workflow_id: id.to_string(),
        template_id: None,
        steps: steps
            .iter()
            .map(|step| HeadlessExecutionBatchStep {
                index: step.index + 1,
                action: step.canonical_action.clone(),
                risk: step.risk,
                payload: step.payload.clone(),
            })
            .collect(),
        warnings: Vec::new(),
    }
}

fn presets(bundle: &Value) -> &[Value] {
    bundle
        .get("automation_presets")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn find_preset<'a>(bundle: &'a Value, selector: &str) -> AutomationResult<&'a Value> {
    let selector = selector.trim();
    if selector.is_empty() {
        return Err("automation command requires --preset <id|name>".to_string());
    }
    presets(bundle)
        .iter()
        .find(|preset| preset.get("presetId").and_then(Value::as_str) == Some(selector))
        .or_else(|| {
            presets(bundle)
                .iter()
                .find(|preset| preset.get("name").and_then(Value::as_str) == Some(selector))
        })
        .ok_or_else(|| format!("could not find automation preset \"{selector}\""))
}

fn preset_summary(preset: &Value) -> AutomationResult<AutomationPresetSummary> {
    let macro_value = preset.get("macro").unwrap_or(&Value::Null);
    let steps = macro_value
        .get("steps")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    Ok(AutomationPresetSummary {
        preset_id: required_string(preset, "presetId", "automation preset")?.to_string(),
        project_id: required_string(preset, "projectId", "automation preset")?.to_string(),
        name: required_string(preset, "name", "automation preset")?.to_string(),
        updated_at: required_string(preset, "updatedAt", "automation preset")?.to_string(),
        macro_id: macro_value
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string),
        step_count: steps.len(),
        actions: steps
            .iter()
            .filter_map(|step| step.get("action").and_then(Value::as_str))
            .map(str::to_string)
            .collect(),
    })
}

fn required_string<'a>(value: &'a Value, key: &str, label: &str) -> AutomationResult<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{label} is missing {key}"))
}

fn object_or_empty(value: Value) -> Value {
    if value.is_object() { value } else { json!({}) }
}

fn canonical_action(action: &str) -> &str {
    match action.trim() {
        "health_check" | "api_health" => "service_health",
        "api_project_create" => "project_create",
        "api_project_update" => "project_update",
        "api_project_delete" => "project_delete",
        "api_model_create" => "model_create",
        "api_model_version_create" => "model_version_create",
        "api_workflow_submit_catalog" => "workflow_submit_catalog",
        "api_workflow_submit_graph" => "workflow_submit_graph",
        "api_job_wait" | "job_poll" => "job_wait",
        "api_job_fetch" | "job_status" => "job_fetch",
        "api_result_fetch" | "job_fetch_result" => "result_fetch",
        "api_direct_mesh_solve" => "direct_mesh_solve",
        "api_solve_from_model_version" => "solve_from_model_version",
        "api_solve_and_wait_from_model_version" => "solve_and_wait_from_model_version",
        "goto" | "navigate" | "browser_open_page" => "open_page",
        "browser_click" => "click",
        "fill" | "input" | "browser_type" => "type",
        "keyboard_press" | "browser_press" => "press",
        "select_option" | "browser_select" => "select",
        "sleep" | "wait_for" | "browser_wait" => "wait",
        "expect_text" | "browser_assert_text" => "assert_text",
        "screenshot" | "export_snapshot" | "browser_snapshot" => "snapshot",
        other => other,
    }
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn risk_label(risk: HeadlessRisk) -> &'static str {
    match risk {
        HeadlessRisk::Normal => "normal",
        HeadlessRisk::Sensitive => "sensitive",
        HeadlessRisk::Destructive => "destructive",
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
