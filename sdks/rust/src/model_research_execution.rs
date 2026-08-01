use crate::{
    ControlPlaneClient, KyuubikiSession, MODEL_HEADLESS_PLAN_SCHEMA_VERSION, ModelHeadlessPlan,
    SdkError, SdkResult,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::time::Duration;

pub const MODEL_PLAN_APPROVAL_SCHEMA_VERSION: &str = "kyuubiki.model-plan-approval/v1";
pub const MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION: &str =
    "kyuubiki.model-research-execution-receipt/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedModelPlanStep {
    pub index: usize,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelPlanApproval {
    pub schema_version: String,
    pub approval_id: String,
    pub session_id: String,
    pub workflow_id: String,
    pub authority: String,
    pub issued_at: String,
    pub approved_steps: Vec<ApprovedModelPlanStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelActionDispatch {
    pub authority: String,
    pub output: Value,
}

pub trait ModelActionDispatcher {
    fn dispatch_model_action(
        &self,
        action: &str,
        payload: &Value,
    ) -> SdkResult<ModelActionDispatch>;
}

pub trait ModelApprovalVerifier {
    fn verify_model_approval(
        &self,
        plan: &ModelHeadlessPlan,
        approval: &ModelPlanApproval,
    ) -> SdkResult<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelResearchExecutionStatus {
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelResearchExecutionRecord {
    pub index: usize,
    pub action: String,
    #[serde(default)]
    pub job_id: Option<String>,
    pub authority: Option<String>,
    pub output: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelResearchExecutionReceipt {
    pub schema_version: String,
    pub plan_schema_version: String,
    pub session_id: String,
    pub workflow_id: String,
    pub status: ModelResearchExecutionStatus,
    pub execution_authority: String,
    pub approval_id: Option<String>,
    pub completed_steps: usize,
    pub failed_step: Option<usize>,
    pub records: Vec<ModelResearchExecutionRecord>,
}

pub fn execute_model_headless_plan<D: ModelActionDispatcher, V: ModelApprovalVerifier + ?Sized>(
    dispatcher: &D,
    plan: &ModelHeadlessPlan,
    approval: Option<&ModelPlanApproval>,
    approval_verifier: &V,
) -> SdkResult<ModelResearchExecutionReceipt> {
    validate_execution_request(plan, approval)?;
    if let Some(approval) = approval {
        approval_verifier.verify_model_approval(plan, approval)?;
    }
    let mut records = Vec::with_capacity(plan.steps.len());

    for step in &plan.steps {
        let job_id = action_job_id(&step.action, &step.payload);
        match dispatcher.dispatch_model_action(&step.action, &step.payload) {
            Ok(dispatched) => records.push(ModelResearchExecutionRecord {
                index: step.index,
                action: step.action.clone(),
                job_id,
                authority: Some(dispatched.authority),
                output: Some(dispatched.output),
                error: None,
            }),
            Err(error) => {
                records.push(ModelResearchExecutionRecord {
                    index: step.index,
                    action: step.action.clone(),
                    job_id,
                    authority: None,
                    output: None,
                    error: Some(bounded_error(&error)),
                });
                return Ok(build_receipt(
                    plan,
                    approval,
                    ModelResearchExecutionStatus::Failed,
                    Some(step.index),
                    records,
                ));
            }
        }
    }

    Ok(build_receipt(
        plan,
        approval,
        ModelResearchExecutionStatus::Completed,
        None,
        records,
    ))
}

fn action_job_id(action: &str, payload: &Value) -> Option<String> {
    if matches!(
        action,
        "job_wait" | "result_fetch" | "result_chunk_fetch" | "job_cancel"
    ) {
        payload.get("job_id")?.as_str().map(str::to_string)
    } else {
        None
    }
}

pub struct SessionModelActionDispatcher<'a> {
    session: &'a KyuubikiSession,
    poll_interval: Duration,
    timeout: Duration,
}

impl<'a> SessionModelActionDispatcher<'a> {
    pub fn new(session: &'a KyuubikiSession) -> Self {
        Self {
            session,
            poll_interval: Duration::from_millis(500),
            timeout: Duration::from_secs(300),
        }
    }

    pub fn with_wait_bounds(
        mut self,
        poll_interval: Duration,
        timeout: Duration,
    ) -> SdkResult<Self> {
        if poll_interval.is_zero() || timeout.is_zero() || poll_interval > timeout {
            return validation_error(
                "model dispatcher wait bounds require 0 < poll_interval <= timeout",
            );
        }
        if timeout > Duration::from_secs(86_400) {
            return validation_error("model dispatcher timeout cannot exceed 24 hours");
        }
        self.poll_interval = poll_interval;
        self.timeout = timeout;
        Ok(self)
    }

    fn control_plane(&self) -> SdkResult<&ControlPlaneClient> {
        self.session.control_plane.as_ref().ok_or_else(|| {
            SdkError::Transport("control plane client is not configured".to_string())
        })
    }
}

impl ModelActionDispatcher for SessionModelActionDispatcher<'_> {
    fn dispatch_model_action(
        &self,
        action: &str,
        payload: &Value,
    ) -> SdkResult<ModelActionDispatch> {
        let (authority, output) = match action {
            "service_health" => ("control_plane", self.control_plane()?.health()?),
            "protocol_describe" => ("control_plane", self.control_plane()?.protocol()?),
            "agents_describe" => ("control_plane", self.control_plane()?.agents()?),
            "workflow_catalog_list" => (
                "control_plane",
                self.control_plane()?.list_workflow_catalog()?,
            ),
            "operator_catalog_list" => (
                "control_plane",
                self.control_plane()?.list_workflow_operators()?,
            ),
            "fem_submit" => (
                "control_plane",
                self.control_plane()?.submit_fem_job(
                    required_string(payload, "solve_kind")?,
                    required_value(payload, "payload")?,
                )?,
            ),
            "direct_solver_rpc" => (
                "solver_rpc",
                self.session.solve_direct(
                    required_string(payload, "solve_kind")?,
                    required_value(payload, "payload")?.clone(),
                )?,
            ),
            "workflow_submit_catalog" => (
                "control_plane",
                self.control_plane()?.submit_workflow_catalog_job(
                    required_string(payload, "workflow_id")?,
                    required_value(payload, "input_artifacts")?,
                )?,
            ),
            "workflow_submit_graph" => (
                "control_plane",
                self.control_plane()?.submit_workflow_graph_job(
                    required_value(payload, "graph")?,
                    required_value(payload, "input_artifacts")?,
                )?,
            ),
            "operator_task_prepare" => (
                "control_plane",
                self.control_plane()?
                    .prepare_operator_task(required_value(payload, "task")?)?,
            ),
            "operator_task_execute" => (
                "control_plane",
                self.control_plane()?
                    .execute_operator_task(required_value(payload, "task")?)?,
            ),
            "operator_task_batch_prepare" => (
                "control_plane",
                self.control_plane()?
                    .prepare_operator_task_batch(required_value(payload, "batch")?)?,
            ),
            "operator_task_batch_execute" => (
                "control_plane",
                self.control_plane()?
                    .execute_operator_task_batch(required_value(payload, "batch")?)?,
            ),
            "job_wait" => {
                let outcome = self.session.wait_for_job(
                    required_string(payload, "job_id")?,
                    self.poll_interval,
                    self.timeout,
                )?;
                (
                    "control_plane",
                    json!({ "terminal": outcome.terminal, "history": outcome.history }),
                )
            }
            "result_fetch" => (
                "control_plane",
                self.control_plane()?
                    .fetch_result(required_string(payload, "job_id")?)?,
            ),
            "result_chunk_fetch" => (
                "control_plane",
                self.control_plane()?.fetch_result_chunk(
                    required_string(payload, "job_id")?,
                    required_string(payload, "kind")?,
                    optional_usize(payload, "offset")?,
                    optional_usize(payload, "limit")?,
                )?,
            ),
            "job_cancel" => (
                "control_plane",
                self.control_plane()?
                    .cancel_job(required_string(payload, "job_id")?)?,
            ),
            _ => return validation_error(format!("unsupported model action: {action}")),
        };
        Ok(ModelActionDispatch {
            authority: authority.to_string(),
            output,
        })
    }
}

fn validate_execution_request(
    plan: &ModelHeadlessPlan,
    approval: Option<&ModelPlanApproval>,
) -> SdkResult<()> {
    let mut errors = Vec::new();
    if plan.schema_version != MODEL_HEADLESS_PLAN_SCHEMA_VERSION {
        errors.push(format!(
            "unsupported model plan schema_version: {}",
            plan.schema_version
        ));
    }
    if !plan.ok || !plan.issues.is_empty() {
        errors.push("model plan must be valid and issue-free before dispatch".to_string());
    }
    if plan.steps.is_empty() {
        errors.push("model plan contains no steps".to_string());
    }
    for (offset, step) in plan.steps.iter().enumerate() {
        if step.index != offset + 1 {
            errors.push("model plan step indexes must be contiguous and one-based".to_string());
            break;
        }
    }

    let gated = plan
        .steps
        .iter()
        .filter(|step| step.requires_confirmation)
        .map(|step| (step.index, step.action.clone()))
        .collect::<HashSet<_>>();
    let approved = match approval {
        Some(approval) => validate_approval(plan, approval, &gated, &mut errors),
        None => HashSet::new(),
    };
    for (index, action) in &gated {
        if !approved.contains(&(*index, action.clone())) {
            errors.push(format!(
                "step {index} ({action}) requires an exact caller-issued approval"
            ));
        }
    }

    errors.sort();
    errors.dedup();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(SdkError::Validation { errors })
    }
}

fn validate_approval(
    plan: &ModelHeadlessPlan,
    approval: &ModelPlanApproval,
    gated: &HashSet<(usize, String)>,
    errors: &mut Vec<String>,
) -> HashSet<(usize, String)> {
    if approval.schema_version != MODEL_PLAN_APPROVAL_SCHEMA_VERSION {
        errors.push(format!(
            "unsupported model approval schema_version: {}",
            approval.schema_version
        ));
    }
    if approval.session_id != plan.session_id || approval.workflow_id != plan.workflow_id {
        errors.push("model approval does not match plan session and workflow".to_string());
    }
    for (name, value) in [
        ("approval_id", approval.approval_id.as_str()),
        ("authority", approval.authority.as_str()),
        ("issued_at", approval.issued_at.as_str()),
    ] {
        if value.trim().is_empty() {
            errors.push(format!("model approval {name} is required"));
        }
    }

    let mut approved = HashSet::new();
    for step in &approval.approved_steps {
        let key = (step.index, step.action.clone());
        if !approved.insert(key.clone()) {
            errors.push(format!(
                "model approval repeats step {} ({})",
                step.index, step.action
            ));
        }
        if !gated.contains(&key) {
            errors.push(format!(
                "model approval references a non-gated or mismatched step {} ({})",
                step.index, step.action
            ));
        }
    }
    approved
}

fn build_receipt(
    plan: &ModelHeadlessPlan,
    approval: Option<&ModelPlanApproval>,
    status: ModelResearchExecutionStatus,
    failed_step: Option<usize>,
    records: Vec<ModelResearchExecutionRecord>,
) -> ModelResearchExecutionReceipt {
    let completed_steps = records
        .iter()
        .filter(|record| record.error.is_none())
        .count();
    ModelResearchExecutionReceipt {
        schema_version: MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION.to_string(),
        plan_schema_version: plan.schema_version.clone(),
        session_id: plan.session_id.clone(),
        workflow_id: plan.workflow_id.clone(),
        status,
        execution_authority: "kyuubiki-headless-sdk".to_string(),
        approval_id: approval.map(|approval| approval.approval_id.clone()),
        completed_steps,
        failed_step,
        records,
    }
}

fn required_string<'a>(payload: &'a Value, key: &str) -> SdkResult<&'a str> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| SdkError::Validation {
            errors: vec![format!(
                "model action payload requires non-empty string {key}"
            )],
        })
}

fn required_value<'a>(payload: &'a Value, key: &str) -> SdkResult<&'a Value> {
    payload
        .get(key)
        .filter(|value| !value.is_null())
        .ok_or_else(|| SdkError::Validation {
            errors: vec![format!("model action payload requires {key}")],
        })
}

fn optional_usize(payload: &Value, key: &str) -> SdkResult<Option<usize>> {
    payload
        .get(key)
        .map(|value| {
            value
                .as_u64()
                .and_then(|number| usize::try_from(number).ok())
                .ok_or_else(|| SdkError::Validation {
                    errors: vec![format!(
                        "model action payload {key} must be an unsigned integer"
                    )],
                })
        })
        .transpose()
}

fn bounded_error(error: &SdkError) -> String {
    let message = error.to_string();
    let boundary = message
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= 2_048)
        .last()
        .unwrap_or(0);
    if message.len() <= 2_048 {
        message
    } else {
        format!("{}...", &message[..boundary])
    }
}

fn validation_error<T>(message: impl Into<String>) -> SdkResult<T> {
    Err(SdkError::Validation {
        errors: vec![message.into()],
    })
}
