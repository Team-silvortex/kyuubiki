use kyuubiki_headless_sdk::{
    MODEL_HEADLESS_PLAN_SCHEMA_VERSION, MODEL_RESEARCH_FRONTIER_SCHEMA_VERSION,
    MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION, MaterialResearchBundle, ModelFrontierDigestVerifier,
    ModelFrontierVerifier, ModelReceiptVerifier, ModelResearchExecutionReceipt,
    ModelResearchExecutionRecord, ModelResearchExecutionStatus, ModelResearchFrontier,
    ModelResearchFrontierEvidence, ModelResearchFrontierStage, ModelResearchValidationStage,
    SdkError, SdkResult, WorkflowGraphDefinition, compute_model_research_frontier_digest,
    validate_model_research_frontier_result,
};
use serde_json::{Value, json};

struct Verifier(bool);

impl ModelFrontierVerifier for Verifier {
    fn verify_model_frontier(&self, _frontier: &ModelResearchFrontier) -> SdkResult<()> {
        verdict(self.0)
    }
}

impl ModelReceiptVerifier for Verifier {
    fn verify_model_receipt(&self, _receipt: &ModelResearchExecutionReceipt) -> SdkResult<()> {
        verdict(self.0)
    }
}

#[test]
fn validates_bound_workflow_result_without_overclaiming() {
    let report = validate_model_research_frontier_result(
        &frontier(),
        &receipt(result_payload("completed"), "job-validation-001"),
        &graph(),
        None,
        &Verifier(true),
        &Verifier(true),
    )
    .expect("validation report");

    assert_eq!(
        report.stage,
        ModelResearchValidationStage::WorkflowResultValidated
    );
    assert_eq!(report.claim_boundary, "screening_only_not_qualification");
    assert!(report.external_validation_required);
    assert_eq!(report.origin_plan_digest, digest('0'));
    assert_eq!(report.result_plan_digest, digest('0'));
    assert_eq!(
        report.workflow_result.artifact_keys,
        ["thermo_summary.result"]
    );
}

#[test]
fn validates_retained_screening_bundle() {
    let bundle: MaterialResearchBundle = serde_json::from_str(include_str!(
        "../../../schemas/examples.material-research-bundle.json"
    ))
    .expect("bundle fixture");
    let report = validate_model_research_frontier_result(
        &frontier(),
        &receipt(result_payload("completed"), "job-validation-001"),
        &graph(),
        Some(&bundle),
        &Verifier(true),
        &Verifier(true),
    )
    .expect("screening report");

    assert_eq!(
        report.stage,
        ModelResearchValidationStage::ScreeningBundleValidated
    );
    assert_eq!(
        report.material_bundle.expect("bundle evidence").bundle_id,
        bundle.bundle_id
    );
    assert!(
        report
            .next_actions
            .contains(&"external_validation_required".to_string())
    );
}

#[test]
fn digest_verifier_reaches_result_validation() {
    let frontier = frontier();
    let frontier_digest =
        compute_model_research_frontier_digest(&frontier).expect("frontier digest");
    let verifier = ModelFrontierDigestVerifier::new(frontier_digest).expect("digest verifier");
    let report = validate_model_research_frontier_result(
        &frontier,
        &receipt(result_payload("completed"), "job-validation-001"),
        &graph(),
        None,
        &verifier,
        &Verifier(true),
    )
    .expect("digest-verified validation report");
    assert_eq!(report.origin_plan_digest, digest('0'));
}

#[test]
fn rejects_wrong_job_or_unverified_evidence() {
    let error = validate_model_research_frontier_result(
        &frontier(),
        &receipt(result_payload("completed"), "job-guessed"),
        &graph(),
        None,
        &Verifier(true),
        &Verifier(true),
    )
    .expect_err("wrong job binding");
    assert!(error.to_string().contains("does not match"));

    let error = validate_model_research_frontier_result(
        &frontier(),
        &receipt(result_payload("completed"), "job-validation-001"),
        &graph(),
        None,
        &Verifier(false),
        &Verifier(true),
    )
    .expect_err("frontier verifier");
    assert!(error.to_string().contains("caller verifier rejected"));
}

#[test]
fn rejects_non_completed_runtime() {
    let error = validate_model_research_frontier_result(
        &frontier(),
        &receipt(result_payload("running"), "job-validation-001"),
        &graph(),
        None,
        &Verifier(true),
        &Verifier(true),
    )
    .expect_err("running result");
    assert!(error.to_string().contains("status must be completed"));
}

#[test]
fn rejects_result_receipt_from_another_verified_plan() {
    let mut receipt = receipt(result_payload("completed"), "job-validation-001");
    receipt.plan_digest = digest('1');
    let error = validate_model_research_frontier_result(
        &frontier(),
        &receipt,
        &graph(),
        None,
        &Verifier(true),
        &Verifier(true),
    )
    .expect_err("plan digest mismatch");
    assert!(error.to_string().contains("does not match"));
}

fn frontier() -> ModelResearchFrontier {
    ModelResearchFrontier {
        schema_version: MODEL_RESEARCH_FRONTIER_SCHEMA_VERSION.to_string(),
        session_id: "research-session".to_string(),
        workflow_id: "workflow.heat-to-thermo-quad-2d".to_string(),
        origin_plan_digest: digest('0'),
        stage: ModelResearchFrontierStage::ReadyToValidate,
        job_id: Some("job-validation-001".to_string()),
        next_action: None,
        transition_count: 3,
        evidence: ModelResearchFrontierEvidence {
            plan_digest: digest('0'),
            approval_id: Some("approval-test".to_string()),
            action: "result_fetch".to_string(),
            record_index: 1,
            authority: Some("control_plane".to_string()),
            job_status: None,
        },
        blocking_reason: None,
    }
}

fn graph() -> WorkflowGraphDefinition {
    serde_json::from_str(include_str!(
        "../../../schemas/examples.workflow-graph.json"
    ))
    .expect("graph fixture")
}

fn result_payload(status: &str) -> Value {
    json!({
        "result": {
            "workflow_id": "workflow.heat-to-thermo-quad-2d",
            "run_id": "run-validation-001",
            "status": status,
            "artifacts": {
                "result/thermal_plane_quad_2d": {
                    "artifact_id": "artifact.thermo.result",
                    "artifact_type": "result/thermal_plane_quad_2d",
                    "dataset_value": "thermo_result"
                }
            }
        }
    })
}

fn receipt(output: Value, job_id: &str) -> ModelResearchExecutionReceipt {
    ModelResearchExecutionReceipt {
        schema_version: MODEL_RESEARCH_RECEIPT_SCHEMA_VERSION.to_string(),
        plan_schema_version: MODEL_HEADLESS_PLAN_SCHEMA_VERSION.to_string(),
        session_id: "research-session".to_string(),
        workflow_id: "workflow.heat-to-thermo-quad-2d".to_string(),
        plan_digest: digest('0'),
        status: ModelResearchExecutionStatus::Completed,
        execution_authority: "kyuubiki-headless-sdk".to_string(),
        approval_id: Some("approval-test".to_string()),
        completed_steps: 1,
        failed_step: None,
        records: vec![ModelResearchExecutionRecord {
            index: 1,
            action: "result_fetch".to_string(),
            job_id: Some(job_id.to_string()),
            authority: Some("control_plane".to_string()),
            output: Some(output),
            error: None,
        }],
    }
}

fn digest(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
}

fn verdict(allow: bool) -> SdkResult<()> {
    if allow {
        Ok(())
    } else {
        Err(SdkError::Validation {
            errors: vec!["caller verifier rejected evidence".to_string()],
        })
    }
}
