use kyuubiki_headless_sdk::{
    HeadlessModelRisk, MODEL_HEADLESS_PLAN_SCHEMA_VERSION, MODEL_RESEARCH_FRONTIER_SCHEMA_VERSION,
    MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION, ModelActionDispatch, ModelActionDispatcher,
    ModelApprovalVerifier, ModelFrontierDigestVerifier, ModelFrontierVerifier, ModelHeadlessPlan,
    ModelHeadlessPlanStep, ModelPlanApproval, ModelReceiptVerifier, ModelResearchExecutionReceipt,
    ModelResearchExecutionRecord, ModelResearchExecutionStatus, ModelResearchFrontier,
    ModelResearchFrontierStage, SdkError, SdkResult, advance_model_research_frontier,
    build_model_research_frontier_proposal, compute_model_research_frontier_digest,
    execute_model_headless_plan, start_model_research_frontier,
};
use serde_json::{Value, json};
use std::path::PathBuf;

struct TestReceiptVerifier(bool);
struct TestFrontierVerifier(bool);

impl ModelFrontierVerifier for TestFrontierVerifier {
    fn verify_model_frontier(&self, _frontier: &ModelResearchFrontier) -> SdkResult<()> {
        if self.0 {
            Ok(())
        } else {
            Err(SdkError::Validation {
                errors: vec!["caller frontier verifier rejected frontier".to_string()],
            })
        }
    }
}

struct AllowApprovalVerifier;

impl ModelApprovalVerifier for AllowApprovalVerifier {
    fn verify_model_approval(
        &self,
        _plan: &ModelHeadlessPlan,
        _approval: &ModelPlanApproval,
    ) -> SdkResult<()> {
        Ok(())
    }
}

struct WaitDispatcher;

impl ModelActionDispatcher for WaitDispatcher {
    fn dispatch_model_action(
        &self,
        _action: &str,
        _payload: &Value,
    ) -> SdkResult<ModelActionDispatch> {
        Ok(ModelActionDispatch {
            authority: "control_plane".to_string(),
            output: json!({"terminal": {"job": {"status": "completed"}}}),
        })
    }
}

impl ModelReceiptVerifier for TestReceiptVerifier {
    fn verify_model_receipt(&self, _receipt: &ModelResearchExecutionReceipt) -> SdkResult<()> {
        if self.0 {
            Ok(())
        } else {
            Err(SdkError::Validation {
                errors: vec!["caller receipt verifier rejected receipt".to_string()],
            })
        }
    }
}

#[test]
fn verified_submission_binds_real_job_id_into_next_proposal() {
    let receipt = receipt(
        "workflow_submit_catalog",
        None,
        json!({"job": {"job_id": "job-real-001", "status": "queued"}}),
        ModelResearchExecutionStatus::Completed,
        None,
    );
    let frontier =
        start_model_research_frontier(&receipt, &TestReceiptVerifier(true)).expect("frontier");
    assert_eq!(frontier.stage, ModelResearchFrontierStage::WaitingForJob);
    assert_eq!(frontier.job_id.as_deref(), Some("job-real-001"));
    assert_eq!(frontier.origin_plan_digest, receipt.plan_digest);
    assert_eq!(frontier.evidence.plan_digest, receipt.plan_digest);

    let proposal = build_model_research_frontier_proposal(&frontier, &TestFrontierVerifier(true))
        .expect("proposal");
    assert_eq!(proposal.calls[0].action, "job_wait");
    assert_eq!(proposal.calls[0].payload["job_id"], "job-real-001");
}

#[test]
fn unverified_receipt_cannot_create_frontier() {
    let receipt = receipt(
        "workflow_submit_catalog",
        None,
        json!({"job": {"job_id": "job-real-001"}}),
        ModelResearchExecutionStatus::Completed,
        None,
    );
    let error = start_model_research_frontier(&receipt, &TestReceiptVerifier(false))
        .expect_err("verifier gate");
    assert!(error.to_string().contains("receipt verifier rejected"));
}

#[test]
fn wait_and_fetch_advance_to_validation_without_guessing_ids() {
    let submitted = receipt(
        "workflow_submit_graph",
        None,
        json!({"job": {"job_id": "job-real-002"}}),
        ModelResearchExecutionStatus::Completed,
        None,
    );
    let waiting =
        start_model_research_frontier(&submitted, &TestReceiptVerifier(true)).expect("waiting");
    let mut waited = receipt(
        "job_wait",
        Some("job-real-002"),
        json!({"terminal": {"job": {"job_id": "job-real-002", "status": "completed"}}, "history": []}),
        ModelResearchExecutionStatus::Completed,
        None,
    );
    waited.plan_digest = digest('1');
    let fetch = advance_model_research_frontier(
        &waiting,
        &waited,
        &TestFrontierVerifier(true),
        &TestReceiptVerifier(true),
    )
    .expect("fetch frontier");
    assert_eq!(fetch.stage, ModelResearchFrontierStage::ReadyToFetchResult);
    assert_eq!(fetch.origin_plan_digest, digest('0'));
    assert_eq!(fetch.evidence.plan_digest, digest('1'));
    let proposal = build_model_research_frontier_proposal(&fetch, &TestFrontierVerifier(true))
        .expect("fetch proposal");
    assert_eq!(proposal.calls[0].action, "result_fetch");
    assert_eq!(proposal.calls[0].payload["job_id"], "job-real-002");

    let mut result = receipt(
        "result_fetch",
        Some("job-real-002"),
        json!({"result": {"artifacts": []}}),
        ModelResearchExecutionStatus::Completed,
        None,
    );
    result.plan_digest = digest('2');
    let validate = advance_model_research_frontier(
        &fetch,
        &result,
        &TestFrontierVerifier(true),
        &TestReceiptVerifier(true),
    )
    .expect("validation frontier");
    assert_eq!(validate.stage, ModelResearchFrontierStage::ReadyToValidate);
    assert!(validate.next_action.is_none());
    assert_eq!(validate.origin_plan_digest, digest('0'));
    assert_eq!(validate.evidence.plan_digest, digest('2'));
}

#[test]
fn malformed_plan_digest_cannot_enter_frontier_chain() {
    let mut submitted = receipt(
        "workflow_submit_catalog",
        None,
        json!({"job": {"job_id": "job-real-005"}}),
        ModelResearchExecutionStatus::Completed,
        None,
    );
    submitted.plan_digest = "sha256:NOT-A-DIGEST".to_string();
    let error = start_model_research_frontier(&submitted, &TestReceiptVerifier(true))
        .expect_err("digest shape");
    assert!(error.to_string().contains("receipt"));
}

#[test]
fn mismatched_job_binding_is_rejected() {
    let submitted = receipt(
        "fem_submit",
        None,
        json!({"job": {"job_id": "job-real-003"}}),
        ModelResearchExecutionStatus::Completed,
        None,
    );
    let waiting =
        start_model_research_frontier(&submitted, &TestReceiptVerifier(true)).expect("waiting");
    let wrong = receipt(
        "job_wait",
        Some("job-guessed"),
        json!({"terminal": {"job": {"status": "completed"}}}),
        ModelResearchExecutionStatus::Completed,
        None,
    );
    let error = advance_model_research_frontier(
        &waiting,
        &wrong,
        &TestFrontierVerifier(true),
        &TestReceiptVerifier(true),
    )
    .expect_err("job binding");
    assert!(error.to_string().contains("job_id does not match"));
}

#[test]
fn terminal_failure_and_execution_failure_block_progression() {
    let submitted = receipt(
        "workflow_submit_catalog",
        None,
        json!({"job": {"job_id": "job-real-004"}}),
        ModelResearchExecutionStatus::Completed,
        None,
    );
    let waiting =
        start_model_research_frontier(&submitted, &TestReceiptVerifier(true)).expect("waiting");
    let failed_job = receipt(
        "job_wait",
        Some("job-real-004"),
        json!({"terminal": {"job": {"status": "failed"}}}),
        ModelResearchExecutionStatus::Completed,
        None,
    );
    let blocked = advance_model_research_frontier(
        &waiting,
        &failed_job,
        &TestFrontierVerifier(true),
        &TestReceiptVerifier(true),
    )
    .expect("blocked");
    assert_eq!(blocked.stage, ModelResearchFrontierStage::Blocked);
    assert_eq!(
        blocked.blocking_reason.as_deref(),
        Some("job reached terminal status failed")
    );

    let dispatch_failed = receipt(
        "workflow_submit_catalog",
        None,
        Value::Null,
        ModelResearchExecutionStatus::Failed,
        Some("control plane unavailable"),
    );
    let blocked = start_model_research_frontier(&dispatch_failed, &TestReceiptVerifier(true))
        .expect("failed initial frontier");
    assert_eq!(blocked.stage, ModelResearchFrontierStage::Blocked);
    assert_eq!(
        blocked.blocking_reason.as_deref(),
        Some("control plane unavailable")
    );
}

#[test]
fn repository_frontier_fixture_matches_sdk_contract() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture =
        std::fs::read_to_string(root.join("schemas/examples.model-research-frontier.json"))
            .expect("fixture");
    let frontier: ModelResearchFrontier = serde_json::from_str(&fixture).expect("parse fixture");
    assert_eq!(
        frontier.schema_version,
        MODEL_RESEARCH_FRONTIER_SCHEMA_VERSION
    );
    let expected_digest = "sha256:aba8f2d4289d4385f07fcb065f65b26a71d7c606397dd9d20700d604b9b25902";
    assert_eq!(
        compute_model_research_frontier_digest(&frontier).expect("frontier digest"),
        expected_digest
    );
    let verifier = ModelFrontierDigestVerifier::new(expected_digest).expect("digest verifier");
    let proposal =
        build_model_research_frontier_proposal(&frontier, &verifier).expect("verified proposal");
    let mut changed = frontier.clone();
    changed.transition_count += 1;
    assert_ne!(
        compute_model_research_frontier_digest(&changed).expect("changed digest"),
        expected_digest
    );
    assert!(build_model_research_frontier_proposal(&changed, &verifier).is_err());
    changed.evidence.action = "ResultFetch".to_string();
    assert!(compute_model_research_frontier_digest(&changed).is_err());
    assert!(ModelFrontierDigestVerifier::new("sha256:not-valid").is_err());
    assert_eq!(
        proposal.calls[0].payload["job_id"],
        "job-material-envelope-001"
    );
}

#[test]
fn inconsistent_frontier_state_is_rejected() {
    let submitted = receipt(
        "workflow_submit_catalog",
        None,
        json!({"job": {"job_id": "job-real-006"}}),
        ModelResearchExecutionStatus::Completed,
        None,
    );
    let mut frontier =
        start_model_research_frontier(&submitted, &TestReceiptVerifier(true)).expect("frontier");
    frontier.next_action = Some("result_fetch".to_string());
    let error = build_model_research_frontier_proposal(&frontier, &TestFrontierVerifier(true))
        .expect_err("state mismatch");
    assert!(error.to_string().contains("stage and next action"));
}

#[test]
fn unverified_frontier_cannot_generate_or_advance() {
    let submitted = receipt(
        "workflow_submit_catalog",
        None,
        json!({"job": {"job_id": "job-real-007"}}),
        ModelResearchExecutionStatus::Completed,
        None,
    );
    let frontier =
        start_model_research_frontier(&submitted, &TestReceiptVerifier(true)).expect("frontier");
    let error = build_model_research_frontier_proposal(&frontier, &TestFrontierVerifier(false))
        .expect_err("frontier verifier");
    assert!(error.to_string().contains("frontier verifier rejected"));
}

#[test]
fn execution_receipt_retains_narrow_job_binding() {
    let plan = ModelHeadlessPlan {
        schema_version: MODEL_HEADLESS_PLAN_SCHEMA_VERSION.to_string(),
        session_id: "research-session".to_string(),
        workflow_id: "workflow.material".to_string(),
        ok: true,
        ready_without_confirmation: true,
        issues: vec![],
        steps: vec![ModelHeadlessPlanStep {
            index: 1,
            action: "job_wait".to_string(),
            category: Some("observation".to_string()),
            risk: HeadlessModelRisk::Normal,
            payload: json!({"job_id": "job-bound-001"}),
            requires_confirmation: false,
            confirmation_reason: None,
            output_keys: vec!["job".to_string()],
        }],
    };
    let receipt = execute_model_headless_plan(&WaitDispatcher, &plan, None, &AllowApprovalVerifier)
        .expect("receipt");
    assert_eq!(receipt.records[0].job_id.as_deref(), Some("job-bound-001"));
}

fn receipt(
    action: &str,
    job_id: Option<&str>,
    output: Value,
    status: ModelResearchExecutionStatus,
    error: Option<&str>,
) -> ModelResearchExecutionReceipt {
    ModelResearchExecutionReceipt {
        schema_version: MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION.to_string(),
        plan_schema_version: MODEL_HEADLESS_PLAN_SCHEMA_VERSION.to_string(),
        session_id: "research-session".to_string(),
        workflow_id: "workflow.material".to_string(),
        plan_digest: digest('0'),
        status,
        execution_authority: "kyuubiki-headless-sdk".to_string(),
        approval_id: Some("approval-test".to_string()),
        completed_steps: usize::from(error.is_none()),
        failed_step: error.map(|_| 1),
        records: vec![ModelResearchExecutionRecord {
            index: 1,
            action: action.to_string(),
            job_id: job_id.map(str::to_string),
            authority: error.is_none().then(|| "control_plane".to_string()),
            output: error.is_none().then_some(output),
            error: error.map(str::to_string),
        }],
    }
}

fn digest(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
}
