use crate::{
    MODEL_COLLABORATION_SCHEMA_VERSION, MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION,
    ModelCollaborationPolicy, ModelCollaborationSession, ModelProvider, ModelToolCall,
    ModelWorkflowProposal, build_model_collaboration_request, compile_model_proposal,
    model_collaboration_tools, normalize_model_response, sanitize_model_context,
};
use serde_json::{Value, json};

fn session() -> ModelCollaborationSession {
    ModelCollaborationSession {
        schema_version: MODEL_COLLABORATION_SCHEMA_VERSION.to_string(),
        session_id: "session.model-test".to_string(),
        workflow_id: "workflow.model-test".to_string(),
        objective: "Check the service and submit a small thermal solve".to_string(),
        language: "en".to_string(),
        created_at: "2026-08-01T00:00:00Z".to_string(),
        policy: ModelCollaborationPolicy::default(),
    }
}

#[test]
fn default_catalog_is_service_only_and_blocks_elevated_risks() {
    let tools = model_collaboration_tools(&ModelCollaborationPolicy::default());
    assert!(!tools.is_empty());
    assert!(
        tools
            .iter()
            .all(|tool| tool.runtime_style == crate::HeadlessRuntimeStyle::ServiceOnly)
    );
    assert!(
        tools
            .iter()
            .all(|tool| tool.risk == crate::HeadlessRisk::Normal)
    );
    assert!(tools.iter().any(|tool| tool.action == "service_health"));
    assert!(!tools.iter().any(|tool| tool.action == "snapshot"));
    assert!(!tools.iter().any(|tool| tool.action == "project_delete"));
}

#[test]
fn model_context_redacts_nested_credentials_and_bearer_values() {
    let (sanitized, paths) = sanitize_model_context(&json!({
        "project": { "name": "test", "api_key": "secret" },
        "headers": ["Bearer token-value", "application/json"]
    }));
    assert_eq!(sanitized["project"]["api_key"], "[REDACTED]");
    assert_eq!(sanitized["headers"][0], "[REDACTED]");
    assert!(paths.contains(&"/project/api_key".to_string()));
    assert!(paths.contains(&"/headers/0".to_string()));
}

#[test]
fn request_projects_provider_specific_tool_shapes() {
    for (provider, expected_key) in [
        (ModelProvider::OpenAi, "parameters"),
        (ModelProvider::Anthropic, "input_schema"),
    ] {
        let request = build_model_collaboration_request(
            provider,
            session(),
            json!({ "project_id": "project.alpha" }),
        )
        .expect("provider request");
        let first = request.tools.as_array().unwrap().first().unwrap();
        assert!(first.get("name").is_some());
        assert!(first.get(expected_key).is_some());
    }
    let openai_chat = build_model_collaboration_request(
        ModelProvider::OpenAiChat,
        session(),
        json!({ "project_id": "project.alpha" }),
    )
    .expect("OpenAI-compatible Chat request");
    let function = &openai_chat.tools[0]["function"];
    assert!(function.get("name").is_some());
    assert!(function.get("parameters").is_some());
    let gemini = build_model_collaboration_request(
        ModelProvider::Gemini,
        session(),
        json!({ "project_id": "project.alpha" }),
    )
    .expect("Gemini request");
    let declarations = gemini.tools[0]["functionDeclarations"]
        .as_array()
        .expect("Gemini function declarations");
    assert!(declarations[0].get("name").is_some());
    assert!(declarations[0].get("parameters").is_some());
}

#[test]
fn oversized_context_is_rejected_after_redaction() {
    let mut session = session();
    session.policy.max_context_bytes = 8;
    let error = build_model_collaboration_request(
        ModelProvider::Canonical,
        session,
        json!({ "value": "too large" }),
    )
    .unwrap_err();
    assert_eq!(error.code, "context_too_large");
}

#[test]
fn normalizes_openai_responses_and_chat_completions() {
    let responses = json!({
        "output": [
            { "type": "message", "content": [{ "type": "output_text", "text": "Check first" }] },
            { "type": "function_call", "call_id": "call_1", "name": "service_health", "arguments": "{}" }
        ]
    });
    let proposal =
        normalize_model_response(ModelProvider::OpenAi, "session.model-test", &responses)
            .expect("OpenAI Responses proposal");
    assert_eq!(proposal.calls[0].id.as_deref(), Some("call_1"));
    assert_eq!(proposal.calls[0].action, "service_health");
    assert_eq!(proposal.summary, "Check first");

    let chat = json!({
        "choices": [{ "message": {
            "content": "Use the service health action",
            "tool_calls": [{
                "id": "call_2",
                "type": "function",
                "function": { "name": "service_health", "arguments": "{}" }
            }]
        }}]
    });
    let proposal = normalize_model_response(ModelProvider::OpenAi, "session.model-test", &chat)
        .expect("OpenAI Chat proposal");
    assert_eq!(proposal.calls[0].id.as_deref(), Some("call_2"));
}

#[test]
fn normalizes_anthropic_tool_use_blocks() {
    let response = json!({
        "content": [
            { "type": "text", "text": "Check runtime health" },
            { "type": "tool_use", "id": "toolu_1", "name": "service_health", "input": {} }
        ],
        "stop_reason": "tool_use"
    });
    let proposal =
        normalize_model_response(ModelProvider::Anthropic, "session.model-test", &response)
            .expect("Anthropic proposal");
    assert_eq!(proposal.calls[0].id.as_deref(), Some("toolu_1"));
    assert_eq!(proposal.summary, "Check runtime health");
}

#[test]
fn normalizes_both_gemini_function_call_envelopes() {
    let generate_content = json!({
        "candidates": [{ "content": { "parts": [
            { "text": "Check runtime health" },
            { "functionCall": { "id": "gemini_1", "name": "service_health", "args": {} } }
        ]}}]
    });
    let proposal = normalize_model_response(
        ModelProvider::Gemini,
        "session.model-test",
        &generate_content,
    )
    .expect("Gemini generateContent proposal");
    assert_eq!(proposal.calls[0].id.as_deref(), Some("gemini_1"));

    let interactions = json!({
        "steps": [{
            "type": "function_call",
            "id": "gemini_2",
            "name": "service_health",
            "arguments": {}
        }]
    });
    let proposal =
        normalize_model_response(ModelProvider::Gemini, "session.model-test", &interactions)
            .expect("Gemini Interactions proposal");
    assert_eq!(proposal.calls[0].id.as_deref(), Some("gemini_2"));
}

#[test]
fn malformed_tool_arguments_fail_closed() {
    let response = json!({
        "output": [{
            "type": "function_call",
            "call_id": "call_bad",
            "name": "service_health",
            "arguments": "not-json"
        }]
    });
    let error = normalize_model_response(ModelProvider::OpenAi, "session.model-test", &response)
        .unwrap_err();
    assert_eq!(error.code, "invalid_tool_arguments");
}

#[test]
fn valid_proposal_compiles_into_existing_execution_plan() {
    let proposal = ModelWorkflowProposal {
        schema_version: MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION.to_string(),
        session_id: "session.model-test".to_string(),
        summary: "Check runtime health".to_string(),
        calls: vec![ModelToolCall {
            id: Some("call_1".to_string()),
            action: "service_health".to_string(),
            payload: json!({}),
            reason: Some("Confirm runtime availability".to_string()),
        }],
    };
    let compilation = compile_model_proposal(&session(), &proposal).expect("compile proposal");
    assert!(compilation.ok, "{:?}", compilation.issues);
    assert_eq!(compilation.batch.steps.len(), 1);
    assert_eq!(compilation.plan.steps[0].action, "service_health");
    assert_eq!(compilation.plan.confirmation_count, 0);
}

#[test]
fn policy_rejects_destructive_and_mismatched_proposals() {
    let proposal = ModelWorkflowProposal {
        schema_version: MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION.to_string(),
        session_id: "different-session".to_string(),
        summary: String::new(),
        calls: vec![ModelToolCall {
            id: None,
            action: "project_delete".to_string(),
            payload: json!({ "project_id": "project.alpha" }),
            reason: None,
        }],
    };
    let compilation = compile_model_proposal(&session(), &proposal).expect("compile proposal");
    assert!(!compilation.ok);
    assert!(
        compilation
            .issues
            .iter()
            .any(|issue| issue.contains("session_id"))
    );
    assert!(
        compilation
            .issues
            .iter()
            .any(|issue| issue.contains("blocked"))
    );
}

#[test]
fn canonical_proposal_preserves_structured_payloads() {
    let response = json!({
        "schema_version": MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION,
        "session_id": "session.model-test",
        "summary": "Check runtime",
        "calls": [{ "action": "service_health", "payload": {} }]
    });
    let proposal =
        normalize_model_response(ModelProvider::Canonical, "session.model-test", &response)
            .expect("canonical proposal");
    assert_eq!(proposal.calls[0].payload, Value::Object(Default::default()));
}

#[test]
fn repository_proposal_fixture_deserializes_and_compiles() {
    let proposal: ModelWorkflowProposal = serde_json::from_str(include_str!(
        "../../../../../schemas/examples.model-workflow-proposal.json"
    ))
    .expect("repository collaboration fixture");
    let mut session = session();
    session.session_id = proposal.session_id.clone();
    let compilation = compile_model_proposal(&session, &proposal).expect("compile fixture");
    assert!(compilation.ok, "{:?}", compilation.issues);
    assert_eq!(compilation.batch.steps.len(), 2);
}

#[test]
fn repository_session_fixture_builds_every_provider_request() {
    let session: ModelCollaborationSession = serde_json::from_str(include_str!(
        "../../../../../schemas/examples.model-collaboration-session.json"
    ))
    .expect("repository collaboration session fixture");
    for provider in [
        ModelProvider::OpenAi,
        ModelProvider::OpenAiChat,
        ModelProvider::Anthropic,
        ModelProvider::Gemini,
        ModelProvider::Canonical,
    ] {
        let request = build_model_collaboration_request(
            provider,
            session.clone(),
            json!({ "project": "screening" }),
        )
        .expect("provider request from fixture");
        assert_eq!(request.session.session_id, session.session_id);
        assert_eq!(
            request.tools.as_array().unwrap().len(),
            if provider == ModelProvider::Gemini {
                1
            } else {
                2
            }
        );
    }
}
