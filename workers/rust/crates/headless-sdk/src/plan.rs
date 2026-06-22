use crate::{HeadlessEngine, HeadlessExecutionBatch, HeadlessRisk, validate_batch};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessPlanBinding {
    pub source_step: usize,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessPlanStep {
    pub index: usize,
    pub action: String,
    pub engine: Option<HeadlessEngine>,
    pub category: Option<String>,
    pub risk: HeadlessRisk,
    pub requires_confirmation: bool,
    pub confirmation_flag: Option<String>,
    pub confirmation_reason: String,
    pub payload: Value,
    pub bindings: Vec<HeadlessPlanBinding>,
    pub output_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessPlanConfirmation {
    pub index: usize,
    pub action: String,
    pub risk: HeadlessRisk,
    pub flag: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessPlanCompatibility {
    pub service_only_ok: bool,
    pub service_only_reason: String,
    pub browser_session_required: bool,
    pub hybrid_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessExecutorPlan {
    pub executor: String,
    pub compatible: bool,
    pub issue_count: usize,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessExecutionPlan {
    pub schema_version: String,
    pub workflow_id: String,
    pub ok: bool,
    pub validation: crate::HeadlessValidationReport,
    pub policy: Option<crate::HeadlessPolicySummary>,
    pub compatibility: HeadlessPlanCompatibility,
    pub executor_matrix: Vec<HeadlessExecutorPlan>,
    pub confirmation_count: usize,
    pub confirmations: Vec<HeadlessPlanConfirmation>,
    pub steps: Vec<HeadlessPlanStep>,
}

pub fn build_execution_plan(batch: &HeadlessExecutionBatch) -> HeadlessExecutionPlan {
    let validation = validate_batch(batch);
    let policy = validation.policy.clone();
    let steps = batch
        .steps
        .iter()
        .map(|step| {
            let contract = crate::find_action_contract(&step.action);
            let confirmation = resolve_confirmation(step.risk);
            HeadlessPlanStep {
                index: step.index,
                action: step.action.clone(),
                engine: contract.map(|entry| entry.engine),
                category: contract.map(|entry| entry.category.to_string()),
                risk: step.risk,
                requires_confirmation: confirmation.is_some(),
                confirmation_flag: confirmation.as_ref().map(|entry| entry.0.to_string()),
                confirmation_reason: confirmation
                    .as_ref()
                    .map(|entry| entry.1.to_string())
                    .unwrap_or_else(|| "normal-risk step".to_string()),
                payload: step.payload.clone(),
                bindings: collect_bindings(&step.payload),
                output_keys: contract
                    .map(|entry| {
                        entry
                            .output_keys
                            .iter()
                            .map(|key| (*key).to_string())
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        })
        .collect::<Vec<_>>();
    let confirmations = steps
        .iter()
        .filter_map(|step| {
            Some(HeadlessPlanConfirmation {
                index: step.index,
                action: step.action.clone(),
                risk: step.risk,
                flag: step.confirmation_flag.clone()?,
                reason: step.confirmation_reason.clone(),
            })
        })
        .collect::<Vec<_>>();

    HeadlessExecutionPlan {
        schema_version: "kyuubiki.headless-plan/v1".to_string(),
        workflow_id: batch.workflow_id.clone(),
        ok: validation.ok,
        validation,
        policy: policy.clone(),
        compatibility: build_compatibility(policy.as_ref()),
        executor_matrix: build_executor_matrix(batch),
        confirmation_count: confirmations.len(),
        confirmations,
        steps,
    }
}

pub fn build_executor_matrix(batch: &HeadlessExecutionBatch) -> Vec<HeadlessExecutorPlan> {
    ["mock", "service", "hybrid"]
        .iter()
        .map(|executor| {
            let issues = crate::collect_executor_compatibility_issues(batch, executor);
            HeadlessExecutorPlan {
                executor: (*executor).to_string(),
                compatible: issues.is_empty(),
                issue_count: issues.len(),
                issues,
            }
        })
        .collect()
}

fn resolve_confirmation(risk: HeadlessRisk) -> Option<(&'static str, &'static str)> {
    match risk {
        HeadlessRisk::Sensitive => Some((
            "--allow-sensitive",
            "sensitive step must be explicitly allowed before live execution",
        )),
        HeadlessRisk::Destructive => Some((
            "--allow-destructive",
            "destructive step must be explicitly allowed before live execution",
        )),
        HeadlessRisk::Normal => None,
    }
}

fn build_compatibility(policy: Option<&crate::HeadlessPolicySummary>) -> HeadlessPlanCompatibility {
    let service_only_ok = policy
        .map(|entry| entry.safe_for_service_only)
        .unwrap_or(false);
    let has_browser = policy
        .map(|entry| entry.required_engines.contains(&HeadlessEngine::Browser))
        .unwrap_or(false);
    let has_service = policy
        .map(|entry| entry.required_engines.contains(&HeadlessEngine::Service))
        .unwrap_or(false);
    HeadlessPlanCompatibility {
        service_only_ok,
        service_only_reason: if service_only_ok {
            "all steps are service-backed".to_string()
        } else {
            "workflow includes browser-backed steps".to_string()
        },
        browser_session_required: policy
            .map(|entry| entry.needs_desktop_browser)
            .unwrap_or(false),
        hybrid_required: has_browser && has_service,
    }
}

fn collect_bindings(value: &Value) -> Vec<HeadlessPlanBinding> {
    let mut bindings = Vec::new();
    collect_bindings_into(value, &mut bindings);
    bindings
}

fn collect_bindings_into(value: &Value, bindings: &mut Vec<HeadlessPlanBinding>) {
    match value {
        Value::String(text) => {
            if let Some(binding) = parse_binding(text) {
                bindings.push(binding);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_bindings_into(item, bindings);
            }
        }
        Value::Object(fields) => {
            for item in fields.values() {
                collect_bindings_into(item, bindings);
            }
        }
        _ => {}
    }
}

fn parse_binding(text: &str) -> Option<HeadlessPlanBinding> {
    let inner = text.trim().strip_prefix("{{")?.strip_suffix("}}")?.trim();
    let rest = inner.strip_prefix("steps.")?;
    let (step_text, output_path) = rest.split_once(".result.")?;
    Some(HeadlessPlanBinding {
        source_step: step_text.parse().ok()?,
        output: output_path.trim().to_string(),
    })
}
