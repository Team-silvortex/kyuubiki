use crate::{SdkError, SdkResult, project_model_tools, sanitize_model_context};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MODEL_COLLABORATION_SCHEMA_VERSION: &str = "kyuubiki.model-collaboration/v1";
pub const MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION: &str = "kyuubiki.model-workflow-proposal/v1";
pub const MODEL_HEADLESS_PLAN_SCHEMA_VERSION: &str = "kyuubiki.model-headless-plan/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelProvider {
    #[serde(rename = "openai")]
    OpenAi,
    #[serde(rename = "openai_chat")]
    OpenAiChat,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "gemini")]
    Gemini,
    #[serde(rename = "canonical")]
    Canonical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadlessModelRisk {
    Normal,
    Sensitive,
    Destructive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadlessModelRuntime {
    Service,
    Direct,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCollaborationPolicy {
    #[serde(default)]
    pub allowed_actions: Vec<String>,
    #[serde(default)]
    pub allowed_categories: Vec<String>,
    #[serde(default = "default_max_steps")]
    pub max_steps: usize,
    #[serde(default = "default_max_context_bytes")]
    pub max_context_bytes: usize,
    #[serde(default = "default_true")]
    pub service_only: bool,
    #[serde(default)]
    pub allow_sensitive: bool,
    #[serde(default)]
    pub allow_destructive: bool,
}

impl Default for ModelCollaborationPolicy {
    fn default() -> Self {
        Self {
            allowed_actions: Vec::new(),
            allowed_categories: Vec::new(),
            max_steps: default_max_steps(),
            max_context_bytes: default_max_context_bytes(),
            service_only: true,
            allow_sensitive: false,
            allow_destructive: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCollaborationSession {
    pub schema_version: String,
    pub session_id: String,
    pub workflow_id: String,
    pub objective: String,
    #[serde(default = "default_language")]
    pub language: String,
    pub created_at: String,
    #[serde(default)]
    pub policy: ModelCollaborationPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessModelTool {
    pub action: String,
    pub category: String,
    pub description: String,
    pub risk: HeadlessModelRisk,
    pub runtime: HeadlessModelRuntime,
    pub required_payload_keys: Vec<String>,
    pub output_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCollaborationRequest {
    pub schema_version: String,
    pub provider: ModelProvider,
    pub session: ModelCollaborationSession,
    pub instructions: Vec<String>,
    pub context: Value,
    pub redacted_paths: Vec<String>,
    pub tools: Value,
    pub output_contract: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelToolCall {
    #[serde(default)]
    pub id: Option<String>,
    pub action: String,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelWorkflowProposal {
    pub schema_version: String,
    pub session_id: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub calls: Vec<ModelToolCall>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelHeadlessPlanStep {
    pub index: usize,
    pub action: String,
    pub category: Option<String>,
    pub risk: HeadlessModelRisk,
    pub payload: Value,
    pub requires_confirmation: bool,
    pub confirmation_reason: Option<String>,
    pub output_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelHeadlessPlan {
    pub schema_version: String,
    pub session_id: String,
    pub workflow_id: String,
    pub ok: bool,
    pub ready_without_confirmation: bool,
    pub issues: Vec<String>,
    pub steps: Vec<ModelHeadlessPlanStep>,
}

pub fn rust_headless_model_tools(policy: &ModelCollaborationPolicy) -> Vec<HeadlessModelTool> {
    base_model_tools()
        .into_iter()
        .filter(|tool| policy_allows_tool(policy, tool))
        .collect()
}

pub fn build_model_collaboration_request(
    provider: ModelProvider,
    session: ModelCollaborationSession,
    context: Value,
) -> SdkResult<ModelCollaborationRequest> {
    validate_session(&session)?;
    let (context, redacted_paths) = sanitize_model_context(&context);
    let context_bytes = serde_json::to_vec(&context)?.len();
    if context_bytes > session.policy.max_context_bytes {
        return validation_error(format!(
            "sanitized model context uses {context_bytes} bytes; policy allows {}",
            session.policy.max_context_bytes
        ));
    }
    let tools = rust_headless_model_tools(&session.policy);
    if tools.is_empty() {
        return validation_error("model collaboration policy exposes no Headless tools");
    }
    Ok(ModelCollaborationRequest {
        schema_version: MODEL_COLLABORATION_SCHEMA_VERSION.to_string(),
        provider,
        instructions: vec![
            format!("Plan only for this objective: {}", session.objective),
            "Use only supplied Headless tools and never invent action names.".to_string(),
            format!(
                "Return no more than {} tool calls.",
                session.policy.max_steps
            ),
            "Return tool calls as an untrusted proposal; never claim that execution occurred."
                .to_string(),
        ],
        context,
        redacted_paths,
        tools: project_model_tools(provider, &tools),
        output_contract: MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION.to_string(),
        session,
    })
}

pub fn build_model_headless_plan(
    session: &ModelCollaborationSession,
    proposal: &ModelWorkflowProposal,
) -> SdkResult<ModelHeadlessPlan> {
    validate_session(session)?;
    let available = rust_headless_model_tools(&session.policy);
    let mut issues = Vec::new();
    if proposal.schema_version != MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION {
        issues.push(format!(
            "unsupported proposal schema_version: {}",
            proposal.schema_version
        ));
    }
    if proposal.session_id != session.session_id {
        issues.push("proposal session_id does not match collaboration session".to_string());
    }
    if proposal.calls.is_empty() {
        issues.push("model proposal contains no tool calls".to_string());
    }
    if proposal.calls.len() > session.policy.max_steps {
        issues.push(format!(
            "model proposal contains {} calls; policy allows {}",
            proposal.calls.len(),
            session.policy.max_steps
        ));
    }
    let steps = proposal
        .calls
        .iter()
        .enumerate()
        .map(|(offset, call)| {
            let tool = available.iter().find(|tool| tool.action == call.action);
            if tool.is_none() {
                issues.push(format!(
                    "step {} action {} is unknown or blocked by policy",
                    offset + 1,
                    call.action
                ));
            }
            if !call.payload.is_object() {
                issues.push(format!(
                    "step {} ({}) payload must be a JSON object",
                    offset + 1,
                    call.action
                ));
            }
            if let Some(tool) = tool {
                for key in &tool.required_payload_keys {
                    if !has_present_value(&call.payload, key) {
                        issues.push(format!(
                            "step {} ({}) is missing required payload key {}",
                            offset + 1,
                            call.action,
                            key
                        ));
                    }
                }
                validate_known_payload(offset + 1, &call.action, &call.payload, &mut issues);
            }
            let risk = tool.map_or(HeadlessModelRisk::Normal, |tool| tool.risk);
            ModelHeadlessPlanStep {
                index: offset + 1,
                action: call.action.clone(),
                category: tool.map(|tool| tool.category.clone()),
                risk,
                payload: call.payload.clone(),
                requires_confirmation: risk != HeadlessModelRisk::Normal,
                confirmation_reason: confirmation_reason(risk).map(str::to_string),
                output_keys: tool
                    .map(|tool| tool.output_keys.clone())
                    .unwrap_or_default(),
            }
        })
        .collect::<Vec<_>>();
    issues.sort();
    issues.dedup();
    Ok(ModelHeadlessPlan {
        schema_version: MODEL_HEADLESS_PLAN_SCHEMA_VERSION.to_string(),
        session_id: session.session_id.clone(),
        workflow_id: session.workflow_id.clone(),
        ok: issues.is_empty(),
        ready_without_confirmation: issues.is_empty()
            && steps.iter().all(|step| !step.requires_confirmation),
        issues,
        steps,
    })
}

fn base_model_tools() -> Vec<HeadlessModelTool> {
    vec![
        tool(
            "service_health",
            "discovery",
            "Check control-plane health.",
            HeadlessModelRisk::Normal,
            HeadlessModelRuntime::Service,
            &[],
            &["health"],
        ),
        tool(
            "protocol_describe",
            "discovery",
            "Read protocol compatibility and service endpoints.",
            HeadlessModelRisk::Normal,
            HeadlessModelRuntime::Service,
            &[],
            &["protocol"],
        ),
        tool(
            "agents_describe",
            "discovery",
            "List reachable agents and capabilities.",
            HeadlessModelRisk::Normal,
            HeadlessModelRuntime::Service,
            &[],
            &["agents"],
        ),
        tool(
            "workflow_catalog_list",
            "discovery",
            "List centrally owned workflow templates.",
            HeadlessModelRisk::Normal,
            HeadlessModelRuntime::Service,
            &[],
            &["workflows"],
        ),
        tool(
            "operator_catalog_list",
            "discovery",
            "List workflow operator descriptors.",
            HeadlessModelRisk::Normal,
            HeadlessModelRuntime::Service,
            &[],
            &["operators"],
        ),
        tool(
            "fem_submit",
            "solve",
            "Submit a FEM solve kind and model payload.",
            HeadlessModelRisk::Sensitive,
            HeadlessModelRuntime::Service,
            &["solve_kind", "payload"],
            &["job"],
        ),
        tool(
            "direct_solver_rpc",
            "solve",
            "Call a configured solver agent without Orchestra.",
            HeadlessModelRisk::Sensitive,
            HeadlessModelRuntime::Direct,
            &["solve_kind", "payload"],
            &["result"],
        ),
        tool(
            "workflow_submit_catalog",
            "workflow",
            "Submit a catalog workflow job.",
            HeadlessModelRisk::Sensitive,
            HeadlessModelRuntime::Service,
            &["workflow_id", "input_artifacts"],
            &["job"],
        ),
        tool(
            "workflow_submit_graph",
            "workflow",
            "Submit a validated inline workflow graph.",
            HeadlessModelRisk::Sensitive,
            HeadlessModelRuntime::Service,
            &["graph", "input_artifacts"],
            &["job"],
        ),
        tool(
            "operator_task_prepare",
            "task_ir",
            "Preflight one language-neutral Operator TaskIR envelope.",
            HeadlessModelRisk::Normal,
            HeadlessModelRuntime::Service,
            &["task"],
            &["preparation"],
        ),
        tool(
            "operator_task_execute",
            "task_ir",
            "Execute one prepared Operator TaskIR envelope.",
            HeadlessModelRisk::Sensitive,
            HeadlessModelRuntime::Service,
            &["task"],
            &["execution"],
        ),
        tool(
            "operator_task_batch_prepare",
            "task_ir",
            "Preflight an Operator TaskIR batch.",
            HeadlessModelRisk::Normal,
            HeadlessModelRuntime::Service,
            &["batch"],
            &["preparation"],
        ),
        tool(
            "operator_task_batch_execute",
            "task_ir",
            "Execute an Operator TaskIR batch.",
            HeadlessModelRisk::Sensitive,
            HeadlessModelRuntime::Service,
            &["batch"],
            &["execution"],
        ),
        tool(
            "job_wait",
            "observation",
            "Poll a job until it reaches a terminal state.",
            HeadlessModelRisk::Normal,
            HeadlessModelRuntime::Service,
            &["job_id"],
            &["job"],
        ),
        tool(
            "result_fetch",
            "observation",
            "Fetch the retained result bundle for a job.",
            HeadlessModelRisk::Normal,
            HeadlessModelRuntime::Service,
            &["job_id"],
            &["result"],
        ),
        tool(
            "result_chunk_fetch",
            "observation",
            "Fetch one bounded result chunk.",
            HeadlessModelRisk::Normal,
            HeadlessModelRuntime::Service,
            &["job_id", "kind"],
            &["chunk"],
        ),
        tool(
            "job_cancel",
            "lifecycle",
            "Cancel a running job after explicit approval.",
            HeadlessModelRisk::Destructive,
            HeadlessModelRuntime::Service,
            &["job_id"],
            &["job"],
        ),
    ]
}

fn tool(
    action: &str,
    category: &str,
    description: &str,
    risk: HeadlessModelRisk,
    runtime: HeadlessModelRuntime,
    required_payload_keys: &[&str],
    output_keys: &[&str],
) -> HeadlessModelTool {
    HeadlessModelTool {
        action: action.to_string(),
        category: category.to_string(),
        description: description.to_string(),
        risk,
        runtime,
        required_payload_keys: required_payload_keys
            .iter()
            .map(|key| (*key).to_string())
            .collect(),
        output_keys: output_keys.iter().map(|key| (*key).to_string()).collect(),
    }
}

fn validate_session(session: &ModelCollaborationSession) -> SdkResult<()> {
    let mut errors = Vec::new();
    if session.schema_version != MODEL_COLLABORATION_SCHEMA_VERSION {
        errors.push(format!(
            "unsupported session schema_version: {}",
            session.schema_version
        ));
    }
    for (name, value) in [
        ("session_id", session.session_id.as_str()),
        ("workflow_id", session.workflow_id.as_str()),
        ("objective", session.objective.as_str()),
        ("created_at", session.created_at.as_str()),
    ] {
        if value.trim().is_empty() {
            errors.push(format!("{name} is required"));
        }
    }
    if session.policy.max_steps == 0 || session.policy.max_context_bytes == 0 {
        errors.push("max_steps and max_context_bytes must be greater than zero".to_string());
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(SdkError::Validation { errors })
    }
}

fn policy_allows_tool(policy: &ModelCollaborationPolicy, tool: &HeadlessModelTool) -> bool {
    (policy.allowed_actions.is_empty()
        || policy
            .allowed_actions
            .iter()
            .any(|action| action == &tool.action))
        && (policy.allowed_categories.is_empty()
            || policy
                .allowed_categories
                .iter()
                .any(|category| category == &tool.category))
        && (policy.allow_sensitive || tool.risk != HeadlessModelRisk::Sensitive)
        && (policy.allow_destructive || tool.risk != HeadlessModelRisk::Destructive)
        && (!policy.service_only || tool.runtime == HeadlessModelRuntime::Service)
}

fn has_present_value(payload: &Value, key: &str) -> bool {
    payload.get(key).is_some_and(|value| match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        _ => true,
    })
}

fn validate_known_payload(index: usize, action: &str, payload: &Value, errors: &mut Vec<String>) {
    if !payload.is_object() {
        return;
    }
    let (string_keys, object_keys): (&[&str], &[&str]) = match action {
        "fem_submit" | "direct_solver_rpc" => (&["solve_kind"], &["payload"]),
        "workflow_submit_catalog" => (&["workflow_id"], &["input_artifacts"]),
        "workflow_submit_graph" => (&[], &["graph", "input_artifacts"]),
        "operator_task_prepare" | "operator_task_execute" => (&[], &["task"]),
        "operator_task_batch_prepare" | "operator_task_batch_execute" => (&[], &["batch"]),
        "job_wait" | "result_fetch" | "job_cancel" => (&["job_id"], &[]),
        "result_chunk_fetch" => (&["job_id", "kind"], &[]),
        _ => (&[], &[]),
    };
    for key in string_keys {
        if payload
            .get(*key)
            .is_some_and(|value| value.as_str().is_none_or(str::is_empty))
        {
            errors.push(format!(
                "step {index} ({action}) payload key {key} must be a non-empty string"
            ));
        }
    }
    for key in object_keys {
        if payload.get(*key).is_some_and(|value| !value.is_object()) {
            errors.push(format!(
                "step {index} ({action}) payload key {key} must be a JSON object"
            ));
        }
    }
    if action == "result_chunk_fetch" {
        for key in ["offset", "limit"] {
            if payload
                .get(key)
                .is_some_and(|value| value.as_u64().is_none())
            {
                errors.push(format!(
                    "step {index} ({action}) payload key {key} must be an unsigned integer"
                ));
            }
        }
    }
}

fn confirmation_reason(risk: HeadlessModelRisk) -> Option<&'static str> {
    match risk {
        HeadlessModelRisk::Normal => None,
        HeadlessModelRisk::Sensitive => {
            Some("sensitive Headless action requires explicit approval before dispatch")
        }
        HeadlessModelRisk::Destructive => {
            Some("destructive Headless action requires explicit approval before dispatch")
        }
    }
}

fn validation_error<T>(message: impl Into<String>) -> SdkResult<T> {
    Err(SdkError::Validation {
        errors: vec![message.into()],
    })
}

fn default_max_steps() -> usize {
    12
}
fn default_max_context_bytes() -> usize {
    64 * 1024
}
fn default_true() -> bool {
    true
}
fn default_language() -> String {
    "en".to_string()
}
