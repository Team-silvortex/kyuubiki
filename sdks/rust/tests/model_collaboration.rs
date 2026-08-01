use kyuubiki_headless_sdk::{
    HeadlessModelRisk, HeadlessModelRuntime, MODEL_COLLABORATION_SCHEMA_VERSION,
    MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION, ModelCollaborationPolicy, ModelCollaborationSession,
    ModelProvider, ModelToolCall, ModelWorkflowProposal, build_model_collaboration_request,
    build_model_headless_plan, normalize_model_response, rust_headless_model_tools,
};
use serde_json::json;
use std::{fs, path::Path};

fn session(allow_sensitive: bool) -> ModelCollaborationSession {
    ModelCollaborationSession {
        schema_version: MODEL_COLLABORATION_SCHEMA_VERSION.to_string(),
        session_id: "session.rust-headless-model".to_string(),
        workflow_id: "workflow.rust-headless-model".to_string(),
        objective: "Discover the runtime and submit one bounded solve".to_string(),
        language: "en".to_string(),
        created_at: "2026-08-01T00:00:00Z".to_string(),
        policy: ModelCollaborationPolicy {
            allow_sensitive,
            ..ModelCollaborationPolicy::default()
        },
    }
}

#[test]
fn default_catalog_is_read_only_and_service_owned() {
    let tools = rust_headless_model_tools(&ModelCollaborationPolicy::default());
    assert!(tools.iter().any(|tool| tool.action == "service_health"));
    assert!(
        tools
            .iter()
            .any(|tool| tool.action == "operator_task_prepare")
    );
    assert!(
        tools
            .iter()
            .all(|tool| tool.risk == HeadlessModelRisk::Normal)
    );
    assert!(
        tools
            .iter()
            .all(|tool| tool.runtime == HeadlessModelRuntime::Service)
    );
    assert!(!tools.iter().any(|tool| tool.action == "fem_submit"));
    assert!(!tools.iter().any(|tool| tool.action == "job_cancel"));
}

#[test]
fn service_only_policy_excludes_direct_solver_even_when_sensitive_is_allowed() {
    let service_tools = rust_headless_model_tools(&ModelCollaborationPolicy {
        allow_sensitive: true,
        ..ModelCollaborationPolicy::default()
    });
    assert!(service_tools.iter().any(|tool| tool.action == "fem_submit"));
    assert!(
        !service_tools
            .iter()
            .any(|tool| tool.action == "direct_solver_rpc")
    );

    let all_tools = rust_headless_model_tools(&ModelCollaborationPolicy {
        allow_sensitive: true,
        service_only: false,
        ..ModelCollaborationPolicy::default()
    });
    assert!(
        all_tools
            .iter()
            .any(|tool| tool.action == "direct_solver_rpc")
    );
}

#[test]
fn provider_requests_share_one_filtered_catalog() {
    for provider in [
        ModelProvider::OpenAi,
        ModelProvider::OpenAiChat,
        ModelProvider::Anthropic,
        ModelProvider::Gemini,
        ModelProvider::Canonical,
    ] {
        let request = build_model_collaboration_request(
            provider,
            session(false),
            json!({ "authorization": "Bearer secret-value", "project": "demo" }),
        )
        .expect("provider request");
        assert_eq!(request.context["authorization"], "[REDACTED]");
        assert!(!request.tools.as_array().unwrap().is_empty());
    }
}

#[test]
fn normalizes_provider_calls_into_shared_proposal() {
    let openai = json!({
        "output": [{
            "type": "function_call", "call_id": "call_1",
            "name": "service_health", "arguments": "{}"
        }]
    });
    let anthropic = json!({
        "content": [{
            "type": "tool_use", "id": "toolu_1",
            "name": "service_health", "input": {}
        }]
    });
    let gemini = json!({
        "candidates": [{ "content": { "parts": [{
            "functionCall": { "id": "gemini_1", "name": "service_health", "args": {} }
        }]}}]
    });
    let openai_chat = json!({
        "choices": [{ "message": { "tool_calls": [{
            "id": "chat_1", "function": {
                "name": "service_health", "arguments": "{}"
            }
        }]}}]
    });
    let gemini_interactions = json!({
        "steps": [{
            "type": "function_call", "id": "gemini_2",
            "name": "service_health", "arguments": {}
        }]
    });
    let canonical = json!({
        "schema_version": MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION,
        "session_id": "session.rust-headless-model",
        "calls": [{ "action": "service_health", "payload": {} }]
    });
    for (provider, response) in [
        (ModelProvider::OpenAi, openai),
        (ModelProvider::OpenAiChat, openai_chat),
        (ModelProvider::Anthropic, anthropic),
        (ModelProvider::Gemini, gemini),
        (ModelProvider::Gemini, gemini_interactions),
        (ModelProvider::Canonical, canonical),
    ] {
        let proposal = normalize_model_response(provider, "session.rust-headless-model", &response)
            .expect("normalized proposal");
        assert_eq!(proposal.calls[0].action, "service_health");
    }
}

#[test]
fn oversized_context_is_rejected_after_redaction() {
    let mut bounded = session(false);
    bounded.policy.max_context_bytes = 16;
    let error = build_model_collaboration_request(
        ModelProvider::OpenAi,
        bounded,
        json!({ "token": "secret", "payload": "this remains too large" }),
    )
    .expect_err("oversized context must fail closed");
    assert!(error.to_string().contains("policy allows 16"));
}

#[test]
fn sensitive_calls_remain_confirmation_gated() {
    let proposal = ModelWorkflowProposal {
        schema_version: MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION.to_string(),
        session_id: "session.rust-headless-model".to_string(),
        summary: "Submit a bounded solve".to_string(),
        calls: vec![ModelToolCall {
            id: Some("call_1".to_string()),
            action: "fem_submit".to_string(),
            payload: json!({
                "solve_kind": "thermal_frame_3d",
                "payload": { "model": {} }
            }),
            reason: Some("Run the requested solve".to_string()),
        }],
    };
    let plan = build_model_headless_plan(&session(true), &proposal).expect("Headless plan");
    assert!(plan.ok, "{:?}", plan.issues);
    assert!(!plan.ready_without_confirmation);
    assert!(plan.steps[0].requires_confirmation);
}

#[test]
fn proposal_cannot_use_hidden_or_incomplete_actions() {
    let proposal = ModelWorkflowProposal {
        schema_version: MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION.to_string(),
        session_id: "session.rust-headless-model".to_string(),
        summary: String::new(),
        calls: vec![ModelToolCall {
            id: None,
            action: "fem_submit".to_string(),
            payload: json!({ "solve_kind": "thermal_frame_3d" }),
            reason: None,
        }],
    };
    let hidden = build_model_headless_plan(&session(false), &proposal).expect("hidden plan");
    assert!(!hidden.ok);

    let incomplete = build_model_headless_plan(&session(true), &proposal).expect("incomplete plan");
    assert!(!incomplete.ok);
    assert!(
        incomplete
            .issues
            .iter()
            .any(|issue| issue.contains("payload"))
    );
}

#[test]
fn model_research_bootstrap_reaches_valid_first_plan() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bootstrap: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("docs/model-research-bootstrap.json"))
            .expect("model research bootstrap"),
    )
    .expect("bootstrap JSON");
    for document in bootstrap["required_documents"].as_array().unwrap() {
        assert!(root.join(document["path"].as_str().unwrap()).is_file());
    }
    let first = &bootstrap["first_research"];
    let session: ModelCollaborationSession = serde_json::from_str(
        &fs::read_to_string(root.join(first["session_fixture"].as_str().unwrap()))
            .expect("shared session fixture"),
    )
    .expect("session JSON");
    let proposal: ModelWorkflowProposal = serde_json::from_str(
        &fs::read_to_string(root.join(first["proposal_fixture"].as_str().unwrap()))
            .expect("shared proposal fixture"),
    )
    .expect("proposal JSON");
    let request =
        build_model_collaboration_request(ModelProvider::Canonical, session.clone(), json!({}))
            .expect("bootstrap request");
    let plan = build_model_headless_plan(&session, &proposal).expect("bootstrap plan");
    assert_eq!(request.output_contract, proposal.schema_version);
    assert!(plan.ok, "{:?}", plan.issues);
    assert!(!plan.ready_without_confirmation);
}
