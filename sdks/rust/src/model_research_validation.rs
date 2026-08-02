use crate::{
    MATERIAL_RESEARCH_BUNDLE_SCHEMA_VERSION, MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION,
    MaterialResearchBundle, ModelFrontierVerifier, ModelReceiptVerifier,
    ModelResearchExecutionReceipt, ModelResearchExecutionStatus, ModelResearchFrontier,
    ModelResearchFrontierStage, SdkError, SdkResult, WorkflowGraphDefinition,
    validate_material_research_bundle, validate_model_research_frontier,
    validate_workflow_result_against_graph,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

pub const MODEL_RESEARCH_VALIDATION_REPORT_SCHEMA_VERSION: &str =
    "kyuubiki.model-research-validation-report/v2";

const CLAIM_BOUNDARY: &str = "screening_only_not_qualification";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelResearchValidationStage {
    WorkflowResultValidated,
    ScreeningBundleValidated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelResearchWorkflowValidation {
    pub graph_id: String,
    pub graph_version: String,
    pub runtime_status: String,
    pub artifact_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelResearchBundleValidation {
    pub schema_version: String,
    pub bundle_id: String,
    pub study: String,
    pub reliability_decision: String,
    pub validation_readiness_score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelResearchValidationReport {
    pub schema_version: String,
    pub session_id: String,
    pub workflow_id: String,
    pub job_id: String,
    pub origin_plan_digest: String,
    pub result_plan_digest: String,
    pub stage: ModelResearchValidationStage,
    pub claim_boundary: String,
    pub external_validation_required: bool,
    pub workflow_result: ModelResearchWorkflowValidation,
    pub material_bundle: Option<ModelResearchBundleValidation>,
    pub next_actions: Vec<String>,
}

pub fn validate_model_research_frontier_result<
    R: ModelReceiptVerifier + ?Sized,
    F: ModelFrontierVerifier + ?Sized,
>(
    frontier: &ModelResearchFrontier,
    result_receipt: &ModelResearchExecutionReceipt,
    graph: &WorkflowGraphDefinition,
    bundle: Option<&MaterialResearchBundle>,
    frontier_verifier: &F,
    receipt_verifier: &R,
) -> SdkResult<ModelResearchValidationReport> {
    validate_frontier_binding(frontier)?;
    frontier_verifier.verify_model_frontier(frontier)?;
    validate_result_receipt(frontier, result_receipt)?;
    receipt_verifier.verify_model_receipt(result_receipt)?;
    if graph.id != frontier.workflow_id {
        return validation_error("workflow graph id does not match research frontier workflow_id");
    }

    let record = result_receipt
        .records
        .last()
        .expect("receipt validation requires a record");
    let payload = record
        .output
        .as_ref()
        .expect("receipt validation requires output");
    let validated = validate_workflow_result_against_graph(graph, payload)?;
    let runtime_status = validated
        .workflow_runtime
        .status
        .as_deref()
        .ok_or_else(|| validation("workflow result runtime status is required"))?;
    if runtime_status != "completed" {
        return validation_error("workflow result runtime status must be completed");
    }
    if let Some(workflow_id) = validated.workflow_runtime.workflow_id.as_deref()
        && workflow_id != frontier.workflow_id
    {
        return validation_error("workflow result runtime workflow_id does not match frontier");
    }
    let mut artifact_keys = validated.artifacts.keys().cloned().collect::<Vec<_>>();
    artifact_keys.sort();
    if artifact_keys.is_empty() {
        return validation_error("workflow result validation produced no retained artifacts");
    }

    let (stage, material_bundle, next_actions) = match bundle {
        Some(bundle) => {
            validate_material_research_bundle(bundle)?;
            let readiness_score = bundle
                .validation_evidence
                .pointer("/validation_readiness/score")
                .and_then(Value::as_f64)
                .ok_or_else(|| {
                    validation("material bundle validation readiness score is missing")
                })?;
            let mut actions = string_array(
                bundle
                    .validation_evidence
                    .pointer("/validation_readiness/next_validation_actions"),
            )?;
            let mut seen = HashSet::new();
            actions.retain(|action| seen.insert(action.clone()));
            if !actions
                .iter()
                .any(|action| action == "external_validation_required")
            {
                actions.push("external_validation_required".to_string());
            }
            (
                ModelResearchValidationStage::ScreeningBundleValidated,
                Some(ModelResearchBundleValidation {
                    schema_version: MATERIAL_RESEARCH_BUNDLE_SCHEMA_VERSION.to_string(),
                    bundle_id: bundle.bundle_id.clone(),
                    study: bundle.study.clone(),
                    reliability_decision: bundle.summary.reliability_decision.clone(),
                    validation_readiness_score: readiness_score,
                }),
                actions,
            )
        }
        None => (
            ModelResearchValidationStage::WorkflowResultValidated,
            None,
            vec![
                "build_or_attach_material_research_bundle".to_string(),
                "external_validation_required".to_string(),
            ],
        ),
    };

    Ok(ModelResearchValidationReport {
        schema_version: MODEL_RESEARCH_VALIDATION_REPORT_SCHEMA_VERSION.to_string(),
        session_id: frontier.session_id.clone(),
        workflow_id: frontier.workflow_id.clone(),
        job_id: frontier.job_id.clone().expect("frontier binding validated"),
        origin_plan_digest: frontier.origin_plan_digest.clone(),
        result_plan_digest: result_receipt.plan_digest.clone(),
        stage,
        claim_boundary: CLAIM_BOUNDARY.to_string(),
        external_validation_required: true,
        workflow_result: ModelResearchWorkflowValidation {
            graph_id: validated.graph_id,
            graph_version: validated.graph_version,
            runtime_status: runtime_status.to_string(),
            artifact_keys,
        },
        material_bundle,
        next_actions,
    })
}

fn validate_frontier_binding(frontier: &ModelResearchFrontier) -> SdkResult<()> {
    validate_model_research_frontier(frontier)?;
    if frontier.stage != ModelResearchFrontierStage::ReadyToValidate
        || frontier.next_action.is_some()
        || frontier.blocking_reason.is_some()
        || frontier
            .job_id
            .as_deref()
            .is_none_or(|job_id| job_id.trim().is_empty())
    {
        return validation_error("research frontier is not ready for result validation");
    }
    Ok(())
}

fn validate_result_receipt(
    frontier: &ModelResearchFrontier,
    receipt: &ModelResearchExecutionReceipt,
) -> SdkResult<()> {
    let record = receipt
        .records
        .last()
        .ok_or_else(|| validation("result receipt has no execution record"))?;
    if receipt.schema_version != MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION
        || receipt.execution_authority != "kyuubiki-headless-sdk"
        || receipt.status != ModelResearchExecutionStatus::Completed
        || receipt.session_id != frontier.session_id
        || receipt.workflow_id != frontier.workflow_id
        || !valid_plan_digest(&receipt.plan_digest)
        || receipt.plan_digest != frontier.evidence.plan_digest
        || record.action != "result_fetch"
        || record.job_id.as_deref() != frontier.job_id.as_deref()
        || record.authority.as_deref().is_none_or(str::is_empty)
        || record.output.is_none()
        || record.error.is_some()
    {
        return validation_error("result receipt does not match the verified research frontier");
    }
    Ok(())
}

fn string_array(value: Option<&Value>) -> SdkResult<Vec<String>> {
    let items = value
        .and_then(Value::as_array)
        .ok_or_else(|| validation("material bundle next validation actions must be an array"))?;
    let actions = items
        .iter()
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()
        .filter(|actions| !actions.is_empty() && actions.iter().all(|item| !item.is_empty()))
        .ok_or_else(|| {
            validation("material bundle next validation actions must be non-empty strings")
        })?;
    Ok(actions.into_iter().map(str::to_string).collect())
}

fn valid_plan_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn validation(message: impl Into<String>) -> SdkError {
    SdkError::Validation {
        errors: vec![message.into()],
    }
}

fn validation_error<T>(message: impl Into<String>) -> SdkResult<T> {
    Err(validation(message))
}
