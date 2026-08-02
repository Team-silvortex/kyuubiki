use crate::{
    MODEL_HEADLESS_PLAN_SCHEMA_VERSION, MODEL_PLAN_APPROVAL_SCHEMA_VERSION, ModelHeadlessPlan,
    SdkError, SdkResult,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

pub const MODEL_PLAN_APPROVAL_REQUEST_SCHEMA_VERSION: &str =
    "kyuubiki.model-plan-approval-request/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPlanApprovalRequestStep {
    pub index: usize,
    pub action: String,
    pub risk: String,
    pub confirmation_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPlanApprovalRequest {
    pub schema_version: String,
    pub plan_schema_version: String,
    pub plan_digest: String,
    pub session_id: String,
    pub workflow_id: String,
    pub status: String,
    pub execution_authority: String,
    pub approval_schema_version: String,
    pub required_steps: Vec<ModelPlanApprovalRequestStep>,
}

pub fn compute_model_headless_plan_digest(plan: &ModelHeadlessPlan) -> SdkResult<String> {
    let value = serde_json::to_value(plan)?;
    let digest = Sha256::digest(canonical_json(&value).as_bytes());
    Ok(format!("sha256:{digest:x}"))
}

pub fn build_model_plan_approval_request(
    plan: &ModelHeadlessPlan,
) -> SdkResult<ModelPlanApprovalRequest> {
    validate_plan(plan)?;
    let required_steps = plan
        .steps
        .iter()
        .filter(|step| step.requires_confirmation)
        .map(|step| {
            let confirmation_reason = step
                .confirmation_reason
                .clone()
                .filter(|reason| !reason.trim().is_empty())
                .ok_or_else(|| SdkError::Validation {
                    errors: vec![format!(
                        "gated model plan step {} requires a confirmation_reason",
                        step.index
                    )],
                })?;
            let risk = match step.risk {
                crate::HeadlessModelRisk::Sensitive => "sensitive",
                crate::HeadlessModelRisk::Destructive => "destructive",
                crate::HeadlessModelRisk::Normal => {
                    return Err(SdkError::Validation {
                        errors: vec![format!(
                            "gated model plan step {} has invalid risk",
                            step.index
                        )],
                    });
                }
            };
            Ok(ModelPlanApprovalRequestStep {
                index: step.index,
                action: step.action.clone(),
                risk: risk.to_string(),
                confirmation_reason,
            })
        })
        .collect::<SdkResult<Vec<_>>>()?;
    let status = if required_steps.is_empty() {
        "not_required"
    } else {
        "approval_required"
    };
    Ok(ModelPlanApprovalRequest {
        schema_version: MODEL_PLAN_APPROVAL_REQUEST_SCHEMA_VERSION.to_string(),
        plan_schema_version: plan.schema_version.clone(),
        plan_digest: compute_model_headless_plan_digest(plan)?,
        session_id: plan.session_id.clone(),
        workflow_id: plan.workflow_id.clone(),
        status: status.to_string(),
        execution_authority: "none_approval_request_only".to_string(),
        approval_schema_version: MODEL_PLAN_APPROVAL_SCHEMA_VERSION.to_string(),
        required_steps,
    })
}

fn validate_plan(plan: &ModelHeadlessPlan) -> SdkResult<()> {
    let mut errors = Vec::new();
    if plan.schema_version != MODEL_HEADLESS_PLAN_SCHEMA_VERSION {
        errors.push(format!(
            "unsupported model plan schema_version: {}",
            plan.schema_version
        ));
    }
    if !plan.ok || !plan.issues.is_empty() {
        errors.push("model plan must be valid and issue-free before approval".to_string());
    }
    if plan.session_id.trim().is_empty() || plan.workflow_id.trim().is_empty() {
        errors.push("model plan session_id and workflow_id are required".to_string());
    }
    if plan.steps.is_empty() {
        errors.push("model plan contains no steps".to_string());
    }
    if plan
        .steps
        .iter()
        .enumerate()
        .any(|(offset, step)| step.index != offset + 1)
    {
        errors.push("model plan step indexes must be contiguous and one-based".to_string());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        errors.sort();
        errors.dedup();
        Err(SdkError::Validation { errors })
    }
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(object) => canonical_object_json(object),
        Value::Array(values) => {
            let parts = values.iter().map(canonical_json).collect::<Vec<_>>();
            format!("[{}]", parts.join(","))
        }
        Value::Number(number) => canonical_number_json(number),
        _ => serde_json::to_string(value).expect("JSON scalar should encode"),
    }
}

fn canonical_object_json(object: &Map<String, Value>) -> String {
    let mut keys = object.keys().collect::<Vec<_>>();
    keys.sort();
    let parts = keys
        .into_iter()
        .map(|key| {
            let encoded_key = serde_json::to_string(key).expect("JSON object key should encode");
            format!("{encoded_key}:{}", canonical_json(&object[key]))
        })
        .collect::<Vec<_>>();
    format!("{{{}}}", parts.join(","))
}

fn canonical_number_json(number: &serde_json::Number) -> String {
    if let Some(value) = number.as_i64() {
        return value.to_string();
    }
    if let Some(value) = number.as_u64() {
        return value.to_string();
    }
    let value = number.as_f64().expect("JSON number should be finite");
    let mut encoded = format!("{value:.15}");
    while encoded.ends_with('0') {
        encoded.pop();
    }
    if encoded.ends_with('.') {
        encoded.push('0');
    }
    encoded
}
