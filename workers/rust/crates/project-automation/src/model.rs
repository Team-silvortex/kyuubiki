use kyuubiki_headless_sdk::{HeadlessEngine, HeadlessExecutionStepReport, HeadlessRisk};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationPresetSummary {
    pub preset_id: String,
    pub project_id: String,
    pub name: String,
    pub updated_at: String,
    pub macro_id: Option<String>,
    pub step_count: usize,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationSource {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroStep {
    pub action: String,
    #[serde(default = "empty_object", skip_serializing_if = "is_empty_object")]
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroDraft {
    pub id: String,
    pub steps: Vec<MacroStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroSummary {
    pub id: String,
    pub step_count: usize,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroValidationReport {
    pub ok: bool,
    pub issue_count: usize,
    pub issues: Vec<String>,
    pub summary: MacroSummary,
}

fn empty_object() -> Value {
    Value::Object(Default::default())
}

fn is_empty_object(value: &Value) -> bool {
    value.as_object().is_some_and(|object| object.is_empty())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationMetadata {
    pub macro_id: String,
    pub generated_at: String,
    pub step_count: usize,
    pub action_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutomationRiskSummary {
    pub highest_risk: HeadlessRisk,
    pub sensitive_step_count: usize,
    pub destructive_step_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequiredConfirmation {
    pub step_index: usize,
    pub action: String,
    pub risk: HeadlessRisk,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationPlanStep {
    pub index: usize,
    pub action: String,
    #[serde(skip_serializing)]
    pub canonical_action: String,
    #[serde(skip_serializing)]
    pub engine: HeadlessEngine,
    pub risk: HeadlessRisk,
    pub requires_confirmation: bool,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationPlan {
    pub id: String,
    pub step_count: usize,
    pub actions: Vec<String>,
    pub payload: Value,
    pub state: Value,
    pub steps: Vec<AutomationPlanStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationEnvelope {
    pub schema_version: String,
    pub source: AutomationSource,
    pub metadata: AutomationMetadata,
    pub risk_summary: AutomationRiskSummary,
    pub required_confirmations: Vec<RequiredConfirmation>,
    pub plan: AutomationPlan,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AutomationRunOptions {
    pub execute: bool,
    pub allow_sensitive: bool,
    pub allow_destructive: bool,
    pub api_base_url: Option<String>,
    pub api_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestedCapabilities {
    pub allow_sensitive: bool,
    pub allow_destructive: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationStepReport {
    pub index: usize,
    pub action: String,
    pub risk: HeadlessRisk,
    pub requires_confirmation: bool,
    pub payload: Value,
    pub status: String,
    pub message: String,
    pub result: Value,
}

impl AutomationStepReport {
    pub(crate) fn from_sdk(
        step: &HeadlessExecutionStepReport,
        original: &AutomationPlanStep,
        execute: bool,
    ) -> Self {
        let status = match step.status.as_str() {
            "executed" | "executed_mock_browser" => "completed",
            "dry_run" => "simulated",
            other => other,
        };
        let message = match status {
            "completed" => "Step executed.",
            "simulated" => "Dry-run simulation completed.",
            "blocked" => "Step requires explicit risk confirmation.",
            "failed" => step
                .result_preview
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("Step execution failed."),
            _ if execute => "Step execution completed.",
            _ => "Step simulation completed.",
        };
        Self {
            index: original.index,
            action: original.action.clone(),
            risk: step.risk,
            requires_confirmation: step.requires_confirmation,
            payload: step.payload.clone(),
            status: status.to_string(),
            message: message.to_string(),
            result: step.result_preview.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationRunReport {
    pub schema_version: String,
    pub source: AutomationSource,
    pub metadata: AutomationMetadata,
    pub started_at: String,
    pub completed_at: String,
    pub status: String,
    pub dry_run: bool,
    pub requested_capabilities: RequestedCapabilities,
    pub risk_summary: AutomationRiskSummary,
    pub blocked_by_confirmation: Option<AutomationStepReport>,
    pub failed_step: Option<AutomationStepReport>,
    pub executed_step_count: usize,
    pub steps: Vec<AutomationStepReport>,
}
