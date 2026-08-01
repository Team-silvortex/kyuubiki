use super::{
    AutomationResult, AutomationRunOptions, AutomationRunReport, AutomationSource, MacroDraft,
    MacroStep, MacroSummary, MacroValidationReport, batch_from_steps, build_envelope_from_macro,
    build_unresolved_plan_step, canonical_action, object_or_empty, run_envelope,
};
use kyuubiki_headless_sdk::{HeadlessActionCapability, action_capability_manifest, validate_batch};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

pub fn list_automation_action_capabilities() -> Vec<HeadlessActionCapability> {
    action_capability_manifest()
}

pub fn inspect_macro_file(path: &str) -> AutomationResult<MacroSummary> {
    let draft = load_valid_macro(path)?;
    Ok(summary(&draft))
}

pub fn validate_macro_file(path: &str) -> AutomationResult<MacroValidationReport> {
    let value = read_macro_value(path)?;
    let summary = raw_summary(&value);
    let Some(object) = value.as_object() else {
        return Ok(validation_report(
            summary,
            vec!["invalid macro document".to_string()],
        ));
    };
    let mut issues = Vec::new();
    if object
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        issues.push("macro id is missing".to_string());
    }
    let raw_steps = match object.get("steps").and_then(Value::as_array) {
        Some(steps) if !steps.is_empty() => steps,
        _ => {
            issues.push("macro has no steps".to_string());
            return Ok(validation_report(summary, issues));
        }
    };
    let mut parsed_steps = Vec::with_capacity(raw_steps.len());
    for (index, step) in raw_steps.iter().enumerate() {
        match parse_step((index, step)) {
            Ok(step) => parsed_steps.push(step),
            Err(issue) => issues.push(issue),
        }
    }
    if !issues.is_empty() {
        return Ok(validation_report(summary, issues));
    }
    let draft = MacroDraft {
        id: summary.id.clone(),
        steps: parsed_steps,
    };
    let steps = draft
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| build_unresolved_plan_step(index, step))
        .collect::<AutomationResult<Vec<_>>>()?;
    let batch = batch_from_steps(&draft.id, &steps);
    let validation = validate_batch(&batch);
    Ok(validation_report(summary, validation.issues))
}

pub fn normalize_macro_file(input: &str, output: &str) -> AutomationResult<String> {
    let draft = load_valid_macro(input)?;
    let output = absolute_path(output)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(
        &output,
        serde_json::to_vec_pretty(&draft).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write {}: {error}", output.display()))?;
    Ok(output.to_string_lossy().to_string())
}

pub fn render_macro_file(
    path: &str,
    payload: Value,
    state: Value,
) -> AutomationResult<super::AutomationEnvelope> {
    let input_path = absolute_path(path)?;
    let draft = load_valid_macro(path)?;
    build_envelope_from_macro(
        AutomationSource {
            kind: "macro_file".to_string(),
            preset_id: None,
            preset_name: None,
            project_id: None,
            updated_at: None,
            input_path: Some(input_path.to_string_lossy().to_string()),
        },
        &draft,
        object_or_empty(payload),
        object_or_empty(state),
    )
}

pub fn run_macro_file(
    path: &str,
    payload: Value,
    state: Value,
    options: &AutomationRunOptions,
) -> AutomationResult<AutomationRunReport> {
    let envelope = render_macro_file(path, payload, state)?;
    run_envelope(&envelope, options)
}

pub(crate) fn parse_macro_value(value: Value) -> AutomationResult<MacroDraft> {
    let object = value
        .as_object()
        .ok_or_else(|| "invalid macro document".to_string())?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("macro/imported")
        .to_string();
    let steps = object
        .get("steps")
        .and_then(Value::as_array)
        .filter(|steps| !steps.is_empty())
        .ok_or_else(|| "macro document does not contain any steps".to_string())?
        .iter()
        .enumerate()
        .map(parse_step)
        .collect::<AutomationResult<Vec<_>>>()?;
    Ok(MacroDraft { id, steps })
}

fn parse_step((index, value): (usize, &Value)) -> AutomationResult<MacroStep> {
    let action = value
        .get("action")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("step {index} is missing action"))?;
    if kyuubiki_headless_sdk::find_action_contract(canonical_action(action)).is_none() {
        return Err(format!("step {index} uses unsupported action \"{action}\""));
    }
    let payload = value.get("payload").cloned().unwrap_or_else(|| json!({}));
    if !payload.is_object() {
        return Err(format!("step {index} has an invalid payload"));
    }
    Ok(MacroStep {
        action: action.to_string(),
        payload,
    })
}

fn load_valid_macro(path: &str) -> AutomationResult<MacroDraft> {
    let draft = parse_macro_value(read_macro_value(path)?)?;
    let report = validate_draft(&draft);
    if report.ok {
        Ok(draft)
    } else {
        Err(report.issues.join("; "))
    }
}

fn validate_draft(draft: &MacroDraft) -> MacroValidationReport {
    let steps = draft
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| build_unresolved_plan_step(index, step))
        .collect::<AutomationResult<Vec<_>>>();
    match steps {
        Ok(steps) => {
            let validation = validate_batch(&batch_from_steps(&draft.id, &steps));
            validation_report(summary(draft), validation.issues)
        }
        Err(issue) => validation_report(summary(draft), vec![issue]),
    }
}

fn read_macro_value(path: &str) -> AutomationResult<Value> {
    let path = absolute_path(path)?;
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("invalid {}: {error}", path.display()))
}

fn absolute_path(path: &str) -> AutomationResult<PathBuf> {
    let path = Path::new(path);
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .map_err(|error| format!("failed to resolve current directory: {error}"))
    }
}

fn summary(draft: &MacroDraft) -> MacroSummary {
    MacroSummary {
        id: draft.id.clone(),
        step_count: draft.steps.len(),
        actions: draft.steps.iter().map(|step| step.action.clone()).collect(),
    }
}

fn raw_summary(value: &Value) -> MacroSummary {
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("macro/imported")
        .to_string();
    let raw_steps = value
        .get("steps")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let actions = raw_steps
        .iter()
        .filter_map(|step| step.get("action").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<Vec<_>>();
    MacroSummary {
        id,
        step_count: raw_steps.len(),
        actions,
    }
}

fn validation_report(summary: MacroSummary, issues: Vec<String>) -> MacroValidationReport {
    MacroValidationReport {
        ok: issues.is_empty(),
        issue_count: issues.len(),
        issues,
        summary,
    }
}
