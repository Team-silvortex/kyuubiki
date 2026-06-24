use crate::{HeadlessEngine, HeadlessRisk, HeadlessRuntimeStyle, find_action_contract};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessTemplateDescriptor {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub runtime_style: HeadlessRuntimeStyle,
    pub category: &'static str,
    pub tags: &'static [&'static str],
}

impl HeadlessTemplateDescriptor {
    pub fn default_workflow_id(&self) -> String {
        format!("template.{}", self.id)
    }

    pub fn to_snapshot(&self) -> HeadlessTemplateSnapshot {
        HeadlessTemplateSnapshot {
            id: self.id.to_string(),
            title: self.title.to_string(),
            description: self.description.to_string(),
            runtime_style: self.runtime_style,
            category: self.category.to_string(),
            tags: self.tags.iter().map(|tag| (*tag).to_string()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessTemplateSnapshot {
    pub id: String,
    pub title: String,
    pub description: String,
    pub runtime_style: HeadlessRuntimeStyle,
    pub category: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessWorkflowStep {
    pub action: String,
    #[serde(default)]
    pub payload: Value,
}

impl HeadlessWorkflowStep {
    pub fn new(action: &str, payload: Value) -> Self {
        Self {
            action: action.to_string(),
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessWorkflowDraft {
    pub id: String,
    pub steps: Vec<HeadlessWorkflowStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessWorkflowDocument {
    pub schema_version: String,
    pub exported_at: String,
    pub language: String,
    pub workflow: HeadlessWorkflowDraft,
    #[serde(default)]
    pub template: Option<HeadlessTemplateSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessExecutionBatchStep {
    pub index: usize,
    pub action: String,
    pub risk: HeadlessRisk,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessExecutionBatch {
    pub schema_version: String,
    pub exported_at: String,
    pub language: String,
    pub workflow_id: String,
    pub steps: Vec<HeadlessExecutionBatchStep>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessBatchSummary {
    pub schema_version: String,
    pub workflow_id: String,
    pub exported_at: String,
    pub language: String,
    pub step_count: usize,
    pub warning_count: usize,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessPolicySummary {
    pub required_engines: Vec<HeadlessEngine>,
    pub engine_counts: HashMap<String, usize>,
    pub risk_counts: HashMap<String, usize>,
    pub recommended_runtime: HeadlessRuntimeStyle,
    pub needs_desktop_browser: bool,
    pub safe_for_service_only: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessValidationReport {
    pub ok: bool,
    pub issue_count: usize,
    pub issues: Vec<String>,
    pub warning_count: usize,
    pub warnings: Vec<String>,
    pub summary: Option<HeadlessBatchSummary>,
    pub policy: Option<HeadlessPolicySummary>,
}

pub fn normalize_workflow_document(
    document: &HeadlessWorkflowDocument,
) -> Result<HeadlessExecutionBatch, String> {
    if document.workflow.id.trim().is_empty() || document.workflow.steps.is_empty() {
        return Err(
            "Headless workflow document does not contain a valid workflow draft.".to_string(),
        );
    }
    let mut warnings = Vec::new();
    let steps = document
        .workflow
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            if step.action.trim().is_empty() {
                return Err(format!("Headless workflow step {} is invalid.", index + 1));
            }
            collect_inline_template_warnings(&step.payload, &mut warnings);
            Ok(HeadlessExecutionBatchStep {
                index: index + 1,
                action: step.action.clone(),
                risk: find_action_contract(&step.action)
                    .map(|it| it.risk)
                    .unwrap_or(HeadlessRisk::Normal),
                payload: step.payload.clone(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
        exported_at: document.exported_at.clone(),
        language: document.language.clone(),
        workflow_id: document.workflow.id.clone(),
        steps,
        warnings,
    })
}

pub fn summarize_batch(batch: &HeadlessExecutionBatch) -> HeadlessBatchSummary {
    HeadlessBatchSummary {
        schema_version: batch.schema_version.clone(),
        workflow_id: batch.workflow_id.clone(),
        exported_at: batch.exported_at.clone(),
        language: batch.language.clone(),
        step_count: batch.steps.len(),
        warning_count: batch.warnings.len(),
        actions: batch.steps.iter().map(|step| step.action.clone()).collect(),
    }
}

pub fn validate_batch(batch: &HeadlessExecutionBatch) -> HeadlessValidationReport {
    let mut issues = Vec::new();
    if batch.schema_version != "kyuubiki.headless-execution-batch/v1" {
        issues.push(format!(
            "unsupported schema_version: {}",
            batch.schema_version
        ));
    }
    if batch.workflow_id.trim().is_empty() {
        issues.push("workflow_id is missing".to_string());
    }
    if batch.steps.is_empty() {
        issues.push("headless batch has no steps".to_string());
    }
    let mut known_outputs = HashMap::new();
    for (array_index, step) in batch.steps.iter().enumerate() {
        let step_number = array_index + 1;
        if step.index != step_number {
            issues.push(format!(
                "step {} index should be {}, received {}",
                step_number, step_number, step.index
            ));
        }
        if let Some(contract) = find_action_contract(&step.action) {
            for key in missing_required_keys(contract.id, &step.payload) {
                issues.push(format!(
                    "step {} ({}) is missing required payload key {}",
                    step_number, step.action, key
                ));
            }
            known_outputs.insert(
                step_number,
                contract
                    .output_keys
                    .iter()
                    .map(|key| (*key).to_string())
                    .collect::<BTreeSet<_>>(),
            );
        } else {
            issues.push(format!(
                "step {} references unsupported action {}",
                step_number, step.action
            ));
        }
        collect_binding_issues(&step.payload, step_number, &known_outputs, &mut issues);
    }
    HeadlessValidationReport {
        ok: issues.is_empty(),
        issue_count: issues.len(),
        issues,
        warning_count: batch.warnings.len(),
        warnings: batch.warnings.clone(),
        summary: Some(summarize_batch(batch)),
        policy: Some(build_policy_summary(batch)),
    }
}

fn collect_inline_template_warnings(value: &Value, warnings: &mut Vec<String>) {
    match value {
        Value::String(text) if text.contains("{{") && parse_binding(text).is_none() => {
            warnings.push(format!(
                "Unresolved inline template kept as literal: {text}"
            ));
        }
        Value::Array(items) => items
            .iter()
            .for_each(|item| collect_inline_template_warnings(item, warnings)),
        Value::Object(fields) => fields
            .values()
            .for_each(|value| collect_inline_template_warnings(value, warnings)),
        _ => {}
    }
}

fn collect_binding_issues(
    value: &Value,
    step_index: usize,
    known_outputs: &HashMap<usize, BTreeSet<String>>,
    issues: &mut Vec<String>,
) {
    match value {
        Value::String(text) => {
            if let Some((referenced_step, referenced_output)) = parse_binding(text) {
                if referenced_step >= step_index {
                    issues.push(format!(
                        "step {} cannot bind to future-or-self step {}",
                        step_index, referenced_step
                    ));
                } else if let Some(outputs) = known_outputs.get(&referenced_step) {
                    if !outputs.contains(&referenced_output) {
                        issues.push(format!(
                            "step {} references unavailable output \"{}\" from step {}",
                            step_index, referenced_output, referenced_step
                        ));
                    }
                } else {
                    issues.push(format!(
                        "step {} references missing source step {}",
                        step_index, referenced_step
                    ));
                }
            }
        }
        Value::Array(items) => items
            .iter()
            .for_each(|item| collect_binding_issues(item, step_index, known_outputs, issues)),
        Value::Object(fields) => fields
            .values()
            .for_each(|value| collect_binding_issues(value, step_index, known_outputs, issues)),
        _ => {}
    }
}

fn parse_binding(text: &str) -> Option<(usize, String)> {
    let trimmed = text.trim();
    let inner = trimmed.strip_prefix("{{")?.strip_suffix("}}")?.trim();
    let rest = inner.strip_prefix("steps.")?;
    let (step_text, output_path) = rest.split_once(".result.")?;
    Some((step_text.parse().ok()?, output_path.trim().to_string()))
}

fn has_present_value(payload: &Value, key: &str) -> bool {
    payload
        .get(key)
        .map(|value| match value {
            Value::Null => false,
            Value::String(text) => !text.trim().is_empty(),
            _ => true,
        })
        .unwrap_or(false)
}

fn missing_required_keys(action: &str, payload: &Value) -> Vec<&'static str> {
    match action {
        "open_page" => (!has_present_value(payload, "url") && !has_present_value(payload, "href"))
            .then_some("url")
            .into_iter()
            .collect(),
        "click" => (!has_present_value(payload, "selector")
            && !has_present_value(payload, "target"))
        .then_some("selector")
        .into_iter()
        .collect(),
        "type" => {
            let has_selector =
                has_present_value(payload, "selector") || has_present_value(payload, "target");
            let has_value = has_present_value(payload, "value")
                || has_present_value(payload, "text")
                || has_present_value(payload, "input");
            [
                (!has_selector).then_some("selector"),
                (!has_value).then_some("value"),
            ]
            .into_iter()
            .flatten()
            .collect()
        }
        "wait" => {
            let has_selector =
                has_present_value(payload, "selector") || has_present_value(payload, "target");
            let has_duration = has_present_value(payload, "duration")
                || has_present_value(payload, "durationMs")
                || has_present_value(payload, "timeout");
            (!has_selector && !has_duration)
                .then_some("selector|duration")
                .into_iter()
                .collect()
        }
        "select" => {
            let has_selector =
                has_present_value(payload, "selector") || has_present_value(payload, "target");
            let has_value =
                has_present_value(payload, "value") || has_present_value(payload, "values");
            [
                (!has_selector).then_some("selector"),
                (!has_value).then_some("value"),
            ]
            .into_iter()
            .flatten()
            .collect()
        }
        "assert_text" => {
            let has_selector =
                has_present_value(payload, "selector") || has_present_value(payload, "target");
            [
                (!has_selector).then_some("selector"),
                (!has_present_value(payload, "text")).then_some("text"),
            ]
            .into_iter()
            .flatten()
            .collect()
        }
        "direct_mesh_solve" => {
            let has_study = has_present_value(payload, "study_kind")
                || has_present_value(payload, "model_id")
                || has_present_value(payload, "model_version_id");
            let has_source = has_present_value(payload, "input")
                || has_present_value(payload, "model_payload")
                || has_present_value(payload, "model_id")
                || has_present_value(payload, "model_version_id");
            [
                (!has_study).then_some("study_kind|model_id|model_version_id"),
                (!has_source).then_some("input|model_payload|model_id|model_version_id"),
                (!has_present_value(payload, "endpoints")).then_some("endpoints"),
            ]
            .into_iter()
            .flatten()
            .collect()
        }
        "project_update" => {
            let has_patch =
                has_present_value(payload, "name") || has_present_value(payload, "description");
            [
                (!has_present_value(payload, "project_id")).then_some("project_id"),
                (!has_patch).then_some("name|description"),
            ]
            .into_iter()
            .flatten()
            .collect()
        }
        _ => find_action_contract(action)
            .map(|contract| {
                contract
                    .required_payload_keys
                    .iter()
                    .copied()
                    .filter(|key| !has_present_value(payload, key))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn build_policy_summary(batch: &HeadlessExecutionBatch) -> HeadlessPolicySummary {
    let contracts = batch
        .steps
        .iter()
        .filter_map(|step| find_action_contract(&step.action))
        .collect::<Vec<_>>();
    let browser_count = contracts
        .iter()
        .filter(|contract| contract.engine == HeadlessEngine::Browser)
        .count();
    let service_count = contracts
        .iter()
        .filter(|contract| contract.engine == HeadlessEngine::Service)
        .count();
    let sensitive_count = contracts
        .iter()
        .filter(|contract| contract.risk == HeadlessRisk::Sensitive)
        .count();
    let destructive_count = contracts
        .iter()
        .filter(|contract| contract.risk == HeadlessRisk::Destructive)
        .count();
    let runtime = if browser_count > 0 && service_count > 0 {
        HeadlessRuntimeStyle::Hybrid
    } else if browser_count > 0 {
        HeadlessRuntimeStyle::BrowserOnly
    } else if service_count > 0 {
        HeadlessRuntimeStyle::ServiceOnly
    } else {
        HeadlessRuntimeStyle::Unknown
    };
    let mut notes = Vec::new();
    if browser_count > 0 {
        notes.push("Includes browser-backed steps; live execution needs a desktop session that can launch a local browser.".to_string());
    }
    if sensitive_count > 0 {
        notes.push(
            "Includes sensitive steps; live execution should pass --allow-sensitive after review."
                .to_string(),
        );
    }
    if destructive_count > 0 {
        notes.push("Includes destructive steps; live execution should pass --allow-destructive only after explicit confirmation.".to_string());
    }
    HeadlessPolicySummary {
        required_engines: contracts
            .iter()
            .map(|contract| contract.engine)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        engine_counts: HashMap::from([
            ("browser".to_string(), browser_count),
            ("service".to_string(), service_count),
        ]),
        risk_counts: HashMap::from([
            (
                "normal".to_string(),
                contracts
                    .len()
                    .saturating_sub(sensitive_count + destructive_count),
            ),
            ("sensitive".to_string(), sensitive_count),
            ("destructive".to_string(), destructive_count),
        ]),
        recommended_runtime: runtime,
        needs_desktop_browser: browser_count > 0,
        safe_for_service_only: browser_count == 0,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_future_binding_reference() {
        let mut document = crate::build_template_document("workflow_submit_monitor", None).unwrap();
        document.workflow.steps[1].payload =
            serde_json::json!({ "job_id": "{{steps.9.result.job_id}}" });
        let report = validate_batch(&normalize_workflow_document(&document).unwrap());
        assert!(!report.ok);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.contains("future-or-self step 9"))
        );
    }
}
