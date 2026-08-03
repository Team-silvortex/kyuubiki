use crate::{
    HeadlessEngine, HeadlessExecutionBatch, HeadlessExecutionBatchStep, HeadlessExecutionPlan,
    HeadlessRisk, HeadlessRuntimeStyle, action_capability_manifest, build_execution_plan,
    find_action_contract,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::fmt;

pub const MODEL_COLLABORATION_SCHEMA_VERSION: &str = "kyuubiki.model-collaboration/v1";
pub const MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION: &str = "kyuubiki.model-workflow-proposal/v1";
pub const MODEL_PROPOSAL_COMPILATION_SCHEMA_VERSION: &str =
    "kyuubiki.model-proposal-compilation/v1";

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
pub struct ModelCollaborationTool {
    pub action: String,
    pub description: String,
    pub category: String,
    pub risk: HeadlessRisk,
    pub runtime_style: HeadlessRuntimeStyle,
    pub input_schema: Value,
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
pub struct ModelProposalCompilation {
    pub schema_version: String,
    pub session_id: String,
    pub ok: bool,
    pub issue_count: usize,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
    pub batch: HeadlessExecutionBatch,
    pub plan: HeadlessExecutionPlan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCollaborationError {
    pub code: String,
    pub message: String,
}

impl ModelCollaborationError {
    pub(crate) fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ModelCollaborationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ModelCollaborationError {}

pub fn model_collaboration_tools(policy: &ModelCollaborationPolicy) -> Vec<ModelCollaborationTool> {
    action_capability_manifest()
        .into_iter()
        .filter(|capability| policy_allows_action(policy, &capability.action))
        .map(|capability| ModelCollaborationTool {
            description: tool_description(
                &capability.action,
                &capability.category,
                &capability.required_payload_keys,
                &capability.output_keys,
            ),
            input_schema: action_input_schema(&capability.required_payload_keys),
            action: capability.action,
            category: capability.category,
            risk: capability.risk,
            runtime_style: capability.runtime_style,
            output_keys: capability.output_keys,
        })
        .collect()
}

pub fn build_model_collaboration_request(
    provider: ModelProvider,
    session: ModelCollaborationSession,
    context: Value,
) -> Result<ModelCollaborationRequest, ModelCollaborationError> {
    validate_session(&session)?;
    let (context, redacted_paths) = sanitize_model_context(&context);
    let context_bytes = serde_json::to_vec(&context)
        .map_err(|error| ModelCollaborationError::new("invalid_context", error.to_string()))?
        .len();
    if context_bytes > session.policy.max_context_bytes {
        return Err(ModelCollaborationError::new(
            "context_too_large",
            format!(
                "sanitized context uses {context_bytes} bytes; policy allows {}",
                session.policy.max_context_bytes
            ),
        ));
    }
    let tools = model_collaboration_tools(&session.policy);
    if tools.is_empty() {
        return Err(ModelCollaborationError::new(
            "empty_tool_catalog",
            "session policy does not expose any Kyuubiki actions",
        ));
    }
    Ok(ModelCollaborationRequest {
        schema_version: MODEL_COLLABORATION_SCHEMA_VERSION.to_string(),
        provider,
        instructions: collaboration_instructions(&session),
        tools: crate::project_model_tools(provider, &tools),
        context,
        redacted_paths,
        output_contract: MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION.to_string(),
        session,
    })
}

pub fn compile_model_proposal(
    session: &ModelCollaborationSession,
    proposal: &ModelWorkflowProposal,
) -> Result<ModelProposalCompilation, ModelCollaborationError> {
    validate_session(session)?;
    let mut issues = Vec::new();
    if proposal.schema_version != MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION {
        issues.push(format!(
            "unsupported proposal schema_version: {}",
            proposal.schema_version
        ));
    }
    if proposal.session_id != session.session_id {
        issues.push("proposal session_id does not match the collaboration session".to_string());
    }
    if proposal.calls.is_empty() {
        issues.push("model proposal contains no tool calls".to_string());
    }
    if proposal.calls.len() > session.policy.max_steps {
        issues.push(format!(
            "model proposal contains {} steps; policy allows {}",
            proposal.calls.len(),
            session.policy.max_steps
        ));
    }

    let steps = proposal
        .calls
        .iter()
        .enumerate()
        .map(|(offset, call)| {
            let risk = match find_action_contract(&call.action) {
                Some(contract) => {
                    if !policy_allows_action(&session.policy, &call.action) {
                        issues.push(format!(
                            "step {} action {} is blocked by the collaboration policy",
                            offset + 1,
                            call.action
                        ));
                    }
                    contract.risk
                }
                None => {
                    issues.push(format!(
                        "step {} references unsupported action {}",
                        offset + 1,
                        call.action
                    ));
                    HeadlessRisk::Normal
                }
            };
            if !call.payload.is_object() {
                issues.push(format!(
                    "step {} ({}) payload must be a JSON object",
                    offset + 1,
                    call.action
                ));
            }
            HeadlessExecutionBatchStep {
                index: offset + 1,
                action: call.action.clone(),
                risk,
                payload: call.payload.clone(),
            }
        })
        .collect::<Vec<_>>();
    let batch = HeadlessExecutionBatch {
        schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
        exported_at: session.created_at.clone(),
        language: session.language.clone(),
        workflow_id: session.workflow_id.clone(),
        template_id: None,
        steps,
        warnings: Vec::new(),
    };
    let plan = build_execution_plan(&batch);
    issues.extend(plan.validation.issues.iter().cloned());
    issues.sort();
    issues.dedup();
    let warnings = plan.validation.warnings.clone();
    Ok(ModelProposalCompilation {
        schema_version: MODEL_PROPOSAL_COMPILATION_SCHEMA_VERSION.to_string(),
        session_id: session.session_id.clone(),
        ok: issues.is_empty() && plan.ok,
        issue_count: issues.len(),
        issues,
        warnings,
        batch,
        plan,
    })
}

pub fn sanitize_model_context(context: &Value) -> (Value, Vec<String>) {
    let mut redacted_paths = Vec::new();
    let sanitized = sanitize_value(context, "", &mut redacted_paths);
    (sanitized, redacted_paths)
}

pub(crate) fn parse_tool_arguments(value: &Value) -> Result<Value, ModelCollaborationError> {
    let parsed = match value {
        Value::String(text) => serde_json::from_str(text).map_err(|error| {
            ModelCollaborationError::new("invalid_tool_arguments", error.to_string())
        })?,
        other => other.clone(),
    };
    if !parsed.is_object() {
        return Err(ModelCollaborationError::new(
            "invalid_tool_arguments",
            "tool arguments must decode to a JSON object",
        ));
    }
    Ok(parsed)
}

fn validate_session(session: &ModelCollaborationSession) -> Result<(), ModelCollaborationError> {
    if session.schema_version != MODEL_COLLABORATION_SCHEMA_VERSION {
        return Err(ModelCollaborationError::new(
            "unsupported_schema",
            format!(
                "unsupported session schema_version: {}",
                session.schema_version
            ),
        ));
    }
    for (field, value) in [
        ("session_id", session.session_id.as_str()),
        ("workflow_id", session.workflow_id.as_str()),
        ("objective", session.objective.as_str()),
        ("created_at", session.created_at.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ModelCollaborationError::new(
                "invalid_session",
                format!("{field} is required"),
            ));
        }
    }
    if session.policy.max_steps == 0 || session.policy.max_context_bytes == 0 {
        return Err(ModelCollaborationError::new(
            "invalid_policy",
            "max_steps and max_context_bytes must be greater than zero",
        ));
    }
    Ok(())
}

fn policy_allows_action(policy: &ModelCollaborationPolicy, action: &str) -> bool {
    let Some(contract) = find_action_contract(action) else {
        return false;
    };
    (policy.allowed_actions.is_empty()
        || policy.allowed_actions.iter().any(|entry| entry == action))
        && (policy.allowed_categories.is_empty()
            || policy
                .allowed_categories
                .iter()
                .any(|entry| entry == contract.category))
        && (!policy.service_only || contract.engine == HeadlessEngine::Service)
        && (policy.allow_sensitive || contract.risk != HeadlessRisk::Sensitive)
        && (policy.allow_destructive || contract.risk != HeadlessRisk::Destructive)
}

fn action_input_schema(required_keys: &[String]) -> Value {
    let properties = required_keys
        .iter()
        .map(|key| {
            (
                key.clone(),
                json!({ "description": format!("Required by `{key}` contract") }),
            )
        })
        .collect::<Map<_, _>>();
    json!({
        "type": "object",
        "properties": properties,
        "required": required_keys,
        "additionalProperties": true
    })
}

fn tool_description(
    action: &str,
    category: &str,
    required_keys: &[String],
    output_keys: &[String],
) -> String {
    let required = if required_keys.is_empty() {
        "no required payload keys".to_string()
    } else {
        format!("required payload keys: {}", required_keys.join(", "))
    };
    let outputs = if output_keys.is_empty() {
        "no declared outputs".to_string()
    } else {
        format!("declared outputs: {}", output_keys.join(", "))
    };
    format!(
        "Run the Kyuubiki `{action}` action in the `{category}` capability family; {required}; {outputs}."
    )
}

fn collaboration_instructions(session: &ModelCollaborationSession) -> Vec<String> {
    vec![
        format!("Plan only for this objective: {}", session.objective),
        "Use only the supplied Kyuubiki tools; never invent action names.".to_string(),
        format!("Return no more than {} tool calls.", session.policy.max_steps),
        "Treat tool calls as an untrusted proposal; Kyuubiki validates and confirms before execution."
            .to_string(),
        "Use {{steps.N.result.KEY}} bindings only for declared outputs from earlier steps."
            .to_string(),
    ]
}

fn sanitize_value(value: &Value, path: &str, redacted_paths: &mut Vec<String>) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| {
                    let next_path = format!("{}/{}", path, escape_json_pointer(key));
                    let sanitized = if sensitive_key(key) {
                        redacted_paths.push(next_path);
                        Value::String("[REDACTED]".to_string())
                    } else {
                        sanitize_value(value, &next_path, redacted_paths)
                    };
                    (key.clone(), sanitized)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    sanitize_value(value, &format!("{path}/{index}"), redacted_paths)
                })
                .collect(),
        ),
        Value::String(text) if bearer_token(text) => {
            redacted_paths.push(if path.is_empty() {
                "/".to_string()
            } else {
                path.to_string()
            });
            Value::String("[REDACTED]".to_string())
        }
        other => other.clone(),
    }
}

fn sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['-', '.'], "_");
    [
        "token",
        "secret",
        "password",
        "api_key",
        "apikey",
        "authorization",
        "credential",
        "private_key",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn bearer_token(value: &str) -> bool {
    value
        .trim_start()
        .get(..7)
        .map(|prefix| prefix.eq_ignore_ascii_case("bearer "))
        .unwrap_or(false)
}

fn escape_json_pointer(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
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
