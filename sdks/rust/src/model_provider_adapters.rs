use crate::{
    HeadlessModelTool, MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION, ModelProvider, ModelToolCall,
    ModelWorkflowProposal, SdkError, SdkResult,
};
use serde_json::{Map, Value, json};

pub fn project_model_tools(provider: ModelProvider, tools: &[HeadlessModelTool]) -> Value {
    let definitions = tools
        .iter()
        .map(|tool| {
            let parameters = tool_parameters(&tool.required_payload_keys);
            match provider {
                ModelProvider::OpenAi => json!({
                    "type": "function", "name": tool.action,
                    "description": tool.description, "parameters": parameters, "strict": false
                }),
                ModelProvider::OpenAiChat => json!({
                    "type": "function", "function": {
                        "name": tool.action, "description": tool.description,
                        "parameters": parameters, "strict": false
                    }
                }),
                ModelProvider::Anthropic => json!({
                    "name": tool.action, "description": tool.description,
                    "input_schema": parameters
                }),
                ModelProvider::Gemini => json!({
                    "name": tool.action, "description": tool.description,
                    "parameters": parameters
                }),
                ModelProvider::Canonical => {
                    serde_json::to_value(tool).expect("Headless model tool is serializable")
                }
            }
        })
        .collect::<Vec<_>>();
    match provider {
        ModelProvider::Gemini => json!([{ "functionDeclarations": definitions }]),
        _ => Value::Array(definitions),
    }
}

pub fn normalize_model_response(
    provider: ModelProvider,
    session_id: &str,
    response: &Value,
) -> SdkResult<ModelWorkflowProposal> {
    match provider {
        ModelProvider::OpenAi | ModelProvider::OpenAiChat => {
            normalize_openai_response(session_id, response)
        }
        ModelProvider::Anthropic => normalize_anthropic_response(session_id, response),
        ModelProvider::Gemini => normalize_gemini_response(session_id, response),
        ModelProvider::Canonical => normalize_canonical_response(session_id, response),
    }
}

pub fn sanitize_model_context(context: &Value) -> (Value, Vec<String>) {
    let mut redacted_paths = Vec::new();
    let sanitized = sanitize_value(context, "", &mut redacted_paths);
    (sanitized, redacted_paths)
}

fn tool_parameters(required_keys: &[String]) -> Value {
    let properties = required_keys
        .iter()
        .map(|key| {
            (
                key.clone(),
                json!({ "description": format!("Required `{key}` payload") }),
            )
        })
        .collect::<Map<_, _>>();
    json!({
        "type": "object", "properties": properties,
        "required": required_keys, "additionalProperties": true
    })
}

fn normalize_openai_response(
    session_id: &str,
    response: &Value,
) -> SdkResult<ModelWorkflowProposal> {
    let mut calls = Vec::new();
    let mut summaries = Vec::new();
    for item in response
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if item.get("type").and_then(Value::as_str) == Some("function_call") {
            calls.push(parse_call(
                item.get("call_id").or_else(|| item.get("id")),
                item.get("name"),
                item.get("arguments"),
            )?);
        } else if item.get("type").and_then(Value::as_str) == Some("message") {
            collect_openai_content(item.get("content"), &mut summaries);
        }
    }
    for choice in response
        .get("choices")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let message = choice.get("message").unwrap_or(&Value::Null);
        collect_text(message.get("content"), &mut summaries);
        for item in message
            .get("tool_calls")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let function = item.get("function").unwrap_or(&Value::Null);
            calls.push(parse_call(
                item.get("id"),
                function.get("name"),
                function.get("arguments"),
            )?);
        }
    }
    proposal(session_id, summaries, calls)
}

fn normalize_anthropic_response(
    session_id: &str,
    response: &Value,
) -> SdkResult<ModelWorkflowProposal> {
    let mut calls = Vec::new();
    let mut summaries = Vec::new();
    for block in response
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match block.get("type").and_then(Value::as_str) {
            Some("tool_use") => calls.push(parse_call(
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
) -> SdkResult<ModelWorkflowProposal> {
    let mut calls = Vec::new();
    let mut summaries = Vec::new();
    for candidate in response
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let parts = candidate
            .get("content")
            .and_then(|content| content.get("parts"))
            .and_then(Value::as_array);
        for part in parts.into_iter().flatten() {
            collect_text(part.get("text"), &mut summaries);
            if let Some(function) = part.get("functionCall") {
                calls.push(parse_call(
                    function.get("id"),
                    function.get("name"),
                    function.get("args"),
                )?);
            }
        }
    }
    for step in response
        .get("steps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if step.get("type").and_then(Value::as_str) == Some("function_call") {
            calls.push(parse_call(
                step.get("id"),
                step.get("name"),
                step.get("arguments"),
            )?);
        }
    }
    proposal(session_id, summaries, calls)
}

fn normalize_canonical_response(
    session_id: &str,
    response: &Value,
) -> SdkResult<ModelWorkflowProposal> {
    let mut proposal: ModelWorkflowProposal = serde_json::from_value(response.clone())?;
    if proposal.session_id != session_id {
        return validation_error("canonical proposal session_id does not match requested session");
    }
    for call in &mut proposal.calls {
        call.payload = parse_arguments(&call.payload)?;
    }
    Ok(proposal)
}

fn parse_call(
    id: Option<&Value>,
    name: Option<&Value>,
    arguments: Option<&Value>,
) -> SdkResult<ModelToolCall> {
    let action = name
        .and_then(Value::as_str)
        .ok_or_else(|| SdkError::Validation {
            errors: vec!["provider tool call is missing a string name".to_string()],
        })?;
    let arguments = arguments.ok_or_else(|| SdkError::Validation {
        errors: vec![format!("provider tool call {action} is missing arguments")],
    })?;
    Ok(ModelToolCall {
        id: id.and_then(Value::as_str).map(str::to_string),
        action: action.to_string(),
        payload: parse_arguments(arguments)?,
        reason: None,
    })
}

fn parse_arguments(value: &Value) -> SdkResult<Value> {
    let parsed = match value {
        Value::String(text) => serde_json::from_str(text)?,
        other => other.clone(),
    };
    if parsed.is_object() {
        Ok(parsed)
    } else {
        validation_error("provider tool arguments must decode to a JSON object")
    }
}

fn proposal(
    session_id: &str,
    summaries: Vec<String>,
    calls: Vec<ModelToolCall>,
) -> SdkResult<ModelWorkflowProposal> {
    if calls.is_empty() {
        return validation_error("provider response contains no supported tool calls");
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

fn sanitize_value(value: &Value, path: &str, redacted_paths: &mut Vec<String>) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| {
                    let next_path =
                        format!("{}/{}", path, key.replace('~', "~0").replace('/', "~1"));
                    let value = if sensitive_key(key) {
                        redacted_paths.push(next_path);
                        Value::String("[REDACTED]".to_string())
                    } else {
                        sanitize_value(value, &next_path, redacted_paths)
                    };
                    (key.clone(), value)
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
        Value::String(text)
            if text
                .trim_start()
                .get(..7)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("bearer ")) =>
        {
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

fn validation_error<T>(message: impl Into<String>) -> SdkResult<T> {
    Err(SdkError::Validation {
        errors: vec![message.into()],
    })
}
