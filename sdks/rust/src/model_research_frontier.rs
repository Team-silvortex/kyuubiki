use crate::model_plan_approval::compute_canonical_json_digest;
use crate::{
    MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION, MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION,
    ModelResearchExecutionReceipt, ModelResearchExecutionStatus, ModelToolCall,
    ModelWorkflowProposal, SdkError, SdkResult,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const MODEL_RESEARCH_FRONTIER_SCHEMA_VERSION: &str = "kyuubiki.model-research-frontier/v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelResearchFrontierStage {
    WaitingForJob,
    ReadyToFetchResult,
    ReadyToValidate,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelResearchFrontierEvidence {
    pub approval_id: Option<String>,
    pub plan_digest: String,
    pub action: String,
    pub record_index: usize,
    pub authority: Option<String>,
    pub job_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelResearchFrontier {
    pub schema_version: String,
    pub session_id: String,
    pub workflow_id: String,
    pub origin_plan_digest: String,
    pub stage: ModelResearchFrontierStage,
    pub job_id: Option<String>,
    pub next_action: Option<String>,
    pub transition_count: usize,
    pub evidence: ModelResearchFrontierEvidence,
    pub blocking_reason: Option<String>,
}

pub trait ModelReceiptVerifier {
    fn verify_model_receipt(&self, receipt: &ModelResearchExecutionReceipt) -> SdkResult<()>;
}

pub trait ModelFrontierVerifier {
    fn verify_model_frontier(&self, frontier: &ModelResearchFrontier) -> SdkResult<()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelFrontierDigestVerifier {
    expected_digest: String,
}

impl ModelFrontierDigestVerifier {
    pub fn new(expected_digest: impl Into<String>) -> SdkResult<Self> {
        let expected_digest = expected_digest.into();
        if !valid_plan_digest(&expected_digest) {
            return validation_error("expected research frontier digest is invalid");
        }
        Ok(Self { expected_digest })
    }
}

impl ModelFrontierVerifier for ModelFrontierDigestVerifier {
    fn verify_model_frontier(&self, frontier: &ModelResearchFrontier) -> SdkResult<()> {
        verify_model_research_frontier_digest(frontier, &self.expected_digest)
    }
}

pub fn start_model_research_frontier<V: ModelReceiptVerifier + ?Sized>(
    receipt: &ModelResearchExecutionReceipt,
    verifier: &V,
) -> SdkResult<ModelResearchFrontier> {
    validate_receipt(receipt)?;
    verifier.verify_model_receipt(receipt)?;
    let record = last_record(receipt)?;
    if receipt.status == ModelResearchExecutionStatus::Failed {
        return Ok(blocked_frontier(
            receipt,
            record,
            &receipt.plan_digest,
            None,
            1,
        ));
    }
    if !matches!(
        record.action.as_str(),
        "fem_submit" | "workflow_submit_catalog" | "workflow_submit_graph"
    ) {
        return validation_error(
            "initial research receipt must end with a supported job submission",
        );
    }
    let job_id = record
        .output
        .as_ref()
        .and_then(|output| output.pointer("/job/job_id"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| validation("job submission receipt did not contain job.job_id"))?
        .to_string();

    Ok(frontier(
        receipt,
        record,
        &receipt.plan_digest,
        FrontierTransition {
            stage: ModelResearchFrontierStage::WaitingForJob,
            job_id: Some(job_id),
            next_action: Some("job_wait"),
            job_status: None,
            blocking_reason: None,
            transition_count: 1,
        },
    ))
}

pub fn compute_model_research_frontier_digest(
    current: &ModelResearchFrontier,
) -> SdkResult<String> {
    validate_model_research_frontier(current)?;
    Ok(compute_canonical_json_digest(&serde_json::to_value(
        current,
    )?))
}

pub fn verify_model_research_frontier_digest(
    current: &ModelResearchFrontier,
    expected_digest: &str,
) -> SdkResult<()> {
    if !valid_plan_digest(expected_digest) {
        return validation_error("expected research frontier digest is invalid");
    }
    if compute_model_research_frontier_digest(current)? != expected_digest {
        return validation_error("persisted research frontier digest does not match trusted state");
    }
    Ok(())
}

pub fn advance_model_research_frontier<
    R: ModelReceiptVerifier + ?Sized,
    F: ModelFrontierVerifier + ?Sized,
>(
    current: &ModelResearchFrontier,
    receipt: &ModelResearchExecutionReceipt,
    frontier_verifier: &F,
    receipt_verifier: &R,
) -> SdkResult<ModelResearchFrontier> {
    validate_model_research_frontier(current)?;
    frontier_verifier.verify_model_frontier(current)?;
    validate_receipt(receipt)?;
    receipt_verifier.verify_model_receipt(receipt)?;
    if receipt.session_id != current.session_id || receipt.workflow_id != current.workflow_id {
        return validation_error("research receipt does not match frontier session and workflow");
    }
    let expected = current
        .next_action
        .as_deref()
        .ok_or_else(|| validation("research frontier has no executable next action"))?;
    let record = last_record(receipt)?;
    if receipt.status == ModelResearchExecutionStatus::Failed {
        return Ok(blocked_frontier(
            receipt,
            record,
            &current.origin_plan_digest,
            current.job_id.clone(),
            current.transition_count + 1,
        ));
    }
    if record.action != expected {
        return validation_error(format!(
            "research receipt ended with {}; frontier requires {expected}",
            record.action
        ));
    }
    if record.job_id.as_deref() != current.job_id.as_deref() {
        return validation_error("research receipt job_id does not match frontier binding");
    }

    match expected {
        "job_wait" => advance_wait(current, receipt, record),
        "result_fetch" => Ok(frontier(
            receipt,
            record,
            &current.origin_plan_digest,
            FrontierTransition {
                stage: ModelResearchFrontierStage::ReadyToValidate,
                job_id: current.job_id.clone(),
                next_action: None,
                job_status: None,
                blocking_reason: None,
                transition_count: current.transition_count + 1,
            },
        )),
        _ => validation_error(format!("unsupported frontier next action: {expected}")),
    }
}

pub fn build_model_research_frontier_proposal<V: ModelFrontierVerifier + ?Sized>(
    current: &ModelResearchFrontier,
    verifier: &V,
) -> SdkResult<ModelWorkflowProposal> {
    validate_model_research_frontier(current)?;
    verifier.verify_model_frontier(current)?;
    let action = current
        .next_action
        .as_ref()
        .ok_or_else(|| validation("research frontier has no executable next action"))?;
    let job_id = current
        .job_id
        .as_ref()
        .ok_or_else(|| validation("research frontier has no bound job_id"))?;
    Ok(ModelWorkflowProposal {
        schema_version: MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION.to_string(),
        session_id: current.session_id.clone(),
        summary: format!("Advance verified research frontier with {action}"),
        calls: vec![ModelToolCall {
            id: Some(format!(
                "frontier-{}-{action}",
                current.transition_count + 1
            )),
            action: action.clone(),
            payload: json!({ "job_id": job_id }),
            reason: Some("Use the job identifier retained from verified execution evidence".into()),
        }],
    })
}

fn advance_wait(
    current: &ModelResearchFrontier,
    receipt: &ModelResearchExecutionReceipt,
    record: &crate::ModelResearchExecutionRecord,
) -> SdkResult<ModelResearchFrontier> {
    let status = record
        .output
        .as_ref()
        .and_then(|output| output.pointer("/terminal/job/status"))
        .and_then(Value::as_str)
        .ok_or_else(|| validation("job_wait receipt did not contain terminal.job.status"))?;
    match status {
        "completed" => Ok(frontier(
            receipt,
            record,
            &current.origin_plan_digest,
            FrontierTransition {
                stage: ModelResearchFrontierStage::ReadyToFetchResult,
                job_id: current.job_id.clone(),
                next_action: Some("result_fetch"),
                job_status: Some(status),
                blocking_reason: None,
                transition_count: current.transition_count + 1,
            },
        )),
        "failed" | "cancelled" => Ok(frontier(
            receipt,
            record,
            &current.origin_plan_digest,
            FrontierTransition {
                stage: ModelResearchFrontierStage::Blocked,
                job_id: current.job_id.clone(),
                next_action: None,
                job_status: Some(status),
                blocking_reason: Some(format!("job reached terminal status {status}")),
                transition_count: current.transition_count + 1,
            },
        )),
        _ => validation_error(format!("job_wait returned non-terminal status {status}")),
    }
}

struct FrontierTransition<'a> {
    stage: ModelResearchFrontierStage,
    job_id: Option<String>,
    next_action: Option<&'a str>,
    job_status: Option<&'a str>,
    blocking_reason: Option<String>,
    transition_count: usize,
}

fn frontier(
    receipt: &ModelResearchExecutionReceipt,
    record: &crate::ModelResearchExecutionRecord,
    origin_plan_digest: &str,
    transition: FrontierTransition<'_>,
) -> ModelResearchFrontier {
    ModelResearchFrontier {
        schema_version: MODEL_RESEARCH_FRONTIER_SCHEMA_VERSION.to_string(),
        session_id: receipt.session_id.clone(),
        workflow_id: receipt.workflow_id.clone(),
        origin_plan_digest: origin_plan_digest.to_string(),
        stage: transition.stage,
        job_id: transition.job_id,
        next_action: transition.next_action.map(str::to_string),
        transition_count: transition.transition_count,
        evidence: ModelResearchFrontierEvidence {
            approval_id: receipt.approval_id.clone(),
            plan_digest: receipt.plan_digest.clone(),
            action: record.action.clone(),
            record_index: record.index,
            authority: record.authority.clone(),
            job_status: transition.job_status.map(str::to_string),
        },
        blocking_reason: transition.blocking_reason,
    }
}

fn blocked_frontier(
    receipt: &ModelResearchExecutionReceipt,
    record: &crate::ModelResearchExecutionRecord,
    origin_plan_digest: &str,
    job_id: Option<String>,
    transition_count: usize,
) -> ModelResearchFrontier {
    frontier(
        receipt,
        record,
        origin_plan_digest,
        FrontierTransition {
            stage: ModelResearchFrontierStage::Blocked,
            job_id,
            next_action: None,
            job_status: None,
            blocking_reason: Some(
                record
                    .error
                    .clone()
                    .unwrap_or_else(|| "research execution failed".to_string()),
            ),
            transition_count,
        },
    )
}

fn validate_receipt(receipt: &ModelResearchExecutionReceipt) -> SdkResult<()> {
    if receipt.schema_version != MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION
        || receipt.execution_authority != "kyuubiki-headless-sdk"
    {
        return validation_error("unsupported or untrusted research execution receipt");
    }
    if receipt.session_id.trim().is_empty()
        || receipt.workflow_id.trim().is_empty()
        || !valid_plan_digest(&receipt.plan_digest)
        || receipt.records.is_empty()
    {
        return validation_error("research execution receipt is incomplete");
    }
    let final_record = receipt.records.last().expect("records checked above");
    match receipt.status {
        ModelResearchExecutionStatus::Completed
            if final_record.error.is_some()
                || final_record.output.is_none()
                || final_record.authority.is_none() =>
        {
            return validation_error("completed research receipt has an invalid final record");
        }
        ModelResearchExecutionStatus::Failed if final_record.error.is_none() => {
            return validation_error("failed research receipt has no final error");
        }
        _ => {}
    }
    Ok(())
}

pub fn validate_model_research_frontier(current: &ModelResearchFrontier) -> SdkResult<()> {
    if current.schema_version != MODEL_RESEARCH_FRONTIER_SCHEMA_VERSION
        || current.session_id.trim().is_empty()
        || current.workflow_id.trim().is_empty()
        || !valid_plan_digest(&current.origin_plan_digest)
        || !valid_plan_digest(&current.evidence.plan_digest)
        || !valid_evidence(&current.evidence)
        || current.transition_count == 0
        || current.transition_count == usize::MAX
    {
        return validation_error("research frontier is incomplete or uses an unsupported schema");
    }
    let valid_state = match current.stage {
        ModelResearchFrontierStage::WaitingForJob => {
            has_job_id(current)
                && current.next_action.as_deref() == Some("job_wait")
                && current.blocking_reason.is_none()
        }
        ModelResearchFrontierStage::ReadyToFetchResult => {
            has_job_id(current)
                && current.next_action.as_deref() == Some("result_fetch")
                && current.blocking_reason.is_none()
        }
        ModelResearchFrontierStage::ReadyToValidate => {
            has_job_id(current)
                && current.next_action.is_none()
                && current.blocking_reason.is_none()
        }
        ModelResearchFrontierStage::Blocked => {
            current.next_action.is_none()
                && current
                    .blocking_reason
                    .as_deref()
                    .is_some_and(|reason| !reason.trim().is_empty())
        }
    };
    if !valid_state {
        return validation_error("research frontier stage and next action are inconsistent");
    }
    Ok(())
}

fn valid_plan_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_evidence(evidence: &ModelResearchFrontierEvidence) -> bool {
    let valid_action = evidence
        .action
        .bytes()
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && evidence
            .action
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    valid_action
        && evidence.record_index > 0
        && evidence
            .job_status
            .as_deref()
            .is_none_or(|status| matches!(status, "completed" | "failed" | "cancelled"))
}

fn has_job_id(current: &ModelResearchFrontier) -> bool {
    current
        .job_id
        .as_deref()
        .is_some_and(|job_id| !job_id.trim().is_empty())
}

fn last_record(
    receipt: &ModelResearchExecutionReceipt,
) -> SdkResult<&crate::ModelResearchExecutionRecord> {
    receipt
        .records
        .last()
        .ok_or_else(|| validation("research execution receipt has no records"))
}

fn validation(message: impl Into<String>) -> SdkError {
    SdkError::Validation {
        errors: vec![message.into()],
    }
}

fn validation_error<T>(message: impl Into<String>) -> SdkResult<T> {
    Err(validation(message))
}
