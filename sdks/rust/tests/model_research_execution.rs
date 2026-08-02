use kyuubiki_headless_sdk::{
    ApprovedModelPlanStep, HeadlessModelRisk, MODEL_COLLABORATION_SCHEMA_VERSION,
    MODEL_PLAN_APPROVAL_SCHEMA_VERSION, MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION,
    ModelActionDispatch, ModelActionDispatcher, ModelApprovalVerifier, ModelCollaborationPolicy,
    ModelCollaborationSession, ModelPlanApproval, ModelResearchExecutionStatus, ModelToolCall,
    ModelWorkflowProposal, SessionModelActionDispatcher, build_model_headless_plan,
    compute_model_headless_plan_digest, execute_model_headless_plan,
};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;

struct FakeDispatcher {
    seen: Mutex<Vec<String>>,
    fail_on: Option<String>,
}

impl FakeDispatcher {
    fn new(fail_on: Option<&str>) -> Self {
        Self {
            seen: Mutex::new(Vec::new()),
            fail_on: fail_on.map(str::to_string),
        }
    }
}

impl ModelActionDispatcher for FakeDispatcher {
    fn dispatch_model_action(
        &self,
        action: &str,
        _payload: &Value,
    ) -> kyuubiki_headless_sdk::SdkResult<ModelActionDispatch> {
        self.seen
            .lock()
            .expect("seen lock")
            .push(action.to_string());
        if self.fail_on.as_deref() == Some(action) {
            return Err(kyuubiki_headless_sdk::SdkError::Transport(
                "injected dispatcher failure".to_string(),
            ));
        }
        Ok(ModelActionDispatch {
            authority: "test-dispatcher".to_string(),
            output: json!({ "action": action, "ok": true }),
        })
    }
}

struct TestApprovalVerifier {
    allow: bool,
}

impl ModelApprovalVerifier for TestApprovalVerifier {
    fn verify_model_approval(
        &self,
        _plan: &kyuubiki_headless_sdk::ModelHeadlessPlan,
        _approval: &ModelPlanApproval,
    ) -> kyuubiki_headless_sdk::SdkResult<()> {
        if self.allow {
            Ok(())
        } else {
            Err(kyuubiki_headless_sdk::SdkError::Validation {
                errors: vec!["caller approval verifier rejected approval".to_string()],
            })
        }
    }
}

#[test]
fn rejects_unapproved_plan_before_any_dispatch() {
    let session = collaboration_session();
    let plan = build_model_headless_plan(&session, &first_proposal()).expect("plan");
    let dispatcher = FakeDispatcher::new(None);

    let verifier = TestApprovalVerifier { allow: true };
    let error = execute_model_headless_plan(&dispatcher, &plan, None, &verifier)
        .expect_err("approval gate");
    assert!(
        error
            .to_string()
            .contains("requires an exact caller-issued approval")
    );
    assert!(dispatcher.seen.lock().expect("seen lock").is_empty());
}

#[test]
fn executes_exactly_approved_plan_and_retains_authority() {
    let session = collaboration_session();
    let plan = build_model_headless_plan(&session, &first_proposal()).expect("plan");
    let approval = approval_for(&plan, 2, "workflow_submit_catalog");
    let dispatcher = FakeDispatcher::new(None);
    let verifier = TestApprovalVerifier { allow: true };

    let receipt = execute_model_headless_plan(&dispatcher, &plan, Some(&approval), &verifier)
        .expect("receipt");
    assert_eq!(receipt.status, ModelResearchExecutionStatus::Completed);
    assert_eq!(receipt.completed_steps, 2);
    assert_eq!(receipt.approval_id.as_deref(), Some("approval-test-001"));
    assert_eq!(receipt.plan_digest, approval.plan_digest);
    assert_eq!(
        receipt.records[1].authority.as_deref(),
        Some("test-dispatcher")
    );
    assert_eq!(
        *dispatcher.seen.lock().expect("seen lock"),
        vec!["service_health", "workflow_submit_catalog"]
    );
}

#[test]
fn rejects_unverified_approval_before_any_dispatch() {
    let session = collaboration_session();
    let plan = build_model_headless_plan(&session, &first_proposal()).expect("plan");
    let approval = approval_for(&plan, 2, "workflow_submit_catalog");
    let dispatcher = FakeDispatcher::new(None);
    let verifier = TestApprovalVerifier { allow: false };

    let error = execute_model_headless_plan(&dispatcher, &plan, Some(&approval), &verifier)
        .expect_err("verifier gate");
    assert!(error.to_string().contains("verifier rejected approval"));
    assert!(dispatcher.seen.lock().expect("seen lock").is_empty());
}

#[test]
fn rejects_plan_payload_changed_after_approval_before_any_dispatch() {
    let session = collaboration_session();
    let mut plan = build_model_headless_plan(&session, &first_proposal()).expect("plan");
    let approval = approval_for(&plan, 2, "workflow_submit_catalog");
    plan.steps[1].payload["input_artifacts"]["material_rows"]["rows"] =
        json!([{ "case_id": "injected-after-approval" }]);
    let dispatcher = FakeDispatcher::new(None);
    let verifier = TestApprovalVerifier { allow: true };

    let error = execute_model_headless_plan(&dispatcher, &plan, Some(&approval), &verifier)
        .expect_err("digest gate");
    assert!(error.to_string().contains("plan_digest does not match"));
    assert!(dispatcher.seen.lock().expect("seen lock").is_empty());
}

#[test]
fn retains_partial_failure_instead_of_claiming_completion() {
    let session = collaboration_session();
    let proposal = ModelWorkflowProposal {
        schema_version: MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION.to_string(),
        session_id: session.session_id.clone(),
        summary: "bounded discovery".to_string(),
        calls: vec![
            ModelToolCall {
                id: Some("health".to_string()),
                action: "service_health".to_string(),
                payload: json!({}),
                reason: None,
            },
            ModelToolCall {
                id: Some("protocol".to_string()),
                action: "protocol_describe".to_string(),
                payload: json!({}),
                reason: None,
            },
        ],
    };
    let plan = build_model_headless_plan(&session, &proposal).expect("plan");
    let dispatcher = FakeDispatcher::new(Some("protocol_describe"));
    let verifier = TestApprovalVerifier { allow: true };

    let receipt =
        execute_model_headless_plan(&dispatcher, &plan, None, &verifier).expect("failure receipt");
    assert_eq!(receipt.status, ModelResearchExecutionStatus::Failed);
    assert_eq!(receipt.completed_steps, 1);
    assert_eq!(receipt.failed_step, Some(2));
    assert!(
        receipt.records[1]
            .error
            .as_deref()
            .is_some_and(|error| { error.contains("injected dispatcher failure") })
    );
}

#[test]
fn plan_rejects_malformed_model_payload_types() {
    let session = collaboration_session();
    let mut proposal = first_proposal();
    proposal.calls[1].payload["workflow_id"] = json!(42);
    proposal.calls[1].payload["input_artifacts"] = json!([]);

    let plan = build_model_headless_plan(&session, &proposal).expect("plan report");
    assert!(!plan.ok);
    assert!(
        plan.issues
            .iter()
            .any(|issue| issue.contains("non-empty string"))
    );
    assert!(
        plan.issues
            .iter()
            .any(|issue| issue.contains("JSON object"))
    );
}

#[test]
fn session_dispatcher_reaches_real_control_plane_routes() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let addr = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept request");
            let request = read_http_request(&mut stream);
            let first_line = request.lines().next().unwrap_or_default().to_string();
            let body = if first_line.starts_with("GET /api/health ") {
                r#"{"status":"ok","service":"kyuubiki"}"#
            } else {
                r#"{"job":{"job_id":"job-research-001","status":"queued"}}"#
            };
            requests.push(request);
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        }
        requests
    });

    let session = collaboration_session();
    let plan = build_model_headless_plan(&session, &first_proposal()).expect("plan");
    let approval = approval_for(&plan, 2, "workflow_submit_catalog");
    let headless =
        kyuubiki_headless_sdk::KyuubikiSession::from_control_plane(&format!("http://{addr}"), None)
            .expect("headless session");
    let dispatcher = SessionModelActionDispatcher::new(&headless);
    let verifier = TestApprovalVerifier { allow: true };

    let receipt = execute_model_headless_plan(&dispatcher, &plan, Some(&approval), &verifier)
        .expect("receipt");
    assert_eq!(receipt.status, ModelResearchExecutionStatus::Completed);
    assert_eq!(
        receipt.records[0].authority.as_deref(),
        Some("control_plane")
    );
    assert_eq!(
        receipt.records[1]
            .output
            .as_ref()
            .and_then(|value| { value.get("job")?.get("job_id")?.as_str() }),
        Some("job-research-001")
    );

    let requests = server.join().expect("server thread");
    assert!(requests[0].starts_with("GET /api/health HTTP/1.1"));
    assert!(requests[1].starts_with(
        "POST /api/v1/workflows/catalog/workflow.material-study-envelope-ranking-json/jobs HTTP/1.1"
    ));
    assert!(requests[1].contains("\"input_artifacts\""));
}

#[test]
fn repository_bootstrap_fixtures_reach_approved_execution() {
    let schemas = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas");
    let session: ModelCollaborationSession = serde_json::from_slice(
        &std::fs::read(schemas.join("examples.model-collaboration-session.json"))
            .expect("session fixture"),
    )
    .expect("parse session fixture");
    let proposal: ModelWorkflowProposal = serde_json::from_slice(
        &std::fs::read(schemas.join("examples.model-workflow-proposal.json"))
            .expect("proposal fixture"),
    )
    .expect("parse proposal fixture");
    let approval: ModelPlanApproval = serde_json::from_slice(
        &std::fs::read(schemas.join("examples.model-plan-approval.json"))
            .expect("approval fixture"),
    )
    .expect("parse approval fixture");
    let plan = build_model_headless_plan(&session, &proposal).expect("plan");
    let dispatcher = FakeDispatcher::new(None);
    let verifier = TestApprovalVerifier { allow: true };

    let receipt = execute_model_headless_plan(&dispatcher, &plan, Some(&approval), &verifier)
        .expect("approved fixture execution");
    assert_eq!(receipt.status, ModelResearchExecutionStatus::Completed);
    assert_eq!(receipt.completed_steps, proposal.calls.len());
}

fn collaboration_session() -> ModelCollaborationSession {
    ModelCollaborationSession {
        schema_version: MODEL_COLLABORATION_SCHEMA_VERSION.to_string(),
        session_id: "research-session-test".to_string(),
        workflow_id: "workflow.material-study-envelope-ranking-json".to_string(),
        objective: "Run one bounded material screening study.".to_string(),
        language: "en".to_string(),
        created_at: "2026-08-01T00:00:00Z".to_string(),
        policy: ModelCollaborationPolicy {
            allowed_actions: vec![
                "service_health".to_string(),
                "protocol_describe".to_string(),
                "workflow_submit_catalog".to_string(),
            ],
            allow_sensitive: true,
            ..ModelCollaborationPolicy::default()
        },
    }
}

fn first_proposal() -> ModelWorkflowProposal {
    ModelWorkflowProposal {
        schema_version: MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION.to_string(),
        session_id: "research-session-test".to_string(),
        summary: "Health check and bounded catalog submission.".to_string(),
        calls: vec![
            ModelToolCall {
                id: Some("health".to_string()),
                action: "service_health".to_string(),
                payload: json!({}),
                reason: None,
            },
            ModelToolCall {
                id: Some("submit".to_string()),
                action: "workflow_submit_catalog".to_string(),
                payload: json!({
                    "workflow_id": "workflow.material-study-envelope-ranking-json",
                    "input_artifacts": { "material_rows": { "rows": [] } }
                }),
                reason: None,
            },
        ],
    }
}

fn approval_for(
    plan: &kyuubiki_headless_sdk::ModelHeadlessPlan,
    index: usize,
    action: &str,
) -> ModelPlanApproval {
    assert_eq!(plan.steps[index - 1].risk, HeadlessModelRisk::Sensitive);
    ModelPlanApproval {
        schema_version: MODEL_PLAN_APPROVAL_SCHEMA_VERSION.to_string(),
        approval_id: "approval-test-001".to_string(),
        session_id: plan.session_id.clone(),
        workflow_id: plan.workflow_id.clone(),
        plan_digest: compute_model_headless_plan_digest(plan).expect("plan digest"),
        authority: "integration-test".to_string(),
        issued_at: "2026-08-01T00:01:00Z".to_string(),
        approved_steps: vec![ApprovedModelPlanStep {
            index,
            action: action.to_string(),
        }],
    }
}

fn read_http_request(stream: &mut std::net::TcpStream) -> String {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let mut expected = None;
    loop {
        let count = stream.read(&mut chunk).expect("read request");
        if count == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..count]);
        if expected.is_none() {
            if let Some(header_end) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
                let header_text = String::from_utf8_lossy(&buffer[..header_end]);
                let content_length = header_text
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(str::to_string)
                    })
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(0);
                expected = Some(header_end + 4 + content_length);
            }
        }
        if expected.is_some_and(|expected| buffer.len() >= expected) {
            break;
        }
    }
    String::from_utf8(buffer).expect("utf-8 request")
}
