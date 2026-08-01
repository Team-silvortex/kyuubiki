use crate::model_collaboration::parse_tool_arguments;
use crate::{
    MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION, ModelCollaborationError, ModelCollaborationTool,
    ModelProvider, ModelToolCall, ModelWorkflowProposal,
};
use serde_json::{Value, json};

pub fn project_model_tools(provider: ModelProvider, tools: &[ModelCollaborationTool]) -> Value {
    let projected = tools
        .iter()
        .map(|tool| match provider {
            ModelProvider::OpenAi => json!({
                "type": "function",
            "name": tool.action,
            "description": tool.description,
            "parameters": tool.input_schema,
                "strict": false
            }),
            ModelProvider::OpenAiChat => json!({
                "type": "function",
                "function": {
                    "name": tool.action,
                    "description": tool.description,
                    "parameters": tool.input_schema,
                    "strict": false
                }
            }),
            ModelProvider::Anthropic => json!({
                "name": tool.action,
                "description": tool.description,
                "input_schema": tool.input_schema
            }),
            ModelProvider::Gemini => json!({
                "name": tool.action,
                "description": tool.description,
                "parameters": tool.input_schema
            }),
            ModelProvider::Canonical => {
                serde_json::to_value(tool).expect("model collaboration tool is serializable")
            }
        })
        .collect::<Vec<_>>();
    match provider {
        ModelProvider::Gemini => json!([{ "functionDeclarations": projected }]),
        _ => Value::Array(projected),
    }
}

pub fn normalize_model_response(
    provider: ModelProvider,
    session_id: &str,
    response: &Value,
) -> Result<crate::ModelWorkflowProposal, ModelCollaborationError> {
    match provider {
        ModelProvider::OpenAi | ModelProvider::OpenAiChat => {
            normalize_openai_response(session_id, response)
        }
        ModelProvider::Anthropic => normalize_anthropic_response(session_id, response),
        ModelProvider::Gemini => normalize_gemini_response(session_id, response),
        ModelProvider::Canonical => normalize_canonical_response(session_id, response),
    }
}

fn normalize_openai_response(
    session_id: &str,
    response: &Value,
) -> Result<ModelWorkflowProposal, ModelCollaborationError> {
    let mut calls = Vec::new();
    let mut summaries = Vec::new();

    if let Some(items) = response.get("output").and_then(Value::as_array) {
        for item in items {
            match item.get("type").and_then(Value::as_str) {
                Some("function_call") => calls.push(tool_call(
                    item.get("call_id").or_else(|| item.get("id")),
                    item.get("name"),
                    item.get("arguments"),
                )?),
                Some("message") => collect_openai_content(item.get("content"), &mut summaries),
                _ => {}
            }
        }
    }

    if let Some(choices) = response.get("choices").and_then(Value::as_array) {
        for choice in choices {
            let message = choice.get("message").unwrap_or(&Value::Null);
            collect_text(message.get("content"), &mut summaries);
            if let Some(tool_calls) = message.get("tool_calls").and_then(Value::as_array) {
                for item in tool_calls {
                    let function = item.get("function").unwrap_or(&Value::Null);
                    calls.push(tool_call(
                        item.get("id"),
                        function.get("name"),
                        function.get("arguments"),
                    )?);
                }
            }
        }
    }
    proposal(session_id, summaries, calls)
}

fn normalize_anthropic_response(
    session_id: &str,
    response: &Value,
) -> Result<ModelWorkflowProposal, ModelCollaborationError> {
    let mut calls = Vec::new();
    let mut summaries = Vec::new();
    for block in response
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match block.get("type").and_then(Value::as_str) {
            Some("tool_use") => calls.push(tool_call(
                block.get("id"),
                block.get("name"),
                block.get("input"),
            )?),
            Some("text") => collect_text(block.get("text"), &mut summaries),
            _ => {}
        }
    }
    proposal(session_id, summaries, calls)
}

fn normalize_gemini_response(
    session_id: &str,
    response: &Value,
) -> Result<ModelWorkflowProposal, ModelCollaborationError> {
    let mut calls = Vec::new();
    let mut summaries = Vec::new();
    if let Some(candidates) = response.get("candidates").and_then(Value::as_array) {
        for candidate in candidates {
            let parts = candidate
                .get("content")
                .and_then(|content| content.get("parts"))
                .and_then(Value::as_array);
            for part in parts.into_iter().flatten() {
                collect_text(part.get("text"), &mut summaries);
                if let Some(function) = part.get("functionCall") {
                    calls.push(tool_call(
                        function.get("id"),
                        function.get("name"),
                        function.get("args"),
                    )?);
                }
            }
        }
    }
    if let Some(steps) = response.get("steps").and_then(Value::as_array) {
        for step in steps {
            match step.get("type").and_then(Value::as_str) {
                Some("function_call") => calls.push(tool_call(
                    step.get("id"),
                    step.get("name"),
                    step.get("arguments"),
                )?),
                Some("text") => collect_text(step.get("text"), &mut summaries),
                _ => {}
            }
        }
    }
    proposal(session_id, summaries, calls)
}

fn normalize_canonical_response(
    session_id: &str,
    response: &Value,
) -> Result<ModelWorkflowProposal, ModelCollaborationError> {
    let mut proposal: ModelWorkflowProposal =
        serde_json::from_value(response.clone()).map_err(|error| {
            ModelCollaborationError::new("malformed_provider_response", error.to_string())
        })?;
    if proposal.session_id != session_id {
        return Err(ModelCollaborationError::new(
            "session_mismatch",
            "canonical proposal session_id does not match the requested session",
        ));
    }
    for call in &mut proposal.calls {
        call.payload = parse_tool_arguments(&call.payload)?;
    }
    Ok(proposal)
}

fn tool_call(
    id: Option<&Value>,
    name: Option<&Value>,
    arguments: Option<&Value>,
) -> Result<ModelToolCall, ModelCollaborationError> {
    let action = name.and_then(Value::as_str).ok_or_else(|| {
        ModelCollaborationError::new(
            "malformed_provider_response",
            "tool call is missing a string name",
        )
    })?;
    let arguments = arguments.ok_or_else(|| {
        ModelCollaborationError::new(
            "malformed_provider_response",
            format!("tool call {action} is missing arguments"),
        )
    })?;
    Ok(ModelToolCall {
        id: id.and_then(Value::as_str).map(str::to_string),
        action: action.to_string(),
        payload: parse_tool_arguments(arguments)?,
        reason: None,
    })
}

fn proposal(
    session_id: &str,
    summaries: Vec<String>,
    calls: Vec<ModelToolCall>,
) -> Result<ModelWorkflowProposal, ModelCollaborationError> {
    if calls.is_empty() {
        return Err(ModelCollaborationError::new(
            "no_tool_calls",
            "provider response did not contain a supported tool call",
        ));
    }
    Ok(ModelWorkflowProposal {
        schema_version: MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION.to_string(),
        session_id: session_id.to_string(),
        summary: summaries.join("\n"),
        calls,
    })
}

fn collect_openai_content(content: Option<&Value>, summaries: &mut Vec<String>) {
    for part in content.and_then(Value::as_array).into_iter().flatten() {
        if matches!(
            part.get("type").and_then(Value::as_str),
            Some("output_text" | "text")
        ) {
            collect_text(part.get("text"), summaries);
        }
    }
}

fn collect_text(value: Option<&Value>, summaries: &mut Vec<String>) {
    if let Some(text) = value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        summaries.push(text.to_string());
    }
}
